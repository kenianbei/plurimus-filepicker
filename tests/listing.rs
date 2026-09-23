//! The listing: what a picker lists, and how the rows and field draw.

mod support;

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy_ecs::hierarchy::Children;
use plurimus_core::ratatui_core::layout::Rect;
use plurimus_core::ratatui_core::style::{Color, Style};
use plurimus_core::ratatui_core::text::Line;
use plurimus_filepicker::{
    FilePickerDecorator, FilePickerFloor, FilePickerLook, RowDecoration, file_picker,
};
use plurimus_ui::{InteractionDisabled, UiArea};
use plurimus_widgets::{ListBoxCursor, ListBoxSelectionMarker, ListBoxStripe};
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
    assert_eq!(row_texts(&app), ["../", "sub/", "Zeta/", "a.txt", "b.txt"]);
}

#[test]
fn a_picker_over_the_working_directory_lists_it() {
    let mut app = app();
    spawn_picker(&mut app, std::path::Path::new("."));

    let texts = row_texts(&app);
    assert!(texts.iter().any(|row| row.starts_with("src/")), "{texts:?}");
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
        ["../", "sub/", "Zeta/", ".hidden", "a.txt", "b.txt"]
    );
    let styled = composed_styled_frame(&app);
    assert!(styled.contains("mods:DIM"), "hidden row is dim: {styled}");
}

#[test]
fn extensions_list_matching_files_and_every_directory() {
    let dir = scratch();
    std::fs::write(dir.path().join("c.toml"), "").unwrap();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    assert_eq!(
        row_texts(&app),
        ["../", "sub/", "Zeta/", "a.txt", "b.txt", "c.toml"]
    );

    app.world_mut()
        .entity_mut(entity)
        .insert(FilePickerLook::default().with_extensions(["TXT"]));
    app.update();
    assert_eq!(row_texts(&app), ["../", "sub/", "Zeta/", "a.txt", "b.txt"]);

    app.world_mut()
        .entity_mut(entity)
        .insert(FilePickerLook::default().with_extensions(["toml"]));
    app.update();
    assert_eq!(row_texts(&app), ["../", "sub/", "Zeta/", "c.toml"]);
}

#[test]
fn set_path_relists_and_filters() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());

    set_path(&mut app, entity, "sub/");
    assert_eq!(row_texts(&app), ["../", "inner.rs"]);

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
fn cursor_opens_after_the_parent_row_and_on_the_first_match() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    let list = list_of(&mut app, entity);
    let row = |app: &bevy_app::App, index: usize| app.world().get::<Children>(list).unwrap()[index];

    assert_eq!(cursor(&app, list), Some(row(&app, 1)));
    set_path(&mut app, entity, "t");
    assert_eq!(row_texts(&app), ["a.txt", "b.txt", "Zeta/"]);
    assert_eq!(cursor(&app, list), Some(row(&app, 0)));

    set_path(&mut app, entity, "Zeta/");
    assert_eq!(row_texts(&app), ["../"]);
    assert_eq!(cursor(&app, list), Some(row(&app, 0)));
}

#[test]
fn no_parent_row_at_the_root() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());

    set_path(&mut app, entity, "/");
    assert_ne!(row_texts(&app)[0], "../");
}

#[test]
fn unreadable_directory_lists_only_the_parent_and_keeps_the_field() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    set_path(&mut app, entity, "nope/");

    assert_eq!(row_texts(&app), ["../"]);
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

const BADGE: &str = "X";

#[test]
fn a_decorator_badges_files_and_leaves_directories_alone() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    let badge = FilePickerDecorator(Box::new(|_: &Path| {
        RowDecoration::default().with_trailing(BADGE)
    }));
    app.world_mut().entity_mut(entity).insert(badge);
    app.update();

    let texts = row_texts(&app);
    let badged: Vec<&str> = texts
        .iter()
        .filter(|row| row.ends_with(BADGE))
        .map(|row| row.split_whitespace().next().unwrap_or_default())
        .collect();
    assert_eq!(badged, ["a.txt", "b.txt"], "{texts:?}");
}

