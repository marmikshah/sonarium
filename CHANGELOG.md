# Changelog

## 1.10.0 — 2026-07-29

tono becomes a production-grade music-as-code platform: compose full songs in
idiomatic Rust or Python, compile them once into a hashed, validated Program,
render the mix or stems offline, and run the same Program through a
sample-accurate scheduled runtime — all byte-identical, everywhere. Engine
revision 5 retires the per-platform determinism limitation: documents stamped
`engine: 5` render bit-for-bit on every supported target (older revisions are
untouched forever). The faces multiply: a typed Python API with py.typed and wheels, prebuilt
CLI binaries, and the typed Song/Pattern/Program/Performance API frozen
stable as of rc.1 —
see docs/release-gates.md for the full gate walk and docs/migration.md to
move existing work forward.

### Changed
- **The composition API is frozen (rc.1).** Every release gate in issue #52
  is met and evidenced in docs/release-gates.md, so the surface that was
  experimental through the alphas — `Song`/`Pattern`/`Phrase`, the pattern
  algebra, the `music` harmony types, `Song::compile` → `Program`,
  `Transport`/`Performance`, and the typed Python API — is **stable as of
  1.10.0-rc.1** (docs/api-tiers.md). Frozen with it: the document schema
  (SCHEMA_VERSION 2), the Program bundle format (PROGRAM_VERSION 2), and
  the DSP engine (ENGINE_VERSION 5). From here to v1.10.0, release-blocking
  fixes only.
- **Selected-range rendering**: `Program::render_range_frames` /
  `render_range_bars` (a slice of the full render, so tails crossing the
  boundary sound as in the full mix), and `program.render_range()` in
  Python.
- **Python wheels are build-from-source only for now.** The 3-platform
  wheel matrix is expensive in CI minutes, and this is a zero-budget
  project — the pipeline stays validated but manual-only
  (`workflow_dispatch`), and the crate README says so plainly. Prebuilt
  wheels come back if users ask for them.
- **The C ABI and WASM faces are out of scope for now.** Both shipped
  during the beta slices (stable `extern "C"` surface + C smoke test;
  wasm-bindgen API + AudioWorklet runtime, byte-identical to the offline
  bounce) and were dropped before release as overkill — native hosts
  embed `tono-core` directly. They can return on demand; the release
  gate in docs/release-gates.md records the decision.
- **House structure and contributor contract aligned.** The CLI package
  moved to `crates/tono-cli` (the root is a pure virtual workspace
  manifest; the `tono` crate name and publish path are unchanged), and the
  repository has one self-contained contributor surface: `RULE.md` (the core
  principle: *Less is more. Explicit is always better than implicit.*).
  Direct Cargo commands are the portable gates
  used by hooks and hosted CI, and the same CI runner builds and tests an
  installed Python wheel when core or binding inputs change.
- **User docs, revamped for scanning.** README leads with the
  compose → compile → run story; quickstart is a five-step card flow; the
  cookbook gained a complete node-vocabulary reference (delay, flanger,
  phaser included — they were undocumented) with every topic compressed to
  tables and caption-tagged recipes; runtime/migration/performance docs are
  Before/After and tables; all seven crate/face READMEs and the landing
  page follow the same code-first style. Fact fixes along the way:
  `.stats.json` layer field names, the streaming claims (v2 tracks roots
  stream natively), the Instrument capability list, migration's
  `bpm`/song-level `seed` names, the frames-estimate formula, and the
  README's broken `.hit()` chain.

### Deferred
- **Opus export** — evaluated for alpha.2 and deferred to the portability
  slice (1.10.0-beta.1): a production Opus encoder means binding libopus
  (a system dependency with its own versioning — the wrong thing to gate
  the composition slice on). WAV, FLAC, and OGG Vorbis remain the
  deterministic export formats; the decision is recorded here so the
  roadmap's "codecs" line has an explicit answer.

### Deprecated
- **The legacy JSON-string Python API** (`tono.render(doc_json)`,
  `Patch(json)`, `AdaptiveMusic.add_layer(doc_json)`) — deprecated per
  docs/api-tiers.md; the typed `tono.Song`/`tono.Program` API is the
  successor. It keeps working through v1.10.

### Fixed
- **A program swap rendered on the audio path.** Executing a scheduled
  `Command::Swap` built the new program's source (a full probe render, or a
  full offline bounce for a non-streamable program) inside
  `Performance::fill` — allocation and O(duration) work at the exact musical
  boundary, the violation the real-time gate exists to prevent. Swap
  sources now build at schedule time (the same discipline stingers already
  had), keyed by the command's seq and replayed swaps included; the new
  `rt_alloc` gate proves zero allocations across a swap.
- **Two RustSec advisories in `pyo3`** — RUSTSEC-2026-0176 (out-of-bounds
  read in `PyList`/`PyTuple` iterator `nth`) and RUSTSEC-2026-0177
  (missing `Sync` bound on `PyCFunction::new_closure` closures), both in
  pyo3 0.28.3. tono-py now builds on pyo3 + numpy 0.29, and
  `cargo deny check advisories` is green again.
- **Two contract violations found by the new proptest suite**, each pinned by
  a regression test: a tracks automation lane with exactly one point whose
  time is NaN (or any single-point lane at `sample_rate` 0) indexed past the
  lane end and panicked the renderer on an unvalidated doc — a single
  breakpoint now holds flat; and `vary::mutate`'s multiplicative jitter
  overflowed parameters validated without an upper bound (modal decay, filter
  q, ADSR times, …) to ±inf, breaking its still-valid promise — the jitter
  now clamps to `[min, f32::MAX]`.
- **A latent hang in the threaded SPSC test** — the pump/drain soak could
  spin forever under load (a full-block drain guard vs. a non-multiple
  tail); it now drains the exact tail once the producer is finished.
