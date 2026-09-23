# plurimus_filepicker Architecture

`plurimus_filepicker` is a single crate: a file picker widget written against
plurimus's public widget engine, outside the plurimus workspace. It depends on
`plurimus_core`, `plurimus_ui`, `plurimus_widgets`, and `plurimus_term` at the
same version an app pulls through the `plurimus` facade, so cargo unifies them
and the picker's components sit beside the app's own widgets.

`FilePickerPlugin` installs the crate's systems and observers. It requires
`CorePlugin` first and adds `WidgetsPlugin` when absent.

## Entities

A picker is two entities the crate manages and rows it respawns:

- **The root**, spawned by the app with `file_picker(base)`: `FilePicker` plus
  `UiWidget`. `FilePicker` holds a `TextInput` and the base path; the field's
  text is the whole state. The root requires `Hovered`, `StylistCache`,
  `FilePickerKeys`, `TextInputKeys`, `FilePickerLook`, `FilePickerFloor`,
  `FilePickerMatchStyle`, `ComputedWidgetArea`, and two private components:
  `Listing`, the entries read for the directory last named, and `BuiltRows`, the
  filter the rows were last built for. The app gives the root its `UiArea`.
- **The list**, a child `ListBox` the crate spawns on the frame after the root
  appears, with `ListBoxKeys` pruned to Up, Down, PageUp and PageDown, a
  `ScrollArea`, and the `TabIndex` that `listbox()` carries. The root records it
  in a private `PickerList(Entity)`. The list holds focus: the engine's list
  keys act only on the focused `ListBox`, so the tab stop is the list, and
  `InputFocus` set to the root is redirected to it on the next frame.
- **Rows**, `list_item` children of the list, each carrying a private `Entry`
  naming the directory entry it stands for. With `FilePickerLook::accepts_new`,
  a filter that names a file rather than `.` or `..`, and no entry read with
  that exact name, one more row follows the matches: the filter as an `Entry`
  that is not a directory, badged `new` through `ListItemTrailing`, so `Enter`
  and `Complete` treat it as any file. A listing with nothing left has one dim
  "no match" row with no `Entry` and no cursor.

## Path model

`path.rs` is pure. `split_at_separator` cuts the field at its last separator,
either separator on Windows. `field_directory` expands a leading `~` through
`std::env::home_dir`, joins the directory text to the base when relative, and
normalizes lexically: `.` dropped, `..` popping the segment before it, held at a
root, accumulating below a relative start. The working directory resolves to the
empty path, which keeps prefix checks against a floor honest;
`FilePicker::directory` names it `.`. `directory_text` is the inverse for
writing a directory back into the field: relative beneath the base, climbing
with `..` when both are relative, absolute otherwise, always with a trailing
separator. The floor check is lexical; a symlink below the floor can lead out.

## Frame

In `PreUpdate`, before `UiSystems::Areas` and focus dispatch, chained:
`install_file_picker_list` on `Added<FilePicker>`, `mirror_disabled`, which
copies `InteractionDisabled` on and off the list because the engine does not
pass it to children, and `redirect_focus`.

After `InputFocusSystems::Dispatch` and `UiSystems::Areas`, before
`WidgetSystems::Layout`, chained, so a key that edited the field lands the same
frame ahead of the engine's row passes:

1. `relist` runs when the picker or its floor changed. It resolves the floor
   against the base the way the field is resolved, writes the field back to the
   floor when the directory falls outside it, and reads the directory with
   `std::fs::read_dir` only when it differs from the one last read. Entries sort
   directories first, then names case-insensitively; a directory that cannot be
   read lists nothing. An entry's kind comes from the directory read, with a
   stat only for symlinks.
2. `rebuild_rows` runs when the listing, look, match style, or filter changed.
   It despawns the old rows last child first, keeps entries the look admits
   (hidden ones only with `hidden`, files only with an extension in `extensions`
   when that list is set, directories always), ranks them through
   `matching::find_match` stably on score, spawns a row per hit with the matched
   characters styled and a separator suffix on a directory, dims hidden rows,
   appends the typed row when the look accepts a new name, and writes
   `ActiveDescendant` to the first row.
3. `place_file_picker_parts` runs when the root's area, order, or list changed.
   It cuts the root's area with `layout::split_area` into the one-row field and
   the rest, and writes the list's `ComputedWidgetArea`, `UiArea::Fixed` through
   `local_area`, and `UiOrder` one above the root's.

In `Update`, `style_file_pickers` in `WidgetSystems::Style` draws the root's
`UiWidget`: the prompt, the field's text, and the theme caret under the cursor
while the list holds focus, as a `Paragraph` scrolled so the caret stays in
view. It is gated by `StylistCache::redraws` over the field and the area's
width, and treats the picker as focused when its list is.

## Matching

`matching::find_match` is a case-insensitive subsequence match taking the
leftmost hit per filter character. The score is a start bonus for a hit at the
first character or a smaller one after `.`, `-`, `_` or a space, less the gaps
between hits. `light_matches` builds the row's line with hit runs in
`FilePickerMatchStyle`.

## Keys and events

Keys reach the root by bubbling from the focused list, which has already taken
its own. `file_picker_key`, a global `FocusedInput<KeyboardInput>` observer on a
`FilePicker` without `InteractionDisabled`, scans `FilePickerKeys` through
`first_bound`, then Backspace on an empty filter as `Parent`, then hands the key
to the field's `TextInput::handle` with the root's `TextInputKeys`. A bound key
is consumed; a key the field took is consumed and marks the picker changed;
anything else bubbles on.

`FilePickerAction`: `Parent` writes the parent of the listed directory through
`directory_text`; `Enter` completes a directory or triggers
`ValueChange<PathBuf>` on the root for a file, skipped on a key repeat;
`Complete` writes the cursor's entry into the field with a separator after a
directory; `ToggleHidden` flips `FilePickerLook::hidden`; `Close` triggers
`ModalDismiss { entity: root }`.

Two more global observers: `PointerPress` on the root focuses its list unless
the root has `PressFocusDisabled`, and a `Click` with count two or more on the
list applies `Enter` to the cursor's row.

## Tests

Tests drive a full `App` headlessly: `CorePlugin` and `FilePickerPlugin`, a
`TerminalCamera`, a `TerminalSize`, keys and mouse written as `plurimus_term`
messages, and the composed frame read from the render sub-app.
`tests/support/mod.rs` carries those helpers, copied from plurimus's unpublished
`plurimus_test`, plus a scratch directory from `tempfile`.
