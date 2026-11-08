use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Added, Changed, Commands, Component, Entity, Has, Or, Query, ResMut, With, Without,
};
use plurimus_core::ratatui_core::layout::{Position, Rect, Size};
use plurimus_core::{CameraViewports, ComputedUiCamera, UiArea, UiOrder, local_area};
use plurimus_ui::bevy_input_focus::tab_navigation::TabIndex;
use plurimus_ui::bevy_input_focus::{FocusCause, InputFocus};
use plurimus_ui::{
    ComputedWidgetArea, KeyBinding, PressFocusDisabled, ScrollArea, ScrollOffset, apply_offset,
};
use plurimus_widgets::{
    ActiveDescendant, ListBoxCursor, ListBoxSelectionMarker, ListBoxStripe, TextInputKeys, listbox,
    text_field,
};

use crate::keys::FilePickerKeys;
use crate::layout::split_area;
use crate::picker::{FilePicker, FilePickerLook};

/// The picker's two children, spawned in the first `PreUpdate` after it.
#[derive(Component, Debug, Clone, Copy)]
pub struct FilePickerParts {
    /// The path field: a `TextField` the engine edits, which holds the
    /// picker's focus and is its tab stop.
    pub field: Entity,
    /// The `ListBox` of entries, stepped from the field.
    pub list: Entity,
}

/// Marks a picker's field.
///
/// The field carries `TextField`, so the engine's observers edit it
/// wherever `WidgetsPlugin` runs. A host may dress it as upstream's text
/// input instead, in a window or on a plurimus node, by removing
/// `TextField` and adding `bevy_ui_widgets::TextInput`: the picker then
/// reads and writes the field's `EditableText`, which wins over its
/// `TextInput` when both are there. That field takes the arrows, Home and
/// End for its caret, so the picker's arrow bindings never reach the
/// picker; a host rebinds them in [`FilePickerKeys`] or triggers
/// [`FilePickerCommand`].
///
/// [`FilePickerKeys`]: crate::FilePickerKeys
/// [`FilePickerCommand`]: crate::FilePickerCommand
#[derive(Component, Debug, Clone, Copy)]
pub struct FilePickerField;

/// Marks a picker's list.
#[derive(Component, Debug, Clone, Copy)]
pub struct FilePickerList;

/// Spawns the field and the list under a new picker.
pub(crate) fn install_file_picker_parts(
    pickers: Query<(Entity, &FilePicker), Added<FilePicker>>,
    mut commands: Commands,
) {
    for (picker, state) in &pickers {
        let field = commands
            .spawn((
                text_field(state.opened_on()),
                FilePickerField,
                UiArea::Fixed(Rect::ZERO),
                UiOrder(0),
                ChildOf(picker),
            ))
            .id();
        // No `TabIndex` and no focus on a press: the field holds focus and
        // the picker steps the list from it.
        let list = commands
            .spawn((
                listbox(),
                FilePickerList,
                PressFocusDisabled,
                ScrollArea::new(Size::ZERO),
                UiArea::Fixed(Rect::ZERO),
                UiOrder(0),
                ChildOf(picker),
            ))
            .remove::<TabIndex>()
            .id();
        commands
            .entity(picker)
            .insert(FilePickerParts { field, list });
    }
}

type Pickers<'w, 's> = Query<
    'w,
    's,
    (
        Ref<'static, ComputedWidgetArea>,
        &'static ComputedUiCamera,
        Option<Ref<'static, UiOrder>>,
        Ref<'static, FilePickerLook>,
        Ref<'static, FilePickerParts>,
    ),
    With<FilePicker>,
>;

type Parts<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut UiArea,
        &'static mut ComputedWidgetArea,
        &'static mut UiOrder,
        Option<&'static mut ScrollOffset>,
        Option<&'static ActiveDescendant>,
    ),
    Without<FilePicker>,
>;

