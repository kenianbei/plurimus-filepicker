//! The path field as a text field: a paste, the clipboard keys, and the
//! selection drawn.

mod support;

use bevy_app::App;
use bevy_ecs::hierarchy::ChildOf;
use bevy_input::keyboard::Key;
use plurimus_core::ratatui_core::style::{Modifier, Style};
use plurimus_term::{KeyCode, LastCopied, ModifierKey};
use plurimus_ui::bevy_input_focus::InputFocus;
use plurimus_ui::{InteractionDisabled, UiTheme};
use support::{
    Heard, ROWS, app, composed_styled_frame, listening_parent, paste, picker, press_chord,
    press_key, row_texts, scratch, spawn_picker, spawn_picker_with, type_text,
};

/// The field row's first cells: the two of the prompt, `ab`, and one more.
const FIELD_CELLS: usize = 5;

fn ctrl(app: &mut App, character: char) {
    press_chord(app, ModifierKey::ControlLeft, KeyCode::Char(character));
}

/// Asserts the field row draws `ab` underlined and no caret after it.
fn assert_selection_drawn(app: &App, when: &str) {
    let styled = composed_styled_frame(app);
    let cells = &styled.lines().nth(ROWS as usize + 1).unwrap()[..FIELD_CELLS];
    let underlined = styled
        .lines()
        .find(|line| line.ends_with("UNDERLINED"))
        .and_then(|line| line.chars().next())
        .unwrap_or_else(|| panic!("nothing is underlined {when}: {styled}"));
    let plain = cells.chars().next().unwrap();
    let selected = [plain, plain, underlined, underlined, plain];
    assert_eq!(cells, String::from_iter(selected), "{when}: {styled}");
}

fn last_copied(app: &App) -> Option<&str> {
    app.world().resource::<LastCopied>().0.as_deref()
}

#[test]
fn a_paste_lands_in_the_field_and_filters() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());

    paste(&mut app, "sub/in");

    assert_eq!(picker(&app, entity).path(), "sub/in");
    assert_eq!(row_texts(&app), ["inner.rs"]);
}

#[test]
fn a_paste_replaces_the_selection() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    type_text(&mut app, "ab");
    ctrl(&mut app, 'a');

    paste(&mut app, "sub/");

    assert_eq!(picker(&app, entity).path(), "sub/");
}

#[test]
fn a_disabled_picker_takes_no_paste() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker_with(&mut app, dir.path(), InteractionDisabled);

    paste(&mut app, "ab");

    assert_eq!(picker(&app, entity).path(), "");
}

#[test]
fn copy_sends_the_selection_and_paste_inserts_it() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    type_text(&mut app, "ab");

    ctrl(&mut app, 'a');
    ctrl(&mut app, 'c');
    assert_eq!(last_copied(&app), Some("ab"));
    assert_eq!(picker(&app, entity).path(), "ab");

    press_key(&mut app, KeyCode::End);
    ctrl(&mut app, 'v');
    assert_eq!(picker(&app, entity).path(), "abab");
}

#[test]
fn cut_sends_the_selection_and_deletes_it() {
    let dir = scratch();
    let mut app = app();
    let entity = spawn_picker(&mut app, dir.path());
    type_text(&mut app, "ab");

    ctrl(&mut app, 'a');
    ctrl(&mut app, 'x');

    assert_eq!(last_copied(&app), Some("ab"));
    assert_eq!(picker(&app, entity).path(), "");
}

#[test]
fn a_clipboard_key_with_nothing_to_act_on_stays_with_the_picker() {
    let dir = scratch();
    let mut app = app();
    let parent = listening_parent(&mut app);
    spawn_picker_with(&mut app, dir.path(), ChildOf(parent));

    for character in ['c', 'x', 'v', 'h'] {
        ctrl(&mut app, character);
    }

    let heard: Vec<&Key> = app
        .world()
        .resource::<Heard>()
        .0
        .iter()
        .filter(|key| matches!(key, Key::Character(_)))
        .collect();
    assert_eq!(heard, [&Key::Character("h".into())], "only the unbound");
    assert_eq!(last_copied(&app), None);
}

#[test]
fn a_selection_is_drawn_in_place_of_the_caret() {
    let dir = scratch();
    let mut app = app();
    let underline = Style::new().add_modifier(Modifier::UNDERLINED);
    app.insert_resource(UiTheme::default().with_selection(underline));
    spawn_picker(&mut app, dir.path());
    type_text(&mut app, "ab");

    ctrl(&mut app, 'a');

    assert_selection_drawn(&app, "focused");

    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert_selection_drawn(&app, "without focus");
}
