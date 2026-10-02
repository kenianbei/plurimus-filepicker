use std::path::MAIN_SEPARATOR_STR;

use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, On, Query, ResMut, With, Without};
use bevy_ecs::system::SystemParam;
use bevy_input::keyboard::KeyboardInput;
use plurimus_term::bevy_compat::HeldModifiers;
use plurimus_ui::bevy_input_focus::{FocusCause, FocusedInput, InputFocus};
use plurimus_ui::{
    Click, ComputedDisabled, Key, KeyBinding, ModalDismiss, PointerPress, PressFocusDisabled,
    ValueChange, first_bound,
};
use plurimus_widgets::{ActiveDescendant, ListBox, TextInputKeys};

use crate::listing::{Entry, Listing};
use crate::parts::PickerList;
use crate::path::{directory_text, normalize};
use crate::picker::{FilePicker, FilePickerLook};

const TOGGLE_HIDDEN_CHARACTER: &str = ".";
const DOUBLE_CLICK: u8 = 2;

/// What a bound key does to a picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FilePickerAction {
    /// The field becomes the parent of the listed directory, filter cleared;
    /// nothing at a root or at the floor.
    Parent,
    /// Descends into the cursor's directory, or chooses the cursor's file.
    Enter,
    /// The field becomes the cursor's entry, a directory gaining its `/`.
    Complete,
    /// Flips whether dot-entries are listed.
    ToggleHidden,
    /// Triggers `ModalDismiss` on the picker; the app decides what it means.
    Close,
}

/// The picker's key bindings, scanned first to last.
///
/// The child list takes `Up`, `Down`, `PageUp` and `PageDown` before these
/// are consulted, and the field takes whatever these leave.
#[derive(Component, Debug, Clone)]
pub struct FilePickerKeys(pub Vec<(KeyBinding, FilePickerAction)>);

impl Default for FilePickerKeys {
    fn default() -> Self {
        Self(vec![
            (Key::Enter.into(), FilePickerAction::Enter),
            (Key::Tab.into(), FilePickerAction::Complete),
            (Key::ArrowRight.into(), FilePickerAction::Complete),
            (Key::ArrowLeft.into(), FilePickerAction::Parent),
            (
                KeyBinding::new(Key::Character(TOGGLE_HIDDEN_CHARACTER.into())).with_ctrl(),
                FilePickerAction::ToggleHidden,
            ),
            (Key::Escape.into(), FilePickerAction::Close),
        ])
    }
}

/// Everything an action reads or writes on a picker.
#[derive(SystemParam)]
pub(crate) struct PickerAccess<'w, 's> {
    pickers: Query<
        'w,
        's,
        (
            &'static mut FilePicker,
            &'static FilePickerKeys,
            &'static TextInputKeys,
            &'static PickerList,
            &'static mut FilePickerLook,
            &'static Listing,
        ),
        Without<ComputedDisabled>,
    >,
    cursors: Query<'w, 's, &'static ActiveDescendant>,
    entries: Query<'w, 's, &'static Entry>,
    commands: Commands<'w, 's>,
}

impl PickerAccess<'_, '_> {
    fn apply(&mut self, picker: Entity, action: FilePickerAction, is_repeat: bool) {
        match action {
            FilePickerAction::Parent => self.go_to_parent(picker),
            FilePickerAction::Enter => self.enter(picker, is_repeat),
            FilePickerAction::Complete => self.complete(picker),
            FilePickerAction::ToggleHidden => {
                if let Ok((.., mut look, _)) = self.pickers.get_mut(picker) {
                    look.hidden = !look.hidden;
                }
            }
            FilePickerAction::Close => self.commands.trigger(ModalDismiss { entity: picker }),
        }
    }

    fn cursor_entry(&self, picker: Entity) -> Option<Entry> {
        let (_, _, _, list, ..) = self.pickers.get(picker).ok()?;
        let row = self.cursors.get(list.0).ok()?.0?;
        self.entries.get(row).ok().cloned()
    }

    fn go_to_parent(&mut self, picker: Entity) {
        let Ok((mut state, .., listing)) = self.pickers.get_mut(picker) else {
            return;
        };
        if !listing.has_parent {
            return;
        }
        let parent = normalize(
            &state
                .resolved_directory()
                .join(std::path::Component::ParentDir),
        );
        let text = directory_text(&parent, state.base());
        state.set_path(text);
    }

    fn enter(&mut self, picker: Entity, is_repeat: bool) {
        let Some(entry) = self.cursor_entry(picker) else {
            return;
        };
        if entry.is_dir {
            self.complete(picker);
            return;
        }
        if is_repeat {
            return;
        }
        let Ok((state, ..)) = self.pickers.get(picker) else {
            return;
        };
        let chosen = state.resolved_directory().join(&entry.name);
        self.commands
            .trigger(ValueChange::new(picker, chosen, true));
    }

    fn complete(&mut self, picker: Entity) {
        let Some(entry) = self.cursor_entry(picker) else {
            return;
        };
        let Ok((mut state, ..)) = self.pickers.get_mut(picker) else {
            return;
        };
        let suffix = if entry.is_dir { MAIN_SEPARATOR_STR } else { "" };
        state.replace_filter(&format!("{}{suffix}", entry.name));
    }
}

/// The picker's bindings first, then the field for whatever is left, so an
/// unbound character types. Reached by bubbling from the focused child list.
pub(crate) fn file_picker_key(
    mut input: On<FocusedInput<KeyboardInput>>,
    held: HeldModifiers,
    mut access: PickerAccess,
) {
    let picker = input.focused_entity;
    let Ok((_, keys, ..)) = access.pickers.get(picker) else {
        return;
    };
    let held = held.get();
    if let Some(action) = first_bound(&keys.0, &input.input, held) {
        input.propagate(false);
        access.apply(picker, action, input.input.repeat);
        return;
    }
    let Ok((mut state, _, field_keys, ..)) = access.pickers.get_mut(picker) else {
        return;
    };
    if state
        .bypass_change_detection()
        .field_mut()
        .handle(field_keys, &input.input, held)
    {
        state.set_changed();
        input.propagate(false);
    }
}

/// A press on the field row focuses the picker's list.
pub(crate) fn file_picker_press(
    press: On<PointerPress>,
    lists: Query<&PickerList, Without<PressFocusDisabled>>,
    mut focus: ResMut<InputFocus>,
) {
    if let Ok(list) = lists.get(press.entity) {
        focus.set(list.0, FocusCause::Pressed);
    }
}

/// A double click on a row is `Enter` on it.
pub(crate) fn file_picker_click(
    click: On<Click>,
    lists: Query<&ChildOf, With<ListBox>>,
    mut access: PickerAccess,
) {
    if click.count < DOUBLE_CLICK {
        return;
    }
    if let Ok(child_of) = lists.get(click.entity) {
        access.apply(child_of.parent(), FilePickerAction::Enter, false);
    }
}
