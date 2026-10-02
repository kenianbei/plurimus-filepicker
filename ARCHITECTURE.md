# plurimus_filepicker Architecture

`plurimus_filepicker` is a single crate: a file picker widget written against
plurimus's public widget engine, outside the plurimus workspace. It depends on
`plurimus_core`, `plurimus_ui`, `plurimus_widgets`, and `plurimus_term` at the
same version an app pulls through the `plurimus` facade, so cargo unifies them
and the picker's components sit beside the app's own widgets. `bevy_input_focus`
is not a dependency of its own: the crate reaches it through `plurimus_ui`'s
re-export, at the version the engine uses.

`FilePickerPlugin` installs the crate's systems and observers. It requires
`CorePlugin` first and adds `WidgetsPlugin` when absent.

## Entities

A picker is two entities the crate manages and rows it respawns:

- **The root**, spawned by the app with `file_picker(base)`: `FilePicker` plus
  `UiWidget`. `FilePicker` holds a `TextInput` and the base path; the field's
  text is the whole state. The root requires `Hovered`, `StylistCache`,
  `FilePickerKeys`, `TextInputKeys`, `FilePickerLook`, `FilePickerFloor`,
  `FilePickerMatchStyle`, `ComputedWidgetArea`, and two private components:
  `Listing`, the entries read for the directory last named, each paired with its
  decoration, and whether that directory has a parent to climb to, and
  `BuiltRows`, the filter the rows were last built for. The app gives the root
  its `UiArea`, and may add `FilePickerDecorator`, a function from a file's path
  to a `RowDecoration`, and `FilePickerSource`, what the picker lists.
- **The list**, a child `ListBox` the crate spawns in the first `PreUpdate`
  after the root appears, with `ListBoxKeys` pruned to Up, Down, PageUp and
  PageDown, a `ScrollArea`, and the `TabIndex` that `listbox()` carries. The
  root records it in a private `PickerList(Entity)`. The list holds focus: the
  engine's list keys act only on the focused `ListBox`, so the tab stop is the
  list, and `InputFocus` set to the root is redirected to it on the next frame.
  `ListBoxCursor`, `ListBoxStripe` and `ListBoxSelectionMarker` on the root are
  copied onto the list when it is installed and whenever they change on the
  root; a removal is not copied.
- **Rows**, `list_item` children of the list, each carrying a private `Entry`
  naming the directory entry it stands for. Where the listed directory has a
  parent, `..` is a directory `Entry` ahead of the entries read, drawn `../`,
  matched, lit and filtered like the rest and never hidden; the cursor opens on
  the first row after it. With `FilePickerLook::accepts_new`, a filter that
  names a file rather than `.` or `..` is a typed name, the one listed extension
  appended when `extensions` has exactly one and the filter has none. With no
  entry read of that exact name, one more row follows the matches: the typed
  name as an `Entry` that is not a directory, badged `new` through
  `ListItemTrailing`, so `Enter` and `Complete` treat it as any file. A file's
  row carries its decoration's trailing line as `ListItemTrailing` and its
  style, over the hidden dim, as `UiStyle`. A listing with nothing left has one
  dim "no match" row with no `Entry` and no cursor.

## Path model

`path.rs` is lexical but for `~`. `split_at_separator` cuts the field at its
last separator, either separator on Windows. `field_directory` expands a leading
`~` through `std::env::home_dir`, joins the directory text to the base when
relative, and normalizes lexically: `.` dropped, `..` popping the segment before
it, held at a root, accumulating below a relative start. The working directory
resolves to the empty path, which keeps prefix checks against a floor honest;
`FilePicker::directory` names it `.`. `has_parent` judges a directory on its
absolute form: an absolute directory as it is, a relative one resolved against
the working directory it is handed, so a `..` chain below a relative base ends
at the real root; with none, a relative directory has no parent.
`directory_text` is the inverse for writing a directory back into the field:
relative beneath the base, climbing with `..` when both are relative, absolute
otherwise, always with a trailing separator. The floor check is lexical; a
symlink below the floor can lead out.

## Sources

`source.rs` holds what a picker lists. `DirectorySource` is a trait with `list`,
a directory's `SourceEntry`s in any order as an `io::Result`, and `current_dir`,
defaulting to `std::env::current_dir`, which `has_parent` resolves a relative
directory against. `SourceEntry` is `#[non_exhaustive]`, a name and whether it
is a directory, built through `SourceEntry::file` and `SourceEntry::directory`.
`FilePickerSource(Arc<dyn DirectorySource>)` on the root is the picker's source;
without it, the private `DiskSource` reads with `std::fs::read_dir`, an entry's
kind from the directory read with a stat only for symlinks. `~` is not the
source's: it expands through `std::env::home_dir` in the path model, which
`FilePicker::directory` shares.

