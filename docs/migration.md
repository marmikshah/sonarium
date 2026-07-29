# Migration guide

How to move between tono versions, formats, and APIs. Newest first.

## v1.10 (this release)

### Python: JSON strings → the typed API

The legacy JSON-string calls are **deprecated** (they keep working through
v1.10; see docs/api-tiers.md). The typed API is the successor — and it's how
you get compiled, hashed, runnable programs instead of one-shot renders.

Before (legacy):

```python
import json, tono
doc = json.dumps({"name": "blip", "duration": 0.3,
                  "root": {"type": "sine", "freq": 880}})
audio = tono.render(doc)                       # mono float32
```

After (typed):

```python
import tono
song = tono.Song("blip-demo", tempo=120)
lead = song.track("lead", tono.instruments.organ("tonewheel"))
riff = tono.Pattern(bars=1)
riff.note("A5", at=0.0, duration=0.5)
song.arrange(lead, riff, bars=1)
program = song.compile(sample_rate=48_000)
audio = program.render()                       # stereo (frames, 2), float32
```

Notes:
- `tono.Engine` live playback stays as-is for instruments/drum kits. For
  scheduled song playback use `tono.Performance` — never `time.sleep` for
  musical timing again.
- Errors change: `song.compile` raises `tono.CompileError` with
  `.diagnostics` (structured, with stable codes) instead of `ValueError`
  with a bare string.
- `Patch(json)` / `AdaptiveMusic.add_layer(doc_json)` keep working; new
  code should construct typed objects.

### Engine revision 4 → 5

Engine 5 is the **deterministic-transcendentals** revision: a document
stamped `"engine": 5` renders byte-identically on every platform (older
revisions stay byte-identical *per platform* — that never changes).

- **New documents/songs stamp 5 by default.** Nothing to do unless you pin
  hashes of old renders.
- **Your old documents are untouched.** An existing file without `"engine"`
  (or with 1–4) renders bit-for-bit as before, forever.
- **To migrate a sound to cross-platform determinism**, set `"engine": 5` —
  and expect its render to *change* slightly (the deterministic kernels are
  not bit-equal to platform libm; that's why it's a new revision). Re-pin
  any hash you recorded (the golden-corpus workflow: render, compare,
  accept the new hash deliberately).
- The deterministic promise now covers **Rust and Python**: an equivalent
  song compiles to the same canonical Program hash from either, on any OS.

### Song JSON: alpha.1 → alpha.2 fields

All alpha.2 fields are optional with defaults, so every older song file
loads unchanged: `tempo_map`, `meter_map`, `pickup`, `sections`, `markers`,
`buses`, and per-track `bus`/`sends`/`automation`/`seed`. Behavior with
empty maps is byte-identical to before. A placement that would land
*between* grid steps under a meter map is a compile error (`T1005`), not a
silent rounding — that's the one new hard stop to know about.

### Program bundles

`PROGRAM_VERSION` is 1. Bundles record the schema/engine revisions they
were compiled with; loaders reject bundles from a *newer* program version
and re-verify the content hash on load (a hand-edited bundle fails T3002).
A checked-in v1 fixture lives in `crates/tono-core/tests/compat/` — the
promise that v1 bundles keep loading.

## v1.9 → v1.10: schema v1 → v2

Schema v2 (`"version": 2`, the default for new documents) gives every track
its own deterministic RNG stream keyed by its `id`, so editing, muting, or
reordering tracks never changes a sibling's noise content. v1 documents
keep the historical shared stream (byte-identical, streaming falls back to
the buffer-backed Player). To migrate: set `"version": 2` and give every
track a stable `id`; expect noise-bearing tracks to re-grain once (that is
the point of v2) and then stay stable forever after.

## CLI quick reference for the v1.10 additions

- `tono compile SONG.json [--inspect]` — compile a song to a Program bundle
  (machine-readable summary with `--inspect`).
- `tono render --stems DIR` — per-track and per-bus stems as stereo WAVs.
- `tono import FILE.mid --song` / `tono midi SONG.json --song` — MIDI
  through the Song model.
- `tono review`, `tono fit`, `tono presets`, `tono catalog` — the
  author-by-inspection loop from the v1.10 feature batch.