- **The deep-review sweep** (line-by-line over the engine-5 kernel, the
  streaming mixer, adaptive scheduling, MIDI, and the instrument), four
  fixes each pinned by a regression test verified failing on the old code:
  - `det::exp` mis-scaled its edge cases — `exp(710.0)` returned a wrong
    finite value instead of `inf`, and deep underflow wrapped back to
    normal-magnitude results instead of an exact subnormal/zero (the scale
    exponent reaches ±1075 inside exp's early-return bounds but was clamped
    to ±1022).
  - Quantized adaptive actions scheduled for the **same frame** fired in a
    `swap_remove`-scrambled order — of three intensity changes on one beat
    the middle call won; ties now fire in schedule order. And a transition
    queued behind a never-rendered reversal could defer to the current
    frame, slipping to the next block's edge (a block-size-dependent
    position); it now lands exactly one frame out, picked up by `fill`'s
    boundary scan.
  - **MIDI export retimes multi-tempo documents** — a 60 bpm seq in a
    120 bpm file played back at half speed; ticks now scale by global/seq
    bpm so every seq keeps its absolute time (the docs already promised
    retiming; the code now does it).
  - A deserialized `InstrumentDesign` with `unison_width > 1` (the builder
    clamps it; serde doesn't) gave unison copies negative pan gains — the
    pan now clamps where the invariant is consumed.

### Added
- **Engine revision 5 — cross-platform byte-identity** (beta.1, issue #52):
  the per-platform libm limitation is retired. New `det` kernels
  (fdlibm-grade `sin`/`cos`/`exp`/`ln`/`powf`/`tanh`/`log10` in pure IEEE
  f64, correctly rounded to f32) drive every transcendental in the render
  path for documents stamped `engine: 5`, and convolution runs a
  fixed-order radix-2 FFT with deterministic twiddles and documented
  power-of-two sizing — output is identical on every supported target by
  construction. Older revisions render bit-for-bit as before (their corpus
  pins are untouched); new documents and songs stamp 5 by default. The
  golden corpus now asserts one shared pin set for engine-5 cases on both
  macOS-arm64 and linux-x86_64, every CI run.
- **Program bundles carry target and capability metadata** — a Program
  records its compile target and answers a machine-readable capability
  list ("offline-render", "stems", "streaming"), derived from the
  document, and `tono compile --inspect` prints both.
- **Prebuilt CLI binaries** — every release tag builds `tono` for
  linux-x86_64, macos-aarch64, and windows-x86_64 with sha256 sidecars,
  uploaded to the GitHub Release.
- **`Transport` — the sample-accurate musical clock** (alpha.3,
  experimental): position in frames with exact conversions to beats and
  bars through the program's tempo/meter maps and pickup (the same shared
  walks the compiler uses); play/pause/stop, seeks by frame/beat/bar, and
  loop ranges that wrap at the boundary or stop at the program end.
- **`Performance` — the scheduled runtime for a compiled Program**
  (alpha.3, experimental): transport + a bounded, submission-ordered
  command queue executing at exact frames — the host never wakes on a
  musical boundary. Playback is native streaming for schema-v2 tracks
  programs (byte-identical to the bounce), the pre-rendered bounce
  otherwise. Commands: play/pause/stop, seeks by frame/beat/bar/section,
  loop ranges, ramped master gain, stingers (loaded at schedule time),
  and crossfaded program swaps whose rejected targets keep the last valid
  program. A full queue rejects and counts; metrics read off the audio
  callback with no allocation on it; command capture + replay reproduces
  a take bit-for-bit; section transitions quantize with latest-wins
  interruption; snapshots restore control state deterministically.
- **`tono.Performance` (Python)** — the same runtime surface, live
  (speakers, on the Engine's pump architecture, with all control off the
  audio callback) or `headless=True` for tests and servers
  (`.fill(frames)` renders manually with the GIL released); scheduling
  via `tono.next_bar()`-style helpers.
- **The streaming mixer** (alpha.3): schema-v2 `tracks` roots now stream
  natively, byte-identical to the offline bounce at any block size — every
  track's graph, per-sample pan/gain with automation lanes (linear/step/exp),
  sidechain duck envelopes, bus routing and post-fader sends, bus insert
  chains with decorrelated reverb, and the master chain, with the peak-limit
  gain probed at load. `StreamGraph::blockers` now reports unstreamable
  parts with context (`track 'pad'`, `bus 'verb'`, `the master chain` + the
  node-level cause); still blocked, with the report saying so: `normalize`,
  `loop` playback, stereo treatments, sampler tracks, and schema-v1 roots.
  A streamable compiled song now comes back from `Song::compile` with no
  streaming warnings.
- **Tempo and meter maps, pickup, sections, markers** (alpha.2, experimental):
  tempo changes at exact rational beat positions applied segment-wise (a
  note crossing one keeps its musical length; placement rounds halves away
  from zero; swing/humanize follow the local tempo); time-signature maps
  with numerator AND denominator (6/8 = 3 quarter-beats) plus
  pickup/anacrusis; named sections and markers preserved into the Program
  for the runtime's quantized transitions. Everything validates with
  stable codes (T1003–T1006), a placement between grid steps is a loud
  error, and songs without maps compile byte-identically. The sampler
  keeps its constant-tempo path (validation rejects a map there).
- **The `music` module** — harmony vocabulary (experimental): `Pitch`,
  `PitchClass`, `Interval`, `Scale`, `Key`, `Chord`, `Voicing` with one
  strict spelling grammar shared with the DSL and no silent guessing —
  "H4", "C", "CM", "C sus4" are loud errors. Transposition is
  bounds-checked; chords invert and arpeggiate; voicings give close,
  drop-2 open, and slash-bass spacings.
- **Pattern and rhythm operations** (experimental): pure transforms on
  song patterns — repeat/concat/layer/slice/transpose/stretch/rotate/
  reverse/quantize, `vel`/`gate` scaling, Euclidean and tuplet
  constructors, and deterministic `probability`/`humanize` per
  (pattern, seed). Exact or loud: off-grid stretch errors name the note;
  transpose preserves `midi:N` form so a kit stays a kit.
- **Automation curves and beat-addressed lanes** — track automation gains
  `step` (hold then jump) and `exp` (geometric between positive
  endpoints, linear fallback) curves; omitted = linear, byte-identical.
  Song automation is addressed in beats and compiled through the tempo
  map segment-wise.
- **Mixer buses, inserts, sends, and returns** — a tracks root gains
  named `buses` with insert chains and return faders; tracks route with
  `bus` and feed post-fader/post-duck copies with `sends`. Bus inserts
  run with id-keyed streams and decorrelated reverb tails; returns land
  on the master bus ahead of the master chain. Additive: no buses =
  the legacy mix, bit-for-bit. Songs carry `buses` plus per-track
  `bus`/`sends`.
- **Stems** — `render_stems` decomposes a mix into every track's
  positioned stereo contribution (pre bus/master; muted tracks silent)
  plus every bus's processed return, each carrying its routing:
  master-routed stems plus bus returns sum to exactly the mix the master
  chain hears. `tono render --stems DIR` writes stereo WAVs per stem;
  `Program.render_stems()` returns a dict of arrays with `stem_routing`.
- **MIDI through Song** — `tono import FILE.mid --song` writes a Song
  (one track per MIDI track, notes direct, first tempo event sets bpm);
  `tono midi SONG.json --song` exports through `to_doc`.
- **The alpha.2 Python surface** — all of the above in the typed API:
  tempo/meter maps and pickup (exact-rational beats via int/Fraction/
  (num, den)), sections/markers, buses with typed effect inserts,
  track routing, automation, pattern ops as `Pattern` methods, and
  `Pitch`/`Key`/`Chord` harmony wrappers — all validating eagerly.
- **`Song::compile() -> Program` — the immutable compiled artifact**
  (experimental through the 1.10.0 alphas). One validation pass collects
  every problem — unknown track/pattern references with their exact paths,
  a resolved document that fails validation — each a structured diagnostic
  with a stable code, severity, path, message, and fix. The returned
  Program carries the resolved document, the musical metadata a transport
  needs (tempo, grid, bars, duration in seconds and frames, a track roster
  with stable declaration-order ids), bounded resource estimates (frames,
  events, peak voices, memory), streaming blockers as offline warnings or
  runtime-target errors, and a
  canonical semantic hash (FNV-1a over sorted-key JSON) that an equivalent
  song reproduces from Rust or Python. Programs serialize as versioned
  bundles (`PROGRAM_VERSION` 2, independent of the schema/engine pins);
  loading rejects a newer revision and re-verifies the hash — never
  recompiles. Architecture decisions and the stable/experimental/internal
  API tiers are recorded in `docs/adr/` and `docs/api-tiers.md`.
- **Exact musical time + typed units + stable ids + structured
  diagnostics** — `Beat`, an exact rational musical position (tuplets and
  repeated transforms never drift), with `beat_to_frames` the single,
  specified crossing to audio frames (halves round away from zero);
  `Frames`/`Samples`/`SampleRate`/`Hertz`/`Decibels`/`Tempo`/`Bars`
  newtypes; `TrackId`/`PatternId`/`PlacementId`/`ParamId`/`BusId`; the
  `Diagnostic`/`CompileError` types behind `Song::compile`.
- **Song-level `seed` and track `mute`/`solo`** — a song's seed stamps the
  compiled document (same song + same seed ⇒ same artifact); solo follows
  console semantics (every non-solo track mutes; a muted solo stays muted).
  Additive: songs without the new fields compile byte-identically.
- **The typed Python API** (experimental): `tono.Song` / `Pattern` /
  `Track` / `Program` and the `tono.instruments` catalog wrap the native
  Rust model — no JSON in the build→compile→render path, the GIL released
  for compile and render, structured `tono.CompileError` exceptions with
  `.diagnostics`, and documented NumPy buffers (stereo `(frames, 2)`
  float32 owned copies). The package ships `py.typed` and complete stubs;
  a cross-language fixture pins that the reference song compiles to the
  same hash from Python and Rust.
- **`tono compile SONG.json [-o FILE] [--sample-rate N] [--inspect]`** —
  compile a song to a Program bundle from the shell; `--inspect` prints
  the machine-readable summary (hash, version pins, roster, estimates,
  warnings) and writes nothing. A failing compile exits non-zero, so it
  doubles as a CI gate for song projects.
- **Criterion benchmarks + proptest validation fuzzing** — `cargo bench -p tono-core`
  runs five criterion benches over the render hot path (osc/env, a tracks
  mix, a piano seq, an FX chain, streaming fill), and CI can run them by
  explicit manual dispatch and publish the numbers (no gate: shared runners
  are too noisy for a hard threshold). `tests/fuzz_validation.rs`
  property-tests the crate contract: parse/validate never panics, a validated
  doc renders finite samples, an unvalidated (poisoned) doc still can't panic
  the renderer, and `mutate` always re-validates.
- **New `convolve` node** — convolution reverb with a deterministic
  synthetic IR (no IR files): a seeded noise burst decaying to −60 dB over
  `decay`, darkened by `damp`, after `predelay`, convolved FFT-fast and
  truncated to the document length like `reverb`. rustfft is a full
  dependency now (previously analysis-only).
- **New `granular` node** — granular texture: Hann-windowed grains of the
  input at `density`, repitched by `pitch` with seeded `spread` jitter (the
  whole schedule is drawn up front, so it's deterministic). Magic shimmer,
  wind, UI clouds.
- Both are offline-only (they need the whole input buffer) and say so through
  the streaming coverage report: `StreamBlocker::OfflineEffect` names the
  node and the fix, and `try_from_doc`'s fallback stays in agreement (pinned
  by the blockers corpus tests).
- **Tracks-level sidechain ducking** — a track can carry
  `sidechain: { source, amount, attack, release }` and duck following another
  track's positioned, post-fader signal (the kick→pad pump at the mixer
  level, byte-for-byte the `duck` node's envelope math). Validation rejects
  unknown sources, self-follows, and follower-of-follower chains; a muted
  source ducks nothing; `amount: 0` and absent fields render bit-identically
  to before. Deterministic regardless of declaration order. Schema-v2
  sidechained mixes stream natively (see the streaming-mixer entry).
- **New `wavetable` node** — a morphing wavetable oscillator with four
  built-in deterministically generated table sets (`basic`, `harmonics`,
  `formant`, `metallic`): a modulatable `position` morphs across each set.
  Table data and lookup are shared by both renderers, so it streams
  byte-identically — position LFOs and freq slides included.
- **7 new catalog voices, 5 new presets** — the catalog grows 24 → 31
  voices: `Brass` (section, stab), `Flute` (concert), `Mallets` (marimba,
  vibraphone, glockenspiel), and `Bells` (tubular) on the new seq waves; the
  factory presets grow 11 → 16 with `brass_stab`, `flute_lead`, `marimba`,
  `bell`, and `dark_pad` (the moody counterpart to `supersaw_pad`). All are
  reachable from `tono catalog` / `tono presets`.
- **New seq voices: `brass`, `flute`, `mallet`, `bell`** — param-free fixed
  models in the style of `organ`/`strings`/`epiano`: a detuned saw pair
  through an opening lowpass (the horn blat; velocity brightens), a sine with
  fade-in vibrato over lowpassed breath noise, a marimba-like fundamental
  with fast strike partials, and an inharmonic bell partial stack whose highs
  die first. The streaming renderer pre-renders seqs via the exact offline
  synthesis, so all four stream byte-identically by construction.
- **New `tremolo` node** — per-sample amplitude modulation
  (`rate` 0..40 Hz, `depth` 0..1) as a closed-form processor. A modulated
  `gain` renders offline but can't stream; `tremolo` streams natively and
  byte-identically at any block size.
- **New `review` archetypes: `footstep`, `powerup`** — target tables for two
  more staple game SFX (a very short low-mid thud; a short bright rising
  flourish), in `tono review --archetype` and the library alike.
- **`StreamGraph::blockers(doc)`** — the streaming renderer's coverage check
  as an actionable report: one `StreamBlocker` per blocking feature
  (`normalize`, `loop` playback, Haas/Wide stereo, a `tracks` root, RNG nodes
  under `engine < 2`, the sampler seq, a modulated filter/EQ/gain cutoff),
  each with a Display message naming the fix. `try_from_doc` now delegates
  its gate to it, so the silent `Option` fallback and the report can never
  disagree (a test pins the equivalence).
- **`tono presets [NAME]` / `tono catalog [NAME]`** — the factory sounds from
  the shell. No NAME lists (16 presets with blurbs; 31 catalog voices by
  family); with NAME it renders a demo — a preset plays a C-major arpeggio
  through the live `Instrument` engine bounced offline, a catalog voice plays
  a scale resolving to a chord (a two-bar groove for the drum kits) — with
  the full `tono render` output set (audio + images + stats).
- **`tono fit REF.wav DOC.json`** — target-driven sound design, automated.
  `tono match` scores a doc against a reference WAV; `tono fit` closes the
  loop with a deterministic seeded hill-climb over `vary::mutate` — every
  candidate is a perturbation of the incumbent, kept when the match score
  improves, with the step size halving after stalled rounds. Writes the best
  doc (`<doc>.fit.json` or `-o`) and prints its final match table. The search
  is a pure function of `(reference, doc, rounds, amount, seed)`.
- **`tono review FILE.json [--archetype KIND]`** — the `review` grader
  (archetype targets + the universal ship checklist) as a CLI subcommand:
  render, grade, and print every finding worst-first with the measured value,
  the target, and the fix to try. Exits non-zero on a FAIL grade, so it works
  as a ship gate in CI. Level metrics measure the stereo pair when there is
  one, matching `tono render`.

## 1.9.0 — 2026-07-19

A full-codebase review hardening pass: the input edges that could render NaN,
hang, panic, or silently corrupt are closed, the real-time layers keep their
promises at any block size, and every fix lands with its regression test.
Every pre-existing document still renders byte-for-byte (the golden corpus is
unchanged); the new validation caps only reject documents that previously
produced NaN, silence, a hang, or ultrasonic output no host can play (a
finite-but-unplayable pitch like `"midi:170"` is an authoring error now —
only unvalidated direct renders see the note-fallback change).

**Versioning policy:** tono never ships a 2.0. Breaking changes land in
ordinary 1.x minors, and deprecated surface is removed directly in the next
minor — no long-lived deprecation shims. The byte-identity promise is a
product guarantee, independent of version numbers.

### Removed
- The surface deprecated at 1.6.0–1.8.0: `tono_core::stream` (use `player`),
  `catalog::Instrument` (use `catalog::Voice`), `tono_core::voice` (use
  `instrument::EnvGen`), `tono::audio::write_wav` (use `write_wav_stereo`),
  `streaming::is_streamable` (use `StreamGraph::try_from_doc`),
  `EffectChain::is_empty` (a constructed chain is never empty), and
  `MixerError::NoSampleRate` (every `Mixer` constructor takes the rate).

### Changed
- **One graph traversal**: `Node::children()` / `children_mut()` (plus
  `walk` / `walk_mut`) is now the single definition of "direct children"
  (mix/mul inputs, chain stages, a tracks' layers then master, a duck's
  trigger) — and every walker uses it, so a new variant can never be silently
  skipped. Two latent omissions it exposed are fixed by construction:
  `humanize`'s transpose now reaches a tracks' **master chain**, and the
  instrument's transpose now reaches **duck triggers**.
- **One modulator evaluator**: the offline `eval_value` is a loop over the
  streaming renderer's `Val` — offline and streaming can no longer diverge
  (the unification also fixed a divergence in the `rand` rate hang clamp).
- **One home for shared kernel math** (`dsp.rs`): modal resonator
  coefficients, the Freeverb layout and feedback, and the delay-line and
  bitcrush clamps — previously mirrored line-by-line between the offline
  effects and their streaming twins. All expression-identical: every render
  is byte-for-byte as before.
- **Structure**: `adaptive.rs` splits into `adaptive/{schedule,sections,
  layers}` and `song.rs` into `song/{compile,phrase}`.
- The CLI parser grows value-less boolean flags (`--watch`).

### Added
- **`tono diff A.json B.json`** — render two documents and report what
  changed: loudness, peak, centroid, envelope metrics with deltas, and the
  sample-domain distance.
- **`tono match REF.wav DOC.json`** — target-driven sound design: score a
  candidate against a reference WAV in the analyzer's own metrics, worst
  offenders first, with an overall distance score.
- **`tono render --watch`** — re-render on every save (mtime polling, no
  dependency); a mid-save invalid doc is reported and watched through.
- **`tono play`** — audition a doc through the speakers (feature-gated on
  `play`, so the default install and crates.io publishing stay lean).
- **docs/quickstart.md** — the guided first ten minutes; the README gains a
  "Where next" routing block, and `docs/README.md` indexes the guides.

### Fixed
- **Validation rejects the overflow regime.** Pitches resolving to non-finite
  Hz (`"midi:10000"`), huge octave numbers (`"A200000000"` — it could panic the
  parser's i32 arithmetic), `super.detune_cents` above 10 octaves, `fm.ratio`
  above 4096, constant frequencies above 100 kHz (and modulated frequency
  endpoints above ±1 MHz), `rand` rates above 10 000/s (a validated doc could
  hang the renderer for hours), and a non-finite `compress.ratio` all fail
  validation with a clear message instead of rendering NaN or hanging.
- **Silent-authoring-error guards.** A `chain` leading with a processor, a
  bare-processor document root, duplicate automation lanes for one target, and
  an all-zero `env` *modulator* ADSR (the flatten footgun `Node::Env` already
  caught) are validation errors now, not digital silence / dead knobs.
- **Unvalidated documents can't panic the renderer** (the codebase's stated
  contract): bitcrush `bits ≥ 32`, negative `piano_inharm`, sample rates below
  40 Hz, and absurd durations (an allocation abort) are clamped defensively;
  `peak_limit` scrubs non-finite samples instead of passing NaN to the
  encoders; and graph validation is depth-capped and stack-safe for
  programmatically-built documents.
- **Adaptive music is block-size invariant.** Quantized stingers, intensity
  changes, and transitions fired up to one host block early; they now apply at
  their exact frame, and section cross-fades compute from an absolute frame
  count — a 128-frame AudioWorklet and a 512-frame cpal callback render
  identical audio. Also: a mid-fade `transition_to` the fade's target is a
  no-op, requesting the previous section cancels the fade back (click-free),
  a mid-fade switch to a third section lets the in-flight fade complete
  before the onward transition (no hard cut), `AdaptiveMusic` honors
  `AudioSource::reset` through trait objects, and layer cross-fades snap at
  their target instead of asymptoting forever.
- **Runtime engine hardening.** A `PatchId`/`ParamId` from another `Engine`
  resolves inert instead of panicking (the documented contract); a param
  change landing mid-crossfade carries the blend weight instead of restarting
  the fade; a NaN `pan`/`glide` can no longer poison the whole mix; `split(0)`
  floors to a one-frame ring instead of silently never playing.
  `Engine`/`Mixer`/`StreamSource` pre-allocate their scratch (no callback
  allocation for blocks up to 8192 frames), and the docs state the real
  threading contracts (`Engine`'s mutating calls are O(duration) — use
  `split` for real-time).
- **Song compile.** `u32` arithmetic on note steps/bars saturates instead of
  panicking in debug or wrapping in release; a degenerate `bpm` (< 1) clamps
  consistently for duration AND note placement (notes past bar 0 used to drop
  silently); `add_track` slugifies and dedups names like the fluent path.
- **Instrument.** MIDI convention honored: `note_on` with velocity 0 is a
  note-off (it used to leak a stuck silent voice); `transpose` leaves an
  unparseable note as authored instead of substituting a silent A4;
  `with_tremolo(0.0, depth)` is off instead of a constant gain cut.
- **CLI.** MIDI export includes seqs inside `duck` triggers (a doc whose only
  seq was the kick trigger exported note-less) and saturates pathological step
  values instead of overflowing; `tono import` / `tono midi` no longer
  silently overwrite an existing default output file; doc names can't escape
  the output directory; OGG encodes in 8192-sample blocks (a whole-render
  block is orders of magnitude slower in libvorbis).
- **tono-py / tono-desktop.** `stinger` renders off the shared pump lock (it
  used to render under it — an audible dropout); `Engine(sample_rate=0)` is
  rejected; the desktop deck keeps up to two fade generations so rapid doc
  swaps don't hard-cut and pre-allocates its callback scratch; `analyze`
  surfaces PNG-encode failures instead of reporting success with empty images.
- `Patch::instantiate` can't panic on NaN parameter specs (a NaN value skips
  the write); `mutate` clamps to every validation bound (the 30 s delay cap
  and the new detune/rand-rate/frequency caps) so a mutated doc always
  re-validates; `humanize`'s coherent transpose reaches `duck` triggers.
