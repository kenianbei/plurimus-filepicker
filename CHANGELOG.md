# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- The picker's parts are entities a host can reach. `FilePickerParts` on the
  picker names its field and its list, marked `FilePickerField` and
  `FilePickerList`; `FilePicker::new(base).with_path(..)` opens the field on a
  path; `FilePickerCommand`, an entity event carrying a `FilePickerAction`,
  applies an action from outside as the bound key would, so a host's own double
  click on a row can be `Enter`.
- The field publishes its caret to the terminal, so screen readers and input
  methods see it, and takes the pointer: a press places the caret, a drag or a
  shifted press selects, a double press takes a word and a triple everything.
  The caret covers a grapheme cluster at its width.
- A host may dress the field as upstream's text input instead, in a window or on
  a plurimus node: with `TextField` removed and `bevy_ui_widgets::TextInput`
  added, the picker reads and writes the field's `EditableText`, which wins over
  its `TextInput` when both are there. Up and Down stay the picker's there: the
  picker drops the caret move that field makes for them and steps the list. The
  field keeps Left, Right, Home and End for its caret, so the picker's Left and
  Right do not reach the picker there; a host rebinds them in `FilePickerKeys`
  or triggers `FilePickerCommand`.

### Changed

- **Breaking:** the path field is a child entity carrying plurimus's
  `TextField`, edited and drawn by the engine, and it holds the picker's focus
  and tab stop where the list did. `FilePicker::path` and `set_path` are gone:
  read and write the field's `TextInput` through `FilePickerParts`.
  `FilePicker::directory` and `filter` take the field's text. Focus given to the
  picker still lands inside it, on the field; a press on the picker or a row
  focuses the field too.
- The field's keys are the picker's `TextInputKeys` less every key
  `FilePickerKeys` binds, copied onto the field whenever either changes, so a
  key bound in both stays the picker's.
- The list's keys are the picker's: `FilePickerAction` gains `Up`, `Down`,
  `PageUp` and `PageDown`, bound by default in `FilePickerKeys` to the keys of
  those names and applicable through `FilePickerCommand`. Up and Down wrap from
  one end of the list to the other, as a plurimus list's arrows do; PageUp and
  PageDown move by the list's height and stop at the ends. A list key that moves
  nothing - an arrow with the cursor on the only row, a page key at the end it
  moves towards - bubbles on to the picker's ancestors, where before Up on the
  first row and Down on the last did.
- A held Enter acts once: a key repeat no longer descends again into the
  directory the first press opened.
- The field's selection ends when it loses focus, as every plurimus field's
  does, including through a focus set on the picker itself, which passes through
  the field; it stayed before.
- **Breaking:** the crate requires plurimus 0.10, which runs on bevy's own
  picking, input and list models, and it depends on `bevy_text` 0.20 directly.
  An app moves to plurimus 0.10 with it. An app left on 0.9 resolves two copies
  of plurimus, and the picker's components do not match the app's. Rust 1.97.1
  is still enough. A press and a double click on the list are `bevy_picking`'s,
  with click counts per widget on the wall clock. Only a primary double click
  chooses, as only the primary button moves a plurimus list's cursor.

## [0.3.0] - 2026-10-27

### Changed

- **Breaking:** the crate requires plurimus 0.9, which is built on bevy 0.20,
  and Rust 1.97.1. An app moves to plurimus 0.9 and bevy 0.20 with it. If an app
  stays on plurimus 0.8, cargo resolves two copies of plurimus, and the picker's
  components do not match the app's.
- The published crate no longer ships the repository's CI workflow, lint and
  formatter configs, or toolchain file. The source, tests, example and docs
  still ship.

## [0.2.0] - 2026-10-01

### Added

- A paste from the terminal goes into the path field at the caret, in place of
  any selection, with control characters dropped. A picker took no bracketed
  paste before.
- The field's selection is drawn, in `UiTheme::selection`, with the caret left
  out while something is selected, and stays drawn when the picker loses focus.
  plurimus 0.8's default `TextInputKeys` select with Shift and Left, Right, Home
  or End, with Ctrl+Shift and Left or Right, and with Ctrl+A; the next character
  typed replaces what is selected.
- Ctrl+C and Ctrl+X copy and cut the field's selection as a `TerminalRequest`,
  and Ctrl+V inserts `LastCopied`, what a widget of the app last copied.
  `plurimus_crossterm` writes a copy to the terminal's clipboard only with
  `CrosstermPlugin::clipboard` set, which `examples/files.rs` now does.

### Changed

- **Breaking:** the crate requires plurimus 0.8. An app moves to plurimus 0.8
  with it; one left on 0.7 resolves two copies of plurimus, and the picker's
  components do not match the app's.
- Up, Down, PageUp and PageDown leave the picker when they cannot move the
  cursor: Up or PageUp on the first row, Down or PageDown on the last. Under
  plurimus 0.7 the list consumed them there; under 0.8 they bubble to the
  picker's ancestors as from any list, so a fresh arrow press at an end moves
  focus to the widget beyond it, within the modal when the picker is in one. A
  held arrow still stops at the end. An app that wants its picker to keep them
  observes `FocusedInput<KeyboardInput>` on the picker and stops them there.
- A picker inside a disabled widget is disabled, as anything inside one is in
  plurimus 0.8: with `InteractionDisabled` on an ancestor it takes no keys,
  where under 0.7 the marker counted only on the picker itself. Its list no
  longer carries an `InteractionDisabled` copied from the picker and carries
  `ComputedDisabled` instead; an app that read the marker on the list reads
  that.
- Ctrl+C, Ctrl+X and Ctrl+V stay with a focused picker, whether or not there was
  anything to copy or paste; they reached the picker's ancestors before. An app
  that acted on one bubbling from a picker takes the binding off the picker's
  `TextInputKeys`. An app that reads `KeyMessage` itself for Ctrl+C still sees
  the key, and now acts on one that also copies; `examples/files.rs` quits on
  Ctrl+Q so that its Ctrl+C is only a copy.

### Fixed

- A listing opens at its top. One that replaced a scrolled listing kept its
  scroll, so climbing out of a long directory, entering one, or typing a filter
  could open with `../` scrolled off above the cursor.
- A picker given its area after its first frame, such as one in a box sized a
  frame late, or one shown after being hidden, opens at its top with its cursor
  showing. It opened scrolled past the cursor's row, with no row marked until a
  key was pressed. Showing a hidden picker therefore puts its list back at its
  cursor, not where the wheel last left it.

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
