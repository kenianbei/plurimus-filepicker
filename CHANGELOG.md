# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `FilePicker`, spawned through `file_picker(base)`: one path field over a
  filtered listing of the directory it names. Everything before the field's last
  separator is the directory, joined to the base when relative; everything after
  it filters the entries with a case-insensitive subsequence match that lights
  the hits. `..`, `~`, and absolute paths are ordinary text, so the picker
  traverses above its base by typing.
- Directories list first, then names case-insensitively, hidden entries dropped
  unless `FilePickerLook::hidden`; a directory that cannot be read lists
  nothing. `FilePickerFloor` stops traversal above a directory by writing the
  field back to it.
- Keys as data in `FilePickerKeys`: Enter descends or chooses, Tab and Right
  complete the cursor's entry, Left and Backspace on an empty filter climb,
  Ctrl+H toggles hidden entries, Escape triggers `ModalDismiss`. Unbound keys
  edit the field; Up, Down, PageUp and PageDown move the list's cursor. A double
  click chooses a row.
- Choosing a file triggers `ValueChange<PathBuf>` on the picker, once per press.
- `FilePickerPlugin` installs the systems and observers, adding `WidgetsPlugin`
  when absent, and `examples/files.rs` opens a picker over the current directory
  in a modal pane.
