//! The pieces of plurimus's unpublished `plurimus_test` a headless picker
//! test needs: an app, input written as a backend would, and the composed
//! frame read from the render sub-app.

#![allow(dead_code)]

use core::fmt::Write as _;
use std::path::Path;

use bevy_app::App;
use bevy_ecs::bundle::Bundle;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{On, ResMut, Resource};
use bevy_input::ButtonState;
use bevy_input::keyboard::{Key, KeyboardInput};
use plurimus_core::ratatui_core::buffer::Buffer;
use plurimus_core::ratatui_core::layout::{Position, Rect};
use plurimus_core::ratatui_core::style::Style;
use plurimus_core::{CorePlugin, FrameBuffer, TerminalCamera, TerminalRenderApp, TerminalSize};
use plurimus_filepicker::{FilePicker, FilePickerPlugin, file_picker};
use plurimus_term::{
    KeyCode, KeyKind, KeyMessage, KeyModifiers, ModifierKey, MouseButton, MouseKind, MouseMessage,
    PasteMessage,
};
use plurimus_ui::UiArea;
use plurimus_ui::bevy_input_focus::{FocusCause, FocusedInput, InputFocus};
use plurimus_widgets::{ActiveDescendant, ListBox};

pub const COLS: u16 = 20;
pub const ROWS: u16 = 8;

pub fn app() -> App {
    let mut app = App::new();
    app.add_plugins((CorePlugin, FilePickerPlugin));
    app.insert_resource(TerminalSize::new(COLS, ROWS));
    app.world_mut().spawn(TerminalCamera::default());
    app
}

/// A picker over `base` filling the terminal, focused, its list spawned.
pub fn spawn_picker(app: &mut App, base: &Path) -> Entity {
    spawn_picker_with(app, base, ())
}

/// [`spawn_picker`] with `extra` on the picker from its first frame.
pub fn spawn_picker_with(app: &mut App, base: &Path, extra: impl Bundle) -> Entity {
    let picker = app
        .world_mut()
        .spawn((
            file_picker(base),
            UiArea::Fixed(Rect::new(0, 0, COLS, ROWS)),
            extra,
        ))
        .id();
    focus(app, picker);
    app.update();
    picker
}

/// The keys pressed that bubbled to a [`listening_parent`].
#[derive(Resource, Default)]
pub struct Heard(pub Vec<Key>);

/// An entity recording in [`Heard`] each key press that reaches it, to spawn
/// a picker under.
pub fn listening_parent(app: &mut App) -> Entity {
    app.init_resource::<Heard>();
    app.world_mut()
        .spawn_empty()
        .observe(
            |input: On<FocusedInput<KeyboardInput>>, mut heard: ResMut<Heard>| {
                if input.input.state == ButtonState::Pressed {
                    heard.0.push(input.input.logical_key.clone());
                }
            },
        )
        .id()
}

pub fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Pressed);
}

pub fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

pub fn set_path(app: &mut App, picker: Entity, path: &str) {
    app.world_mut()
        .get_mut::<FilePicker>(picker)
        .unwrap()
        .set_path(path);
    app.update();
}

pub fn picker(app: &App, entity: Entity) -> &FilePicker {
    app.world().get::<FilePicker>(entity).unwrap()
}

pub fn cursor(app: &App, list: Entity) -> Option<Entity> {
    app.world().get::<ActiveDescendant>(list).unwrap().0
}

pub fn list_of(app: &mut App, picker: Entity) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, bevy_ecs::prelude::With<ListBox>>()
        .iter(app.world())
        .find(|&list| {
            app.world()
                .get::<bevy_ecs::hierarchy::ChildOf>(list)
                .is_some_and(|child_of| child_of.parent() == picker)
        })
        .expect("the picker spawned its list")
}

/// A scratch directory with `sub/`, `Zeta/`, `a.txt`, `b.txt`, `.hidden`.
pub fn scratch() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a scratch directory");
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    std::fs::create_dir(dir.path().join("Zeta")).unwrap();
    std::fs::write(dir.path().join("sub").join("inner.rs"), "").unwrap();
    for name in ["a.txt", "b.txt", ".hidden"] {
        std::fs::write(dir.path().join(name), "").unwrap();
    }
    dir
}

/// The files in [`tall_scratch`]'s `deep/`: more than the list has rows.
pub const DEEP_FILES: usize = 12;
const SHALLOW_FILES: usize = 8;

/// A scratch directory taller than the list either way: `deep/` holding
/// `p00.txt` to `p11.txt`, beside `q0.txt` to `q7.txt`.
pub fn tall_scratch() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let deep = dir.path().join("deep");
    std::fs::create_dir(&deep).unwrap();
    for index in 0..DEEP_FILES {
        std::fs::write(deep.join(format!("p{index:02}.txt")), "").unwrap();
    }
    for index in 0..SHALLOW_FILES {
        std::fs::write(dir.path().join(format!("q{index}.txt")), "").unwrap();
    }
    dir
}

