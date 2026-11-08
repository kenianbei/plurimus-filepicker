use std::borrow::Cow;

use bevy_ecs::change_detection::{DetectChanges, Mut};
use bevy_ecs::prelude::{Entity, Query};
use bevy_ecs::system::SystemParam;
use bevy_text::{EditableText, TextEdit};
use plurimus_widgets::TextInput;

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
            (Some(editor), _) => Some(Cow::Owned(editor.value().to_string())),
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
