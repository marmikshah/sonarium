# tono-play — the programmatic playground

A `cpal` speaker for Rust programs: build a sound and hear it in two lines — the fastest way to audition the engine while developing.

```rust
// play a SoundDoc through the default output device, for 0.6 s
tono_play::play_doc(&doc, 0.6)?;
```

## Run an example

```sh
cargo run -p tono-play --example live_band
```

The recipes: a live band, a song, drums, adaptive music, …

- Not part of the default build (heavy platform deps).
- Also the shared cpal shim the other native faces (the desktop studio, the
  Python extension) build on: device open, the f32 gate, panic containment in
  the callback, and channel spreading — one place the platform plumbing lives.

More: the [examples](examples/) and the project [README](../../README.md).
