//! A file picker widget for [plurimus](https://docs.rs/plurimus): one path
//! field over a filtered listing of the directory it names, built on
//! `plurimus_widgets`' public engine and drawn like any other widget.

use bevy_app::{App, Plugin, PreUpdate, Update};
use bevy_ecs::prelude::IntoScheduleConfigs;
use plurimus_ui::UiSystems;
use plurimus_ui::bevy_input_focus::InputFocusSystems;
use plurimus_widgets::{WidgetSystems, WidgetsPlugin};

mod keys;
mod layout;
mod listing;
mod matching;
mod parts;
mod path;
mod picker;
mod source;
mod style;

pub use keys::{FilePickerAction, FilePickerKeys};
pub use picker::{
    FilePicker, FilePickerDecorator, FilePickerFloor, FilePickerLook, FilePickerMatchStyle,
    RowDecoration, file_picker,
};
pub use source::{DirectorySource, FilePickerSource, SourceEntry};

/// Installs the file picker's systems and observers.
///
/// Requires [`plurimus_core::CorePlugin`] first; adds [`WidgetsPlugin`]
/// itself when absent.
#[derive(Debug, Default)]
pub struct FilePickerPlugin;

impl Plugin for FilePickerPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetsPlugin>() {
            app.add_plugins(WidgetsPlugin);
        }
        app.add_systems(
            PreUpdate,
            (
                (
                    parts::install_file_picker_list,
                    parts::mirror_look,
                    parts::redirect_focus,
                )
                    .chain()
                    .before(UiSystems::Areas)
                    .before(InputFocusSystems::Dispatch),
                // After a key edited the field, before the engine's row
                // passes and the stylists, so a keystroke lands this frame.
                (
                    listing::relist,
                    listing::rebuild_rows,
                    parts::place_file_picker_parts,
                )
                    .chain()
                    .after(InputFocusSystems::Dispatch)
                    .after(UiSystems::Areas)
                    .before(WidgetSystems::Layout),
            ),
        );
        app.add_systems(
            Update,
            style::style_file_pickers.in_set(WidgetSystems::Style),
        );
        app.add_observer(keys::file_picker_key);
        app.add_observer(keys::file_picker_paste);
        app.add_observer(keys::file_picker_press);
        app.add_observer(keys::file_picker_click);
    }
}

#[cfg(test)]
mod tests {
    use super::FilePickerPlugin;
    use bevy_app::App;
    use plurimus_core::CorePlugin;
    use plurimus_widgets::WidgetsPlugin;

    #[test]
    fn plugin_adds_widgets_when_absent() {
        let mut app = App::new();
        app.add_plugins((CorePlugin, FilePickerPlugin));
        assert!(app.is_plugin_added::<WidgetsPlugin>());
    }
}
