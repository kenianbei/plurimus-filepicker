use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy_ecs::prelude::Component;

/// What a picker lists, and what a relative path is resolved against.
///
/// Called synchronously in `PreUpdate`, once per directory read.
pub trait DirectorySource: Send + Sync + 'static {
    /// The entries of `directory`, in any order.
    ///
    /// `directory` is the path the field names, relative when the picker's
    /// base is and `.` for the working directory.
    ///
    /// # Errors
    ///
    /// Whatever the source cannot read; the picker lists nothing for it.
    fn list(&self, directory: &Path) -> io::Result<Vec<SourceEntry>>;

    /// What a relative directory is resolved against to judge whether it has
    /// a parent; `None` gives only an absolute directory a `..`.
    fn current_dir(&self) -> Option<PathBuf> {
        std::env::current_dir().ok()
    }
}

/// One entry a [`DirectorySource`] lists.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SourceEntry {
    /// The entry's name within its directory.
    pub name: String,
    /// Whether the picker descends into it rather than choosing it.
    pub is_dir: bool,
}

impl SourceEntry {
    /// A file named `name`.
    #[must_use]
    pub fn file(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_dir: false,
        }
    }

    /// A directory named `name`.
    #[must_use]
    pub fn directory(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_dir: true,
        }
    }
}

/// The source a picker lists; a picker without one lists the disk.
///
/// Inserting or replacing it reads the directory again; removing it leaves
/// the rows as they were until the next read.
#[derive(Component, Clone)]
pub struct FilePickerSource(pub Arc<dyn DirectorySource>);

/// The filesystem, through `std::fs`.
pub(crate) struct DiskSource;

impl DirectorySource for DiskSource {
    // `file_type` is free on most filesystems; only a symlink costs a stat,
    // so a linked directory can be entered.
    fn list(&self, directory: &Path) -> io::Result<Vec<SourceEntry>> {
        let entries = std::fs::read_dir(directory)?
            .filter_map(Result::ok)
            .map(|entry| SourceEntry {
                is_dir: entry.file_type().is_ok_and(|kind| {
                    kind.is_dir() || (kind.is_symlink() && entry.path().is_dir())
                }),
                name: entry.file_name().to_string_lossy().into_owned(),
            })
            .collect();
        Ok(entries)
    }
}