- **Future-variant guards.** `Node::children()`/`children_mut()` are
  exhaustive over every current variant — adding a variant without a
  traversal decision is a compile error, never a silent skip — and
  `apply_processor` now fails loud (like `render_node`) for a processor with
  no render arm instead of silently passing the signal through.

## 1.8.0 — 2026-07-11

The structure release: a full quality review swept every lens, the god-files
split into module directories, the native faces share one cpal shim, and the
long-deprecated names are staged for removal in the next minor (tono never
ships a 2.0 — removals land in ordinary minors). Every pre-existing
document still renders byte-for-byte (the golden corpus and the
offline/streaming byte-identity fuzz are unchanged).

### Deprecated (removed in the next minor)
- The 1.6.0 rename aliases, now through two minors: `tono_core::stream`
  (use `player`) and `catalog::Instrument` (use `catalog::Voice`).
- `tono_core::voice` — `EnvGen` lives with its only consumer as
  `instrument::EnvGen`; the module shim keeps the old path valid.
- `tono::audio::write_wav` (mono; the stereo writer is the export path),
  `streaming::is_streamable` (call `StreamGraph::try_from_doc` — it was a
  misdocumented full build-and-discard), and `EffectChain::is_empty`.

### Changed
- `SoundDoc::validate()` is filesystem-free: it no longer stats a sampler's
  `.sf2` path (the same valid doc used to validate differently per machine,
  against the core's no-I/O contract). Loaders call the new pure
  `SoundDoc::sf2_paths()` and check existence themselves — the CLI and the
  Python bindings already do, so their behavior is unchanged. A caller that
  relied on `validate()` to catch a missing file now gets the error at load.
- `Node::Seq`'s per-voice knobs are grouped into `serde(flatten)`ed structs
  (`FmKnobs`/`PluckKnobs`/`PianoKnobs`/`BassKnobs`/`Sf2Knobs`). The JSON wire
  shape is untouched; Rust code that pattern-matched the old flat fields on
  this variant must switch to the structs.
- `tono-desktop` drops its `play` subcommand (use
  `tono_play::play_doc` / `make play`, which streams byte-identically).

### Added
- `tono render` writes the documented `smpl` loop chunk again for
  `playback: loop` WAV exports (silently regressed when the MCP server was
  removed).
- `analysis::spectral_frames` + `stats_with`/`stats_stereo_with`/
  `spectrogram_png_with`: one STFT now feeds both the numeric stats and the
  spectrogram (it was computed twice per render analysis).
- `tono_play::Speaker::open_at` (explicit stream rate) and `Speaker::shared`;
  the desktop deck and the Python engine stream through this one cpal shim.
- `make verify-native` (clippy + tests for the off-CI native crates, examples
  included) with a path-filtered CI workflow; `make play EXAMPLE=<name>`;
  `make python-test` / `python-smoke` / `site` / `version` — CI workflows now
  exec make targets only, and CI validates the golden pins on macOS too.
- The GitHub Pages site gains an architecture & getting-started page.

### Fixed
- Real-time hardening in the streaming path now matches the offline renderer
  exactly (empty `arp` steps, `secs == 0` slides, sub-240 Hz filter clamps).
- The instrument's modulation LFOs derive phase from accumulators instead of
  an absolute `f32` clock — vibrato/tremolo/wobble no longer go steppy after
  ~3 hours of live play (frozen after ~6).
- `AdaptiveMusic`: a stinger with unequal channel lengths could stall the
  spent-stinger cull; the loop play-head no longer overflows on very long
  sessions; `add_stem_set` no longer renders the first stem twice.
- Validation rejects NaN/±inf knobs everywhere (a `1e308` JSON literal used
  to cast silently to `inf` and render garbage), and validates
  `compress.threshold`/`makeup` and `super.freq` like their siblings.
- `describe()` fails loud instead of returning an empty map; review summaries
  are no longer ALL CAPS; the LUFS field's doc says gated (the meter always
  was); `tono-py`'s crate type no longer collides with the root crate's rlib.

