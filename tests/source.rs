//! A picker over a `DirectorySource` other than the disk.

mod support;

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use bevy_app::App;
use bevy_ecs::bundle::Bundle;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{On, ResMut, Resource};
use plurimus_filepicker::{
    DirectorySource, FilePickerDecorator, FilePickerSource, RowDecoration, SourceEntry,
};
use plurimus_term::KeyCode;
use plurimus_ui::ValueChange;
use support::{picker, press_key, row_texts, set_path, spawn_picker_with, type_text};

/// Directories and their entries, with no working directory.
struct Workspace(BTreeMap<PathBuf, Vec<SourceEntry>>);

impl DirectorySource for Workspace {
    fn list(&self, directory: &Path) -> io::Result<Vec<SourceEntry>> {
        self.0
            .get(directory)
            .cloned()
            .ok_or_else(|| io::ErrorKind::NotFound.into())
    }

    fn current_dir(&self) -> Option<PathBuf> {
        None
    }
}

/// `/plans/` holding `drafts/`, `b.toml` and `A.toml`, listed unsorted.
fn workspace() -> FilePickerSource {
    let directories = BTreeMap::from([
        (PathBuf::from("/"), vec![SourceEntry::directory("plans")]),
        (
            PathBuf::from("/plans"),
            vec![
                SourceEntry::file("b.toml"),
                SourceEntry::directory("drafts"),
                SourceEntry::file("A.toml"),
            ],
        ),
    ]);
    FilePickerSource(Arc::new(Workspace(directories)))
}

#[derive(Resource, Default)]
struct Chosen(Vec<PathBuf>);

fn spawn_over(app: &mut App, base: &str, extra: impl Bundle) -> Entity {
    spawn_picker_with(app, Path::new(base), (workspace(), extra))
}

#[test]
fn a_source_lists_directories_first_whatever_its_order() {
    let mut app = support::app();
    spawn_over(&mut app, "/plans", ());

    assert_eq!(row_texts(&app), ["../", "drafts/", "A.toml", "b.toml"]);
}

#[test]
fn a_source_without_a_working_directory_still_climbs_to_its_root() {
    let mut app = support::app();
    let entity = spawn_over(&mut app, "/plans", ());

    press_key(&mut app, KeyCode::Left);
    assert_eq!(picker(&app, entity).path(), "/");
    assert_eq!(row_texts(&app), ["plans/"]);
}

#[test]
fn a_directory_the_source_lacks_lists_nothing() {
    let mut app = support::app();
    let entity = spawn_over(&mut app, "/plans", ());

    set_path(&mut app, entity, "/nowhere/");
    assert_eq!(row_texts(&app), ["../"]);
}

#[test]
fn the_decorator_sees_the_sources_paths() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    let decorator = FilePickerDecorator(Box::new(move |path: &Path| {
        log.lock().unwrap().push(path.to_path_buf());
        RowDecoration::default()
    }));
    let mut app = support::app();
    spawn_over(&mut app, "/plans", decorator);

    let mut seen = seen.lock().unwrap().clone();
    seen.sort();
    assert_eq!(seen, ["/plans/A.toml", "/plans/b.toml"].map(PathBuf::from));
}

#[test]
fn enter_chooses_the_sources_path() {
    let mut app = support::app();
    app.init_resource::<Chosen>();
    app.add_observer(
        |change: On<ValueChange<PathBuf>>, mut log: ResMut<Chosen>| {
            log.0.push(change.value.clone());
        },
    );
    spawn_over(&mut app, "/plans", ());

    type_text(&mut app, "b.");
    press_key(&mut app, KeyCode::Enter);
    assert_eq!(
        app.world().resource::<Chosen>().0,
        [PathBuf::from("/plans/b.toml")]
    );
}

#[test]
fn replacing_the_source_reads_the_directory_again() {
    let mut app = support::app();
    let entity = spawn_over(&mut app, "/plans", ());
    let replaced = BTreeMap::from([(PathBuf::from("/plans"), vec![SourceEntry::file("c.toml")])]);

    app.world_mut()
        .entity_mut(entity)
        .insert(FilePickerSource(Arc::new(Workspace(replaced))));
    app.update();
    assert_eq!(row_texts(&app), ["../", "c.toml"]);
}
