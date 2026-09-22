# plurimus_filepicker Architecture

`plurimus_filepicker` is a single crate: a file picker widget written against
plurimus's public widget engine, outside the plurimus workspace. It depends on
`plurimus_core`, `plurimus_ui`, and `plurimus_widgets` at the same version an
app pulls through the `plurimus` facade, so cargo unifies them and the picker's
components sit beside the app's own widgets.

`FilePickerPlugin` installs the crate's systems and observers. It requires
`CorePlugin` first and adds `WidgetsPlugin` when absent.

The crate is a skeleton at this point: the plugin installs nothing beyond its
requirement, and the widget itself is the first feature to land.
