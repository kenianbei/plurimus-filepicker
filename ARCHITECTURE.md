# plurimus_filepicker Architecture

`plurimus_filepicker` is a single crate: a file picker widget written against
plurimus's public widget engine, outside the plurimus workspace. It depends on
`plurimus_core`, `plurimus_ui`, `plurimus_widgets`, and `plurimus_term` at the
same version an app pulls through the `plurimus` facade, so cargo unifies them
and the picker's components sit beside the app's own widgets. `bevy_input_focus`
is not a dependency of its own: the crate reaches it through `plurimus_ui`'s
re-export, at the version the engine uses. `bevy_text` is, at the 0.20 that
`bevy_ui_widgets` already pulls in, for the `EditableText` of a field a host
dressed as upstream's text input. The crate builds against the plurimus after
0.9.0 that runs on `bevy_picking` and `bevy_ui_widgets`' models, through the
`[patch.crates-io]` paths to `../plurimus`.

`FilePickerPlugin` installs the crate's systems and observers. It requires
`CorePlugin` first and adds `WidgetsPlugin` when absent.

## Entities

A picker is three entities and rows the crate respawns:

- **The root**, spawned by the app with `file_picker(base)`: `FilePicker` plus
  `UiWidget`. `FilePicker` holds the base path and the text the field opens
  with, `with_path`, read once when the field is spawned; `directory` and
  `filter` take the field's text. The root requires `Hovered`, `StylistCache`,
  `FilePickerKeys`, `TextInputKeys`, `FilePickerLook`, `FilePickerFloor`,
  `FilePickerMatchStyle`, `ComputedWidgetArea`, and two private components:
  `Listing`, the entries read for the directory last named, each paired with its
  decoration, and whether that directory has a parent to climb to, and
  `BuiltRows`, the filter the rows were last built for. The app gives the root
  its `UiArea`, and may add `FilePickerDecorator`, a function from a file's path
  to a `RowDecoration`, and `FilePickerSource`, what the picker lists. The crate
  inserts `FilePickerParts { field, list }` in the first `PreUpdate` after the
  root appears. The root's `UiWidget` draws the prompt alone.
- **The field**, a child spawned beside the list: `text_field` over the opening
  text, so `TextField`, `TextInput`, `TabIndex(0)` and a `UiWidget`, plus the
  public marker `FilePickerField`, a `UiArea::Fixed` and a `UiOrder`. The
  field's text is the whole state. The engine edits it: `text_input_key` through
  the field's `TextInputKeys`, `text_input_paste`, the clipboard through
  `Clipboard`, and the pointer through `text_input_press` and `text_input_drag`;
  it emits `ValueChange<String>`, clears the selection in `text_input_blur` on
  `FocusLost`, draws the row in `style_text_inputs` and publishes the caret's
  cell in `WidgetCursor`. The picker reads and writes the text through the
  private `field::FieldText`: the field's `EditableText` when it carries one,
  borrowed unless an IME preedit splits it and written with `set_text` and a
  queued `TextEdit::TextEnd`, and its `TextInput` otherwise, written whole. A
  host dresses the field as upstream's text input by removing `TextField` and
  adding `bevy_ui_widgets::TextInput`, whose global observer takes the arrows,
  Home and End for the caret and stops them. `field::reclaim_list_step`,
  observed on the field itself and so run after that global observer, takes back
  a key the root's `FilePickerKeys` binds as a list step: unless an IME is
  composing, it pops a trailing `TextEdit::Up` or `Down` from the editor's
  pending edits and lets the key propagate to the root. The field's
  `TextInputKeys` is the root's less every `KeyBinding` the root's
  `FilePickerKeys` holds, written on `Added<FilePickerParts>` and whenever
  either changes. The field holds focus and is the picker's tab stop.
