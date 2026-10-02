# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Breaking:** the crate requires plurimus 0.8. An app moves to plurimus 0.8
  with it; one left on 0.7 resolves two copies of plurimus, and the picker's
  components do not match the app's.
- A disabled picker's list no longer carries an `InteractionDisabled` copied
  from the picker. The list is disabled as anything inside a disabled widget is
  in plurimus 0.8, and carries `ComputedDisabled`; an app that read the marker
  on the list reads that.

### Fixed

- A listing opens at its top. One that replaced a scrolled listing kept its
  scroll, so climbing out of a long directory, entering one, or typing a filter
  could open with `../` scrolled off above the cursor.
- A picker given its area after its first frame, such as one in a box sized a
  frame late, or one shown after being hidden, opens at its top with its cursor
  showing. It opened scrolled past the cursor's row, with no row marked until a
  key was pressed. Showing a hidden picker therefore puts its list back at its
  cursor, not where the wheel last left it.
- A picker inside a disabled widget takes no keys. With `InteractionDisabled` on
  an ancestor and not on the picker itself, its list ignored the arrows but
  typing still edited the field and Enter still chose.

## [0.1.1] - 2026-09-27

### Added

- `FilePickerSource` on a picker names what it lists: any `DirectorySource`,
  whose `list` returns a directory's entries as `SourceEntry::file` and
  `SourceEntry::directory`, so a picker can browse a virtual workspace, such as
  one kept in a browser's storage, the way it browses the disk. The source is
  handed the path the field names; the decorator and `ValueChange<PathBuf>` see
  the source's paths. A source's `current_dir` resolves relative paths for the
  `..` row, and a source without one still climbs absolute paths. Inserting or
  replacing the source reads the directory again. A picker without one reads the
  disk as before, and `~` is the process's home either way.

### Fixed

- An absolute directory lists `..` and Left climbs from it even where the
  process has no working directory, as on `wasm32-unknown-unknown`.

## [0.1.0] - 2026-09-23

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
