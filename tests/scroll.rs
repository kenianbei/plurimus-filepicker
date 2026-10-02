//! Scrolling: where a listing taller than the list opens.

mod support;

use bevy_app::App;
use bevy_ecs::entity::Entity;
use plurimus_core::ratatui_core::layout::{Position, Rect};
use plurimus_filepicker::file_picker;
use plurimus_term::KeyCode;
use plurimus_ui::{ScrollOffset, UiArea};
use support::{
    COLS, DEEP_FILES, ROWS, app, focus, list_of, press_key, row_texts, rows, spawn_picker,
    tall_scratch,
};

/// Where the engine's reveal leaves a list with no area: past its cursor.
const UNPLACED_OFFSET: Position = Position::new(1, 2);

/// A picker over `deep/` with the cursor on its last row, so the list is
/// scrolled as far as it goes.
fn scrolled_picker(app: &mut App, dir: &tempfile::TempDir) -> Entity {
    let picker = spawn_picker(app, &dir.path().join("deep"));
    for _ in 0..DEEP_FILES {
        press_key(app, KeyCode::Down);
    }
    let list = list_of(app, picker);
    assert_ne!(offset(app, list), Position::ORIGIN);
    list
}

fn offset(app: &App, list: Entity) -> Position {
    app.world().get::<ScrollOffset>(list).unwrap().0
}

/// The first two list rows' labels, without the scrollbar in the last column.
fn top_rows(app: &App) -> Vec<String> {
    row_texts(app)
        .iter()
        .take(2)
        .filter_map(|row| row.split_whitespace().next().map(str::to_owned))
        .collect()
}

#[test]
fn a_parent_listing_opens_at_its_top() {
    let dir = tall_scratch();
    let mut app = app();
    let list = scrolled_picker(&mut app, &dir);

    press_key(&mut app, KeyCode::Left);

    assert_eq!(offset(&app, list), Position::ORIGIN);
    assert_eq!(top_rows(&app), ["../", "deep/"]);
}

/// `.` keeps `..`, so the cursor opens on the second row; on the first, the
/// engine's reveal alone would bring a stale scroll back to the top.
#[test]
fn a_filtered_listing_opens_at_its_top() {
    let dir = tall_scratch();
    let mut app = app();
    let list = scrolled_picker(&mut app, &dir);

    press_key(&mut app, KeyCode::Char('.'));

    assert_eq!(offset(&app, list), Position::ORIGIN);
    assert_eq!(top_rows(&app), ["../", "p00.txt"]);
}

#[test]
fn a_picker_placed_late_opens_at_its_top() {
    let dir = tall_scratch();
    let mut app = app();
    let picker = app
        .world_mut()
        .spawn((
            file_picker(dir.path().join("deep")),
            UiArea::Fixed(Rect::ZERO),
        ))
        .id();
    focus(&mut app, picker);
    app.update();
    app.update();
    let list = list_of(&mut app, picker);
    assert_eq!(offset(&app, list), UNPLACED_OFFSET);

    app.world_mut()
        .entity_mut(picker)
        .insert(UiArea::Fixed(Rect::new(0, 0, COLS, ROWS)));
    app.update();

    assert_eq!(offset(&app, list), Position::ORIGIN);
    assert_eq!(top_rows(&app), ["../", "p00.txt"]);
    assert!(rows(&app)[2].starts_with("> p00.txt"), "{:?}", rows(&app));
}
