//! Keys and pointer: what each binding does, and what the picker emits.

mod support;

use std::path::{Path, PathBuf};

use bevy_app::App;
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::prelude::{On, ResMut, Resource};
use bevy_input::keyboard::Key;
use plurimus_filepicker::{FilePicker, FilePickerAction, FilePickerCommand, FilePickerLook};
use plurimus_ui::bevy_input_focus::InputFocus;
use plurimus_ui::{InteractionDisabled, ModalDismiss, ValueChange};
use support::{
    Heard, character, click, cursor, directory, field_of, focused, list_of, listening_parent, path,
    press_key, press_key_with_ctrl, repeat_key, rows, scratch, set_path, spawn_picker,
    spawn_picker_with, type_text,
};

#[derive(Resource, Default)]
struct Chosen(Vec<(Entity, PathBuf)>);

#[derive(Resource, Default)]
struct Dismissed(Vec<Entity>);

fn logged_app() -> App {
    let mut app = support::app();
    app.init_resource::<Chosen>();
    app.init_resource::<Dismissed>();
    app.add_observer(
        |change: On<ValueChange<PathBuf>>, mut log: ResMut<Chosen>| {
            log.0.push((change.source, change.value.clone()));
        },
    );
    app.add_observer(|dismiss: On<ModalDismiss>, mut log: ResMut<Dismissed>| {
        log.0.push(dismiss.entity);
    });
    app
}

fn chosen(app: &App) -> &[(Entity, PathBuf)] {
    &app.world().resource::<Chosen>().0
}

#[test]
fn typing_filters_and_backspace_edits() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());

    type_text(&mut app, "b.");
    assert_eq!(rows(&app)[0], "> b.");
    assert_eq!(rows(&app)[1], "> b.txt");

    press_key(&mut app, Key::Backspace);
    assert_eq!(path(&app, entity), "b");
}

#[test]
fn down_moves_the_cursor_through_the_list() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());
    let list = list_of(&app, entity);
    let children = app.world().get::<Children>(list).unwrap().to_vec();

    press_key(&mut app, Key::ArrowDown);
    assert_eq!(cursor(&app, list), Some(children[2]));
}

#[test]
fn up_and_down_wrap_at_the_ends() {
    let dir = scratch();
    let mut app = logged_app();
    let parent = listening_parent(&mut app);
    let entity = spawn_picker_with(&mut app, dir.path(), ChildOf(parent));
    let list = list_of(&app, entity);
    let children = app.world().get::<Children>(list).unwrap().to_vec();

    press_key(&mut app, Key::ArrowUp);
    press_key(&mut app, Key::ArrowUp);
    assert_eq!(cursor(&app, list), children.last().copied(), "Up wraps");
    press_key(&mut app, Key::ArrowDown);
    assert_eq!(cursor(&app, list), Some(children[0]), "Down wraps");
    assert_eq!(app.world().resource::<Heard>().0, [], "a wrap moved");
}

#[test]
fn page_keys_at_an_end_leave_the_picker() {
    let dir = scratch();
    let mut app = logged_app();
    let parent = listening_parent(&mut app);
    spawn_picker_with(&mut app, dir.path(), ChildOf(parent));

    for key in [Key::PageUp, Key::PageUp, Key::PageDown, Key::PageDown] {
        press_key(&mut app, key);
    }

    let heard = &app.world().resource::<Heard>().0;
    assert_eq!(
        heard,
        &[Key::PageUp, Key::PageDown],
        "a key that moved stays"
    );
}

#[test]
fn arrows_on_a_lone_row_leave_the_picker() {
    let dir = scratch();
    let mut app = logged_app();
    let parent = listening_parent(&mut app);
    spawn_picker_with(&mut app, dir.path(), ChildOf(parent));
    type_text(&mut app, "a.t");
    assert_eq!(
        rows(&app)[1..].iter().filter(|row| !row.is_empty()).count(),
        1
    );

    press_key(&mut app, Key::ArrowUp);
    press_key(&mut app, Key::ArrowDown);

    let heard = &app.world().resource::<Heard>().0;
    assert_eq!(heard, &[Key::ArrowUp, Key::ArrowDown]);
}

#[test]
fn a_command_steps_the_cursor() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());
    let list = list_of(&app, entity);
    let children = app.world().get::<Children>(list).unwrap().to_vec();

    app.world_mut().trigger(FilePickerCommand {
        entity,
        action: FilePickerAction::Down,
    });
    app.update();

    assert_eq!(cursor(&app, list), Some(children[2]));
}

#[test]
fn enter_on_a_directory_descends() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());

    press_key(&mut app, Key::Enter);
    assert_eq!(path(&app, entity), "sub/");
    assert_eq!(rows(&app)[2], "> inner.rs");
    assert_eq!(chosen(&app), []);
}

