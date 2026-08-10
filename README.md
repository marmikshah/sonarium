<p align="center">
  <img src="docs/logo.png" width="112" alt="tono — a pluck waveform on a dark tile">
</p>
<p align="center">
  <img src="docs/logo-wordmark.png" width="384" alt="tono">
</p>

<p align="center"><strong>Game audio as a pure function — procedural, deterministic, zero-asset.<br>Compose music in code, compile it once, render or run it byte-identically anywhere.</strong></p>

<p align="center">
  <a href="https://github.com/marmikshah/tono/actions/workflows/ci.yml"><img src="https://github.com/marmikshah/tono/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/tono-core"><img src="https://img.shields.io/crates/v/tono-core" alt="crates.io"></a>
  <a href="https://docs.rs/tono-core"><img src="https://img.shields.io/docsrs/tono-core" alt="docs.rs"></a>
  <img src="https://img.shields.io/badge/license-MIT-8c6ee6" alt="license">
</p>

<p align="center">
  <a href="https://marmikshah.github.io/tono/">Showcase</a> ·
  <a href="https://marmikshah.github.io/tono/architecture.html">Architecture</a> ·
  <a href="https://docs.rs/tono-core">API docs</a> ·
  <a href="docs/cookbook.md">Cookbook</a>
</p>

<p align="center">
  <img src="docs/river-flows-spectrogram.png" width="640" alt="spectrogram of River Flows in You, 800 notes on the sampled piano">
</p>

<p align="center"><em>Everything you can hear below was rendered by this engine — no samples, no WAVs shipped.</em></p>

## Hear it

