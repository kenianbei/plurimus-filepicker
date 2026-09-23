//! A file picker over the current directory in a modal pane. Type to
//! filter, Up/Down move, Enter descends or chooses, Left climbs, Tab
//! completes, Ctrl+. shows hidden entries, Esc or ctrl-c quits. The chosen
//! path is printed on exit.

use std::path::PathBuf;
use std::time::Duration;

use bevy_app::{App, AppExit, PreUpdate, ScheduleRunnerPlugin, Startup, Update};
use bevy_ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::{
    Commands, IntoScheduleConfigs, MessageReader, MessageWriter, On, Query, Res, ResMut, Resource,
    With, Without,
};
use bevy_input_focus::tab_navigation::TabGroup;
use bevy_input_focus::{FocusCause, InputFocus};
use plurimus_core::ratatui_core::layout::{Margin, Rect};
use plurimus_core::ratatui_core::text::Line;
use plurimus_core::{CorePlugin, TerminalCamera, TerminalSize, UiArea};
use plurimus_crossterm::CrosstermPlugin;
use plurimus_filepicker::{FilePicker, FilePickerPlugin, file_picker};
use plurimus_term::{KeyCode, KeyKind, KeyMessage};
use plurimus_ui::{ModalDismiss, ModalOpen, UiSystems, ValueChange};
use plurimus_widgets::{ListBoxCursor, Pane, pane};

const COLS: u16 = 60;
const ROWS: u16 = 16;
const TITLE: &str = " files ";
const CURSOR: &str = "▸ ";
const FRAME_TICK: Duration = Duration::from_millis(16);

#[derive(Resource, Default)]
struct Chosen(Option<PathBuf>);

fn main() {
    let mut app = App::new();
    app.add_plugins((
        ScheduleRunnerPlugin::run_loop(FRAME_TICK),
        CorePlugin,
        CrosstermPlugin::default(),
        FilePickerPlugin,
    ));
    app.init_resource::<Chosen>();
    app.add_systems(Startup, spawn);
    app.add_systems(PreUpdate, centre.before(UiSystems::Areas));
    app.add_systems(Update, quit_on_ctrl_c);
    app.add_observer(choose);
    app.add_observer(dismiss);
    app.run();
    let chosen = app.world_mut().remove_resource::<Chosen>();
    // The terminal is restored when the app drops; print after it.
    drop(app);
    match chosen.and_then(|chosen| chosen.0) {
        Some(path) => println!("{}", path.display()),
        None => println!("nothing chosen"),
    }
}

fn spawn(mut commands: Commands, mut focus: ResMut<InputFocus>) {
    commands.spawn(TerminalCamera::default());
    let frame = commands
        .spawn((
            pane(TITLE),
            ModalOpen,
            TabGroup::modal(),
            UiArea::Fixed(Rect::ZERO),
        ))
        .id();
    let picker = commands
        .spawn((
            file_picker("."),
            ListBoxCursor(Line::from(CURSOR)),
            UiArea::Fixed(Rect::ZERO),
            ChildOf(frame),
        ))
        .id();
    focus.set(picker, FocusCause::Navigated);
}

fn centre(
    size: Res<TerminalSize>,
    mut panes: Query<&mut UiArea, (With<Pane>, Without<FilePicker>)>,
    mut pickers: Query<&mut UiArea, With<FilePicker>>,
) {
    if !size.is_changed() {
        return;
    }
    let cols = COLS.min(size.cols);
    let rows = ROWS.min(size.rows);
    let outer = Rect::new((size.cols - cols) / 2, (size.rows - rows) / 2, cols, rows);
    for mut area in &mut panes {
        area.set_if_neq(UiArea::Fixed(outer));
    }
    for mut area in &mut pickers {
        area.set_if_neq(UiArea::Fixed(outer.inner(Margin::new(1, 1))));
    }
}

fn choose(
    change: On<ValueChange<PathBuf>>,
    mut chosen: ResMut<Chosen>,
    mut exit: MessageWriter<AppExit>,
) {
    chosen.0 = Some(change.value.clone());
    exit.write(AppExit::Success);
}

// Esc on the picker and a click outside the pane both land here.
fn dismiss(_dismissed: On<ModalDismiss>, mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}

fn quit_on_ctrl_c(mut keys: MessageReader<KeyMessage>, mut exit: MessageWriter<AppExit>) {
    for key in keys.read() {
        if key.kind == KeyKind::Press && key.modifiers.ctrl && key.code == KeyCode::Char('c') {
            exit.write(AppExit::Success);
        }
    }
}