#[test]
fn a_decoration_style_lays_over_the_hidden_dim() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    let red = FilePickerDecorator(Box::new(|_: &Path| {
        RowDecoration::default().with_style(Style::new().fg(Color::Red))
    }));
    app.world_mut()
        .entity_mut(entity)
        .insert((red, FilePickerLook::default().with_hidden(true)));
    app.update();

    let styled = composed_styled_frame(&app);
    assert!(
        styled
            .lines()
            .any(|line| line.contains("fg:Some(Red)") && line.contains("DIM")),
        "{styled}"
    );
}

#[test]
fn a_decorator_runs_once_per_directory_read() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let decorator = FilePickerDecorator(Box::new(move |_: &Path| {
        counted.fetch_add(1, Ordering::Relaxed);
        RowDecoration::default()
    }));
    app.world_mut().entity_mut(entity).insert(decorator);
    app.update();
    assert_eq!(calls.load(Ordering::Relaxed), 3, "three files, hidden too");

    set_path(&mut app, entity, "a");
    set_path(&mut app, entity, "ab");
    assert_eq!(calls.load(Ordering::Relaxed), 3, "filtering reads nothing");

    set_path(&mut app, entity, "sub/");
    assert_eq!(calls.load(Ordering::Relaxed), 4, "sub/ holds one file");
}

const CURSOR: &str = "» ";

fn list_cursor(app: &bevy_app::App, list: bevy_ecs::entity::Entity) -> Option<&Line<'static>> {
    app.world()
        .get::<ListBoxCursor>(list)
        .map(|cursor| &cursor.0)
}

#[test]
fn a_cursor_spawned_on_the_picker_dresses_its_list() {
    let dir = scratch();
    let mut app = app();
    let entity = app
        .world_mut()
        .spawn((
            file_picker(dir.path()),
            UiArea::Fixed(Rect::new(0, 0, support::COLS, support::ROWS)),
            ListBoxCursor(Line::from(CURSOR)),
        ))
        .id();
    focus(&mut app, entity);
    app.update();

    assert!(
        rows(&app).iter().any(|row| row.starts_with(CURSOR)),
        "{:?}",
        rows(&app)
    );
    let list = list_of(&mut app, entity);
    assert_eq!(list_cursor(&app, list), Some(&Line::from(CURSOR)));
}

#[test]
fn look_given_to_a_live_picker_reaches_its_list() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    let list = list_of(&mut app, entity);
    let stripe = Style::new().bg(Color::Blue);
    app.world_mut().entity_mut(entity).insert((
        ListBoxStripe(stripe),
        ListBoxSelectionMarker,
        ListBoxCursor(Line::from(CURSOR)),
    ));
    app.update();
    app.world_mut()
        .entity_mut(entity)
        .insert(ListBoxCursor(Line::from("- ")));
    app.update();

    let world = app.world();
    assert_eq!(
        world.get::<ListBoxStripe>(list).map(|stripe| stripe.0),
        Some(stripe)
    );
    assert!(world.get::<ListBoxSelectionMarker>(list).is_some());
    assert_eq!(list_cursor(&app, list), Some(&Line::from("- ")));
}

#[test]
fn a_picker_added_to_a_dressed_entity_dresses_its_list() {
    let dir = scratch();
    let mut app = app();
    let entity = app
        .world_mut()
        .spawn((
            UiArea::Fixed(Rect::new(0, 0, support::COLS, support::ROWS)),
            ListBoxCursor(Line::from(CURSOR)),
        ))
        .id();
    app.update();
    app.world_mut()
        .entity_mut(entity)
        .insert(file_picker(dir.path()));
    app.update();

    let list = list_of(&mut app, entity);
    assert_eq!(list_cursor(&app, list), Some(&Line::from(CURSOR)));
}
