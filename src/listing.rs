use std::cmp::Reverse;
use std::path::{MAIN_SEPARATOR_STR, Path, PathBuf};

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Commands, Component, Entity, Mut, Query, Ref};
use plurimus_core::ratatui_core::style::{Modifier, Style};
use plurimus_core::ratatui_core::text::{Line, Span};
use plurimus_ui::UiStyle;
use plurimus_widgets::{ActiveDescendant, ListItemTrailing, list_item};

use crate::matching::{Match, find_match, light_matches};
use crate::parts::PickerList;
use crate::path::{directory_text, has_parent, or_current, resolve};
use crate::picker::{
    FilePicker, FilePickerDecorator, FilePickerFloor, FilePickerLook, FilePickerMatchStyle,
    RowDecoration,
};
use crate::source::{DirectorySource, DiskSource, FilePickerSource};

const HIDDEN_PREFIX: char = '.';
const PARENT: &str = "..";
const NO_MATCH: &str = "no match";
const NEW_BADGE: &str = "new";
const DIM: Style = Style::new().add_modifier(Modifier::DIM);
const UNDECORATED: RowDecoration = RowDecoration {
    trailing: None,
    style: Style::new(),
};

/// One entry of the listed directory; on a row, the entry it stands for.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub name: String,
    pub is_dir: bool,
}

impl Entry {
    pub fn is_hidden(&self) -> bool {
        !self.is_parent() && self.name.starts_with(HIDDEN_PREFIX)
    }

    pub fn is_parent(&self) -> bool {
        self.name == PARENT
    }
}

/// The entries read for the directory the field last named.
#[derive(Component, Debug, Default)]
pub(crate) struct Listing {
    pub directory: Option<PathBuf>,
    pub entries: Vec<(Entry, RowDecoration)>,
    /// The directory is neither a root nor the floor.
    pub has_parent: bool,
}

/// The filter the rows were last built for.
#[derive(Component, Debug, Default)]
pub(crate) struct BuiltRows {
    filter: String,
}

/// Reads the directory from the picker's source, or the disk, whenever the
/// field names a new one, clamping the field to the floor first. The floor
/// resolves against the base the way the field does, so a relative floor
/// means what a relative field means.
pub(crate) fn relist(
    mut pickers: Query<(
        Mut<FilePicker>,
        Ref<FilePickerFloor>,
        Option<Ref<FilePickerDecorator>>,
        Option<Ref<FilePickerSource>>,
        &mut Listing,
    )>,
) {
    for (mut picker, floor, decorator, source, mut listing) in &mut pickers {
        let floor_changed = floor.is_changed();
        let is_reread = decorator.as_ref().is_some_and(DetectChanges::is_changed)
            || source.as_ref().is_some_and(DetectChanges::is_changed);
        if !picker.is_changed() && !floor_changed && !is_reread {
            continue;
        }
        if is_reread {
            listing.directory = None;
        }
        let floor = floor
            .0
            .as_deref()
            .map(|floor| resolve(floor, picker.base()));
        let mut directory = picker.resolved_directory();
        if let Some(floor) = &floor
            && !directory.starts_with(floor)
        {
            let text = directory_text(floor, picker.base());
            picker.set_path(text);
            directory.clone_from(floor);
        }
        let is_read = listing.directory.as_ref() == Some(&directory);
        if is_read && !floor_changed {
            continue;
        }
        let source: &dyn DirectorySource =
            source.as_deref().map_or(&DiskSource, |source| &*source.0);
        let is_climbable =
            floor.as_ref() != Some(&directory) && has_parent(&directory, source.current_dir());
        if listing.has_parent != is_climbable {
            listing.has_parent = is_climbable;
        }
        if !is_read {
            listing.entries =
                read_entries(source, &or_current(directory.clone()), decorator.as_deref());
            listing.directory = Some(directory);
        }
    }
}

fn read_entries(
    source: &dyn DirectorySource,
    directory: &Path,
    decorator: Option<&FilePickerDecorator>,
) -> Vec<(Entry, RowDecoration)> {
    let mut entries: Vec<(Entry, RowDecoration)> = source
        .list(directory)
        .unwrap_or_default()
        .into_iter()
        .map(|entry| {
            let decoration = decorator
                .filter(|_| !entry.is_dir)
                .map(|decorator| (decorator.0)(&directory.join(&entry.name)))
                .unwrap_or_default();
            let entry = Entry {
                name: entry.name,
                is_dir: entry.is_dir,
            };
            (entry, decoration)
        })
        .collect();
    entries.sort_by_cached_key(|(entry, _)| (!entry.is_dir, entry.name.to_lowercase()));
    entries
}

