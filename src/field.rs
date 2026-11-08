use std::borrow::Cow;

use bevy_ecs::change_detection::{DetectChanges, Mut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{Entity, On, Query, Without};
use bevy_ecs::system::SystemParam;
use bevy_input::keyboard::KeyboardInput;
use bevy_text::{EditableText, TextEdit};
use plurimus_term::HeldModifiers;
use plurimus_ui::bevy_input_focus::FocusedInput;
use plurimus_ui::{ComputedDisabled, first_bound};
use plurimus_widgets::TextInput;

use crate::keys::{FilePickerAction, FilePickerKeys};

/// The field's text, wherever its host keeps it: in `EditableText` when the
/// host dressed the field as upstream's text input, which is then the text
/// both a window and plurimus's node bridge treat as the truth, and in the
/// engine's `TextInput` otherwise.
#[derive(SystemParam)]
pub(crate) struct FieldText<'w, 's> {
    fields: Query<
        'w,
        's,
        (
            Option<Mut<'static, EditableText>>,
            Option<Mut<'static, TextInput>>,
        ),
    >,
}

impl FieldText<'_, '_> {
    pub(crate) fn value(&self, field: Entity) -> Option<Cow<'_, str>> {
        match self.fields.get(field).ok()? {
            (Some(editor), _) => {
                // Split around an IME preedit, so whole in `head` unless composing.
                let mut halves = editor.into_inner().value().into_iter();
                let head = halves.next().unwrap_or_default();
                let tail = halves.next().unwrap_or_default();
                Some(if tail.is_empty() {
                    Cow::Borrowed(head)
                } else {
                    Cow::Owned(format!("{head}{tail}"))
                })
            }
            (None, Some(input)) => Some(Cow::Borrowed(input.into_inner().value())),
            (None, None) => None,
        }
    }

    pub(crate) fn is_changed(&self, field: Entity) -> bool {
        match self.fields.get(field) {
            Ok((Some(editor), _)) => editor.is_changed(),
            Ok((None, Some(input))) => input.is_changed(),
            _ => false,
        }
    }

    /// Replaces the text, caret at its end, which ends any selection.
    pub(crate) fn set(&mut self, field: Entity, text: String) {
        match self.fields.get_mut(field) {
            Ok((Some(mut editor), _)) => {
                editor.editor_mut().set_text(&text);
                editor.queue_edit(TextEdit::TextEnd(false));
            }
            Ok((None, Some(mut input))) => *input = TextInput::new(text),
            _ => {}
        }
    }
}

/// Upstream's field takes Up and Down for its caret even in one row. Observed
/// on the field, so after upstream's global observer: a list step the picker
/// binds drops the field's vertical edit and bubbles on to the picker.
pub(crate) fn reclaim_list_step(
    mut input: On<FocusedInput<KeyboardInput>>,
    held: HeldModifiers,
    mut fields: Query<(&ChildOf, &mut EditableText)>,
    pickers: Query<&FilePickerKeys, Without<ComputedDisabled>>,
) {
    let Ok((parent, mut editor)) = fields.get_mut(input.focused_entity) else {
        return;
    };
    let Ok(keys) = pickers.get(parent.parent()) else {
        return;
    };
    let is_list_step =
        first_bound(&keys.0, &input.input, held.get()).is_some_and(FilePickerAction::is_list_step);
    if !is_list_step || editor.is_composing() {
        return;
    }
    if matches!(
        editor.pending_edits.last(),
        Some(TextEdit::Up(_) | TextEdit::Down(_))
    ) {
        editor.pending_edits.pop();
    }
    input.propagate(true);
}
