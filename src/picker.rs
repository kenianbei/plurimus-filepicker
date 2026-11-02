use std::path::{Path, PathBuf};

use bevy_ecs::prelude::{Bundle, Component};
use plurimus_core::UiWidget;
use plurimus_core::ratatui_core::style::{Modifier, Style};
use plurimus_core::ratatui_core::text::Line;
use plurimus_ui::{ComputedWidgetArea, Hovered, StylistCache};
use plurimus_widgets::TextInputKeys;

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
///
/// The field is a child `EditableText`, named by [`FilePickerParts`] on the
/// picker and holding its focus: the engine edits its `TextInput` through
/// the picker's [`TextInputKeys`], less the keys [`FilePickerKeys`] binds,
/// places its caret by pointer and publishes it to the terminal. To move
/// the field, write a new `TextInput` to that entity.
///
/// [`FilePickerParts`]: crate::FilePickerParts
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
    base: PathBuf,
    opened_on: String,
}

impl FilePicker {
    /// A picker whose relative paths are joined to `base`, opened on it.
    #[must_use]
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self {
            base: base.into(),
            opened_on: String::new(),
        }
    }

    /// Opened with this text in the field, caret at its end. Read once,
    /// when the field is spawned.
    #[must_use]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.opened_on = path.into();
        self
    }

    /// The directory the list shows for the field's `text`: what precedes
    /// its last separator, joined to the base when relative, lexically
    /// normalized.
    #[must_use]
    pub fn directory(&self, text: &str) -> PathBuf {
        or_current(self.resolved_directory(text))
    }

    /// What follows the last separator of the field's `text`.
    #[must_use]
    pub fn filter(text: &str) -> &str {
        split_at_separator(text).1
    }

    /// [`Self::directory`] with the working directory as the empty path,
    /// the form the floor is compared in.
    pub(crate) fn resolved_directory(&self, text: &str) -> PathBuf {
        field_directory(text, &self.base)
    }

    /// The base relative paths are joined to.
    #[must_use]
    pub fn base(&self) -> &Path {
        &self.base
    }

    pub(crate) fn opened_on(&self) -> &str {
        &self.opened_on
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

/// Decorates each file of a directory as it is read, from the file's path.
///
/// Runs inside the read, once per file per directory read and never per
/// keystroke; a slow decorator is a slow directory change. Directories,
/// `..`, and the typed new row are not decorated. Inserting or replacing it
/// reads the directory again; removing it leaves the rows decorated until
/// the next read.
#[derive(Component)]
pub struct FilePickerDecorator(pub Box<dyn Fn(&Path) -> RowDecoration + Send + Sync>);

/// What a [`FilePickerDecorator`] adds to one file's row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct RowDecoration {
    /// Drawn right-aligned on the row.
    pub trailing: Option<Line<'static>>,
    /// Laid over the row, above the dim of a hidden entry.
    pub style: Style,
}

impl RowDecoration {
    /// With this line drawn right-aligned on the row.
    #[must_use]
    pub fn with_trailing(mut self, trailing: impl Into<Line<'static>>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }

    /// With this style laid over the row.
    #[must_use]
    pub const fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

/// A file picker opened on `base`, drawn like any other widget.
///
/// Its field and list are children spawned in the next `PreUpdate` and
/// named by [`FilePickerParts`]; focusing the picker lands on the field,
/// which is the tab stop. `ListBoxCursor`, `ListBoxStripe` and
/// `ListBoxSelectionMarker` put on the picker are copied onto the list when
/// it is spawned and whenever they change; removing one does not.
///
/// [`FilePickerParts`]: crate::FilePickerParts
#[must_use]
pub fn file_picker(base: impl Into<PathBuf>) -> impl Bundle {
    (FilePicker::new(base), UiWidget::default())
}