## 1.7.0 — 2026-07-11

Audio real-time safety and mixer/adaptive correctness from a full review of the
1.6.0 sprint, plus phase-locked stem sets. Every pre-existing document still
renders byte-for-byte (the golden corpus is unchanged).

### Added
- **Phase-locked stem sets** on `AdaptiveMusic`: `add_stem_set(stems,
  duration_beats)` forces every stem onto one shared loop length (from the tempo,
  or the first stem's natural length without one) so layered intensity
  cross-fades stay sample-aligned and never drift phase; returns the grid length
  in frames. Plus `LoopBuffer::from_doc_len(doc, frames)` — render and loop a doc
  at an exact frame count.
- Off-lock entry points so a real-time wrapper never renders under a lock:
  `AdaptiveMusic::add_section_buffer`, `stinger_stereo`, `stinger_stereo_at`
  (mirroring `add_layer`). The doc-taking `add_section`/`stinger`/`stinger_at`
  now delegate to them.

### Fixed
- **Mixer**: the master fader (`set_bus_gain(MASTER, …)`) was a no-op; sources
  added directly to an FX bus were silently dropped. `write_interleaved` no
  longer reads past a short source slice.
- **Adaptive music**: a transition to the already-current section double-filled a
  buffer (audible speed-up); pending transitions now dedup/supersede; `duck()`
  ramps in instead of stepping (no click) and recovery snaps to unity.
- **Render path**: guarded divide-by-zero / NaN on unvalidated docs (empty `Arp`
  steps, `Slide` `secs == 0`, `soft_limit` `ceil == 0`, low-sample-rate filter
  clamps).
- **Real-time callbacks**: `tono-play` no longer blocks the audio thread on the
  control lock (`try_lock` + silence); all cpal callbacks are wrapped in
  `catch_unwind` (a render panic can no longer unwind across the C frame);
  `tono-py` `Engine::new` no longer leaks the audio thread + stream on a
  pump-spawn failure.

## 1.6.0 — 2026-07-11

The game-audio release: live DSP buses, voice management, beat-quantized
interactive music, and Python bindings — plus a verified bug sweep, a corrected
output stage behind engine revision 4, the native pattern station, and an
organization/API pass. Every pre-existing document still renders byte-for-byte
(a golden corpus now pins this in CI).

### Added
- **Python bindings** (`crates/tono-py`, PyO3): a live `Engine` owning the
  output stream (drum kit, preset instruments, adaptive music, zero-asset patch
  triggers — the audio thread never touches Python), and a numpy pull API
  (`tono.render`, `Patch.render(**params)`), deterministic and CI-testable.
  Build with `make python`; abi3 wheels build in CI.
- **Live DSP effects on mixer buses**: sources feed named buses with insert
  chains (reverb/EQ/compressor/delay/…), post-fader sends into shared FX/return
  buses, and a master chain — all reusing the streaming effect kernels, so a
  bus stays byte-identical to the offline processors. `Mixer::new_at`, `bus`,
  `fx_bus`, `add_to`, `set_bus_effects`, `master_effects`, `set_send`.
- **Voice management**: an opt-in polyphony budget with priority stealing.
  `Engine::set_max_voices`, `Priority` (`LOW`/`NORMAL`/`HIGH`/`CRITICAL`),
  `play_prioritized` / `play_looping_prioritized` / `set_priority`; the victim
  declicks instead of hard-cutting, an outranked voice is denied, and a flood is
  hard-bounded at 2× the budget. `DrumKit::with_max_voices` tunes the kit's cap.
- **Interactive music v2** on `AdaptiveMusic`: a musical clock
  (`set_tempo`/`beats`/`bars`), `Quantize` (`Beat`/`Bar`/`Bars(n)`) scheduling
  for `set_intensity_at` and `stinger_at`, and horizontal **sections**
  (`add_section` + `transition_to`) that cross-fade on the bar — swap "explore"
  for "battle" without a mid-phrase cut.
- **The pattern station** (`make desktop`): a native Tauri studio with
  real-time audio — an FL-style step grid over the catalog instruments,
  click-free live editing, per-track faders, undo, and per-edit
  LUFS/spectrogram feedback. Off the default build and CI.
- `AdaptiveMusic` transport for beat-locked games: `pause`/`resume`/`is_paused`,
  `reset` (rewinds the position clock to 0 and every layer to its loop head),
  and `position_frames()` — the musical clock a game derives its beat position
  from. Plus `duck(depth, release)`, a fast master sidechain for stingers/SFX
  independent of the slower intensity cross-fade.
- `AudioSource::reset()` (default no-op; `LoopBuffer` overrides it) so a
  transport can rewind a looping source to its head.
- `runtime::spsc` generalizes the wait-free split over any `AudioSource`
  (`Pump<S>`; `Controller = Pump<Engine>` unchanged), and
  `runtime::write_interleaved` is the one channel-spread every output adapter
  shares. `tono midi` prints its notes/tracks summary.
- Infrastructure: a `v*` tag now auto-creates its GitHub Release with the
  CHANGELOG section as notes; the showcase site deploys to GitHub Pages.

### Fixed (no rendered bytes change)
- `delay.secs` is bounded — an unbounded value passed `validate()` then aborted
  the process on an arbitrary allocation; constants/modulator endpoints must be
  finite (1e308 rendered NaN buffers); automation lanes are validated.
- The split engine no longer loses frames when over-pumped, and an odd-length
  underrun no longer permanently swaps L/R.
- `StreamSource` carries the bounce's peak-limit gain (streams matched the raw
  graph, playing louder than the bounce); loop/stereo docs fall back to the
  `Player` instead of playing un-looped/un-widened; the `fold` waveshaper can
  no longer hang the audio thread on a non-finite sample.