- **The list**, a child spawned from `listbox()` with its `TabIndex` removed, so
  `ListBox`, which requires `ActiveDescendant`, with `Hovered`, `StylistCache`,
  `ComputedWidgetArea` and a `UiWidget`, and carrying the public marker
  `FilePickerList`, `PressFocusDisabled`, a `ScrollArea`, a `UiArea::Fixed` and
  a `UiOrder`. It binds no keys: upstream's list keys act on a focused list, and
  this one never holds focus. `ListBoxCursor`, `ListBoxStripe` and
  `ListBoxSelectionMarker` on the root are copied onto the list on
  `Added<FilePickerParts>` and whenever they change on the root; a removal is
  not copied.
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
`install_file_picker_parts` on `Added<FilePicker>`, `mirror_list_look`,
`mirror_field_keys`, and `redirect_focus`, which moves `InputFocus` from a root
to its field with `FocusCause::Navigated`. The field loses focus for that frame
and regains it, so a focus set on the root ends the field's selection.

After `InputFocusSystems::Dispatch` and `UiSystems::Areas`, before
`WidgetSystems::Layout`, chained, so a key that edited the field lands the same
frame ahead of the engine's row passes:

1. `relist` runs when the field's text, its `EditableText` or else its
   `TextInput`, is marked changed, which the engine does for any key or press it
   handles on the field and not only for an edit, or when the floor, the
   decorator, or the source changed; a changed decorator or source marks the
   directory unread, and a removed one goes unnoticed until the next read. It
   resolves the floor against the base the way the field is resolved, writes the
   floor into the field through `FieldText` when the directory falls outside it,
   and stops there when the directory is the one last read and the floor did not
   change. Otherwise it records whether the directory has a parent, `has_parent`
   over the source's `current_dir` and not the floor, and lists the directory
   through the source, or `DiskSource`, when it differs from the one last read.
   The source is handed the resolved directory, `.` for the working directory,
   and the decorator, when there is one, that directory joined with each file's
   name. Entries sort directories first, then names case-insensitively; a
   directory the source cannot list has no entries.
2. `rebuild_rows` runs when the listing, look, match style, or the filter, read
   from the field's text, changed. It despawns the old rows last child first,
   puts `..` ahead of the entries when the listing has a parent, keeps entries
   the look admits (hidden ones only with `hidden`, files only with an extension
   in `extensions` when that list is set, directories always), ranks them
   through `matching::find_match` stably on score, spawns a row per hit with the
   matched characters styled and a separator suffix on a directory, dims hidden
   rows, lays a file's decoration over its row, appends the typed row when the
   look accepts a new name, writes `ActiveDescendant` to the first row that is
   not `..`, or to the first row, and puts the list's `ScrollOffset` back at the
   origin through `apply_offset`, so a scroll from the rows it replaced does not
   carry over; the engine's reveal of the cursor's row still follows in
   `WidgetSystems::Layout`.
3. `place_file_picker_parts` runs when the root's area, order, look, or parts
   changed. `layout::split_area` cuts the root's area into the first row after
   the prompt's width in cells, the field's, and the rest, the list's, and
   writes each part's `ComputedWidgetArea`, `UiArea::Fixed` through
   `local_area`, and `UiOrder` one above the root's. Where the list's area goes
   from empty to having cells, a picker placed after its rows or shown after
   being hidden, it also puts the list's `ScrollOffset` back at the origin
   through `apply_offset` and re-inserts the list's `ActiveDescendant`, which is
   immutable, so it reads as changed: the engine reveals a cursor into a list
   with no area and leaves the offset past it, and reveals again only when the
   cursor changes.

In `Update`, `style_file_pickers` in `WidgetSystems::Style` draws the root's
`UiWidget`: the prompt as a `Paragraph` in the root's state style, focused when
the field is. It is gated by `StylistCache::redraws` with no value bits, and
redraws on a changed theme or look. The field row's text, selection and caret
are the engine's `TextField` on the field, in the field's own state style.

## Matching

`matching::find_match` is a case-insensitive subsequence match taking the
leftmost hit per filter character. The score is a start bonus for a hit at the
first character or a smaller one after `.`, `-`, `_` or a space, less the gaps
between hits. `light_matches` builds the row's line with hit runs in
`FilePickerMatchStyle`.

## Keys and events

