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
  unless `FilePickerLook::hidden`, files dropped unless their extension is in
  `FilePickerLook::extensions` when that is set. `..` lists first in any
  directory with a parent, filtered like any entry, and is all an unreadable
  directory lists. `FilePickerFloor` stops traversal above a directory by
  writing the field back to it; no `..` is listed there or at the root, and Left
  does nothing at either.
- With `FilePickerLook::accepts_new`, the filter is offered after the matches as
  a file to create, badged `new`, unless an entry already has that name; Enter
  on it chooses the path, so a save dialog can be a picker. With exactly one
  extension in `FilePickerLook::extensions`, a name typed without one is offered
  and checked with it: `copy` offers `copy.toml`, and offers nothing beside an
  existing `copy.toml`. A typed extension is kept as typed.
- `ListBoxCursor`, `ListBoxStripe` and `ListBoxSelectionMarker` given to a
  picker dress its list: they are copied onto the list when it is spawned and
  whenever they change on the picker, so an app themes the list at the picker's
  spawn. Removing one from the picker leaves the list as it was.
- `FilePickerDecorator` on a picker maps each file's path to a `RowDecoration`:
  a line drawn at the right of the row and a style laid over it, above the dim
  of a hidden file. It runs once per file each time a directory is read, never
  per keystroke, and inserting or replacing it reads the directory again.
  Directories, `..` and the typed new row are not decorated.
- Keys as data in `FilePickerKeys`: Enter descends or chooses, Tab and Right
  complete the cursor's entry, Left climbs, Ctrl+. toggles hidden entries,
  Escape triggers `ModalDismiss`. Unbound keys edit the field; Up, Down, PageUp
  and PageDown move the list's cursor. A double click chooses a row.
- Choosing a file triggers `ValueChange<PathBuf>` on the picker, once per press.
- `FilePickerPlugin` installs the systems and observers, adding `WidgetsPlugin`
  when absent, and `examples/files.rs` opens a picker over the current directory
  in a modal pane.