**[▶ The showcase site](https://marmikshah.github.io/tono/)** — recognizable
classics rebuilt from scratch (retro-coin, the Nokia tune, THX-style deep note,
a complete piano piece, a full band demo), plus game-ready BGM loops and
ambient beds. Every one a deterministic render; no samples anywhere.

## Compose, compile, run — music as code

Write a whole song with typed objects in **Rust or Python** — tracks from 31
catalog instruments, patterns on a bar grid, tempo and time-signature maps,
beat-addressed automation, mixer buses and sends — then **compile it once**
into a hashed, validated **Program**: resolved document, metadata, resource
estimates, and a canonical hash that's identical from Rust or Python, on any
OS.

```python
import tono

song = tono.Song("night-drive", tempo=122, seed=7)
drums = song.track("drums", tono.instruments.drums("tr808"))
bass = song.track("bass", tono.instruments.bass("finger"))

beat = tono.Pattern(bars=1)
beat.hit("kick", beats=[0, 2])
beat.hit("snare", beats=[1, 3])
riff = tono.Pattern(bars=1)
riff.notes(["C2", "C2", "Eb2", "G2"], durations=0.5)

song.arrange(drums, beat, bars=range(8))
song.arrange(bass, riff, bars=range(0, 8, 2))
song.automate(bass, "gain", [(0, 0.2), (8, 0.9)], curve="exp")

program = song.compile(sample_rate=48_000)
mix = program.render()                      # stereo float32 (frames, 2)
stems = program.render_stems()              # every track + bus, separate
outro = program.render_range(bars=(6, 8))   # a slice of the mix
```

The same Program **runs live** with sample-accurate scheduling — musical
timing is the engine's job, never your loop's:

```python
with tono.Performance(program, headless=False) as perf:
    perf.play()
    perf.set_gain(0.8, at=tono.next_bar())       # lands exactly on the bar
    perf.transition("chorus", at=tono.next_bar()) # section swap, quantized
```

And the guarantee underneath: a document stamped `engine: 5` renders
**byte-identical on every platform** — the same song gives the same bytes in
Rust, Python, C, and the browser, while every older document keeps its
historical render forever, pinned by revision.

## Why tono

- **Sounds are data.** A sound is a JSON synthesis graph; rendering it is a
  pure function → byte-identical audio, every run. Test it, diff it, cache it,
  ship no asset files.
- **Zero-asset SFX.** A patch renders infinite variations from gameplay
  parameters — impacts that scale with collision force, footsteps that vary by
  surface. No sample library.
- **A real music runtime.** Sample-accurate transport, commands executed at
  exact frames, quantized section transitions, stingers, crossfaded swaps —
  plus live DSP buses, polyphony caps with priority stealing, and adaptive
  intensity stems.
- **An ear built in.** Every render returns a spectrogram, a waveform, and
  LUFS/spectral stats — "does it sound right?" becomes numbers and pictures.

## Quick start

```sh
cargo install tono       # the CLI
cargo add tono-core      # …or the engine as a library
```

```sh
cat > blip.json <<'EOF'
{ "name": "blip", "duration": 0.3, "engine": 5,
  "root": { "type": "mul", "inputs": [
    { "type": "sine", "freq": 880 },
    { "type": "env", "a": 0.002, "d": 0.08, "s": 0.0, "r": 0.05 } ] } }
EOF

tono render blip.json -o out/
#   out/blip.wav         out/blip.png (spectrogram)
#   out/blip_wave.png    out/blip.stats.json (peak/RMS/LUFS/spectral)
```

That loop — write a doc, render, read the images and stats, refine — is all an
authoring tool needs to shape sound by inspection. The
[cookbook](docs/cookbook.md) has the full node vocabulary and recipes.

**Where next?** Pick your path:

- **New here?** [docs/quickstart.md](docs/quickstart.md) — the guided first
  ten minutes (hear a sound, change it on purpose).
- **Compose music** — the cookbook's
  [Songs chapter](docs/cookbook.md#songs--from-a-composition-to-a-program):
  tempo/meter maps, automation, buses, stems, patterns, harmony.
- **Make sounds** — the [cookbook](docs/cookbook.md), then `tono diff`,
  `tono match REF.wav DOC.json`, and `tono render --watch` for the loop.
  Hear the factory sounds with `tono presets` / `tono catalog`.
- **Embed in a game** — [docs/runtime.md](docs/runtime.md) (Engine/Mixer
  runtime, parametric patches, and `Performance` for scheduled songs).
- **Python** — [crates/tono-py](crates/tono-py).
- **No code** — the desktop pattern station ([build it](crates/tono-desktop)).

All guides: [docs/README.md](docs/README.md).

## Recipes — the lazy answers

Copy-paste, runnable, no tour. (The live-play, patch, and adaptive ones are
also runnable in [crates/tono-play/examples](crates/tono-play/examples); the
song flow is end-to-end in `crates/tono-core/examples/compose.rs`.)

**Play notes live** — a drum kit and a piano in one mix, driven from your code:

```rust
use tono_core::{drumkit::DrumKit, instrument::{Instrument, Note}, presets::preset, runtime::Mixer};
use tono_play::{Speaker, device_sample_rate};

let sr = device_sample_rate()?;
let mut mixer = Mixer::new(sr);
let drums = mixer.add(DrumKit::general_midi(sr));
let piano = mixer.add(Instrument::new(preset("fm_tine").unwrap(), sr)?);
let speaker = Speaker::open(mixer)?;

speaker.control(|m| m.get_mut::<DrumKit>(drums).unwrap().note_on(Note(36), 1.0));
speaker.control(|m| m.get_mut::<Instrument>(piano).unwrap().note_on(Note::C4, 0.8));
speaker.control(|m| m.get_mut::<Instrument>(piano).unwrap().note_off(Note::C4));
```

**Write a song** — the fluent builder, catalog instruments, one timeline:

```rust
use tono_core::{catalog::{GrandPiano, Bass, Drums}, song::{CompileOptions, Song}};

let song = Song::new("demo", 120.0)
    .add(GrandPiano::grand(), |t| { t.at(0.0).chord(&["C4","E4","G4"], 4.0); })
    .add(Bass::finger(), |t| { t.play("C2", 2.0).play("G1", 2.0); })
    .add(Drums::acoustic(), |t| { t.kick().rest(1.0).snare().rest(1.0); });
let program = song.compile(&CompileOptions::default())?;  // hashed, validated
let (left, right) = program.render_stereo();
```

**Rhythm and harmony helpers** — Euclidean beats, pattern transforms, and
chord answers without a theory book:

```rust
use tono_core::{music::{Chord, Key}, song::pattern};

let tresillo = pattern::euclidean("tresillo", 3, 8, "midi:36", 1, 16)?;
let bright = pattern::transpose(&tresillo, 7)?;           // up a fifth
let bassline = Key::from_name("A minor")?.degree_pitch(1, 2)?;   // A2
let stabs = Chord::from_name("Cm7")?.invert(1);           // first inversion
```

**Mix like a console** — buses, sends, and stems:

```json
"buses": [ { "id": "verb", "gain": 0.8, "effects": [
    { "type": "reverb", "room": 0.6, "mix": 1.0 } ] } ]
```

```python
song.add_bus("verb", gain=0.8, effects=[("reverb", {"room": 0.6, "mix": 1.0})])
keys.route("verb")            # keys land on the shared reverb bus
drums.send("verb", 0.3)       # plus a post-fader copy
program.render_stems()        # every track + bus, pre-master, as separate files
```

**Zero-asset SFX** — one patch, endless variations from gameplay parameters:

```rust
use std::collections::BTreeMap;
use tono_core::patch::Patch;

let patch: Patch = serde_json::from_str(include_str!("impact.patch.json"))?;
let hit = patch.render(&BTreeMap::from([
    ("hardness".into(), force), ("size".into(), object_size),
]))?;                              // mono samples, byte-identical per input
```

**Adaptive game music** — stems and section swaps on the beat:

```rust
use tono_core::adaptive::{AdaptiveMusic, Quantize};

let mut music = AdaptiveMusic::new(48_000);
music.set_tempo(120.0, 4);
music.add_section("explore", &explore_doc);
let battle = music.add_section("battle", &battle_doc);

music.transition_to(battle, Quantize::Bar);  // combat! — swaps on the next bar
music.set_intensity(0.9);                    // stems swell with the action
music.stinger_at(&boss_hit, Quantize::Bar);  // lands on the downbeat
```

**Python** — the same engine, one import:

```python
import tono

engine = tono.Engine(48000)                  # owns the stream + render thread
engine.drumkit().note_on(36, 1.0)            # kick
engine.load_patch(impact_json).trigger(hardness=0.8, size=0.3)  # zero WAVs
```

The Rust crates install from crates.io; the Python extension
[builds from source](crates/tono-py/README.md). More:
[embedding & patches](docs/runtime.md) · [API docs](https://docs.rs/tono-core).

## One engine, five faces

Every face renders the same audio byte-identically:

- **CLI** — `cargo install tono` — render to audio + spectrogram + stats,
  compile songs, export stems and MIDI.
- **Rust library** — `cargo add tono-core` — the engine embedded in a game or tool.
- **Python** — typed songs, programs, and live playback with numpy;
  [build from source](crates/tono-py).
- **Pattern station** — a Tauri studio: a step grid over catalog instruments,
  live audio, undo — [build](crates/tono-desktop).
- **Playground** — hear Rust snippets through the speakers —
  [examples](crates/tono-play).

The last two are developer faces that live in this repo — the
[architecture guide](https://marmikshah.github.io/tono/architecture.html)
covers them and the rest of the codebase.

## License

[MIT](LICENSE) — permissive, no warranty.