## Frame

In `PreUpdate`, before `UiSystems::Areas` and focus dispatch, chained:
`install_file_picker_list` on `Added<FilePicker>`, `mirror_look`, which copies
the list's look components from the root on `Added<PickerList>` or a change, and
`redirect_focus`.

After `InputFocusSystems::Dispatch` and `UiSystems::Areas`, before
`WidgetSystems::Layout`, chained, so a key that edited the field lands the same
frame ahead of the engine's row passes:

1. `relist` runs when the picker, its floor, its decorator, or its source
   changed; a changed decorator or source marks the directory unread, and a
   removed one goes unnoticed until the next read. It resolves the floor against
   the base the way the field is resolved, writes the field back to the floor
   when the directory falls outside it, and stops there when the directory is
   the one last read and the floor did not change. Otherwise it records whether
   the directory has a parent, `has_parent` over the source's `current_dir` and
   not the floor, and lists the directory through the source, or `DiskSource`,
   when it differs from the one last read. The source is handed the resolved
   directory, `.` for the working directory, and the decorator, when there is
   one, that directory joined with each file's name. Entries sort directories
   first, then names case-insensitively; a directory the source cannot list has
   no entries.
2. `rebuild_rows` runs when the listing, look, match style, or filter changed.
   It despawns the old rows last child first, puts `..` ahead of the entries
   when the listing has a parent, keeps entries the look admits (hidden ones
   only with `hidden`, files only with an extension in `extensions` when that
   list is set, directories always), ranks them through `matching::find_match`
   stably on score, spawns a row per hit with the matched characters styled and
   a separator suffix on a directory, dims hidden rows, lays a file's decoration
   over its row, appends the typed row when the look accepts a new name, writes
   `ActiveDescendant` to the first row that is not `..`, or to the first row,
   and puts the list's `ScrollOffset` back at the origin through `apply_offset`,
   so a scroll from the rows it replaced does not carry over; the engine's
   reveal of the cursor's row still follows in `WidgetSystems::Layout`.
3. `place_file_picker_parts` runs when the root's area, order, or list changed.
   It cuts the root's area with `layout::split_area` into the one-row field and
   the rest, and writes the list's `ComputedWidgetArea`, `UiArea::Fixed` through
   `local_area`, and `UiOrder` one above the root's. Where that area goes from
   empty to having cells, a picker placed after its rows or shown after being
   hidden, it first puts the list's `ScrollOffset` back at the origin through
   `apply_offset` and marks `ActiveDescendant` changed: the engine reveals a
   cursor into a list with no area and leaves the offset past it, and reveals
   again only when the cursor changes.

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

Keys reach the root by bubbling from the focused list. The list consumes one of
its four only when it moves the cursor, so Up on the first row and Down on the
last reach the root too, which binds neither, and bubble on to the picker's
ancestors. `file_picker_key`, a global `FocusedInput<KeyboardInput>` observer on
a `FilePicker` without `ComputedDisabled`, scans `FilePickerKeys` through
`first_bound`, then hands the key to the field's `TextInput::handle` with the
root's `TextInputKeys`. A bound key is consumed; a key the field took is
consumed and marks the picker changed; anything else bubbles on. The engine
resolves `ComputedDisabled` onto the root and its list from an
`InteractionDisabled` on the root or any ancestor, so the crate copies nothing
to the list, and a disabled picker's keys pass both and bubble on.

`FilePickerAction`: `Parent` writes the parent of the listed directory through
`directory_text` when the listing has one, and nothing at a root or the floor;
`Enter` completes a directory or triggers `ValueChange<PathBuf>` on the root for
a file, skipped on a key repeat; `Complete` writes the cursor's entry into the
field with a separator after a directory; `ToggleHidden` flips
`FilePickerLook::hidden`; `Close` triggers `ModalDismiss { entity: root }`.

Two more global observers: `PointerPress` on the root focuses its list unless
the root has `PressFocusDisabled`, and a `Click` with count two or more on the
list applies `Enter` to the cursor's row.

## Tests

Tests drive a full `App` headlessly: `CorePlugin` and `FilePickerPlugin`, a
`TerminalCamera`, a `TerminalSize`, keys and mouse written as `plurimus_term`
messages, and the composed frame read from the render sub-app.
`tests/support/mod.rs` carries those helpers, copied from plurimus's unpublished
`plurimus_test`, plus two scratch directories from `tempfile`, one of them with
more entries than the list has rows, and `spawn_picker_with`, which spawns a
picker with more components from its first frame. `tests/source.rs` lists an
in-memory `DirectorySource` with no working directory; the `tempfile` tests are
the disk's.
