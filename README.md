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
plurimus = { version = "0.7", features = ["widgets"] }
plurimus_filepicker = "0.1"
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
| Left, Backspace on empty   | climbs to the parent directory               |
| Ctrl+H                     | toggles hidden entries                       |
| Esc                        | triggers `ModalDismiss` on the picker        |

Every binding is data in `FilePickerKeys`. `FilePickerFloor` stops traversal
above a directory; `FilePickerLook` sets the prompt and whether dot-entries are
listed; `FilePickerMatchStyle` styles the matched characters.

## Compatibility

| plurimus_filepicker | plurimus | bevy |
| ------------------- | -------- | ---- |
| 0.1                 | 0.7      | 0.19 |

See [ARCHITECTURE.md](ARCHITECTURE.md) for how the crate is put together.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