- Voice stealing declicks with a ~5 ms fade (instrument + drum kit) instead of
  a hard mid-sample cut.
- MIDI export carries velocity, puts drums on channel 10, and no longer drifts
  on non-divisor grids; CLI flags consume their values and unknown options are
  loud errors; morph no longer lerps `engine`/`seed`; a stack of smaller fixes.

### Engine revision 4 (opt-in via the doc's `engine`; new songs stamp it)
- Loudness normalization measures the whole stereo program with ONE shared gain
  (the per-channel stage collapsed asymmetric mixes toward center), using gated
  BS.1770 loudness at the doc's actual sample rate, and enforces `ceiling_dbtp`
  against a real oversampled true-peak estimate.
- Humanize jitter is seeded per note, so chords stop moving as a block.
- `Song` pins `engine`/`version` at creation: saved projects replay
  byte-identically across kernel upgrades.
- All metering (analysis, CLI, desktop) now reads the stereo pair that ships,
  with oversampled true-peak and gated loudness.

### Changed (API)
- `stream` → `player` (deprecated alias kept); `catalog::Instrument` →
  `catalog::Voice` (deprecated alias kept).
- `#[non_exhaustive]` on the enums and builder-structs that grow every release.
- Dead `voice::{BandOsc, Voice, PolySynth}` removed (only `EnvGen` was used).
- `render`/`dsl` split into focused submodules (same public paths);
  `tono_core::prelude` added; the root `tono` crate re-exports every module;
  `missing_docs` is enforced.

