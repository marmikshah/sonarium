# tono-desktop — the pattern station

The native desktop studio: author SoundDocs by ear — a step grid over catalog instruments, live audio (cpal + a MIDI keyboard), and undo — with the deterministic engine underneath, so what you audition is byte-identical to an offline bounce.

## Build and launch

```sh
cargo build -p tono-desktop --release
./target/release/tono-desktop
```

- A Tauri window over the engine: step grid, catalog instruments, live audition, MIDI input, undo.
- Not part of the default build or CI (webview/cpal/midir are heavy).

More: the project [README](../../README.md) and the [quickstart](../../docs/quickstart.md).