/// Places the field after the prompt on the first row and the list beneath,
/// one order above the picker so they paint over its area.
pub(crate) fn place_file_picker_parts(
    cameras: CameraViewports,
    pickers: Pickers,
    mut parts: Parts,
    mut commands: Commands,
) {
    for (picker_area, camera, order, look, pair) in &pickers {
        let moved = picker_area.is_changed()
            || pair.is_added()
            || look.is_changed()
            || order.as_ref().is_some_and(Ref::is_changed);
        if !moved {
            continue;
        }
        let order = UiOrder(order.map_or(0, |order| order.0).saturating_add(1));
        let viewport = cameras.of(camera.0);
        let prompt = u16::try_from(look.prompt.width()).unwrap_or(u16::MAX);
        let (field, list) = split_area(picker_area.0, prompt);
        place(&mut parts, pair.field, field, viewport, order);
        // The engine reveals the cursor into a list with no area and leaves
        // the offset past it; the reveal has to run again once there is one.
        if place(&mut parts, pair.list, list, viewport, order)
            && let Ok((.., Some(mut offset), Some(cursor))) = parts.get_mut(pair.list)
        {
            apply_offset(pair.list, Position::ORIGIN, &mut offset, &mut commands);
            commands.entity(pair.list).insert(cursor.clone());
        }
    }
}

/// Writes a part's screen and camera-local areas and its order, answering
/// whether its area went from empty to having cells.
fn place(
    parts: &mut Parts,
    part: Entity,
    rect: Rect,
    viewport: Option<Rect>,
    order: UiOrder,
) -> bool {
    let Ok((mut area, mut computed, mut part_order, ..)) = parts.get_mut(part) else {
        return false;
    };
    let is_revealed = computed.0.is_empty() && !rect.is_empty();
    computed.set_if_neq(ComputedWidgetArea(rect));
    let local = viewport.map_or(rect, |viewport| local_area(rect, viewport));
    area.set_if_neq(UiArea::Fixed(local));
    part_order.set_if_neq(order);
    is_revealed
}

type ListLookChanged = Or<(
    Added<FilePickerParts>,
    Changed<ListBoxCursor>,
    Changed<ListBoxStripe>,
    Changed<ListBoxSelectionMarker>,
)>;

/// Copies the list's look down from the picker. A removal is not copied:
/// the engine does not repaint on one either.
pub(crate) fn mirror_list_look(
    pickers: Query<
        (
            &FilePickerParts,
            Option<&ListBoxCursor>,
            Option<&ListBoxStripe>,
            Has<ListBoxSelectionMarker>,
        ),
        ListLookChanged,
    >,
    mut commands: Commands,
) {
    for (parts, cursor, stripe, has_marker) in &pickers {
        let mut list = commands.entity(parts.list);
        if let Some(cursor) = cursor {
            list.insert(cursor.clone());
        }
        if let Some(&stripe) = stripe {
            list.insert(stripe);
        }
        list.insert_if(ListBoxSelectionMarker, || has_marker);
    }
}

type KeysChanged = Or<(
    Added<FilePickerParts>,
    Changed<TextInputKeys>,
    Changed<FilePickerKeys>,
)>;

/// Gives the field the picker's `TextInputKeys` less every key
/// `FilePickerKeys` binds: the engine's observer on the focused field runs
/// before the picker's, so a key left bound here would never bubble to the
/// picker.
pub(crate) fn mirror_field_keys(
    pickers: Query<(&FilePickerParts, &TextInputKeys, &FilePickerKeys), KeysChanged>,
    mut commands: Commands,
) {
    for (parts, text_keys, picker_keys) in &pickers {
        let is_pickers =
            |binding: &KeyBinding| picker_keys.0.iter().any(|(bound, _)| bound == binding);
        let kept = text_keys
            .0
            .iter()
            .filter(|(binding, _)| !is_pickers(binding))
            .cloned()
            .collect();
        commands.entity(parts.field).insert(TextInputKeys(kept));
    }
}

/// Focus given to a picker lands on its field, where the engine edits.
pub(crate) fn redirect_focus(mut focus: ResMut<InputFocus>, parts: Query<&FilePickerParts>) {
    if !focus.is_changed() {
        return;
    }
    let Some(parts) = focus.get().and_then(|focused| parts.get(focused).ok()) else {
        return;
    };
    focus.set(parts.field, FocusCause::Navigated);
}