### Known limitation (documented)
- Byte-identity holds per platform: platform libm (`sin`/`cos`/`exp`/`powf`)
  differs between macOS-arm64 and linux-x86_64, so the golden pins are
  per-platform. Deterministic transcendental kernels are future work.

## 1.5.0 — 2026-07-03

Per-track mixing on the song builder. Instruments gain `.reverb(0..1)` (a reverb
send that wraps the track in a reverb), and `.swing(0..1)` / `.humanize(0..1)`
to override the song-global groove per track. Byte-safe — a dry, unset track is
identical to before.

## 1.4.0 — 2026-07-03

A composition system on the one deterministic core: a catalog of ready-to-play
synthesized instruments and a fluent multi-instrument song builder, plus a deep
pass on the instrument voices. Pure synthesis — no soundfonts, no files — and
every change is byte-safe: existing documents render bit-for-bit as before.

### Instrument catalog + song builder
- **`tono_core::catalog`** — ready-to-play instruments (grand piano, electric
  piano, organ, strings, bass, guitar, drums) with variants, each a tuned voice
  you hand to the song builder.
- **`Song::add(instrument, |t| …)`** — a fluent, beat-timeline builder: place
  notes with `.at(beat).note/.chord`, step a melody with `.play/.rest`, hit drums
  with `.kick/.snare/.hat`. Compiles to the deterministic `tracks` SoundDoc.
- `cargo run -p tono-play --example lofi` / `band` — full songs in a few lines.

### Deeper voices (all byte-safe, opt-in per variant)
- **Grand piano** — an inharmonic additive model (stretched partials, per-partial
  decay, hammer-strike spectrum, detuned unison), gated at `engine` 3; six
  variants (bright/mellow/felt/upright/honky-tonk) via five tone knobs.
- **Drums** — four synthesized kits (classic/acoustic/electronic/808).
- **Bass** — finger/pick/sub/synth via ten tone knobs.
- **Guitar** — nylon/steel/electric via body resonance, pick noise, and tone.

## 1.3.0 — 2026-07-01

The engine becomes a **library + CLI**. The MCP server is removed entirely;
`tono-core` is the published deterministic engine and `tono render` is the CLI
that turns a `SoundDoc` into audio plus analysis images. Install via Cargo
(`cargo add tono-core` / `cargo install tono`).

## 1.2.0 — 2026-06-28

Higher-fidelity synthesis gated so it never breaks byte-stability, a workspace
split, and a leap from "headless engine" to a **studio you can design *and*
play sound in** — a browser playground and an optional native desktop app, both
on the one deterministic core, with the MCP face unchanged.

### Real-time engine + native desktop studio
- **`tono-core::stream::Player`** — the host-agnostic audition seam an audio
  callback fills in blocks. The invariant that makes live editing safe is pinned
  by test: audio served block-by-block is **byte-identical to an offline bounce**
  of the same document.
