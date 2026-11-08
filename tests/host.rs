//! A field a host dressed as upstream's text input: the picker reads and
//! writes its `EditableText`.

mod support;

use bevy_app::App;
use bevy_ecs::entity::Entity;
use bevy_text::EditableText;
use plurimus_filepicker::{FilePickerAction, FilePickerCommand};
use plurimus_widgets::TextField;
use support::{app, field_of, row_texts, scratch, spawn_picker};

/// A picker over `base` whose field is edited through `EditableText`.
fn dressed_picker(app: &mut App, base: &std::path::Path) -> Entity {
    let picker = spawn_picker(app, base);
    let field = field_of(app, picker);
    app.world_mut()
        .entity_mut(field)
        .remove::<TextField>()
        .insert(EditableText::new(""));
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

fn command(app: &mut App, entity: Entity, action: FilePickerAction) {
    app.world_mut()
        .trigger(FilePickerCommand { entity, action });
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
