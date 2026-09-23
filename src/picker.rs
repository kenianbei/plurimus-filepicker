use std::path::{Path, PathBuf};

use bevy_ecs::prelude::{Bundle, Component};
use plurimus_core::UiWidget;
use plurimus_core::ratatui_core::style::{Modifier, Style};
use plurimus_core::ratatui_core::text::Line;
use plurimus_ui::{ComputedWidgetArea, Hovered, StylistCache};
use plurimus_widgets::{TextInput, TextInputKeys};

use crate::keys::FilePickerKeys;
use crate::listing::{BuiltRows, Listing};
use crate::path::{field_directory, or_current, split_at_separator};

const DEFAULT_PROMPT: &str = "> ";

/// A path field over a filtered listing of the directory it names.
///
/// The field is the whole state: everything before its last separator is
/// the directory listed, joined to the base when relative; everything after
/// it filters the entries. `..`, `~`, and an absolute path are ordinary
/// text, which is what makes traversal free.
#[derive(Component, Debug, Clone)]
#[require(
    Hovered,
    StylistCache,
    FilePickerKeys,
    TextInputKeys,
    FilePickerLook,
    FilePickerFloor,
    FilePickerMatchStyle,
    ComputedWidgetArea,
    Listing,
    BuiltRows
)]
pub struct FilePicker {
    field: TextInput,
    base: PathBuf,
}

impl FilePicker {
    /// A picker whose relative paths are joined to `base`, opened on it.
    #[must_use]
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self {
            field: TextInput::default(),
            base: base.into(),
        }
    }

    /// The field's text as typed.
    #[must_use]
    pub fn path(&self) -> &str {
        self.field.value()
    }

    /// Replaces the field's text, caret at the end.
    pub fn set_path(&mut self, path: impl Into<String>) {
        self.field = TextInput::new(path);
    }

    /// The directory the list shows: the field before its last separator,
    /// joined to the base when relative, lexically normalized.
    #[must_use]
    pub fn directory(&self) -> PathBuf {
        or_current(self.resolved_directory())
    }

    /// What follows the field's last separator.
    #[must_use]
    pub fn filter(&self) -> &str {
        split_at_separator(self.field.value()).1
    }

    /// [`Self::directory`] with the working directory as the empty path,
    /// the form the floor is compared in.
    pub(crate) fn resolved_directory(&self) -> PathBuf {
        field_directory(self.field.value(), &self.base)
    }

    /// The base relative paths are joined to.
    #[must_use]
    pub fn base(&self) -> &Path {
        &self.base
    }

    pub(crate) const fn field(&self) -> &TextInput {
        &self.field
    }

    pub(crate) const fn field_mut(&mut self) -> &mut TextInput {
        &mut self.field
    }

    /// Replaces what follows the last separator, keeping the directory as
    /// the user typed it.
    pub(crate) fn replace_filter(&mut self, name: &str) {
        let directory = split_at_separator(self.field.value()).0;
        self.field = TextInput::new(format!("{directory}{name}"));
    }
}

/// The picker's look: whether dot-entries are listed, and what precedes the
/// field.
#[derive(Component, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FilePickerLook {
    /// List entries whose name starts with a dot.
    pub hidden: bool,
    /// Drawn before the field's text.
    pub prompt: Line<'static>,
    /// Offer the filter as a new file name when no entry has it exactly.
    pub accepts_new: bool,
    /// List only files with one of these extensions, compared without
    /// case; empty lists every file. Directories always list.
    pub extensions: Vec<String>,
}

impl FilePickerLook {
    /// With dot-entries listed or not.
    #[must_use]
    pub const fn with_hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// With this prompt before the field.
    #[must_use]
    pub fn with_prompt(mut self, prompt: impl Into<Line<'static>>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// With the filter offered as a new file name or not.
    #[must_use]
    pub const fn with_accepts_new(mut self, accepts_new: bool) -> Self {
        self.accepts_new = accepts_new;
        self
    }

    /// With only files of these extensions listed.
    #[must_use]
    pub fn with_extensions<E: Into<String>>(
        mut self,
        extensions: impl IntoIterator<Item = E>,
    ) -> Self {
        self.extensions = extensions.into_iter().map(Into::into).collect();
        self
    }
}

impl Default for FilePickerLook {
    fn default() -> Self {
        Self {
            hidden: false,
            prompt: Line::from(DEFAULT_PROMPT),
            accepts_new: false,
            extensions: Vec::new(),
        }
    }
}

/// The directory the picker cannot list above; `None` is unbounded.
///
/// A field naming a directory outside the floor is written back to the
/// floor. The check is lexical: a symlink below the floor can still lead
/// out.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct FilePickerFloor(pub Option<PathBuf>);

/// The style laid over the characters of a name the filter matched.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilePickerMatchStyle(pub Style);

impl Default for FilePickerMatchStyle {
    fn default() -> Self {
        Self(Style::new().add_modifier(Modifier::BOLD))
    }
}

/// A file picker opened on `base`, drawn like any other widget.
///
/// Its list is a child spawned on the next frame; focusing the picker lands
/// on that list, which is the tab stop.
#[must_use]
pub fn file_picker(base: impl Into<PathBuf>) -> impl Bundle {
    (FilePicker::new(base), UiWidget::default())
}