/// Respawns the list's rows from the listing and the filter whenever either
/// changed, `..` ahead of the entries when the directory has a parent, the
/// cursor on the first row that is not `..`.
pub(crate) fn rebuild_rows(
    mut pickers: Query<(
        &FilePicker,
        Ref<Listing>,
        Ref<FilePickerLook>,
        Ref<FilePickerMatchStyle>,
        &PickerList,
        &mut BuiltRows,
    )>,
    rows: Query<&Children>,
    mut commands: Commands,
) {
    for (picker, listing, look, lit, list, mut built) in &mut pickers {
        let filter = picker.filter();
        let is_stale =
            listing.is_changed() || look.is_changed() || lit.is_changed() || built.filter != filter;
        if !is_stale {
            continue;
        }
        filter.clone_into(&mut built.filter);
        // Last child first: each despawn then pops the end of `Children`
        // instead of shifting everything after it.
        let old_rows = rows.get(list.0).map(|rows| rows.iter().rev());
        for &row in old_rows.into_iter().flatten() {
            commands.entity(row).despawn();
        }
        let parent = listing.has_parent.then(|| {
            let entry = Entry {
                name: PARENT.to_owned(),
                is_dir: true,
            };
            (entry, UNDECORATED)
        });
        let ranked = ranked(parent.iter().chain(&listing.entries), filter, &look);
        let typed = typed_name(filter, &look, &listing.entries);
        let mut cursor = spawn_rows(&mut commands, list.0, ranked, lit.0);
        if let Some(name) = typed {
            cursor.get_or_insert(spawn_typed_row(&mut commands, list.0, name));
        }
        if cursor.is_none() {
            commands.spawn((list_item(NO_MATCH), UiStyle(DIM), ChildOf(list.0)));
        }
        commands.entity(list.0).insert(ActiveDescendant(cursor));
    }
}

fn spawn_rows(
    commands: &mut Commands,
    list: Entity,
    ranked: Vec<(&(Entry, RowDecoration), Match)>,
    hit_style: Style,
) -> Option<Entity> {
    let mut first = None;
    let mut cursor = None;
    for ((entry, decoration), hit) in ranked {
        let mut label = light_matches(&entry.name, &hit.indices, hit_style);
        if entry.is_dir {
            label.push_span(Span::raw(MAIN_SEPARATOR_STR));
        }
        let dim = if entry.is_hidden() { DIM } else { Style::new() };
        let style = dim.patch(decoration.style);
        let mut row = commands.spawn((list_item(label), entry.clone(), ChildOf(list)));
        row.insert_if(UiStyle(style), || style != Style::new());
        if let Some(trailing) = &decoration.trailing {
            row.insert(ListItemTrailing(trailing.clone()));
        }
        let row = row.id();
        first.get_or_insert(row);
        if !entry.is_parent() {
            cursor.get_or_insert(row);
        }
    }
    cursor.or(first)
}

/// The file the filter would create, with the one listed extension when it
/// names none; `None` for `.`, `..`, or a name any entry read has, shown or
/// not, since a hidden or filtered-out file still exists.
fn typed_name(
    filter: &str,
    look: &FilePickerLook,
    entries: &[(Entry, RowDecoration)],
) -> Option<String> {
    if !look.accepts_new {
        return None;
    }
    let path = Path::new(filter);
    path.file_name()?;
    let name = match look.extensions.as_slice() {
        [extension] if path.extension().is_none() => format!("{filter}.{extension}"),
        _ => filter.to_owned(),
    };
    let is_taken = entries.iter().any(|(entry, _)| entry.name == name);
    (!is_taken).then_some(name)
}

/// The filter itself as a file to create, after the entries so the cursor
/// still opens on the best existing match.
fn spawn_typed_row(commands: &mut Commands, list: Entity, name: String) -> Entity {
    let badge = ListItemTrailing(Line::styled(NEW_BADGE, DIM));
    let label = list_item(name.clone());
    let entry = Entry {
        name,
        is_dir: false,
    };
    commands
        .spawn((label, badge, ChildOf(list)))
        .insert_if(UiStyle(DIM), || entry.is_hidden())
        .insert(entry)
        .id()
}

// Stable on score, so ties keep the listing's order: directories first,
// then names case-insensitively.
fn ranked<'a>(
    entries: impl Iterator<Item = &'a (Entry, RowDecoration)>,
    filter: &str,
    look: &FilePickerLook,
) -> Vec<(&'a (Entry, RowDecoration), Match)> {
    let mut ranked: Vec<(&(Entry, RowDecoration), Match)> = entries
        .filter(|(entry, _)| look.hidden || !entry.is_hidden())
        .filter(|(entry, _)| entry.is_dir || has_listed_extension(&look.extensions, &entry.name))
        .filter_map(|pair| find_match(filter, &pair.0.name).map(|hit| (pair, hit)))
        .collect();
    ranked.sort_by_key(|(_, hit)| Reverse(hit.score));
    ranked
}

fn has_listed_extension(extensions: &[String], name: &str) -> bool {
    if extensions.is_empty() {
        return true;
    }
    Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extensions
                .iter()
                .any(|listed| listed.eq_ignore_ascii_case(extension))
        })
}
