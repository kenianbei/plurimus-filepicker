use std::cmp::Reverse;
use std::path::{MAIN_SEPARATOR_STR, Path, PathBuf};

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{Commands, Component, Entity, Mut, Query, Ref};
use plurimus_core::ratatui_core::style::{Modifier, Style};
use plurimus_core::ratatui_core::text::Span;
use plurimus_ui::UiStyle;
use plurimus_widgets::{ActiveDescendant, list_item};

use crate::matching::{Match, find_match, light_matches};
use crate::parts::PickerList;
use crate::path::{directory_text, or_current, resolve};
use crate::picker::{FilePicker, FilePickerFloor, FilePickerLook, FilePickerMatchStyle};

const HIDDEN_PREFIX: char = '.';
const NO_MATCH: &str = "no match";
const DIM: Style = Style::new().add_modifier(Modifier::DIM);

/// One entry of the listed directory; on a row, the entry it stands for.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub name: String,
    pub is_dir: bool,
}

impl Entry {
    pub fn is_hidden(&self) -> bool {
        self.name.starts_with(HIDDEN_PREFIX)
    }
}

/// The entries read for the directory the field last named.
#[derive(Component, Debug, Default)]
pub(crate) struct Listing {
    pub directory: PathBuf,
    pub entries: Vec<Entry>,
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
        if !picker.is_changed() && !floor.is_changed() {
            continue;
        }
        let mut directory = picker.resolved_directory();
        if let Some(floor) = floor
            .0
            .as_deref()
            .map(|floor| resolve(floor, picker.base()))
            && !directory.starts_with(&floor)
        {
            let text = directory_text(&floor, picker.base());
            picker.set_path(text);
            directory = floor;
        }
        if listing.directory == directory {
            continue;
        }
        listing.entries = read_entries(&or_current(directory.clone()));
        listing.directory = directory;
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

/// Respawns the list's rows from the listing and the filter, cursor on
/// the first, whenever either changed.
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
        let ranked = ranked(&listing.entries, filter, look.hidden);
        let first = spawn_rows(&mut commands, list.0, ranked, lit.0);
        if first.is_none() {
            commands.spawn((list_item(NO_MATCH), UiStyle(DIM), ChildOf(list.0)));
        }
        commands.entity(list.0).insert(ActiveDescendant(first));
    }
}

fn spawn_rows(
    commands: &mut Commands,
    list: Entity,
    ranked: Vec<(&Entry, Match)>,
    hit_style: Style,
) -> Option<Entity> {
    let mut first = None;
    for (entry, hit) in ranked {
        let mut label = light_matches(&entry.name, &hit.indices, hit_style);
        if entry.is_dir {
            label.push_span(Span::raw(MAIN_SEPARATOR_STR));
        }
        let mut row = commands.spawn((list_item(label), entry.clone(), ChildOf(list)));
        if entry.is_hidden() {
            row.insert(UiStyle(DIM));
        }
        first.get_or_insert(row.id());
    }
    first
}

// Stable on score, so ties keep the listing's order: directories first,
// then names case-insensitively.
fn ranked<'a>(entries: &'a [Entry], filter: &str, hidden: bool) -> Vec<(&'a Entry, Match)> {
    let mut ranked: Vec<(&Entry, Match)> = entries
        .iter()
        .filter(|entry| hidden || !entry.is_hidden())
        .filter_map(|entry| find_match(filter, &entry.name).map(|hit| (entry, hit)))
        .collect();
    ranked.sort_by_key(|(_, hit)| Reverse(hit.score));
    ranked
}
