use std::path::MAIN_SEPARATOR_STR;

use bevy_ecs::event::EntityEvent;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Commands, Component, Entity, On, Query, ResMut, With, Without};
use bevy_ecs::system::SystemParam;
use bevy_input::keyboard::KeyboardInput;
use plurimus_term::KeyModifiers;
use plurimus_term::bevy_compat::HeldModifiers;
use plurimus_ui::bevy_input_focus::{FocusCause, FocusedInput, InputFocus};
use plurimus_ui::{
    Click, ComputedDisabled, Key, KeyBinding, ModalDismiss, PointerPress, PressFocusDisabled,
    ValueChange, first_bound,
};
use plurimus_widgets::{ActiveDescendant, ListBoxAccess, ListBoxKeys, TextInput};

use crate::listing::{Entry, Listing};
use crate::parts::{FilePickerList, FilePickerParts};
use crate::path::{directory_text, normalize, split_at_separator};
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
/// A key reaches the picker by bubbling from its focused field, which took
/// its turn first with the picker's `TextInputKeys` less every key bound
/// here, so a key bound in both is the picker's: by default plain `Left`
/// climbs and `Right` completes, while their shifted forms select. What
/// these leave goes to the list's `Up`, `Down`, `PageUp` and `PageDown`,
/// kept while they move its cursor; one that cannot, on the first row or the
/// last, bubbles on to the picker's ancestors.
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

/// Applies a [`FilePickerAction`] to a picker, as its bound key would.
///
/// The picker's own keys and a double click on its list arrive here too, so
/// a host that sees a gesture the picker does not - a window's double click
/// on a row - triggers this on the picker and gets the same `Enter`. A
/// disabled picker ignores it.
#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct FilePickerCommand {
    /// The picker.
    pub entity: Entity,
    /// What to do to it.
    pub action: FilePickerAction,
}

pub(crate) fn apply_file_picker_command(command: On<FilePickerCommand>, mut access: PickerAccess) {
    access.apply(command.entity, command.action);
}

/// Everything an action reads or writes on a picker.
#[derive(SystemParam)]
pub(crate) struct PickerAccess<'w, 's> {
    pickers: Query<
        'w,
        's,
        (
            &'static FilePicker,
            &'static FilePickerParts,
            &'static mut FilePickerLook,
            &'static Listing,
        ),
        Without<ComputedDisabled>,
    >,
    fields: Query<'w, 's, &'static mut TextInput>,
    cursors: Query<'w, 's, &'static ActiveDescendant>,
    entries: Query<'w, 's, &'static Entry>,
    commands: Commands<'w, 's>,
}

impl PickerAccess<'_, '_> {
    fn apply(&mut self, picker: Entity, action: FilePickerAction) {
        match action {
            FilePickerAction::Parent => self.go_to_parent(picker),
            FilePickerAction::Enter => self.enter(picker),
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
        let (_, parts, ..) = self.pickers.get(picker).ok()?;
        let row = self.cursors.get(parts.list).ok()?.0?;
        self.entries.get(row).ok().cloned()
    }

    fn go_to_parent(&mut self, picker: Entity) {
        let Ok((state, parts, _, listing)) = self.pickers.get(picker) else {
            return;
        };
        if !listing.has_parent {
            return;
        }
        let Ok(mut text) = self.fields.get_mut(parts.field) else {
            return;
        };
        let parent = normalize(
            &state
                .resolved_directory(text.value())
                .join(std::path::Component::ParentDir),
        );
        *text = TextInput::new(directory_text(&parent, state.base()));
    }

    fn enter(&mut self, picker: Entity) {
        let Some(entry) = self.cursor_entry(picker) else {
            return;
        };
        if entry.is_dir {
            self.complete(picker);
            return;
        }
        let Ok((state, parts, ..)) = self.pickers.get(picker) else {
            return;
        };
        let Ok(text) = self.fields.get(parts.field) else {
            return;
        };
        let chosen = state.resolved_directory(text.value()).join(&entry.name);
        self.commands.trigger(ValueChange {
            source: picker,
            value: chosen,
            is_final: true,
        });
    }

    fn complete(&mut self, picker: Entity) {
        let Some(entry) = self.cursor_entry(picker) else {
            return;
        };
        let Ok((_, parts, ..)) = self.pickers.get(picker) else {
            return;
        };
        let Ok(mut text) = self.fields.get_mut(parts.field) else {
            return;
        };
        let suffix = if entry.is_dir { MAIN_SEPARATOR_STR } else { "" };
        let directory = split_at_separator(text.value()).0;
        *text = TextInput::new(format!("{directory}{}{suffix}", entry.name));
    }
}

/// The list's keys and the applier they drive.
#[derive(SystemParam)]
pub(crate) struct ListStep<'w, 's> {
    keys: Query<'w, 's, &'static ListBoxKeys>,
    lists: ListBoxAccess<'w, 's>,
}

impl ListStep<'_, '_> {
    /// Moves the list's cursor as `input` binds, answering whether it moved.
    fn step(&mut self, list: Entity, input: &KeyboardInput, held: KeyModifiers) -> bool {
        let Ok(keys) = self.keys.get(list) else {
            return false;
        };
        first_bound(&keys.0, input, held).is_some_and(|action| self.lists.apply(list, action))
    }
}

/// The picker's bindings first, then the list's for whatever is left.
/// Reached by bubbling from the focused field, which has typed what it took.
pub(crate) fn file_picker_key(
    mut input: On<FocusedInput<KeyboardInput>>,
    held: HeldModifiers,
    pickers: Query<(&FilePickerKeys, &FilePickerParts), Without<ComputedDisabled>>,
    mut list: ListStep,
    mut commands: Commands,
) {
    let picker = input.focused_entity;
    let Ok((keys, parts)) = pickers.get(picker) else {
        return;
    };
    let held = held.get();
    if let Some(action) = first_bound(&keys.0, &input.input, held) {
        input.propagate(false);
        // One Enter chooses once, however long the key is held.
        if action != FilePickerAction::Enter || !input.input.repeat {
            commands.trigger(FilePickerCommand {
                entity: picker,
                action,
            });
        }
        return;
    }
    if list.step(parts.list, &input.input, held) {
        input.propagate(false);
    }
}

/// A press on the picker or its list focuses the field, the picker's
/// `PressFocusDisabled` covering both; a press on the field is the engine's.
pub(crate) fn file_picker_press(
    press: On<PointerPress>,
    lists: Query<&ChildOf, With<FilePickerList>>,
    pickers: Query<&FilePickerParts, Without<PressFocusDisabled>>,
    mut focus: ResMut<InputFocus>,
) {
    let picker = lists
        .get(press.entity)
        .map_or(press.entity, ChildOf::parent);
    if let Ok(parts) = pickers.get(picker) {
        focus.set(parts.field, FocusCause::Pressed);
    }
}

/// A double click on a row is `Enter` on it.
pub(crate) fn file_picker_click(
    click: On<Click>,
    lists: Query<&ChildOf, With<FilePickerList>>,
    mut commands: Commands,
) {
    if click.count < DOUBLE_CLICK {
        return;
    }
    if let Ok(child_of) = lists.get(click.entity) {
        commands.trigger(FilePickerCommand {
            entity: child_of.parent(),
            action: FilePickerAction::Enter,
        });
    }
}
