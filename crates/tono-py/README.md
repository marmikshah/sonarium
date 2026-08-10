# tono-py — the Python bindings

The deterministic tono engine plus its live runtime, from Python: typed songs, numpy renders, and a speaker-owning engine.

## Install (build from source only)

Never published to PyPI (the name is taken), and **no prebuilt wheels for
now**: the 3-platform wheel matrix is expensive in CI minutes, and this is a
zero-budget project — so it's build-from-source only until users ask for
wheels (the pipeline exists and is validated: `workflow_dispatch` on the
Wheels workflow, or `maturin build` below).

```sh
pip install maturin
maturin develop -m crates/tono-py/Cargo.toml              # the `tono` module in your env
python3 crates/tono-py/tests/smoke.py                     # the determinism smoke test
maturin build --release -m crates/tono-py/Cargo.toml      # abi3 wheel → target/wheels/
```

- abi3-py39: one wheel per platform covers every CPython 3.9+.
- Requires stable Rust (`rust-version` in the workspace `Cargo.toml`) and CPython 3.9+.

## Compile a song

```python
import tono

song = tono.Song("night-drive", tempo=122.0)
bass = song.track("bass", tono.instruments.bass("finger"))
riff = tono.Pattern(bars=1)
riff.notes(["C2", "C2", "Eb2", "G2"], durations=0.5)
song.arrange(bass, riff, bars=range(4))

program = song.compile(sample_rate=48000)   # tono.CompileError carries .diagnostics
audio = program.render()                    # np.float32, shape (frames, 2), L/R
```

- Typed `Song` / `Pattern` / `Program` wrap the native Rust model (stable —
  frozen at 1.10.0-rc.1): no JSON, `py.typed` stubs in the wheel, and the same
  canonical program hash an equivalent Rust song compiles to.
- `Voice` builders chain: `.gain(..).pan(..).reverb(..).swing(..).humanize(..)`.
- `program.save(path)` / `tono.Program.load(path)` ship the hashed bundle.
- Runnable examples: [`examples/night_drive.py`](examples/night_drive.py)
  (compose → compile → live `Performance`),
  [`examples/golden_hour.py`](examples/golden_hour.py) (a produced 16-bar
  track — compiles, renders, and bounces `golden_hour.wav` you can play), and
  [`examples/fur_elise.py`](examples/fur_elise.py) (Beethoven's bagatelle on
  the sampled grand — 3/8 meter map with a pickup, a tempo-map ritardando,
  per-note dynamics).

## Run it live

```python
with tono.Performance(program, headless=True) as perf:   # or headless=False for speakers
    perf.play()
    perf.set_gain(0.8, at=tono.next_bar())
    perf.transition("chorus", at=tono.next_bar())        # a named section
    audio = perf.fill(program.sample_rate * 10)          # stereo (frames, 2), float32
    print(perf.metrics())                                # frames, commands, queue depth, …
```

Commands schedule at frames, beats, bars, markers, sections, `tono.next_beat()`,
or `tono.next_bar()` — and execute at exact frames, never waiting on Python to
wake up. Seeks, loops, crossfaded swaps, stingers, capture/replay, and
snapshots work the same in both modes.

## Render a sound or patch

| Call | Returns |
| --- | --- |
| `tono.render(doc_json)` | Mono `np.float32` bounce of a SoundDoc — deterministic, CI-testable. |
| `tono.Patch(json).render(**params)` | A parametric SFX patch rendered with named parameter values. |

## Drive the live engine

`tono.Engine(sample_rate=48000)` owns a cpal output stream and a render thread:

| Call | What it gives you |
| --- | --- |
| `engine.instrument("warm_lead")` | A catalog instrument, driven live with `note_on` / `set_param`. |
| `engine.drumkit()` | The GM drum kit. |
| `engine.load_patch(json).trigger(...)` | One-shot SFX patches. |
| `engine.adaptive()` | An adaptive-music bed. |

More: the [cookbook](../../docs/cookbook.md), the [runtime model](../../docs/runtime.md), and the design notes — [ADR 0004 (bindings)](../../docs/adr/0004-python-bindings.md), [ADR 0005 (command delivery)](../../docs/adr/0005-realtime-command-delivery.md).
