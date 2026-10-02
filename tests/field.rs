//! The path field as a text field: a paste, the clipboard keys, and the
//! selection drawn.

mod support;

use bevy_app::App;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{On, ResMut, Resource};
use bevy_input::ButtonState;
use bevy_input::keyboard::{Key, KeyboardInput};
use plurimus_term::{KeyCode, LastCopied, ModifierKey};
use plurimus_ui::InteractionDisabled;
use plurimus_ui::bevy_input_focus::FocusedInput;
use support::{
    app, paste, picker, press_chord, press_key, row_texts, scratch, spawn_picker,
    spawn_picker_with, type_text,
};

/// The keys pressed that bubbled past the picker to its parent.
#[derive(Resource, Default)]
struct Heard(Vec<Key>);

fn ctrl(app: &mut App, character: char) {
    press_chord(app, ModifierKey::ControlLeft, KeyCode::Char(character));
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
    app.init_resource::<Heard>();
    let parent = app
        .world_mut()
        .spawn_empty()
        .observe(
            |input: On<FocusedInput<KeyboardInput>>, mut heard: ResMut<Heard>| {
                if input.input.state == ButtonState::Pressed {
                    heard.0.push(input.input.logical_key.clone());
                }
            },
        )
        .id();
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
