//! The path field as a text field: a paste, the clipboard keys, and the
//! selection drawn.

mod support;

use plurimus_term::{KeyCode, ModifierKey};
use plurimus_ui::InteractionDisabled;
use support::{
    app, paste, picker, press_chord, row_texts, scratch, spawn_picker, spawn_picker_with, type_text,
};

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
    press_chord(&mut app, ModifierKey::ControlLeft, KeyCode::Char('a'));

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