- **Playable synth** — a gated streaming `voice` (band-limited oscillator + ADSR
  with gate-on/off, reusing the renderer's exact kernels) and a `PolySynth`
  voice allocator with voice-stealing. The live-performance path, distinct from
  the byte-identical offline render.
- **`tono-desktop`** — an **optional** Tauri + `cpal` native studio running
  the full node patcher with real-time audio: edits play live, ▶ Play auditions
  the patch, and you **play the patch like an instrument** from the computer
  keyboard (A–K) or a hardware **MIDI** controller (native CoreMIDI via `midir`),
  mixed with the preview. Kept out of the default build / CI (heavy webview/cpal
  deps); built via `make desktop`.

### Manual studio editors (one frontend, web + desktop)
- **Node patcher** picks its backend at boot — WASM + Web Audio in a browser, or
  native `cpal` + the core via Tauri commands on the desktop — so one frontend
  serves both.
- **Piano roll** for `seq` nodes (draw notes, length, bpm/steps-per-beat).
- **Channel-strip mixer** for `tracks` documents — vertical faders, pan, mute,
  **solo** (transient: heard, not saved), and **live per-layer meters** from the
  render's per-layer stats; a master strip with the bus meter + LUFS.
- **Inline modal-modes table** (freq/decay/gain per partial) — closes the last
  "edit in JSON" gap in the patcher.

### Track automation
- **`Track.automation`** — gain/pan lanes of `{t, v}` breakpoints over song time
  (volume rides, pan moves), linearly interpolated. A track with no automation
  stays on the constant fast path, so every existing document renders
  **byte-identically**; tests pin a constant-lane-equals-static invariant and a
  ramp that provably fades. Settable by a caller through the existing graph tools
  (`set_param` / `edit_sound` / `refine_sound`), and drawn in a lane editor in
  the playground mixer.

### Interop
- **`export_midi { id, dest? }`** — write every `seq` to a Standard MIDI File
  (one track per seq) so a melody / drum pattern round-trips into a DAW.

### Repo standards
- Engineering-standards pass: `LICENSE` (dual MIT/Apache), `.editorconfig`,
  the contributor guide, `.env.example`, a `pre-commit` hook, and the canonical Makefile
  targets; default branch is `master`; the committed WASM is built with
  `--remap-path-prefix` so it carries no build-machine paths.

### Tool surface consolidation (30 → 23)
- **Op-based merges** for the admin clusters, so the client picks from a smaller,
  cleaner surface: `history { id, op: status|undo|redo }` (was undo_sound /
  redo_sound / history); `layer { id, op: add|set|remove|duplicate, … }` (was
  add_layer / set_layer / layer_ops); `bank { op: create|add|list, … }` (was
  create_bank / add_to_bank / list_banks); `export_pack { bank_id?, … }` (was
  export_bank / export_all — omit `bank_id` for the whole library). The hot
  authoring loop (author_sound, set_param, edit_sound, analyze, review_sound,
  …) is untouched, and `export` (single sound) stays its own tool.
- **Replay is unaffected.** Each merged op still journals under its original
  name, so every saved session and shipped recipe replays byte-for-byte.

### Workspace + browser playground
- **`tono-core` crate** — the pure, headless engine (graph DSL, DSP,
  deterministic renderer, analysis, critique, graph transforms) extracted into
  its own crate with **no I/O, no MCP, no transport**. The `tono` binary is
  now a thin shell (MCP server, encoders, persistence, daemon) that re-exports
  it, so every existing path is unchanged. One core, three targets: native MCP,
  WASM, and a future in-engine runtime.
- **WASM build + manual node patcher** — `tono-wasm` compiles the core to
  WebAssembly; `make wasm` emits it into `docs/playground/`, a zero-install
  browser studio where a human **builds a sound effect by hand, modular-synth
  style**: drop nodes from a palette (oscillators, envelopes, filters,
  mix/mul…), drag them anywhere, **wire output ports to input ports manually**,
  and tweak each node's parameters inline (sliders / dropdowns / modulator
  pickers) — everything flowing into an `OUT ▶` terminal. Multi-track sounds
  work too: a `mixer` node sums `layer` nodes (each with pan / gain / start
  offset / mute), and the serial processors between the mixer and `OUT` become
  the master chain — i.e. a `tracks` document. The patch serializes to a
  `SoundDoc` (serial effect runs auto-fold into a `chain`) and renders live
  to audio plus the same spectrogram / waveform / analysis the author sees,
  **byte-identically to the native engine**; a two-way JSON drawer exposes the
  exact document the author edits. The SoundFont sampler voice is the only one
  unavailable in the browser.
- **In-memory analysis** — `analysis::stats` (numbers, no filesystem) and
  `spectrogram_png` / `waveform_png` (PNG bytes) split out of the disk-writing
  `analyze`, so a render can hand back feedback without a disk round-trip.

### Engine revisions
- **New `engine` document field** — a DSP-kernel revision number, independent
  of the schema `version`. Omitted ⇒ engine 0 (the original kernels): every
  existing document and session replays **byte-for-byte**. New documents are
  stamped with the current `ENGINE_VERSION`; `refine_sound` preserves a sound's
  existing engine. This is what lets a fidelity upgrade ship without altering
  older renders.

### Anti-aliased distortion (engine 1)
- **`drive` now uses antiderivative anti-aliasing (ADAA)** on engine-1
  documents — the `hard` and `fold` shapers no longer spray inharmonic
  foldback across the spectrum. First-order ADAA with a one-pole DC blocker;
  per-node `"aa": false` opts back into the raw aliasing curve. Legacy
  (engine-0) documents are unaffected and stay bit-exact.

### Physical impacts (new nodes)
- **`modal`** — a resonator bank: N parallel damped sinusoidal partials
  (`modes: [{freq, decay, gain}]`) excited by the incoming chain signal. Bells,
  glass, metal, wood, ceramic, coins, and the resonant body of UI/impact
  sounds — none of which the oscillators voice cleanly. Each mode is a
  normalised two-pole resonator (impulse-response peak ∝ `gain`, decay exact),
  so the bank is cheap, stable, and fully deterministic. Modes are individually
  addressable (`…modes[i].freq`).
- **`impact`** — a strike exciter: a single unit-area force pulse whose
  `hardness` shapes its brightness (which modes light up) and `velocity` its
  energy. The exciter half of the `chain[ impact, modal ]` struck-body pair.
- New example **`docs/examples/struck-bell.json`** (a struck bell + a coin
  ding), replayed in CI like every other recipe.

### Texture & environmental synthesis (new primitives)
- **`dust`** — a sparse stochastic source: a Poisson click train (`density`
  events/sec, each decaying over `decay` seconds; 0 = bare impulses), smoothed
  so overlapping grains sum. The generator behind fire crackle, rain, geiger
  ticks, sparks, and debris. Draws from the layer's deterministic stream like
  `noise`.
- **`rand`** — a random-walk modulator: smooth, NON-periodic drift between
  `from` and `to` at `rate` targets/sec. The organic motion the periodic
  modulators lack — wind gusting, fire flicker, drifting detune. Seeded only
  from its own fields (with an optional `seed` to decorrelate), so it is
  deterministic and stable under sibling edits.
- New example **`docs/examples/fire-and-wind.json`** — a looped campfire
  (`dust` crackle + `rand`-driven roar) and gusting wind (two decorrelated
  `rand` walks), replayed in CI.

### Review loop
- **New `review_sound { id, archetype? }` tool** — a deterministic critique
  engine. Grades a sound against its archetype targets (laser / coin / jump /
  impact / ui / ambience / bgm) and the universal ship checklist (clipping,
  true-peak, head/tail silence, onset count, loop seam), returning PASS / WARN /
  FAIL findings each with the measured value, the target, and the concrete fix.
  Reproducible — a given sound always reviews the same way. Read-only.