#[test]
fn enter_on_a_file_chooses_it_once() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());
    type_text(&mut app, "b.");

    press_key(&mut app, Key::Enter);
    assert_eq!(chosen(&app), [(entity, dir.path().join("b.txt"))]);

    repeat_key(&mut app, Key::Enter);
    assert_eq!(chosen(&app).len(), 1, "a repeat emits nothing");
}

#[test]
fn complete_writes_the_cursor_entry_into_the_field() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());

    press_key(&mut app, Key::Tab);
    assert_eq!(path(&app, entity), "sub/");

    type_text(&mut app, "in");
    press_key(&mut app, Key::ArrowRight);
    assert_eq!(path(&app, entity), "sub/inner.rs");
    assert_eq!(chosen(&app), []);
}

#[test]
fn left_climbs_and_backspace_on_an_empty_filter_edits() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());
    press_key(&mut app, Key::Enter);
    assert_eq!(directory(&app, entity), dir.path().join("sub"));

    press_key(&mut app, Key::ArrowLeft);
    assert_eq!(directory(&app, entity), dir.path());
    assert_eq!(FilePicker::filter(path(&app, entity)), "");

    press_key(&mut app, Key::Enter);
    press_key(&mut app, Key::Backspace);
    assert_eq!(path(&app, entity), "sub");
    assert_eq!(directory(&app, entity), dir.path());
}

#[test]
fn enter_on_the_parent_row_climbs() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());

    press_key(&mut app, Key::ArrowUp);
    press_key(&mut app, Key::Enter);
    assert_eq!(directory(&app, entity), dir.path().parent().unwrap());
    assert_eq!(FilePicker::filter(path(&app, entity)), "");
    assert_eq!(chosen(&app), []);
}

#[test]
fn parent_climbs_above_the_base_and_stops_at_the_root() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());

    press_key(&mut app, Key::ArrowLeft);
    assert_eq!(directory(&app, entity), dir.path().parent().unwrap());

    set_path(&mut app, entity, "/x");
    press_key(&mut app, Key::ArrowLeft);
    assert_eq!(path(&app, entity), "/x", "nothing above the root");
}

#[test]
fn parent_over_a_relative_base_stops_at_the_root() {
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, Path::new("."));
    let depth = std::env::current_dir()
        .expect("a working directory")
        .components()
        .count()
        .saturating_sub(1);

    for _ in 0..depth + 2 {
        press_key(&mut app, Key::ArrowLeft);
    }
    assert_eq!(path(&app, entity), "../".repeat(depth));
}

#[test]
fn ctrl_dot_toggles_hidden_entries() {
    let dir = scratch();
    let mut app = logged_app();
    spawn_picker(&mut app, dir.path());

    press_key_with_ctrl(&mut app, character('h'));
    assert!(
        !rows(&app).iter().any(|row| row.contains(".hidden")),
        "Ctrl+H is unbound"
    );

    press_key_with_ctrl(&mut app, character('.'));
    assert!(rows(&app).iter().any(|row| row.contains(".hidden")));
    assert_eq!(rows(&app)[0], ">", "the chord did not type");

    press_key_with_ctrl(&mut app, character('.'));
    assert!(!rows(&app).iter().any(|row| row.contains(".hidden")));
}

#[test]
fn escape_dismisses_the_picker_as_a_modal() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());

    press_key(&mut app, Key::Escape);
    assert_eq!(app.world().resource::<Dismissed>().0, [entity]);
}

#[test]
fn pressing_the_prompt_focuses_the_field() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());
    let field = field_of(&app, entity);
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_eq!(focused(&app), None);

    click(&mut app, 1, 0);
    assert_eq!(focused(&app), Some(field));
}

fn command(app: &mut App, picker: Entity, action: FilePickerAction) {
    app.world_mut().trigger(FilePickerCommand {
        entity: picker,
        action,
    });
    app.update();
}

#[test]
fn a_command_from_outside_enters_like_the_key() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());

    command(&mut app, entity, FilePickerAction::Enter);
    assert_eq!(path(&app, entity), "sub/", "the cursor's directory");

    command(&mut app, entity, FilePickerAction::Enter);
    assert_eq!(chosen(&app), [(entity, dir.path().join("sub/inner.rs"))]);
}

#[test]
fn a_disabled_picker_ignores_a_command() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker_with(&mut app, dir.path(), InteractionDisabled);

    command(&mut app, entity, FilePickerAction::Enter);

    assert_eq!(path(&app, entity), "");
    assert_eq!(chosen(&app), []);
}

#[test]
fn pressing_a_row_focuses_the_field_and_moves_the_cursor() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());
    let list = list_of(&app, entity);
    let children = app.world().get::<Children>(list).unwrap().to_vec();
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();

    click(&mut app, 3, 2);

    assert_eq!(focused(&app), Some(field_of(&app, entity)));
    assert_eq!(cursor(&app, list), Some(children[1]));
}

