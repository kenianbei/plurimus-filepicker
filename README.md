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

Observe `ValueChange<PathBuf>` on the picker entity to receive the chosen path.

## Compatibility

| plurimus_filepicker | plurimus | bevy |
| ------------------- | -------- | ---- |
| 0.1                 | 0.7      | 0.19 |

See [ARCHITECTURE.md](ARCHITECTURE.md) for how the crate is put together.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
