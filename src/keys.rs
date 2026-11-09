use std::path::MAIN_SEPARATOR_STR;

use bevy_ecs::event::EntityEvent;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Commands, Component, Entity, On, Query, ResMut, With, Without};
use bevy_ecs::system::SystemParam;
use bevy_input::keyboard::KeyboardInput;
use plurimus_term::HeldModifiers;
use plurimus_ui::bevy_input_focus::{FocusCause, FocusedInput, InputFocus};
use plurimus_ui::bevy_picking::events::{PointerClick, PointerPress};
use plurimus_ui::bevy_picking::pointer::PointerButton;
use plurimus_ui::{
    ComputedDisabled, ComputedWidgetArea, Key, KeyBinding, ModalDismiss, PressFocusDisabled,
    ValueChange, first_bound,
};
use plurimus_widgets::ActiveDescendant;

use crate::field::FieldText;
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
    /// Moves the list's cursor up a row, from the first to the last.
    Up,
    /// Moves the list's cursor down a row, from the last to the first.
    Down,
    /// Moves the list's cursor up by the list's height, stopping at the
    /// first row.
    PageUp,
    /// Moves the list's cursor down by the list's height, stopping at the
    /// last row.
    PageDown,
}

impl FilePickerAction {
    pub(crate) const fn is_list_step(self) -> bool {
        matches!(self, Self::Up | Self::Down | Self::PageUp | Self::PageDown)
    }
}

/// The picker's key bindings, scanned first to last.
///
/// A key reaches the picker by bubbling from its focused field, which took
/// its turn first with the picker's `TextInputKeys` less every key bound
/// here, so a key bound in both is the picker's: by default plain `Left`
/// climbs and `Right` completes, while their shifted forms select. What
/// these leave bubbles on to the picker's ancestors, and so does a
/// [`FilePickerAction::Up`], `Down`, `PageUp` or `PageDown` that moved
/// nothing: one row, or a page key at the end it moves towards.
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
            (Key::ArrowUp.into(), FilePickerAction::Up),
            (Key::ArrowDown.into(), FilePickerAction::Down),
            (Key::PageUp.into(), FilePickerAction::PageUp),
            (Key::PageDown.into(), FilePickerAction::PageDown),
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
    fields: FieldText<'w, 's>,
    lists: Query<
        'w,
        's,
        (
            &'static ComputedWidgetArea,
            Option<&'static Children>,
            &'static ActiveDescendant,
        ),
    >,
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
            FilePickerAction::Up
            | FilePickerAction::Down
            | FilePickerAction::PageUp
            | FilePickerAction::PageDown => {
                self.step_cursor(picker, action);
            }
        }
    }

    /// Moves the list's cursor as a list `action` does, answering whether
    /// it moved. A cursor on no row starts from the first.
    fn step_cursor(&mut self, picker: Entity, action: FilePickerAction) -> bool {
        let Ok((_, parts, ..)) = self.pickers.get(picker) else {
            return false;
        };
        let list = parts.list;
        let Ok((area, Some(rows), cursor)) = self.lists.get(list) else {
            return false;
        };
        let Some(last_row) = rows.len().checked_sub(1) else {
            return false;
        };
        let current = cursor
            .0
            .and_then(|cursor| rows.iter().position(|&row| row == cursor));
        let page = usize::from(area.0.height).max(1);
        let next = match (action, current) {
            (FilePickerAction::Up, Some(0)) => last_row,
            (FilePickerAction::Up, Some(index)) => index - 1,
            (FilePickerAction::Down, Some(index)) if index < last_row => index + 1,
            (FilePickerAction::PageUp, Some(index)) => index.saturating_sub(page),
            (FilePickerAction::PageDown, Some(index)) => index.saturating_add(page).min(last_row),
            _ => 0,
        };
        if current == Some(next) {
            return false;
        }
        self.commands
            .entity(list)
            .insert(ActiveDescendant(Some(rows[next])));
        true
    }

    fn cursor_entry(&self, picker: Entity) -> Option<Entry> {
        let (_, parts, ..) = self.pickers.get(picker).ok()?;
        let row = self.lists.get(parts.list).ok()?.2.0?;
        self.entries.get(row).ok().cloned()
    }

    fn go_to_parent(&mut self, picker: Entity) {
        let Ok((state, parts, _, listing)) = self.pickers.get(picker) else {
            return;
        };
        if !listing.has_parent {
            return;
        }
        let Some(text) = self.fields.value(parts.field) else {
            return;
        };
        let parent = normalize(
            &state
                .resolved_directory(&text)
                .join(std::path::Component::ParentDir),
        );
        self.fields
            .set(parts.field, directory_text(&parent, state.base()));
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
        let Some(text) = self.fields.value(parts.field) else {
            return;
        };
        let chosen = state.resolved_directory(&text).join(&entry.name);
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
        let Some(text) = self.fields.value(parts.field) else {
            return;
        };
        let suffix = if entry.is_dir { MAIN_SEPARATOR_STR } else { "" };
        let completed = format!("{}{}{suffix}", split_at_separator(&text).0, entry.name);
        self.fields.set(parts.field, completed);
    }
}

/// The picker's bindings, reached by bubbling from the focused field, which
/// has typed what it took. A list step is applied here rather than through
/// [`FilePickerCommand`], so a step that moved nothing can bubble on.
pub(crate) fn file_picker_key(
    mut input: On<FocusedInput<KeyboardInput>>,
    held: HeldModifiers,
    pickers: Query<&FilePickerKeys, Without<ComputedDisabled>>,
    mut access: PickerAccess,
) {
    let picker = input.focused_entity;
    let Ok(keys) = pickers.get(picker) else {
        return;
    };
    let Some(action) = first_bound(&keys.0, &input.input, held.get()) else {
        return;
    };
    if action.is_list_step() {
        if access.step_cursor(picker, action) {
            input.propagate(false);
        }
        return;
    }
    input.propagate(false);
    // One Enter chooses once, however long the key is held.
    if action != FilePickerAction::Enter || !input.input.repeat {
        access.commands.trigger(FilePickerCommand {
            entity: picker,
            action,
        });
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

/// A primary double click on a row is `Enter` on it.
pub(crate) fn file_picker_click(
    click: On<PointerClick>,
    lists: Query<&ChildOf, With<FilePickerList>>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Primary || click.count < DOUBLE_CLICK {
        return;
    }
    if let Ok(child_of) = lists.get(click.entity) {
        commands.trigger(FilePickerCommand {
            entity: child_of.parent(),
            action: FilePickerAction::Enter,
        });
    }
}