#[test]
fn a_click_moves_the_cursor_and_a_double_click_chooses() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());
    let list = list_of(&app, entity);
    let children = app.world().get::<Children>(list).unwrap().to_vec();

    click(&mut app, 3, 4);
    assert_eq!(cursor(&app, list), Some(children[3]));
    assert!(chosen(&app).is_empty(), "one click chooses nothing");
    assert_eq!(focused(&app), Some(field_of(&app, entity)), "focus stays");

    click(&mut app, 3, 4);
    assert_eq!(chosen(&app), [(entity, dir.path().join("a.txt"))]);
}

fn has_new(app: &App) -> bool {
    rows(app).iter().any(|row| row.ends_with("new"))
}

fn accept_new(app: &mut App, entity: Entity) {
    app.world_mut()
        .entity_mut(entity)
        .insert(FilePickerLook::default().with_accepts_new(true));
    app.update();
}

#[test]
fn typed_row_follows_the_matches_and_a_partial_match_still_opens() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());
    accept_new(&mut app, entity);
    type_text(&mut app, "a");
    assert_eq!(rows(&app)[1], "> a.txt");
    assert!(rows(&app)[3].starts_with("  a") && rows(&app)[3].ends_with("new"));

    press_key(&mut app, Key::Enter);
    press_key(&mut app, Key::ArrowDown);
    press_key(&mut app, Key::ArrowDown);
    press_key(&mut app, Key::Enter);
    assert_eq!(
        chosen(&app),
        [
            (entity, dir.path().join("a.txt")),
            (entity, dir.path().join("a"))
        ]
    );
}

#[test]
fn an_unmatched_name_is_chosen_with_one_enter() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());
    accept_new(&mut app, entity);
    type_text(&mut app, "plan");

    press_key(&mut app, Key::Enter);
    assert_eq!(chosen(&app), [(entity, dir.path().join("plan"))]);
}

#[test]
fn an_exact_existing_name_offers_no_typed_row() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());
    accept_new(&mut app, entity);
    type_text(&mut app, "b.txt");

    assert_eq!(rows(&app)[1], "> b.txt");
    assert!(!has_new(&app), "{:?}", rows(&app));
}

#[test]
fn without_accepts_new_an_unmatched_name_chooses_nothing() {
    let dir = scratch();
    let mut app = logged_app();
    spawn_picker(&mut app, dir.path());
    type_text(&mut app, "plan");

    assert_eq!(rows(&app)[1], "no match");
    press_key(&mut app, Key::Enter);
    assert_eq!(chosen(&app), []);
}

#[test]
fn a_typed_name_survives_the_extension_filter() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());
    app.world_mut().entity_mut(entity).insert(
        FilePickerLook::default()
            .with_accepts_new(true)
            .with_extensions(["toml"]),
    );
    app.update();
    type_text(&mut app, "notes");

    assert!(rows(&app)[1].ends_with("new"), "{:?}", rows(&app));
    press_key(&mut app, Key::Enter);
    assert_eq!(chosen(&app), [(entity, dir.path().join("notes.toml"))]);
}

#[test]
fn a_typed_name_beside_its_listed_file_is_not_new() {
    let dir = scratch();
    std::fs::write(dir.path().join("copy.toml"), "").unwrap();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());
    app.world_mut().entity_mut(entity).insert(
        FilePickerLook::default()
            .with_accepts_new(true)
            .with_extensions(["toml"]),
    );
    app.update();
    type_text(&mut app, "copy");

    assert!(!has_new(&app), "{:?}", rows(&app));
    press_key(&mut app, Key::Enter);
    assert_eq!(chosen(&app), [(entity, dir.path().join("copy.toml"))]);
}

#[test]
fn only_one_listed_extension_is_appended_and_only_to_a_bare_name() {
    let dir = scratch();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());
    let look = FilePickerLook::default().with_accepts_new(true);
    app.world_mut()
        .entity_mut(entity)
        .insert(look.clone().with_extensions(["toml"]));
    set_path(&mut app, entity, "notes.md");
    press_key(&mut app, Key::Enter);

    app.world_mut()
        .entity_mut(entity)
        .insert(look.with_extensions(["toml", "txt"]));
    set_path(&mut app, entity, "notes");
    press_key(&mut app, Key::Enter);

    assert_eq!(
        chosen(&app),
        [
            (entity, dir.path().join("notes.md")),
            (entity, dir.path().join("notes"))
        ]
    );
}

#[test]
fn a_name_kept_off_the_list_is_not_offered_as_new() {
    let dir = scratch();
    std::fs::write(dir.path().join(".secret.toml"), "").unwrap();
    let mut app = logged_app();
    let entity = spawn_picker(&mut app, dir.path());
    app.world_mut().entity_mut(entity).insert(
        FilePickerLook::default()
            .with_accepts_new(true)
            .with_extensions(["toml"]),
    );
    app.update();

    type_text(&mut app, "b.txt");
    assert!(!has_new(&app), "filtered out by extension, still exists");

    set_path(&mut app, entity, ".secret");
    assert!(!has_new(&app), "hidden, still exists");

    set_path(&mut app, entity, "..");
    assert!(!has_new(&app), "not a file name");
}
