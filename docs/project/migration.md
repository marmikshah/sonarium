# Migrate between tono versions

How to move between tono versions, formats, and APIs. Newest first — each
entry is one line of what changes, then the code move.

## v1.10 (this release)

### Python: JSON strings → the typed API

The legacy JSON-string calls are **deprecated** (they keep working through
v1.10; see [api-tiers.md](/project/api-tiers)). The typed API is the successor —
and it's how you get compiled, hashed, runnable programs instead of one-shot
renders.

**Before** — one-shot render from a JSON string:

```python
import json, tono
doc = json.dumps({"name": "blip", "duration": 0.3,
                  "root": {"type": "sine", "freq": 880}})
audio = tono.render(doc)                       # mono float32
```

**After** — compile a typed song to a hashed program:

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
stamped `"engine": 5` renders byte-identically on every platform. Stamp a
document to opt in:

**Before** — no `engine` key (or 1–4): renders bit-for-bit as before,
byte-identical *per platform*:

```json
{ "name": "impact", "duration": 0.4, "root": { "type": "sine", "freq": 220 } }
```

**After** — byte-identical *across platforms*:

```json
{ "engine": 5, "name": "impact", "duration": 0.4, "root": { "type": "sine", "freq": 220 } }
```

- **New documents/songs stamp 5 by default.** Nothing to do unless you pin
  hashes of old renders.
- **Your old documents are untouched.** An existing file without `"engine"`
  (or with 1–4) renders bit-for-bit as before, forever.
- **Expect the render to change slightly** when you stamp 5 — the
  deterministic kernels are not bit-equal to platform libm; that's why it's
  a new revision. Re-pin any hash you recorded (the golden-corpus workflow:
  render, compare, accept the new hash deliberately).
- The deterministic promise now covers **Rust and Python**: an equivalent
  song compiles to the same canonical Program hash from either, on any OS.

### Song JSON: alpha.1 → alpha.2 fields

Every alpha.2 field is optional with a default — older song files load
unchanged, and behavior with empty maps is byte-identical to before.

**Before** — alpha.1: name, bpm, tracks, arrangement:

```json
{
  "name": "song", "bpm": 120,
  "tracks": [{ "name": "lead", "...": "..." }],
  "arrangement": [{ "...": "..." }]
}
```

**After** — alpha.2 adds optional maps, a song-level `seed`, and per-track
routing/automation:

```json
{
  "name": "song", "bpm": 120, "seed": 7,
  "tracks": [{ "name": "lead", "bus": "music",
               "sends": [{ "...": "..." }], "automation": [{ "...": "..." }] }],
  "arrangement": [{ "...": "..." }],
  "tempo_map": [{ "at": { "num": 0, "den": 1 }, "bpm": 140 }],
  "meter_map": [{ "bar": 0, "numerator": 3, "denominator": 4 }],
  "pickup": { "num": 1, "den": 2 },
  "sections": [{ "name": "chorus", "bar": 8, "bars": 8 }],
  "markers":  [{ "name": "drop", "at": { "num": 64, "den": 1 } }],
  "buses": [{ "id": "music", "gain": 1.0 }]
}
```

One new hard stop: a placement that would land *between* grid steps under a
meter map is a compile error (`T1005`), not a silent rounding.

### Program bundles

New in v1.10 — no code move, just the guarantees:

- `PROGRAM_VERSION` is 2. Bundles record the schema/engine revisions they
  were compiled with, and their hash covers the complete semantic bundle.
- Loaders reject bundles from a *newer* program version and re-verify the
  content hash on load (a hand-edited bundle fails `T3002`).
- A checked-in v1 fixture lives in `crates/tono-core/tests/compat/` — the
  promise that legacy document-only hashes keep loading.

## v1.9 → v1.10: schema v1 → v2

Schema v2 gives every track its own deterministic RNG stream keyed by its
`id`, so editing, muting, or reordering tracks never changes a sibling's
noise content.

**Before** — v1: one shared RNG stream across tracks (the historical
default):

```json
{ "version": 1, "tracks": [{ "...": "noise-bearing track" }, { "...": "another" }] }
```

**After** — v2: set `"version": 2` and give every track a stable `id`:

```json
{ "version": 2, "tracks": [{ "id": "hat", "...": "noise-bearing track" },
                           { "id": "snare", "...": "another" }] }
```

- v1 documents keep the shared stream — byte-identical; streaming falls
  back to the buffer-backed Player.
- Expect noise-bearing tracks to re-grain once on migration (that is the
  point of v2), then stay stable forever after.
- `"version": 2` is the default for new documents.

## CLI: the v1.10 additions

| Command | What it does |
|---|---|
| `tono compile SONG.json [--inspect]` | Compile a song to a Program bundle (`--inspect` prints a machine-readable summary). |
| `tono render --stems DIR` | Per-track and per-bus stems as stereo WAVs. |
| `tono import FILE.mid --song` | Import MIDI through the Song model. |
| `tono midi SONG.json --song` | Export a song to MIDI. |
| `tono review` / `tono fit` / `tono presets` / `tono catalog` | The author-by-inspection loop from the v1.10 feature batch. |
