//! The listing: what a picker lists, and how the rows and field draw.

mod support;

use bevy_ecs::hierarchy::Children;
use plurimus_filepicker::{FilePickerFloor, FilePickerLook};
use plurimus_ui::InteractionDisabled;
use support::{
    app, composed_styled_frame, cursor, focus, focused, list_of, picker, rows, scratch, set_path,
    spawn_picker,
};

/// The list's rows without the two-cell cursor gutter the engine draws.
fn row_texts(app: &bevy_app::App) -> Vec<String> {
    rows(app)
        .into_iter()
        .skip(1)
        .filter(|row| !row.is_empty())
        .map(|row| row.chars().skip(2).collect())
        .collect()
}

#[test]
fn lists_directories_first_then_names_case_insensitively_without_hidden() {
    let dir = scratch();
    let mut app = app();
    spawn_picker(&mut app, dir.path());

    assert_eq!(rows(&app)[0], ">");
    assert_eq!(row_texts(&app), ["sub/", "Zeta/", "a.txt", "b.txt"]);
}

#[test]
fn look_hidden_lists_dot_entries_dimmed() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    app.world_mut()
        .entity_mut(entity)
        .insert(FilePickerLook::default().with_hidden(true));
    app.update();

    assert_eq!(
        row_texts(&app),
        ["sub/", "Zeta/", ".hidden", "a.txt", "b.txt"]
    );
    let styled = composed_styled_frame(&app);
    assert!(styled.contains("mods:DIM"), "hidden row is dim: {styled}");
}

#[test]
fn set_path_relists_and_filters() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());

    set_path(&mut app, entity, "sub/");
    assert_eq!(row_texts(&app), ["inner.rs"]);

    set_path(&mut app, entity, "b.");
    assert_eq!(row_texts(&app), ["b.txt"]);
    assert_eq!(picker(&app, entity).directory(), dir.path());
}

#[test]
fn matched_characters_are_lit() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    set_path(&mut app, entity, "at");

    assert_eq!(row_texts(&app), ["a.txt"]);
    let styled = composed_styled_frame(&app);
    assert!(styled.contains("mods:BOLD"), "hits are bold: {styled}");
}

#[test]
fn nothing_matching_draws_one_dim_row_without_cursor() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    set_path(&mut app, entity, "zzz");

    assert_eq!(rows(&app)[1], "no match", "no gutter without a cursor");
    let list = list_of(&mut app, entity);
    assert_eq!(cursor(&app, list), None);
}

#[test]
fn cursor_sits_on_the_first_row_after_every_rebuild() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    let list = list_of(&mut app, entity);
    let first = |app: &bevy_app::App| app.world().get::<Children>(list).unwrap()[0];

    assert_eq!(cursor(&app, list), Some(first(&app)));
    set_path(&mut app, entity, "t");
    assert_eq!(row_texts(&app), ["a.txt", "b.txt", "Zeta/"]);
    assert_eq!(cursor(&app, list), Some(first(&app)));
}

#[test]
fn unreadable_directory_lists_nothing_and_keeps_the_field() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    set_path(&mut app, entity, "nope/");

    assert_eq!(rows(&app)[1], "no match");
    assert_eq!(picker(&app, entity).path(), "nope/");
}

#[test]
fn floor_clamps_the_field_back() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    app.world_mut()
        .entity_mut(entity)
        .insert(FilePickerFloor(Some(dir.path().to_owned())));
    app.update();
    set_path(&mut app, entity, "../");

    let clamped = picker(&app, entity);
    assert_eq!(clamped.directory(), dir.path());
    assert_eq!(clamped.filter(), "");
    assert_eq!(row_texts(&app), ["sub/", "Zeta/", "a.txt", "b.txt"]);
}

#[test]
fn a_relative_floor_resolves_against_the_base() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    app.world_mut()
        .entity_mut(entity)
        .insert(FilePickerFloor(Some("sub".into())));
    app.update();

    let clamped = picker(&app, entity);
    assert_eq!(clamped.path(), "sub/");
    assert_eq!(clamped.directory(), dir.path().join("sub"));
    assert_eq!(row_texts(&app), ["inner.rs"]);
}

#[test]
fn disabling_the_picker_disables_its_list() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    let list = list_of(&mut app, entity);
    app.world_mut()
        .entity_mut(entity)
        .insert(InteractionDisabled);
    app.update();
    assert!(app.world().get::<InteractionDisabled>(list).is_some());

    app.world_mut()
        .entity_mut(entity)
        .remove::<InteractionDisabled>();
    app.update();
    assert!(app.world().get::<InteractionDisabled>(list).is_none());
}

#[test]
fn focusing_the_picker_lands_on_its_list() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    let list = list_of(&mut app, entity);
    assert_eq!(focused(&app), Some(list));

    focus(&mut app, entity);
    app.update();
    assert_eq!(focused(&app), Some(list));
}

#[test]
fn field_shows_prompt_text_and_caret_while_focused() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    set_path(&mut app, entity, "b.");

    assert_eq!(rows(&app)[0], "> b.");
    let styled = composed_styled_frame(&app);
    let caret_row = styled.lines().nth(support::ROWS as usize + 1).unwrap();
    assert_ne!(
        &caret_row[4..5],
        &caret_row[3..4],
        "the caret cell is styled unlike the text before it: {styled}"
    );
}
