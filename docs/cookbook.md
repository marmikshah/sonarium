# The tono cookbook — build sounds and songs as JSON

Author a sound as a **`SoundDoc`** — a JSON synthesis graph — and render it. Rendering is a pure function of `(graph, seed, sample_rate)` → **byte-identical** audio, so a `doc.json` *is* the reproducible artifact. This page is the full node vocabulary plus copy-paste recipes.

**Contents:** [render loop](#render-your-first-sound) · [node vocabulary](#node-vocabulary) · [first SFX](#cook-your-first-sfx) · [stats](#read-the-feedback) · [targets](#judge-a-sound-by-targets) · [match a reference](#match-a-reference-sound) · [seq](#write-music-with-seq) · [songs](#compile-a-song-to-a-program) · [timbres](#design-more-timbres) · [pro techniques](#apply-pro-techniques) · [edit by path](#edit-a-doc-by-path) · [layers](#build-sounds-in-layers) · [loudness](#ship-level-matched-output) · [loops](#loop-ambience-and-bgm) · [streaming](#stream-a-document-live) · [engine revisions](#pick-an-engine-revision) · [determinism](#determinism)

## Render your first sound

The loop: write a doc → render → read the images and stats → change one thing → re-render.

```sh
tono render doc.json -o out/
```

| output | what it is |
|--------|------------|
| `out/<name>.wav` | the audio (`--format wav\|flac\|ogg`) |
| `out/<name>.png` | spectrogram (frequency × time, log frequency axis) |
| `out/<name>_wave.png` | waveform (amplitude × time) |
| `out/<name>.stats.json` | peak / RMS / LUFS / spectral / transient analysis |

A doc skeleton: `{ "name": ..., "duration": secs, "sample_rate": 44100, "seed": 0, "root": <node> }`. `root` is a single node; every node is a mono signal. Multiply a source by an envelope (`mul`), layer sources with `mix`, pipe a source through processors with `chain`. Any numeric param is a constant, a note name, or a modulator.

## Node vocabulary

Document-level fields:

| field | default | notes |
|-------|---------|-------|
| `name` | — | label; names the output files |
| `duration` | 0.3 | length in seconds |
| `sample_rate` | 44100 | Hz |
| `seed` | 0 | drives every `noise` source, `dust` train, and pluck burst |
| `version` | omitted ⇒ 1 | DSL schema version; current **2** (per-track RNG streams) |
| `engine` | omitted ⇒ 0 | DSP-kernel revision; current **5** — see [engine revisions](#pick-an-engine-revision) |
| `stereo` | mono | `wide` / `haas` treatment on the final render |
| `normalize` | unset ⇒ a transparent −0.1 dBFS sample-peak safety limit only | loudness match + true-peak limit |
| `playback` | `oneshot` | or `loop` |
| `root` | — | the signal graph |

**Sources** (output audio):

| node | fields | what it is |
|------|--------|------------|
| `sine` / `triangle` / `sawtooth` | `freq` | basic waves |
| `square` | `freq`, `duty` = 0.5 | pulse wave; modulate `duty` for PWM |
| `noise` | `color` = `white` | `pink` −3 dB/oct (wind/rumble), `brown` −6 dB/oct (booms) |
| `fm` | `freq`, `ratio`, `index` | 2-op FM — bells, e-piano; slide `index` down for a struck attack |
| `super` | `wave` = `sawtooth`, `freq`, `voices` = 7 (1..=16), `detune_cents` = 15 | detuned unison stack (supersaw) |
| `wavetable` | `wave` = `basic`, `freq`, `position` = 0 | morphs across built-in tables (`basic`/`harmonics`/`formant`/`metallic`) |
| `seq` | `bpm`, `wave`, `env`, `notes`, … | note sequencer — see [Write music with `seq`](#write-music-with-seq) |
| `impact` | `hardness` = 0.5, `velocity` = 1 | strike exciter; feed a `modal` bank |
| `dust` | `density`, `decay` = 0.02 | Poisson click train; `decay` 0 = bare impulses |
| `env` | `a`, `d`, `s`, `r`, `punch` | ADSR control signal 0..1 (not audio) |

**Combinators:**

| node | fields | what it is |
|------|--------|------------|
| `mix` | `inputs` | sum (layer) all inputs |
| `mul` | `inputs` | multiply (typically source × envelope) |
| `chain` | `stages` | serial pipe: stage 0 is a source, later stages process it |
| `tracks` | `tracks`, `master`, `buses` | stereo mixing console — document root only |

**Processors** (chain stages):

| node | fields | what it is |
|------|--------|------------|
| `gain` | `amount` | scale the signal (modulatable, but see [streaming](#stream-a-document-live)) |
| `lowpass` / `highpass` / `bandpass` / `notch` | `cutoff`, `q` = 0.707 | resonant filters; `notch` removes a narrow band (hum) |
| `peak` | `cutoff`, `q` = 0.707, `gain_db` = 0 | boost/cut one band (surgical EQ) |
| `lowshelf` / `highshelf` | `cutoff`, `gain_db` = 0 | tilt everything below/above the corner |
| `bitcrush` | `bits` (1..16) | amplitude quantize — crunch |
| `downsample` | `factor` | sample-rate reduction — lo-fi grit |
| `delay` | `secs`, `feedback` = 0 | feedback echo / comb |
| `reverb` | `room` = 0, `mix` = 0 | Schroeder-style reverb |
| `modal` | `modes` = `[{freq, decay=0.4, gain=1}]` (1..=64), `mix` = 1 | resonator bank — a struck body |
| `drive` | `amount`, `shape` = `tanh`, `aa`? | waveshaper: `tanh` warm, `hard` clip, `fold` metallic; ADAA on engine ≥ 1 |
| `ringmod` | `freq` | ring modulation — metallic, robotic |
| `tremolo` | `rate` = 6, `depth` = 0.5 | amp wobble; streams natively |
| `chorus` | `rate` = 1.5, `depth` = 0.5, `mix` = 0.5 | thickening / width |
| `flanger` | `rate` = 0.25, `depth` = 0.5, `feedback` = 0.5, `mix` = 0.5 | jet sweep / metallic whoosh |
| `phaser` | `rate` = 0.4, `depth` = 0.5, `feedback` = 0.3, `mix` = 0.5 | swept all-pass notches |
| `duck` | `trigger`, `amount` = 0.8, `attack` = 0.005, `release` = 0.25 | sidechain pump; the trigger renders silently |
| `compress` | `threshold`, `ratio`, `attack` = 0.005, `release` = 0.08, `makeup` = 0 | glue / loudness |
| `convolve` | `decay` = 1.5, `size` = 0, `predelay` = 0, `damp` = 0.3, `mix` = 0.35 | synthesized-IR convolution reverb — **offline only** |
| `granular` | `grain_ms` = 80 (5..=500), `density` = 25, `pitch` = 1, `spread` = 0.3, `mix` = 0.5 | grain cloud / frozen pad — **offline only** |

**Modulators** (any numeric param, in place of a constant):

| modulator | fields | what it does |
|-----------|--------|--------------|
| constant / note name | `440`, `"C4"`, `"F#3"`, `"Gb5"`, `"midi:60"` | note names resolve to Hz (A4 = 440) |
| `slide` | `from`, `to`, `secs`, `curve` = `lin`\|`exp` | glide, then hold at `to`; `exp` reads as natural pitch glide |
| `lfo` | `shape` = `sine` (`square`/`triangle`/`saw`), `rate`, `depth`, `center` | periodic oscillation around `center` |
| `arp` | `steps`, `rate` | cycle values at `rate` steps/sec |
| `env` | `a`, `d`, `s`, `r`, `punch` + `from`, `to` | ADSR mapped onto the `from`→`to` range |
| `rand` | `from`, `to`, `rate`, `seed` = 0 | smooth random walk; self-seeded and edit-stable |

## Cook your first SFX

**Laser zap** — descending square + noise transient:
```json
{ "name": "laser_zap", "duration": 0.22, "root": {
  "type": "mix", "inputs": [
    { "type": "mul", "inputs": [
      { "type": "square", "duty": 0.25,
        "freq": { "slide": { "from": 880, "to": 180, "secs": 0.18, "curve": "exp" } } },
      { "type": "env", "a": 0.0, "d": 0.18, "s": 0.0, "r": 0.02, "punch": 0.3 } ] },
    { "type": "mul", "inputs": [
      { "type": "noise" },
      { "type": "env", "a": 0.0, "d": 0.04, "s": 0.0, "r": 0.0 } ] } ] } }
```

**Coin pickup** — two ascending blips via arpeggio:
```json
{ "name": "coin", "duration": 0.18, "root": {
  "type": "mul", "inputs": [
    { "type": "square", "duty": 0.5, "freq": { "arp": { "steps": [988, 1319], "rate": 14 } } },
    { "type": "env", "a": 0.0, "d": 0.16, "s": 0.0, "r": 0.0, "punch": 0.2 } ] } }
```

**Explosion** — noise through a falling lowpass:
```json
{ "name": "explosion", "duration": 0.6, "root": {
  "type": "mul", "inputs": [
    { "type": "chain", "stages": [
      { "type": "noise" },
      { "type": "lowpass", "cutoff": { "slide": { "from": 1800, "to": 120, "secs": 0.5, "curve": "exp" } }, "q": 0.7 } ] },
    { "type": "env", "a": 0.0, "d": 0.5, "s": 0.0, "r": 0.1, "punch": 0.6 } ] } }
```

Design tips:

- **Punchy/percussive:** `a: 0` (instant attack), short `d`, `s: 0`, add `punch`.
- **Brightness:** read `spectral_centroid_hz` — higher = brighter. Tame harshness with a `lowpass`; add bite with a `highpass`.
- **Crunch/lo-fi:** `chain` a source into `bitcrush` (low `bits`) or `downsample`.
- **Vibrato:** an `lfo` on a source's `freq`. **Tremolo:** `mul` by an `lfo`-driven `gain` … or just an `env`.

## Read the feedback

`tono render` writes two images plus the numbers in `.stats.json`. Read them against each other:

| output | read it as |
|--------|------------|
| spectrogram `.png` | **log frequency axis** — bass and low-mids (basslines, modal partials, the body) get real vertical space |
| waveform `_wave.png` | envelope shape: sharp vertical onset = punchy; long fade = ringing tail; two humps = double-trigger |
| `attack_time_ms` / `attack_slope_db_per_ms` | onset speed / snappiness — big slope = a click/impact, small = a swell |
| `decay_time_ms` | tail length |
| `onset_count` | trigger count (one hit ⇒ 1; a double-trigger ⇒ 2) |
| `head_silence_ms` / `tail_silence_ms` | dead air to trim |
| `spectral_centroid_hz` | brightness |
| `spectral_flatness` | ≈0 tonal/pitched … ≈1 noisy/hissy |
| `inharmonicity` | energy *off* the harmonic grid — low for a clean tone, high for noise, bells/metal, **and aliasing** (the meter that shows an anti-aliasing fix working) |
| `true_peak_dbfs` | inter-sample peak; keep below 0 |
| `loudness_lufs` | perceived level |
| `crest_factor_db` | big = punchy transient, small = dense/compressed |

To converge a sound toward a reference, render both and compare their `.stats.json` — drive the deltas (centroid/brightness, LUFS, attack, …) toward zero.

## Judge a sound by targets

Judge against concrete targets so the call is reproducible, not taste.

**The universal ship checklist** (every sound):

- no clipping — `true_peak_dbfs` below 0
- trimmed dead air — small `head_silence_ms` / `tail_silence_ms`
- the right `onset_count` (one hit ⇒ 1)
- a clean loop seam for anything that repeats

**Per-archetype targets** — judged mostly on attack, spectral centroid, crest, and duration:

| archetype | character to hit |
|-----------|------------------|
| `laser` | short, bright, falling, very punchy |
| `coin` | two bright blips, moderate punch |
| `jump` | short rising sweep, fast gate |
| `impact` | low-centred body with a ring tail |
| `ui` | tiny, bright, instant |
| `footstep` | a very short, low-mid thud/tap |
| `powerup` | a short bright rising flourish |
| `ambience` | sustained, dark, low crest, looping |
| `bgm` | a mixed musical loop |

For a `laser`, aim for a crest of at least ~12 dB and a `spectral_centroid_hz` in the 2–8 kHz range. If the stats read crest 7 dB, add `punch` and shorten the attack; if the centroid reads 1200 Hz, raise a filter cutoff or pick a brighter wave.

The polish loop: read the stats → apply the single highest-impact fix by editing one field → re-render → if it regressed, revert that edit → repeat until the targets are met. Don't chase a deviation the sound's character justifies (a bell's long tail, a gusting wind's crest) — stop at the targets, not past them.

```sh
tono review doc.json --archetype laser
```

`tono review` runs the whole checklist for you — every finding prints the measured value, the target, and the fix to try, and the command exits non-zero on a FAIL grade, so it doubles as a ship gate in CI. (The same grading is `tono_core::review` in Rust.)

## Match a reference sound

Bring a WAV you like and measure a doc against it:

```sh
tono match REF.wav doc.json      # how far off, metric by metric
tono fit REF.wav doc.json        # close the gap automatically
```

- `tono match` prints the reference-vs-candidate table (brightness, loudness, envelope, duration), worst offenders first, with an overall score in tolerance units.
- `tono fit` hill-climbs that score: each round applies a seeded `vary::mutate` perturbation to the incumbent, keeps it when it scores closer, and halves the step size when the search stalls.
- The search is deterministic — same reference, doc, `--rounds`/`--amount`/`--seed`, same result — and writes the best doc to `<doc>.fit.json` (or `-o`) with its final match table.
- Start broad (`--amount 0.4 --rounds 64`), then re-fit the output with a small `--amount` to polish.

## Write music with `seq`

<a id="music-with-seq"></a>

For tunes, write a `seq` instead of gating a drone. Each note has its own pitch, length (in grid steps), and the shared per-note `env`; gaps are rests; notes can overlap. `steps_per_beat: 4` = sixteenths.

```json
{ "name": "lead_riff", "duration": 2.0, "root": {
  "type": "seq", "bpm": 120, "steps_per_beat": 4, "wave": "square", "duty": 0.5,
  "env": { "a": 0.005, "d": 0.08, "s": 0.3, "r": 0.04 },
  "notes": [
    { "step": 0, "len": 2, "pitch": 523.25 },
    { "step": 2, "len": 2, "pitch": 659.25 },
    { "step": 4, "len": 4, "pitch": 587.33 },
    { "step": 12, "len": 4, "pitch": 440.00 }
  ] } }
```

A note's `pitch` accepts a **note name** instead of Hz — `"C4"`, `"F#3"`, `"Gb5"`, `"midi:60"` (A4 = 440) — and note names work for any `freq`/`pitch` (a `sine`/`square`/`super` `freq` too):
```json
{ "type": "seq", "bpm": 120, "steps_per_beat": 4, "wave": "square", "duty": 0.5,
  "env": { "a": 0.005, "d": 0.1, "s": 0.3, "r": 0.05 },
  "notes": [ { "step": 0, "len": 2, "pitch": "C4" }, { "step": 2, "len": 2, "pitch": "E4" },
             { "step": 4, "len": 4, "pitch": "G4" } ] }
```

**Pitched-drum kick** — a note whose pitch slides down:
```json
{ "type": "seq", "bpm": 120, "wave": "sine",
  "env": { "a": 0.0, "d": 0.18, "s": 0.0, "r": 0.0, "punch": 0.5 },
  "notes": [ { "step": 0, "len": 2, "pitch": { "slide": { "from": 140, "to": 45, "secs": 0.08, "curve": "exp" } } } ] }
```
Use `wave: "noise"` for snares/hats (pitch ignored). Layer voices with `mix` (lead `seq` + bass `seq` + drum `seq`).

### Seq instruments

Beyond the raw chiptune waves (`square`/`triangle`/`sawtooth`/`sine`/`noise`), `seq` ships a core instrument list — pick one per seq and layer seqs like console tracks:

| wave | sound | notes |
|------|-------|-------|
| `piano` | acoustic piano | detuned string pair, velocity brightness, bass rings/treble dies. Parameter-free. |
| `epiano` | e-piano | soft FM body + metal tine ping; velocity opens the tine. Parameter-free. |
| `organ` | tonewheel organ | drawbar harmonics + attack percussion; sustains while held (`env {s:1}`). |
| `strings` | string ensemble | 3 detuned saws, slow bow swell (~150 ms — write notes slightly early), mellowing lowpass. |
| `brass` | brass section | 2 detuned saws, lowpass swells open over ~70 ms (the blat); velocity opens it further. Parameter-free. |
| `flute` | concert flute | sine + vibrato that fades in (~150 ms), breathy lowpassed noise; velocity adds breath. Parameter-free. |
| `mallet` | marimba | sine fundamental + wooden strike partials dying in tens of ms; velocity brightens. Parameter-free. |
| `bell` | struck bell | inharmonic partials (highs die first, hum rings on) + detuned shimmer twin. Parameter-free. |
| `bass` | fingered bass | filtered saw + sine sub; velocity snaps the filter open. |
| `kit` | drum kit | General MIDI map: pitch picks the drum (see below). |
| `sampler` | **real recorded instruments** | plays any SoundFont: `sf2` path + `sf2_preset` (GM program: 0 grand piano, 32 acoustic bass, 48 strings…); `sf2_bank: 128` = GM drum map. The realism instrument. |
| `cowbell` | pitched cowbell | the phonk lead; also GM 56 in the kit. |
| `fm` | FM mallets/bells | tunable: `fm_ratio` 1 = piano-ish, 3.5 = bell, 14 = tine; `fm_index`/`fm_strike`. |
| `pluck` | plucked string | Karplus-Strong guitar/harp/koto; `pluck_decay` sets ring. |

`piano` (engine ≥ 3), `bass`, and `pluck` also accept optional `piano_*` / `bass_*` / `pluck_*` tone knobs (e.g. `piano_hammer`, `pluck_tone`); every default reproduces the base voice bit-for-bit. `kit` takes a voicing: `kit: "classic"` (default) or `acoustic` / `electronic` / `808`.

**A drum groove** — `kit` reads the note pitch as a GM drum number, not a frequency: `midi:36` kick, `38` snare, `42` closed hat, `46` open hat, `41-50` toms, `49` crash, `51` ride, `39` clap, `56` cowbell:

```json
{ "type": "seq", "bpm": 100, "steps_per_beat": 4, "wave": "kit",
  "env": { "s": 1.0 },
  "notes": [
    { "step": 0,  "len": 2, "pitch": "midi:36" },
    { "step": 4,  "len": 2, "pitch": "midi:38", "gain": 0.9 },
    { "step": 8,  "len": 2, "pitch": "midi:36" },
    { "step": 10, "len": 2, "pitch": "midi:36", "gain": 0.7 },
    { "step": 12, "len": 2, "pitch": "midi:38", "gain": 0.9 },
    { "step": 0,  "len": 1, "pitch": "midi:42", "gain": 0.5 },
    { "step": 2,  "len": 1, "pitch": "midi:42", "gain": 0.4 },
    { "step": 4,  "len": 1, "pitch": "midi:42", "gain": 0.5 },
    { "step": 6,  "len": 1, "pitch": "midi:42", "gain": 0.4 },
    { "step": 8,  "len": 1, "pitch": "midi:42", "gain": 0.5 },
    { "step": 10, "len": 1, "pitch": "midi:42", "gain": 0.4 },
    { "step": 12, "len": 1, "pitch": "midi:42", "gain": 0.5 },
    { "step": 14, "len": 2, "pitch": "midi:46", "gain": 0.6 }
  ] }
```

**A band** is a `tracks` root — the mixing console. Each track has its own `pan` (−1..1, equal-power) and `gain`; `master` is the stereo bus chain. The reverb on the master runs with decorrelated left/right tails, and sampler tracks keep their native recorded stereo:

```json
{ "name": "song", "duration": 4.0, "normalize": { "target_lufs": -14 },
  "root": { "type": "tracks",
    "tracks": [
      { "pan": 0.0, "node": { "type": "seq", "bpm": 100, "wave": "kit", "env": { "s": 1 },
          "notes": [ { "step": 0, "len": 2, "pitch": "midi:36" },
                     { "step": 4, "len": 2, "pitch": "midi:38" } ] } },
      { "pan": 0.0, "gain": 1.1, "node": { "type": "seq", "bpm": 100, "wave": "bass",
          "env": { "a": 0.002, "s": 1, "r": 0.05 },
          "notes": [ { "step": 0, "len": 8, "pitch": "A1" } ] } },
      { "pan": -0.3, "node": { "type": "seq", "bpm": 100, "wave": "epiano",
          "env": { "a": 0.002, "s": 1, "r": 0.15 },
          "notes": [ { "step": 0, "len": 8, "pitch": "A3" },
                     { "step": 0, "len": 8, "pitch": "C#4" } ] } },
      { "pan": 0.35, "node": { "type": "seq", "bpm": 100, "wave": "strings",
          "env": { "a": 0.05, "s": 1, "r": 0.4 },
          "notes": [ { "step": 0, "len": 16, "pitch": "E4" } ] } }
    ],
    "master": [
      { "type": "compress", "threshold": -14, "ratio": 3, "makeup": 2 },
      { "type": "reverb", "room": 0.4, "mix": 0.12 }
    ] } }
```

(`mix` still works for mono layering inside one track.)

**Sampler setup**: download any free General MIDI SoundFont once and point `sf2` at it:
```json
{ "type": "seq", "bpm": 70, "wave": "sampler",
  "sf2": "/Users/you/.tono/sf2/gm.sf2", "sf2_preset": 0,
  "env": { "s": 1, "r": 0.2 },
  "notes": [ { "step": 0, "len": 4, "pitch": "C4" } ] }
```

Groove and glue: every seq takes `swing` (0..1 off-beat delay — ~0.55 is a classic shuffle; off-beats are odd steps, so set `steps_per_beat` to the swung subdivision) and `humanize` (0..1 deterministic timing/velocity jitter — 0.1–0.25 is tasteful). The `duck` processor sidechains anything to a trigger (kick-pumped bass/pads):
```json
{ "type": "chain", "stages": [
  { "type": "seq", "...": "the pad" },
  { "type": "duck", "amount": 0.8, "release": 0.25,
    "trigger": { "type": "seq", "wave": "kit", "...": "the kick pattern" } } ] }
```

Two tunable instruments in detail:

- **`fm`** — a two-operator FM voice struck per note: the modulation index (brightness) starts at `fm_index` and decays over `fm_strike` seconds, like a hammer strike, and louder notes (`gain`) ring brighter. `fm_ratio` picks the timbre family: `1` = e-piano / piano, `2` = hollow / clav, `3.5` = bell, `14` = tine.
  ```json
  { "type": "seq", "bpm": 65, "wave": "fm",
    "fm_ratio": 1.0, "fm_index": 5, "fm_strike": 0.25,
    "env": { "a": 0.002, "d": 1.2, "s": 0.0, "r": 0.3 },
    "notes": [ { "step": 0, "len": 4, "pitch": "A4", "gain": 0.9 },
               { "step": 4, "len": 4, "pitch": "C#5", "gain": 0.7 } ] }
  ```
- **`pluck`** — a Karplus-Strong string: a noise burst rings through a tuned feedback loop whose lowpass damps highs faster than lows, exactly like a real string — guitar, harp, koto. `pluck_decay` (0.8..1) sets ring time; low notes naturally ring longer. Pitch is fixed per note (no glides).
  ```json
  { "type": "seq", "bpm": 90, "wave": "pluck", "pluck_decay": 0.996,
    "env": { "a": 0.0, "d": 0.3, "s": 1.0, "r": 0.2 },
    "notes": [ { "step": 0, "len": 4, "pitch": "E3" },
               { "step": 4, "len": 4, "pitch": "A3" },
               { "step": 8, "len": 8, "pitch": "C#4" } ] }
  ```

Layer them: `fm` melody + soft `triangle` doubling + `pluck` arpeggio is a full band. The pluck's noise burst comes from the doc's `seed`, so takes are reproducible.

## Compile a song to a Program

<a id="songs--from-a-composition-to-a-program"></a>

A whole piece is a **Song**: instrument tracks, reusable patterns, and an arrangement on the bar grid (the `song` module in Rust, `tono.Song` in Python — the same model, the same output).

`Song::compile` produces a **Program**:

- the resolved document plus a canonical content **hash** (FNV-1a — identical from Rust or Python for an equivalent song)
- the musical facts: tempo, grid, bars, duration in seconds and frames, the track roster with stable ids
- bounded **resource estimates**: frames, note events, peak voices, memory
- **streaming-coverage warnings** — a plain compiled song (a schema-v2 mixer of built-in waves) streams natively with zero warnings; any blocked part is named and falls back to the buffer-backed `Player` (see [streaming](#stream-a-document-live))

```sh
tono compile SONG.json [-o FILE] [--sample-rate N] [--inspect]
```

- Compilation validates in one pass — every problem at once, each with a stable code, the object path, and the fix. A failing compile exits non-zero, so `tono compile` doubles as a CI gate for song projects.
- `--inspect` prints the machine-readable summary (hash, version pins, roster, estimates, warnings) and writes nothing.
- Without `--inspect` it writes `<name>.program.json` — a versioned bundle that `Program::from_json` / `tono.Program.load` reloads without recompiling. A newer bundle revision is rejected (`T3001`); a hand-edited one fails its hash check (`T3002`).
- Songs carry their own `engine`/`version` pins like documents, plus an optional song-level `seed`; tracks can be `mute`d or `solo`ed (console semantics: solo mutes every non-solo track; a track that is both stays muted). All of it lands in the Program, so a saved bundle reproduces its audio exactly.

### Move the tempo and meter

```json
"tempo_map": [ { "at": { "num": 0, "den": 1 }, "bpm": 120 },
               { "at": { "num": 16, "den": 1 }, "bpm": 90 } ]
```

- `tempo_map` lists changes at exact beat positions (rationals — tuplets never drift), applied segment-wise, so a note crossing a change keeps its musical length.
- `meter_map` moves time signatures the same way — by bar, with numerator AND denominator (6/8 counts 3 quarter-beats a bar) — and `pickup` gives bar 0 a shorter length (anacrusis).
- A placement that would land between grid steps is a compile error (`T1005`), never a silent rounding; raise `steps_per_beat` or move the placement onto the grid.
- Songs without maps compile byte-identically to before. `sections` (named bar ranges) and `markers` (named beat points) are metadata the Program preserves for the runtime's quantized transitions.

### Automate gain and pan

```json
"automation": [ { "target": "gain", "curve": "exp",
  "points": [ { "at": 0.0, "v": 0.2 }, { "at": 16.0, "v": 0.9 } ] } ]
```

- Song tracks automate `gain`/`pan` in beats, compiled through the tempo map segment-wise.
- Each lane picks a curve: `linear` (default), `step` (hold then jump), or `exp` (geometric between positive endpoints).

### Route buses and sends

```json
"buses": [ { "id": "verb", "gain": 0.8, "effects": [ { "type": "reverb", "room": 0.6, "mix": 1.0 } ] } ]
```

- A tracks root (compiled or hand-written) gains named `buses` — submixes with insert chains and return faders.
- A track routes with `bus`, or feeds a bus post-fader with `sends` (the send taps the ducked, automated signal, so the reverb tail pumps with the mix).

### Export stems

- `tono render --stems DIR` writes every track's positioned contribution and every bus's return as separate stereo WAVs (pre-master-chain); the Python `Program.render_stems()` returns the same as arrays.
- Stems carry their routing: a stem's id is the track id or `bus:<id>`; master-routed stems plus bus returns sum to the exact mix the master chain hears, and a bus-routed track's stem is already inside its bus's stem. Muted tracks render as silent stems.

### Transform patterns in code

- The `song::pattern` module (Rust) and `Pattern` methods (Python) transform patterns purely: `repeat` / `concat` / `layer` / `slice` / `transpose` / `stretch` / `rotate` / `reverse` / `quantize`, `vel` / `gate`, `euclidean` and `tuplet` constructors, and deterministic `probability` / `humanize` seeded per pattern.
- `stretch` is exact or errors (off-grid names the note); `transpose` keeps `midi:N` notes as drums.

### Answer harmony questions

```python
Key("A minor").degree(3)   # the 3rd scale degree
Chord("Cm7").arp()         # root-position voicing
```

The `music` module (Rust) / `tono.Pitch`/`Key`/`Chord` (Python) use a strict spelling grammar — never a silent guess at an ambiguous name.

## Design more timbres

- **PWM lead:** `square` with a modulated `duty` — `{ "lfo": { "shape": "sine", "rate": 5, "depth": 0.3, "center": 0.5 } }`.
- **FM bell / e-piano:** `{ "type": "fm", "freq": 440, "ratio": 3.5, "index": { "slide": { "from": 6, "to": 0, "secs": 0.4 } } }` — higher `ratio`/`index` = more metallic; sliding `index` down gives a struck attack.
- **Warmth / distortion:** `chain` into `drive{amount,shape}` — `tanh` warm, `hard` aggressive, `fold` metallic. Pairs well before a `lowpass`. On `engine: 1` documents the shaper is anti-aliased (ADAA) so hard/fold stay clean instead of spraying inharmonic foldback; set `"aa": false` on the node to hear the raw aliasing curve.
- **Struck bodies (bell / glass / metal / coin / UI ping):** the **exciter → resonator** pair — an `impact` into a `modal` bank. `chain[ {type:impact, hardness:0.85}, {type:modal, modes:[{freq,decay,gain}, …]} ]`. Each mode is a damped sine; near-harmonic ratios + a long fundamental = bell, off-harmonic ratios = metal, all-short decays = a glass/UI tick. The hammer's `hardness` sets how far up the bank it reaches; `velocity` its energy. Oscillators can't voice these cleanly — modes can.
- **Fat lead / pad (supersaw):** `{ "type": "super", "wave": "sawtooth", "freq": 220, "voices": 7, "detune_cents": 20 }` — more `voices` / `detune_cents` = wider and thicker. Great through a `lowpass` filter envelope, or as a `mix` layer under a melody.
- **Morphing sweep (wavetable):** `{ "type": "wavetable", "wave": "basic", "freq": 110, "position": { "lfo": { "shape": "sine", "rate": 0.5, "depth": 0.5, "center": 0.5 } } }` — `position` (0..1) crossfades across a built-in table set: `basic` (sine → triangle → square → saw), `harmonics` (a saw growing its partial count — a pure brightness ramp), `formant` (vowel-ish stacks a → e → i → o → u — sweep slowly for vocal morphs), `metallic` (sparse clang stacks). Modulating `position` is the signature move: a slow LFO is an evolving pad, an `env` is a struck brightness decay. Tables are generated at build time (zero assets), band-limited to 32 partials — keep bright positions under ~600 Hz at 44.1 kHz to avoid foldover.
- **Surgical EQ:** `peak{cutoff,q,gain_db}` boosts/cuts a band (e.g. `+6 dB` at 3 kHz for presence); `lowshelf`/`highshelf{cutoff,gain_db}` tilt the lows/highs; `notch{cutoff,q}` removes a resonance or hum. Read `spectral_centroid_hz`, then EQ to hit the brightness you want.

## Apply pro techniques

- **Filter envelope (the "pew"/snap):** drive a filter cutoff with an `env` modulator instead of a slide —
  `{ "type": "lowpass", "cutoff": { "env": { "a": 0, "d": 0.12, "s": 0, "r": 0, "from": 4000, "to": 200 } }, "q": 3 }`.
  High `q` + fast decay = laser/zap snap; slow = sweep.
- **Layered impact:** `mix` a low `sine` (slide pitch down) for body + `noise{color:"brown"}` for weight,
  `mul` by a punchy `env`, then `chain` → `lowpass` (env cutoff) → `drive`. Classic hit design.
- **Textures by noise colour:** `white` = hiss/steam, `pink` = wind/surf/rumble, `brown` = distant booms.
- **Crackle / sparse events (`dust`):** `{ "type": "dust", "density": 80, "decay": 0.025 }` is a Poisson click train — `density` grains/sec, each ringing `decay` seconds (0 = bare impulses). Fire crackle, rain, geiger ticks, sparks, debris. Band-shape it through a `bandpass`/`highpass`, or feed a `modal` for pitched debris.
- **Organic motion (`rand`):** a random-walk modulator — `{ "rand": { "from": 250, "to": 1500, "rate": 0.7, "seed": 1 } }` — drifts non-periodically between `from` and `to`, `rate` new targets/sec. The gusting the periodic `lfo` can't do: wind (on a lowpass `cutoff`), fire flicker (on a `gain`), drifting detune. Give two `rand`s different `seed`s to decorrelate them; the walk is deterministic and edit-stable (seeded from its own fields, never shifts when siblings change).
- **Metallic / clang:** a `modal` bank with off-harmonic mode ratios excited by a hard `impact` (the physical way — see "Struck bodies" above); or, cheaper, `fm` with integer-ish `ratio` (3, 3.5) and high `index`, or `ringmod{freq}` on a tone.
- **Tuning a modal bank:** address one partial at a time — set the field at
  `root.stages[1].modes[0].freq` to 540 (each mode is its own node with its own
  path). Stretch every `decay` for a cathedral bell, shrink them for a desk
  bell; raise `hardness` toward 1 to wake the upper modes. `tono_core::vary::mutate`
  then gives a non-repeating round-robin of hits.
- **Width / thickening:** `chorus{rate,depth,mix}` on pads and leads.
- **Tremolo (streamable amp wobble):** `tremolo{rate,depth}` — gain swings between `1-depth` and 1 at `rate` Hz (defaults 6 / 0.5). Unlike a modulated `gain` (which renders offline but can't stream), tremolo is a closed form of the absolute sample index, so it streams natively and byte-identically to the offline render.
- **Real spaces (`convolve`, offline-only):** `convolve{decay,size,predelay,damp,mix}` is a convolution reverb whose impulse response is *synthesized* — zero assets, no IR files: a noise burst decaying to −60 dB over `decay` seconds (RT60-ish, default 1.5), length-capped by `size` (0 = `decay`), darkened over time by `damp` (0..1, default 0.3), after `predelay` seconds of gap (larger rooms answer later). The IR is deterministic per graph position, so renders are reproducible. Like `reverb`, the tail folds into the document length. A `chain[ impact, convolve ]` is a strike in a hall; small `decay` + high `damp` is a muffled room. **Offline only** — convolution needs the whole input buffer, so streaming refuses it with a named reason (bounce offline, keep the streamed graph causal).
- **Frozen textures (`granular`, offline-only):** `granular{grain_ms,density,pitch,spread,mix}` chops the incoming signal into overlapping Hann grains (`grain_ms` long, `density` per second — defaults 80 / 25), replays each at `pitch` (2 = octave up shimmer, 0.5 = octave-down drone) with deterministic onset jitter and detune of depth `spread`, and crossfades with dry per `mix`. Chain it after any source to smear it into a pad or cloud. **Offline only** — grains read the whole input out of order, so streaming refuses it with a named reason.
- **Glue & loudness:** end a busy chain with `compress{threshold,ratio,attack,release,makeup}`. Watch the
  stats: keep `true_peak_dbfs` below 0, use `loudness_lufs` to match levels across a set, and read
  `crest_factor_db` (big = punchy transient, small = dense/compressed).
- **Variations (round-robin):** `tono_core::vary::mutate(doc, amount, seed)` — a
  Rust API, also `tono vary doc.json -n 8 --amount 0.15 --seed 1` on the CLI —
  with a small `amount` (0.1–0.2) spawns N subtly different takes of a
  footstep / impact / pickup so repeats don't sound identical.
- **Stereo (BGM / ambience):** add a top-level `"stereo"` to the doc —
  `{ "mode": "wide", "amount": 0.6 }` for pseudo-stereo width, or
  `{ "mode": "haas", "ms": 12, "pan": -1 }` for precedence widening. SFX usually stay mono (engine spatialises).

## Edit a doc by path

- Every node has a **path**: `root.inputs[0].freq`, `root.stages[1].cutoff`, `root.stages[1].modes[0].freq`. Paths index into a `chain`'s `stages`, a `mix`/`mul`'s `inputs`, a `seq`'s `notes`, and a `tracks`' `tracks`.
- To change a sound you edit that field in the JSON and re-render — no need to rewrite the whole graph. A field's value can be a number, a modulator object, or a whole node — e.g. set `root.inputs[0].inputs[0].freq` to:
  ```json
  { "slide": { "from": 880, "to": 140, "secs": 0.18, "curve": "exp" } }
  ```
- For programmatic editing there's a small Rust API in `tono_core::edit`:
  `describe(doc)` returns the path → type → params map; `apply_ops(doc, ops)`
  applies a batch of `set{path,value}` · `insert{path,index?,node}` (into a
  `chain`'s `stages` or a `mix`/`mul`'s `inputs`) · `remove{path,index?}` ops in
  one pass; and `morph(a, b, t)` blends two docs.

## Build sounds in layers

Pro SFX are stacks: a transient (the click that says "now"), a body (the identity), a tail (the space). Build them as a **`tracks` root** — the mixer — with one track per component. Track fields:

| field | default | notes |
|-------|---------|-------|
| `id` | backfilled as `layer_<position>` | stable slug, unique within the doc; how edits address the track |
| `node` | — | the track's signal graph |
| `pan` | 0 | −1..1, equal-power |
| `gain` | 1 | 0..2, 1 = unity |
| `at` | 0 | start offset in seconds — a tail 20 ms late is `at: 0.02`, a pre-click 5 ms early against a body at `at: 0.005` |
| `mute` | false | rendered state, not a monitoring convenience: exports ship without muted tracks |
| `automation` | — | song-time `gain`/`pan` lanes |
| `sidechain` | — | duck when another track sounds (below) |
| `bus` / `sends` | — | route to / feed into a mix bus |

A disciplined SFX skeleton is four band-split layers — `sub` / `body` / `top` /
`transient` — each a `chain` of its source into a band-splitting filter and a
one-shot envelope, with a starting `gain`. Fill in the real source per role
(an `fm` or `super` for the body, `noise` → `highpass` for the top, an `impact`
for the transient), then rebalance by reading each layer's contribution.

- **Per-layer stats:** every render's `.stats.json` carries a `layers` array, one entry per track — `{ id, peak_dbfs, rms_dbfs, energy_pct, mute }`, where `energy_pct` is that layer's percentage of the pre-master energy (e.g. `crack 38% • peak −8.1 dBFS | body 52% … | tail 10%`). Nudge a track's `gain` until the split reads right, and edit inside a layer with paths into its `node` (e.g. `root.tracks[0].node.env.d`).
- **One layer per thing you'd fade, pan, time-shift, or analyze separately** — an instrument in a song, a component in an SFX. Use `mix` only for sub-signals that share one envelope/filter; never one track holding a mix of seqs (it makes the per-layer stats useless).
- **Layers are independent by construction:** each track's noise is drawn from a deterministic RNG stream keyed by its `id`, so muting, removing, duplicating, or editing one track never changes a sibling's noise grains. Duplicating a track under a new `id` is a built-in variation — the copy re-grains its noise deterministically from the new id.

**Sidechain ducking between tracks:** a track can carry a `sidechain` that
pulls its level down whenever another track sounds — the classic kick→bass
pump, without nesting a `duck` node:

```json
{ "name": "pump", "duration": 2.0, "version": 2, "root": { "type": "tracks", "tracks": [
    { "id": "kick", "node": { "type": "seq", "bpm": 120, "steps_per_beat": 1, "wave": "sine",
        "env": { "a": 0.001, "d": 0.12, "s": 0.0, "r": 0.02 },
        "notes": [ { "step": 0, "len": 1, "pitch": "A1" }, { "step": 1, "len": 1, "pitch": "A1" } ] } },
    { "id": "pad", "gain": 0.5,
      "sidechain": { "source": "kick", "amount": 0.8, "release": 0.2 },
      "node": { "type": "sawtooth", "freq": 110 } }
] } }
```

- `source` is the driving track's `id`; `amount` (0..1, default 0.8) is the depth, `attack`/`release` (defaults 0.005 / 0.25 s) the ballistics — the same envelope follower the `duck` node uses, so the pump character matches.
- The source renders untouched; only the follower dips, and it ducks when the source actually lands on the bus (the source's `at` offset is honored).
- Several tracks may follow one source, but a source must be a plain track (no follower-of-follower chains — duck directly to the source's source).
- A sidechained mix **streams natively**: the duck envelope advances per sample, so a schema-v2 `tracks` root — sidechains, buses, and all — streams byte-identically to the offline bounce (v1 documents keep the buffer-backed `Player` fallback).

## Ship level-matched output

Add a top-level `normalize` to gain-match to a loudness target and brick-wall
the true peak (so the file never inter-sample clips):
```json
"normalize": { "target_lufs": -16, "ceiling_dbtp": -1 }
```
Pick **one** `target_lufs` for a whole set so every sound plays at the same
perceived level (≈ −16 LUFS for SFX, ≈ −14 for music). To ship a set, render
each doc into the same output folder with the same `target_lufs`; each
`.stats.json` reports the resulting `loudness_lufs` / `true_peak_dbfs` so you
can confirm they match.

## Loop ambience and BGM

For ambience beds, drones, and music that must repeat with no click, set a
top-level `playback`:
```json
"playback": { "mode": "loop", "crossfade_secs": 0.5 }
```

- The renderer extracts the loop region (`start_secs`..`end_secs`, default the whole buffer) and **equal-power crossfades its tail onto its head**, so the rendered file is a seamless loop body (shorter than the source by the crossfade, default 0.1 s).
- The exported WAV carries a `smpl` loop chunk, so host engines loop it at the sample-accurate points with no manual setup.
- Watch the loop seam: if it clicks, raise `crossfade_secs` or match the graph's start/end levels.
- An ambience bed from scratch — slow filter-swept pink noise over a low drone, widened and looped:
  ```json
  { "name": "cave_ambience", "duration": 6.0,
    "playback": { "mode": "loop", "crossfade_secs": 0.5 },
    "stereo": { "mode": "wide", "amount": 0.6 },
    "root": { "type": "mix", "inputs": [
      { "type": "chain", "stages": [
        { "type": "noise", "color": "pink" },
        { "type": "lowpass",
          "cutoff": { "lfo": { "shape": "sine", "rate": 0.1, "depth": 250, "center": 600 } } } ] },
      { "type": "chain", "stages": [
        { "type": "sine", "freq": 55 }, { "type": "gain", "amount": 0.4 } ] } ] } }
  ```
- For melodic BGM, build a `seq` (or layer several with `mix`), give the doc a
  `duration` of an exact number of bars, then loop it. Keep the tail tidy
  (notes that ring past the loop point hurt the seam).

## Stream a document live

The streaming renderer pulls a document block-by-block, byte-identical to the offline render. What a document trips is reported by `StreamGraph::blockers`, each with the fix:

| streams natively | falls back to the buffer-backed `Player` (byte-identical, whole-buffer) |
|------------------|---------------------------------------------------------------|
| every node with constant filter/EQ cutoffs and gain amounts | a `normalize` output stage (whole-buffer op) |
| all modulators on source params (closed forms of the sample index; `rand` carries its walk) | a filter/EQ/gain carrying a modulated cutoff or amount |
| `tremolo` (a closed form of the sample index) | `loop` playback; a `stereo` (Haas/Wide) treatment (write-time ops) |
| `noise`/`dust`/`seq` under engine ≥ 2 (structurally-seeded RNG) | RNG nodes under engine < 2; the `sampler` seq |
| a schema-v2 `tracks` root — sidechains, buses, automation | a schema-v1 `tracks` root |

`convolve` / `granular` are offline-only whole-buffer effects — no streaming form exists; bounce them offline and keep the streamed graph causal.

## Pick an engine revision

A document carries two independent version numbers:

- `version` is the **schema** version (document structure).
- `engine` is the **DSP-kernel** revision — which audio kernels render it.

They are split so a fidelity upgrade never changes the bytes of an older sound: a document with `engine` omitted renders under the original kernels (byte-for-byte forever); new documents are stamped with the current engine and get the upgrades.

| `engine` | what changed |
|----------|--------------|
| omitted (0) | the original kernels — byte-for-byte forever |
| 1 | anti-aliased `drive` (ADAA) |
| 2 | per-node structurally-seeded RNG for `noise`/`dust` (decorrelated siblings; byte-identical streaming randomness) |
| 3 | the inharmonic additive `piano` voice (stretched partials, per-partial decay, hammer spectrum, detuned unison pair) |
| 4 | corrected mixer output stage (joint stereo loudness normalization, gated BS.1770, oversampled true-peak) and per-note humanize jitter |
| 5 | deterministic transcendental kernels (`det`: fdlibm-grade sin/cos/exp/ln/powf/tanh in pure f64) replace platform libm everywhere in the render path; `convolve` runs a fixed-order radix-2 FFT with deterministic twiddles and power-of-two sizing ⇒ **renders byte-identically on every platform** |

- To modernise an existing sound, set `"engine": 5` (the current revision — its output will change; that's the point).
- To keep a legacy sound bit-exact, leave `engine` off.
- New documents and songs stamp 5 by default.

## Determinism

- Rendering is a pure function of `(graph, seed, sample_rate)` — a doc renders **byte-identical** every time. With `engine: 5` (the default for new documents and songs) that identity holds **across platforms**; engine ≤ 4 documents keep their historical per-platform renders bit-for-bit (platform libm's last bits differ between macOS-arm64 and linux-x86_64, though integer-RNG, PolyBLEP, and rational-filter content is identical everywhere).
- The doc's top-level `seed` drives every noise source, `dust` train, and Karplus-Strong pluck burst, so takes are reproducible; change `seed` for a different-but-equivalent roll.
- Because the doc *is* the artifact, version your `.json` files and you can always reproduce the exact WAV — no separate session log needed.