- **New `sound-review-loop` skill** — drives Review → Polish → Review:
  `review_sound` → apply the top finding's fix with one `set_param` → re-review
  → `undo_sound` on a regression → repeat until PASS. The user can supply review
  in their own words at any iteration and it takes over.

### Craft tooling
- **New `scaffold_layered_sfx { base_freq?, seed?, name? }` tool** — generates a
  blank, band-disciplined 4-layer SFX document (sub / body / top / transient),
  each a mixer layer with a stable id, a band-splitting filter, a one-shot
  envelope, and a starting gain. Sources are neutral placeholders the author
  swaps out: a correct multi-layer *structure*, not a preset. Stamped schema v2
  (independent per-layer noise) + the current engine; journaled and replayable.
  New CI-replayed example `docs/examples/layered-sfx-scaffold.json`.

### Analyzer (sharper ears)
- **Log-frequency spectrogram** — the feedback image's frequency axis is now
  logarithmic, so bass/low-mids and modal partials are legible instead of
  crushed into the bottom strip. Image-only; audio bytes are unchanged.
- **New metrics on every render**: `spectral_flatness` (tonal vs. noisy),
  `inharmonicity` (off-harmonic-grid energy — also an aliasing/foldback
  indicator), and `attack_slope_db_per_ms` (transient sharpness). All are
  reporting-only — they never feed the render's loudness/limiting stage, so
  determinism is untouched.

## 1.1.0 — 2026-06-12

Compositional authoring: a sound is now a document you build up in named,
addressable layers, each rendered on its own deterministic stream. Backward
compatible — v1 documents omit the version field, keep their original render
semantics, and replay byte-for-byte.

### Layered authoring
- **Stable layer ids**: every track carries a unique, validated slug id, an
  `at` start offset (applied post-render, so RNG consumption never depends on
  placement), and persisted `mute`. Ids are backfilled deterministically at the
  build chokepoint, so replays mint the same ids.
- **Schema v2 per-layer RNG streams**: each track and the master bus gets its
  own deterministic noise stream keyed by layer id — adding, removing, or muting
  one layer never re-grains a sibling. v1 docs keep the threaded stream.
- New tools — **`add_layer`** (the compositional flow: the first call wraps a
  plain root as a level-compensated layer named after the sound; duplicates
  rejected with the layer listing), **`set_layer`** (mixer fields), **`layer_ops`**
  (remove/duplicate). `set_param` / `edit_sound` take a `layer` arg with
  node-relative paths; `describe_sound` emits per-layer tables with
  ready-to-paste layer-relative paths and a row for every seq note.
- **Per-layer contribution stats**: each render captures every layer's
  post-fader pre-master peak / RMS / energy share from the same pass and prints
  a compact per-layer balance line (muted layers flagged); the stats persist on
  `Analysis`. `morph_sounds` unifies layer identity positionally, so
  independently-minted ids no longer block morphs between same-shaped documents.

### Performance & history
- **Single-pass render**: mixer documents were fully rendered twice per
  build/export (stereo for the WAV, mid for analysis). `render_product()` now
  yields both from one pass; build/export/pack/rehydrate and `make_loop` reuse it.
- Undo history deepens **20 → 100** — compositional editing burns revisions fast
  and graphs are small JSON.

### Fixes
- Mutating tools now build **before** checkpointing, so a rejected graph leaves
  history, redo, and the journal untouched (a failed call used to push a no-op
  revision, wipe redo, and desync replay).
- Replay no longer stamps the current schema version onto version-less journaled
  steps; `rehydrate` backfills track ids and per-layer stats so pre-layering
  mixer docs survive a restart; `humanize` trims the master chain on Tracks roots
  instead of wrapping the root (which validation rejected on every multi-track
  sound). Closes 18 issues from an adversarial branch review.

### Skill & showcases
- Ship a **sound-designer** project skill: the listen-and-fix loop, how to read
  every analysis metric and both feedback images, per-archetype numeric targets,
  symptom-to-fix recipes, the layered workflow, and the ship checklist.
- Three loop-ready game-BGM showcases composed on the console with it —
  **evening-glade** (soft BGM), **iron-gauntlet** (boss battle), **sunny-steps**
  (idle platformer) — replace the phonk remix; both River showcases and the
  retro-coin / jump-8bit SFX got a polish pass. Eleven examples, all replayed in
  CI with playable renders.

## 1.0.0 — 2026-06-07

First release. A headless sound studio for programmatic clients, driven over MCP.

### Instruments & synthesis
- Polyphonic sequencer (`seq`) with a core instrument set: **piano** (detuned
  string pair, velocity brightness, pitch-dependent decay), **e-piano**
  (Rhodes tine), **organ** (tonewheel drawbars + percussion), **strings**
  (ensemble swell), **bass** (filtered + sub), **kit** (full drum kit on the
  General MIDI map), pitched **cowbell**, **pluck** (Karplus-Strong), tunable
  **fm** mallets/bells — plus raw band-limited square/saw/triangle, sine, FM,
  supersaw and three noise colours.
- **`sampler`**: real recorded instruments from any SoundFont (`sf2` path +
  GM program; bank 128 = drum map), rendered deterministically via rustysynth.
- Note-name pitches (`"C4"`, `"midi:60"`), per-parameter modulators
  (`slide`/`lfo`/`arp`/`env`), `swing` + `humanize` groove.

### Production
- **`tracks` mixing console**: per-track equal-power pan and fader onto a true
  stereo bus, master processor chain, decorrelated (Freeverb-spread) reverb
  tails; sampler tracks keep their native stereo.
- Effects: filters + EQ, drive, ringmod, chorus/flanger/phaser, compressor,
  **`duck` sidechain pumping**, bitcrush/downsample, delay, reverb.
- Output stage: LUFS-targeted soft-knee limiting to a true-peak ceiling;
  seamless loops (equal-power crossfade + WAV `smpl` chunk); WAV/FLAC/OGG.

### The authoring loop
- Every render returns analysis (peak/true-peak/RMS/crest, ≈LUFS, spectral
  centroid, transients) plus **spectrogram and waveform images**;
  `compare_sounds` reports deltas + similarity.
- Surgical editing by JSON path (`describe_sound` → `set_param` /
  `edit_sound`), 20-deep undo/redo, persistent slug-id library.
- Variations on authored sounds: `mutate_sound`, `generate_variants`,
  `humanize`, `morph_sounds`.
- Banks → `sounds.json` manifests + engine files (Godot/Unity/Bevy).

### Sessions
- Every mutating call journaled; `save_session` / `replay_session` (and the
  `tono replay` CLI) reproduce a project **byte-for-byte** in a fresh
  directory. Annotated recipe files double as tutorials; nine showcases —
  including the complete *River Flows in You*, its phonk remix, and an
  iconic-sounds pack — replay in CI, with playable renders committed.

### Ops
- stdio + streamable-HTTP transports; self-managing launchd/systemd daemon;
  one-line installer; tagged binary releases (macOS arm64, Linux x86_64,
  Windows); dual-licensed MIT/Apache-2.0.
