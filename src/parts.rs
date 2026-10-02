use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Added, Changed, Commands, Component, Entity, Has, Or, Query, ResMut, With, Without,
};
use plurimus_core::ratatui_core::layout::{Position, Rect, Size};
use plurimus_core::{CameraViewports, ComputedUiCamera, UiArea, UiOrder, local_area};
use plurimus_ui::bevy_input_focus::{FocusCause, InputFocus};
use plurimus_ui::{ComputedWidgetArea, Key, ScrollArea, ScrollOffset, apply_offset};
use plurimus_widgets::{
    ActiveDescendant, ListBox, ListBoxAction, ListBoxCursor, ListBoxKeys, ListBoxSelectionMarker,
    ListBoxStripe, listbox,
};

use crate::layout::split_area;
use crate::picker::FilePicker;

/// The picker's child list, which holds focus and the tab stop for it.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct PickerList(pub Entity);

/// Spawns the list under a new picker.
pub(crate) fn install_file_picker_list(
    pickers: Query<Entity, Added<FilePicker>>,
    mut commands: Commands,
) {
    for picker in &pickers {
        // Enter is the picker's, and space, Home and End are the field's.
        let list_keys = ListBoxKeys(vec![
            (Key::ArrowUp.into(), ListBoxAction::Up),
            (Key::ArrowDown.into(), ListBoxAction::Down),
            (Key::PageUp.into(), ListBoxAction::PageUp),
            (Key::PageDown.into(), ListBoxAction::PageDown),
        ]);
        let list = commands
            .spawn((
                listbox(),
                list_keys,
                ScrollArea::new(Size::ZERO),
                UiArea::Fixed(Rect::ZERO),
                UiOrder(0),
                ChildOf(picker),
            ))
            .id();
        commands.entity(picker).insert(PickerList(list));
    }
}

type Pickers<'w, 's> = Query<
    'w,
    's,
    (
        Ref<'static, ComputedWidgetArea>,
        &'static ComputedUiCamera,
        Option<Ref<'static, UiOrder>>,
        Ref<'static, PickerList>,
    ),
    (With<FilePicker>, Without<ListBox>),
>;

type Lists<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut UiArea,
        &'static mut ComputedWidgetArea,
        &'static mut UiOrder,
        &'static mut ScrollOffset,
        &'static mut ActiveDescendant,
    ),
    (With<ListBox>, Without<FilePicker>),
>;

/// Places the list beneath the field row, one order above the picker so it
/// paints over the rest of the picker's area.
pub(crate) fn place_file_picker_parts(
    cameras: CameraViewports,
    pickers: Pickers,
    mut lists: Lists,
    mut commands: Commands,
) {
    for (picker_area, camera, order, list) in &pickers {
        let moved = picker_area.is_changed()
            || list.is_added()
            || order.as_ref().is_some_and(Ref::is_changed);
        if !moved {
            continue;
        }
        let Ok((mut area, mut computed, mut list_order, mut offset, mut cursor)) =
            lists.get_mut(list.0)
        else {
            continue;
        };
        let (_, rect) = split_area(picker_area.0);
        // The engine reveals the cursor into a list with no area and leaves
        // the offset past it; the reveal has to run again once there is one.
        if computed.0.is_empty() && !rect.is_empty() {
            apply_offset(list.0, Position::ORIGIN, &mut offset, &mut commands);
            cursor.set_changed();
        }
        computed.set_if_neq(ComputedWidgetArea(rect));
        let local = cameras
            .of(camera.0)
            .map_or(rect, |viewport| local_area(rect, viewport));
        area.set_if_neq(UiArea::Fixed(local));
        list_order.set_if_neq(UiOrder(order.map_or(0, |order| order.0).saturating_add(1)));
    }
}

type LookChanged = Or<(
    Added<PickerList>,
    Changed<ListBoxCursor>,
    Changed<ListBoxStripe>,
    Changed<ListBoxSelectionMarker>,
)>;

/// A removal is not copied: the engine does not repaint on one either.
pub(crate) fn mirror_look(
    pickers: Query<
        (
            &PickerList,
            Option<&ListBoxCursor>,
            Option<&ListBoxStripe>,
            Has<ListBoxSelectionMarker>,
        ),
        LookChanged,
    >,
    mut commands: Commands,
) {
    for (list, cursor, stripe, has_marker) in &pickers {
        let mut list = commands.entity(list.0);
        if let Some(cursor) = cursor {
            list.insert(cursor.clone());
        }
        if let Some(&stripe) = stripe {
            list.insert(stripe);
        }
        list.insert_if(ListBoxSelectionMarker, || has_marker);
    }
}

/// Focus given to a picker lands on its list, where the engine's list keys
/// act.
pub(crate) fn redirect_focus(mut focus: ResMut<InputFocus>, lists: Query<&PickerList>) {
    if !focus.is_changed() {
        return;
    }
    let Some(list) = focus.get().and_then(|focused| lists.get(focused).ok()) else {
        return;
    };
    focus.set(list.0, FocusCause::Navigated);
}
