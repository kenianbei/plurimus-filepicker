//! A file picker widget for [plurimus](https://docs.rs/plurimus): one path
//! field over a filtered listing of the directory it names, built on
//! `plurimus_widgets`' public engine and drawn like any other widget.

use bevy_app::{App, Plugin};
use plurimus_widgets::WidgetsPlugin;

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
