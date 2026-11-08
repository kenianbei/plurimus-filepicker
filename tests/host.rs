//! A field a host dressed as upstream's text input, upstream's observers
//! running: the picker reads and writes its `EditableText`.

mod support;

use bevy_app::App;
use bevy_ecs::entity::Entity;
use bevy_input::keyboard::Key;
use bevy_text::{EditableText, TextEdit};
use bevy_ui_widgets::{TextInput, TextInputPlugin};
use bevy_window::Ime;
use plurimus_filepicker::FilePickerAction;
use plurimus_widgets::TextField;
use support::{
    app, command, cursor, field_of, list_of, press_key, row_texts, scratch, spawn_picker,
};

/// A picker over `base` whose field is edited through `EditableText`.
fn dressed_picker(app: &mut App, base: &std::path::Path) -> Entity {
    app.add_message::<Ime>().add_plugins(TextInputPlugin);
    let picker = spawn_picker(app, base);
    let field = field_of(app, picker);
    app.world_mut()
        .entity_mut(field)
        .remove::<TextField>()
        .insert((TextInput, EditableText::new("")));
    app.update();
    picker
}

fn editor_text(app: &App, picker: Entity) -> String {
    let field = field_of(app, picker);
    app.world()
        .get::<EditableText>(field)
        .unwrap()
        .value()
        .to_string()
}

fn set_editor_text(app: &mut App, picker: Entity, text: &str) {
    let field = field_of(app, picker);
    app.world_mut()
        .get_mut::<EditableText>(field)
        .unwrap()
        .editor_mut()
        .set_text(text);
    app.update();
}

#[test]
fn the_editors_text_lists_and_filters() {
    let dir = scratch();
    let mut app = app();
    let picker = dressed_picker(&mut app, dir.path());

    set_editor_text(&mut app, picker, "sub/");
    assert_eq!(row_texts(&app), ["../", "inner.rs"]);

    set_editor_text(&mut app, picker, "b.");
    assert_eq!(row_texts(&app), ["b.txt"]);
}

#[test]
fn actions_write_the_editor() {
    let dir = scratch();
    let mut app = app();
    let picker = dressed_picker(&mut app, dir.path());

    command(&mut app, picker, FilePickerAction::Enter);
    assert_eq!(editor_text(&app, picker), "sub/");
    assert_eq!(row_texts(&app), ["../", "inner.rs"]);

    command(&mut app, picker, FilePickerAction::Parent);
    assert_eq!(editor_text(&app, picker), "");
}

#[test]
fn arrows_step_the_list_not_upstreams_caret() {
    let dir = scratch();
    let mut app = app();
    let picker = dressed_picker(&mut app, dir.path());
    let list = list_of(&app, picker);
    let first = cursor(&app, list);

    press_key(&mut app, Key::ArrowDown);
    assert_ne!(cursor(&app, list), first);
    press_key(&mut app, Key::ArrowUp);
    assert_eq!(cursor(&app, list), first);

    let field = field_of(&app, picker);
    let pending = &app
        .world()
        .get::<EditableText>(field)
        .unwrap()
        .pending_edits;
    assert!(
        !pending
            .iter()
            .any(|edit| matches!(edit, TextEdit::Up(_) | TextEdit::Down(_))),
        "{pending:?}"
    );
}
