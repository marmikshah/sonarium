# tono-py — the Python bindings

The `tono` Python extension: the deterministic engine plus a live runtime,
from Python. Three surfaces:

- **Typed songs** (experimental through the 1.10.0 alphas) — `tono.Song` /
  `tono.Pattern` / `tono.Program` wrap the native Rust model: no JSON, typed
  stubs (`py.typed` ships in the wheel), and the same canonical program hash
  an equivalent Rust song compiles to.
- **Offline render** — `tono.render(doc_json)` → numpy arrays (deterministic,
  CI-testable), `Patch.render(**params)` for parametric patches.
- **Live engine** — `tono.Engine(sample_rate=48000)` owns a cpal output stream
  and a render thread: instruments (`engine.instrument("warm_lead")`), a GM
  drum kit (`engine.drumkit()`), SFX patches (`engine.load_patch(json).trigger(...)`),
  and an adaptive-music bed (`engine.adaptive()`).

## The typed API

```python
import tono

song = tono.Song("night-drive", tempo=122.0)
bass = song.track("bass", tono.instruments.bass("finger"))
drums = song.track("drums", tono.instruments.drums("tr808"))

riff = tono.Pattern(bars=1)
riff.notes(["C2", "C2", "Eb2", "G2"], durations=0.5)
beat = tono.Pattern(bars=1)
beat.hit("kick", beats=[0, 2])
beat.hit("snare", beats=[1, 3])

song.arrange(bass, riff, bars=range(4))
song.arrange(drums, beat, bars=range(4))

program = song.compile(sample_rate=48000)     # tono.CompileError carries .diagnostics
audio = program.render()                      # np.float32, shape (frames, 2), L/R
```

`Voice` builders (`.gain(..).pan(..).reverb(..).swing(..).humanize(..)`) chain;
`program.save(path)` / `tono.Program.load(path)` ship the hashed bundle.

## Build from source

Never published to PyPI (the name is taken) — build it here:

```sh
pip install maturin
maturin develop -m crates/tono-py/Cargo.toml   # the `tono` module in your env
python3 crates/tono-py/tests/smoke.py          # the determinism smoke test
maturin build --release -m crates/tono-py/Cargo.toml  # release abi3 wheel → target/wheels/
```

abi3-py39: one wheel per platform covers every CPython 3.9+.

Requires stable Rust (see `rust-version` in the workspace `Cargo.toml`) and a
CPython 3.9+ install.