fn press_key_kind(app: &mut App, code: KeyCode, modifiers: KeyModifiers, kind: KeyKind) {
    app.world_mut()
        .write_message(KeyMessage::new(code, modifiers, kind));
    app.update();
}

pub fn press_key(app: &mut App, code: KeyCode) {
    press_key_kind(app, code, KeyModifiers::default(), KeyKind::Press);
}

pub fn repeat_key(app: &mut App, code: KeyCode) {
    press_key_kind(app, code, KeyModifiers::default(), KeyKind::Repeat);
}

pub fn type_text(app: &mut App, text: &str) {
    for ch in text.chars() {
        press_key(app, KeyCode::Char(ch));
    }
}

/// A chord presses and releases the modifier itself: nothing expires a
/// hold on the kitty tier the headless app claims.
pub fn press_chord(app: &mut App, modifier: ModifierKey, code: KeyCode) {
    let modifier_code = KeyCode::Modifier(modifier);
    let held = KeyModifiers::from(modifier);
    let none = KeyModifiers::default();
    press_key_kind(app, modifier_code, held, KeyKind::Press);
    press_key_kind(app, code, held, KeyKind::Press);
    press_key_kind(app, code, held, KeyKind::Release);
    press_key_kind(app, modifier_code, none, KeyKind::Release);
}

/// A bracketed paste, as a terminal sends one.
pub fn paste(app: &mut App, text: &str) {
    app.world_mut().write_message(PasteMessage(text.to_owned()));
    app.update();
}

pub fn send_mouse(app: &mut App, kind: MouseKind, x: u16, y: u16) {
    app.world_mut().write_message(MouseMessage::new(
        kind,
        Position::new(x, y),
        KeyModifiers::default(),
    ));
    app.update();
}

pub fn click(app: &mut App, x: u16, y: u16) {
    send_mouse(app, MouseKind::Moved, x, y);
    send_mouse(app, MouseKind::Down(MouseButton::Left), x, y);
    send_mouse(app, MouseKind::Up(MouseButton::Left), x, y);
}

/// The composed frame as one line of cell symbols per row.
pub fn composed_frame(app: &App) -> String {
    frame_to_string(&composed_buffer(app).0)
}

/// The frame's rows, trailing blanks trimmed.
pub fn rows(app: &App) -> Vec<String> {
    composed_frame(app)
        .lines()
        .map(|line| line.trim_end().to_owned())
        .collect()
}

/// The list's rows without the two-cell cursor gutter the engine draws.
pub fn row_texts(app: &App) -> Vec<String> {
    rows(app)
        .into_iter()
        .skip(1)
        .filter(|row| !row.is_empty())
        .map(|row| row.chars().skip(2).collect())
        .collect()
}

/// The composed frame as symbols plus a style map: a letter grid over a
/// legend of distinct non-default styles, `.` marking the default.
pub fn composed_styled_frame(app: &App) -> String {
    frame_to_styled_string(&composed_buffer(app).0)
}

fn composed_buffer(app: &App) -> &FrameBuffer {
    app.sub_app(TerminalRenderApp)
        .world()
        .resource::<FrameBuffer>()
}

fn frame_to_string(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut lines = Vec::with_capacity(area.height as usize);
    for y in area.top()..area.bottom() {
        let mut line = String::with_capacity(area.width as usize);
        for x in area.left()..area.right() {
            if let Some(cell) = buffer.cell((x, y)) {
                line.push_str(cell.symbol());
            }
        }
        lines.push(line);
    }
    lines.join("\n")
}

fn frame_to_styled_string(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut legend: Vec<Style> = Vec::new();
    let mut symbol_rows = Vec::with_capacity(area.height as usize);
    let mut style_rows = Vec::with_capacity(area.height as usize);
    for y in area.top()..area.bottom() {
        let mut symbols = String::new();
        let mut styles = String::new();
        for x in area.left()..area.right() {
            if let Some(cell) = buffer.cell((x, y)) {
                symbols.push_str(cell.symbol());
                styles.push(style_letter(cell.style(), &mut legend));
            }
        }
        symbol_rows.push(symbols);
        style_rows.push(styles);
    }
    let mut out = symbol_rows.join("\n");
    out.push_str("\n--\n");
    out.push_str(&style_rows.join("\n"));
    for (index, style) in legend.iter().enumerate() {
        let _ = write!(
            out,
            "\n{}: fg:{:?} bg:{:?} mods:{:?}",
            letter(index),
            style.fg,
            style.bg,
            style.add_modifier
        );
    }
    out
}

fn style_letter(style: Style, legend: &mut Vec<Style>) -> char {
    if style == Style::default() {
        return '.';
    }
    let index = legend
        .iter()
        .position(|known| *known == style)
        .unwrap_or_else(|| {
            legend.push(style);
            legend.len() - 1
        });
    letter(index)
}

fn letter(index: usize) -> char {
    char::from(b'a' + u8::try_from(index % 26).unwrap_or(0))
}
