# plurimus_filepicker

A file picker widget for [plurimus](https://github.com/kenianbei/plurimus): one
path field over a filtered listing of the directory it names.

The field is the whole state. Everything before its last separator is the
directory the list shows; everything after it filters the entries. Typing `..`,
`~`, or an absolute path traverses; the arrow keys move a cursor over what
matched; Enter chooses a file and emits `ValueChange<PathBuf>` on the picker.
The crate builds on `plurimus_widgets`' public engine - `TextInput`, `ListBox`,
the stylist cache, and `first_bound` key tables - and adds nothing to plurimus
itself.

## Usage

```toml
[dependencies]
plurimus = { version = "0.10", features = ["widgets"] }
plurimus_filepicker = "0.4"
```

```rust,ignore
app.add_plugins(FilePickerPlugin);
commands.spawn(file_picker("."));
```

Give the picker a `UiArea`, focus it, and observe `ValueChange<PathBuf>` on its
entity to receive the chosen path. `cargo run --example files` opens one over
the current directory in a modal pane.

The picker is three entities. The one you spawn draws the prompt and places the
other two, which it spawns in the next `PreUpdate` and names in
`FilePickerParts`: the field, a plurimus `TextField` marked `FilePickerField`
that holds focus, is the tab stop and publishes its caret to the terminal, and
the list, a `ListBox` marked `FilePickerList`. Focus given to the picker lands
on the field. To move the field, write a new `TextInput` to it; to open on a
path, `FilePicker::new(base).with_path(..)`. A host that sees a gesture the
picker does not - a window's double click on a row - triggers
`FilePickerCommand` on the picker and gets the same `Enter`.

A host may dress the field as upstream's text input instead, in a window or on a
plurimus node: remove `TextField` and add `bevy_ui_widgets::TextInput`, and the
picker reads and writes the field's `EditableText`. Up and Down still step the
list: the picker drops the caret move upstream's field makes for them. That
field keeps Left, Right, Home and End for its caret, so Left and Right do not
reach the picker there; rebind them in `FilePickerKeys` - Alt with Left or Right
passes upstream's field except on macOS - or trigger `FilePickerCommand`.

| key                        | does                                         |
| -------------------------- | -------------------------------------------- |
| typing                     | filters the listed directory                 |
| Up, Down, PageUp, PageDown | move the cursor                              |
| Enter                      | descends into a directory, or chooses a file |
| Tab, Right                 | completes the cursor's entry into the field  |
| Left                       | climbs to the parent directory               |
| Ctrl+.                     | toggles hidden entries                       |
| Esc                        | triggers `ModalDismiss` on the picker        |

Up and Down wrap from one end of the list to the other, as a plurimus list's
arrows do; PageUp and PageDown move by the list's height and stop at the ends. A
list key that moves nothing - an arrow with the cursor on the only row, PageUp
on the first row, PageDown on the last - bubbles on to the picker's ancestors,
so a page key pressed at an end moves focus to the widget beyond it, within the
modal when the picker is in one. To keep them, observe
`FocusedInput<KeyboardInput>` on the picker and stop those keys there.

The field takes a plurimus text field's keys:

| key                              | does                                     |
| -------------------------------- | ---------------------------------------- |
| Home, End, Ctrl+Left, Ctrl+Right | move the caret                           |
| Shift with any of those          | selects to where the caret moves         |
| Shift+Left, Shift+Right          | select one character                     |
| Ctrl+A                           | selects the whole field                  |
| Ctrl+C, Ctrl+X                   | copy and cut the selection               |
| Ctrl+V                           | pastes what the app last copied          |
| a paste from the terminal        | goes in at the caret, over any selection |

The picker's bindings are data in `FilePickerKeys` and the field's in
`TextInputKeys`, both on the picker. The field gets the picker's `TextInputKeys`
less every key `FilePickerKeys` binds, so a key bound in both is the picker's:
plain Left and Right climb and complete, and only their shifted forms reach the
field. A copy goes out as a `TerminalRequest`, which `plurimus_crossterm` writes
to the terminal's clipboard only with `CrosstermPlugin::clipboard` set. The
three clipboard keys stay with the field even with nothing to copy or paste;
take them off the picker's `TextInputKeys` to hand them to the app.

`..` lists first in any directory with a parent, so Enter on it climbs too.
`FilePickerFloor` stops traversal above a directory. `FilePickerLook` sets the
prompt, whether dot-entries are listed, which extensions are listed, and whether
the filter is offered as a new file name for a save dialog; with exactly one
extension listed, a name typed without one is offered with it, so `copy` offers
`copy.toml`. `FilePickerMatchStyle` styles the matched characters.

`FilePickerDecorator` badges and styles file rows: a function from a file's path
to a `RowDecoration`, a trailing line and a style, run once per file each time a
directory is read. `ListBoxCursor`, `ListBoxStripe` and `ListBoxSelectionMarker`
put on the picker dress its list, so the list's look is set at the picker's
spawn.

## Compatibility

| plurimus_filepicker | plurimus | bevy |
| ------------------- | -------- | ---- |
| 0.4                 | 0.10     | 0.20 |
| 0.3                 | 0.9      | 0.20 |
| 0.2                 | 0.8      | 0.19 |
| 0.1                 | 0.7      | 0.19 |

See [ARCHITECTURE.md](ARCHITECTURE.md) for how the crate is put together.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