A key is dispatched to the focused field, where the engine's `text_input_key`
takes what the field's `TextInputKeys` bind and consumes it; the clipboard keys
are consumed whether or not they acted. The rest bubbles to the root.
`file_picker_key`, a global `FocusedInput<KeyboardInput>` observer on a root
without `ComputedDisabled`, scans `FilePickerKeys` through `first_bound`, whose
defaults bind Enter, Tab, Right, Left, Ctrl+. and Escape, then Up, Down, PageUp
and PageDown. A list step, `Up`, `Down`, `PageUp` or `PageDown`, is applied
inline through `PickerAccess::step_cursor` and consumed only when the cursor
moved; any other hit is consumed and triggers `FilePickerCommand`, except
`Enter` on a key repeat, which is consumed and does nothing. `step_cursor` reads
the list's children as its rows and its `ComputedWidgetArea`'s height, at least
one, as a page: Up and Down move one row and wrap, PageUp and PageDown move a
page and stop at the first and last rows, a cursor on no row starts from the
first, and a move inserts `ActiveDescendant`. So Up and Down with the cursor on
the only row, PageUp on the first row and PageDown on the last bubble on to the
picker's ancestors; on the "no match" row, which holds no cursor, the first
arrow moves the cursor onto it and is consumed. Because the field's keys lack
everything `FilePickerKeys` binds, a key bound in both is the picker's. The
engine resolves `ComputedDisabled` onto the root, the field and the list from an
`InteractionDisabled` on the root or any ancestor, so a disabled picker's field
types nothing and its keys pass the root and bubble on.

`FilePickerCommand { entity, action }` is a public entity event;
`apply_file_picker_command` hands it to the crate-private `PickerAccess::apply`
on a root without `ComputedDisabled`. `FilePickerAction`: `Parent` writes the
parent of the listed directory through `directory_text` when the listing has
one, and nothing at a root or the floor; `Enter` completes a directory or
triggers `ValueChange<PathBuf>` on the root for a file; `Complete` replaces what
follows the field's last separator with the cursor's entry, a separator after a
directory; `ToggleHidden` flips `FilePickerLook::hidden`; `Close` triggers
`ModalDismiss { entity: root }`; `Up`, `Down`, `PageUp` and `PageDown` step the
cursor as the keys do. `Parent` and `Complete` write the field through
`FieldText`, which ends its selection.

A bracketed paste is the engine's: `text_input_paste` on the focused field.

Two pointer observers on `bevy_picking`'s events: `file_picker_press` focuses
the field with `FocusCause::Pressed` on a `PointerPress` on the root or on the
list, unless the root has `PressFocusDisabled`. On the root the engine's
`press_focus` finds no `TabIndex` at or above it and clears focus; the field
ends up focused because `file_picker_press` runs after it, which bevy's
`EntityHashMap` of global observers decides rather than registration order. The
list's own `PressFocusDisabled` stops that walk, while the engine's
`listbox_press` still moves the cursor. A press on the field is the engine's,
which places the caret and focuses the field through its `TabIndex`.
`file_picker_click` triggers `FilePickerCommand` with `Enter` on a
`PointerClick` of the primary button with count two or more on the list.

## Tests

Tests drive a full `App` headlessly: `CorePlugin` and `FilePickerPlugin`, a
`TerminalCamera`, a `TerminalSize`, a `VirtualWindow` spawned before the first
frame, keys and mouse written as a backend would through `plurimus_term`'s
`TerminalInput` and pastes as `PasteMessage`, and the composed frame read from
the render sub-app. `tests/support/mod.rs` carries those helpers, copied from
plurimus's unpublished `plurimus_test`, keys named by bevy's `Key`, plus two
scratch directories from `tempfile`, one of them with more entries than the list
has rows, `spawn_picker_with`, which spawns a picker with more components from
its first frame, `listening_parent`, which spawns an entity that records the key
presses bubbling past a picker spawned under it, and `path`, `directory`,
`field_of`, `list_of` and `set_path`, which reach the field and the list through
`FilePickerParts`, the last writing a new `TextInput` to the field, and
`command`, which triggers `FilePickerCommand`. `tests/source.rs` lists an
in-memory `DirectorySource` with no working directory; the `tempfile` tests are
the disk's. `tests/host.rs` swaps a field's `TextField` for upstream's
`TextInput`, with `bevy_ui_widgets`' `TextInputPlugin` running, and drives the
picker through the editor and the arrows.
