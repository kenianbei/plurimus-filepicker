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
plurimus = { version = "0.8", features = ["widgets"] }
plurimus_filepicker = "0.2"
```

```rust,ignore
app.add_plugins(FilePickerPlugin);
commands.spawn(file_picker("."));
```

Give the picker a `UiArea`, focus it, and observe `ValueChange<PathBuf>` on its
entity to receive the chosen path. `cargo run --example files` opens one over
the current directory in a modal pane.

| key                        | does                                         |
| -------------------------- | -------------------------------------------- |
| typing                     | filters the listed directory                 |
| Up, Down, PageUp, PageDown | move the cursor                              |
| Enter                      | descends into a directory, or chooses a file |
| Tab, Right                 | completes the cursor's entry into the field  |
| Left                       | climbs to the parent directory               |
| Ctrl+.                     | toggles hidden entries                       |
| Esc                        | triggers `ModalDismiss` on the picker        |

The field edits like any plurimus text field:

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
`TextInputKeys`, both on the picker. `FilePickerKeys` is scanned first, so a key
bound in both is the picker's: plain Left and Right climb and complete, and only
their shifted forms reach the field. A copy goes out as a `TerminalRequest`,
which `plurimus_crossterm` writes to the terminal's clipboard only with
`CrosstermPlugin::clipboard` set. The three clipboard keys stay with the picker
even with nothing to copy or paste; take them off its `TextInputKeys` to hand
them to the app.

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
| 0.2                 | 0.8      | 0.19 |
| 0.1                 | 0.7      | 0.19 |

See [ARCHITECTURE.md](ARCHITECTURE.md) for how the crate is put together.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
