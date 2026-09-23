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
use crate::picker::{FilePicker, FilePickerFloor, FilePickerLook, FilePickerMatchStyle};

const HIDDEN_PREFIX: char = '.';
const PARENT: &str = "..";
const NO_MATCH: &str = "no match";
const NEW_BADGE: &str = "new";
const DIM: Style = Style::new().add_modifier(Modifier::DIM);

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
    pub directory: PathBuf,
    pub entries: Vec<Entry>,
    /// The directory is neither a root nor the floor.
    pub has_parent: bool,
}

/// The filter the rows were last built for.
#[derive(Component, Debug, Default)]
pub(crate) struct BuiltRows {
    filter: String,
}

/// Reads the directory whenever the field names a new one, clamping the
/// field to the floor first. The floor resolves against the base the way
/// the field does, so a relative floor means what a relative field means.
pub(crate) fn relist(mut pickers: Query<(Mut<FilePicker>, Ref<FilePickerFloor>, &mut Listing)>) {
    for (mut picker, floor, mut listing) in &mut pickers {
        let floor_changed = floor.is_changed();
        if !picker.is_changed() && !floor_changed {
            continue;
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
        if listing.directory == directory && !floor_changed {
            continue;
        }
        let is_climbable = floor.as_ref() != Some(&directory) && has_parent(&directory);
        if listing.has_parent != is_climbable {
            listing.has_parent = is_climbable;
        }
        if listing.directory != directory {
            listing.entries = read_entries(&or_current(directory.clone()));
            listing.directory = directory;
        }
    }
}

// `file_type` is free on most filesystems; only a symlink costs a stat, so
// a linked directory can be entered.
fn read_entries(directory: &Path) -> Vec<Entry> {
    let Ok(read) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = read
        .filter_map(Result::ok)
        .map(|entry| Entry {
            name: entry.file_name().to_string_lossy().into_owned(),
            is_dir: entry
                .file_type()
                .is_ok_and(|kind| kind.is_dir() || (kind.is_symlink() && entry.path().is_dir())),
        })
        .collect();
    entries.sort_by_cached_key(|entry| (!entry.is_dir, entry.name.to_lowercase()));
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
        let parent = listing.has_parent.then(|| Entry {
            name: PARENT.to_owned(),
            is_dir: true,
        });
        let ranked = ranked(parent.iter().chain(&listing.entries), filter, &look);
        let is_new = look.accepts_new && is_creatable(filter, &listing.entries);
        let mut cursor = spawn_rows(&mut commands, list.0, ranked, lit.0);
        if is_new {
            cursor.get_or_insert(spawn_typed_row(&mut commands, list.0, filter));
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
    ranked: Vec<(&Entry, Match)>,
    hit_style: Style,
) -> Option<Entity> {
    let mut first = None;
    let mut cursor = None;
    for (entry, hit) in ranked {
        let mut label = light_matches(&entry.name, &hit.indices, hit_style);
        if entry.is_dir {
            label.push_span(Span::raw(MAIN_SEPARATOR_STR));
        }
        let row = commands
            .spawn((list_item(label), entry.clone(), ChildOf(list)))
            .insert_if(UiStyle(DIM), || entry.is_hidden())
            .id();
        first.get_or_insert(row);
        if !entry.is_parent() {
            cursor.get_or_insert(row);
        }
    }
    cursor.or(first)
}

// Against every entry read, not the rows shown: a name a hidden entry or
// the extension filter keeps off the list still exists.
fn is_creatable(filter: &str, entries: &[Entry]) -> bool {
    Path::new(filter).file_name().is_some() && !entries.iter().any(|entry| entry.name == filter)
}

/// The filter itself as a file to create, after the entries so the cursor
/// still opens on the best existing match.
fn spawn_typed_row(commands: &mut Commands, list: Entity, name: &str) -> Entity {
    let entry = Entry {
        name: name.to_owned(),
        is_dir: false,
    };
    let badge = ListItemTrailing(Line::styled(NEW_BADGE, DIM));
    commands
        .spawn((list_item(name.to_owned()), badge, ChildOf(list)))
        .insert_if(UiStyle(DIM), || entry.is_hidden())
        .insert(entry)
        .id()
}

// Stable on score, so ties keep the listing's order: directories first,
// then names case-insensitively.
fn ranked<'a>(
    entries: impl Iterator<Item = &'a Entry>,
    filter: &str,
    look: &FilePickerLook,
) -> Vec<(&'a Entry, Match)> {
    let mut ranked: Vec<(&Entry, Match)> = entries
        .filter(|entry| look.hidden || !entry.is_hidden())
        .filter(|entry| entry.is_dir || has_listed_extension(&look.extensions, &entry.name))
        .filter_map(|entry| find_match(filter, &entry.name).map(|hit| (entry, hit)))
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
