use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut, Ref};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Added, Commands, Component, Entity, Query, RemovedComponents, ResMut, With, Without,
};
use bevy_input_focus::{FocusCause, InputFocus};
use plurimus_core::ratatui_core::layout::{Rect, Size};
use plurimus_core::{CameraViewports, ComputedUiCamera, UiArea, UiOrder, local_area};
use plurimus_ui::{ComputedWidgetArea, InteractionDisabled, Key, ScrollArea};
use plurimus_widgets::{ListBox, ListBoxAction, ListBoxKeys, listbox};

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
    ),
    (With<ListBox>, Without<FilePicker>),
>;

/// Places the list beneath the field row, one order above the picker so it
/// paints over the rest of the picker's area.
pub(crate) fn place_file_picker_parts(
    cameras: CameraViewports,
    pickers: Pickers,
    mut lists: Lists,
) {
    for (picker_area, camera, order, list) in &pickers {
        let moved = picker_area.is_changed()
            || list.is_added()
            || order.as_ref().is_some_and(Ref::is_changed);
        if !moved {
            continue;
        }
        let Ok((mut area, mut computed, mut list_order)) = lists.get_mut(list.0) else {
            continue;
        };
        let (_, rect) = split_area(picker_area.0);
        computed.set_if_neq(ComputedWidgetArea(rect));
        let local = cameras
            .of(camera.0)
            .map_or(rect, |viewport| local_area(rect, viewport));
        area.set_if_neq(UiArea::Fixed(local));
        list_order.set_if_neq(UiOrder(order.map_or(0, |order| order.0).saturating_add(1)));
    }
}

/// The engine does not pass `InteractionDisabled` to children, so the list
/// takes the picker's, and the engine's own key, press and navigation
/// filters do the rest.
pub(crate) fn mirror_disabled(
    disabled: Query<&PickerList, Added<InteractionDisabled>>,
    mut enabled: RemovedComponents<InteractionDisabled>,
    lists: Query<&PickerList>,
    mut commands: Commands,
) {
    for list in &disabled {
        commands.entity(list.0).insert(InteractionDisabled);
    }
    for picker in enabled.read() {
        if let Ok(list) = lists.get(picker) {
            commands.entity(list.0).remove::<InteractionDisabled>();
        }
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
