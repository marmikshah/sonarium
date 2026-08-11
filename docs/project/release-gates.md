# v1.10.0 release gates — the rc.1 walk

Every gate from issue #52, with its evidence. ✅ = met and enforced.
This file is the freeze checklist: rc.1 ships when every row is ✅.

## Functional

- ✅ **A complete multi-track song can be authored in Rust without manually
  writing JSON** — the `song` layer (`Song`/`Pattern`/`Phrase`/catalog
  voices), end-to-end in `crates/tono-core/examples/compose.rs`, plus the
  golden `fluent-song` case.
- ✅ **The equivalent song in Python without JSON** — the typed API
  (`tono.Song`/`Pattern`/`Track`/`Program`/`instruments`), end-to-end in
  `crates/tono-py/examples/night_drive.py` and `tests/test_typed.py`.
- ✅ **Patterns, arrangement, tempo/meter, automation, routing, effects,
  deterministic variation** — pattern algebra (`song::pattern`),
  tempo/meter maps + pickup (T1003–T1005), step/exp/linear lanes (T15xx
  warnings), buses/inserts/sends/returns, deterministic seeds and
  `probability`/`humanize` per (pattern, seed) — all in `song/compile.rs`
  tests and the `fuzz_pattern_properties` suite.
- ✅ **Compile once → offline render AND real-time playback** —
  `Song::compile` → immutable `Program`; offline via
  `render_stereo`/`render_range_*`/`render_stems`, real-time via
  `Performance` (bounce-vs-live byte-identity pinned in
  `runtime::performance::tests`).
- ✅ **Stereo mix, selected range, and stems exported** — `render_stereo`,
  `render_range_frames`/`render_range_bars` (slices of the full render,
  tails intact), `render_stems` (sum rule pinned in `render::tests`), CLI
  `tono render --stems`, Python `render_stems()`/`render_range()`.
- ✅ **Runtime commands scheduled without caller wake-up timing** — the
  bounded submission-ordered queue executes at exact frames; musical
  positions resolve at schedule time (ADR 0005); queue-full rejection is
  defined and counted (`a_full_queue_rejects_and_counts`).

## Determinism and compatibility

- ✅ **Supported deterministic targets produce the same documented output
  hashes** — engine-5 corpus cases pin ONE value asserted on both
  macOS-arm64 and linux-x86_64 in CI (`tests/golden.rs`, v5-* cases).
- ✅ **Program hashes are canonical across Rust and Python** —
  `tests/equivalence.rs` and `crates/tono-py/tests/test_typed.py` pin the
  same value, run in CI on both sides.
- ✅ **Old pinned documents render with historical engine behavior** —
  the engine-revision gates (`effective_engine`) and the full engine ≤ 4
  corpus, unchanged and green.
- ✅ **Historical documents and bundles pass compatibility fixtures** —
  `tests/compat.rs` (alpha.1 song, legacy unpinned song, PROGRAM_VERSION 1
  bundle + tamper case, shipped patch).
- ✅ **Offline and real-time outputs agree where promised** — block-size
  sweeps (1/7/64/333/whole) for every streamable construct including the
  streaming mixer, plus `plays_byte_identical_to_the_bounce`.

## Real-time safety

- ✅ **No allocation, locking, filesystem, blocking, or formatted logging
  on the audio callback** — measured: `tests/rt_alloc.rs` (counting
  allocator) proves zero allocations across `Performance::fill`,
  `StreamSource::fill`, and `Renderer::fill` after one documented scratch
  growth; the runtime has no locks or I/O in fill paths (SPSC rings only).
- ✅ **Bounded queues and pools with documented overflow behavior** —
  command queue cap 4096 (reject + count), voice caps by priority
  (`cap_bounds_the_sounding_voices`), documented in `runtime/performance.rs`
  and `runtime/engine.rs`.
- ✅ **Runtime soak without engine-originated underruns** — `tests/soak.rs`
  (on-demand): 5-minute looped session with scheduled commands, zero
  underruns, exact command counts, byte-identical passes; threaded
  pump/drain identical to the single-threaded reference.
- ✅ **Click-free hot changes and transitions** — crossfaded swaps
  (`swap_crossfades_deterministically` bounds the discontinuity), ramped
  gain (per-frame, block-size invariant), section transitions quantized.
- ✅ **Budgets observable** — `PerformanceMetrics` (frames, commands,
  dropped, queue depth, swaps, stingers) off-callback; compile-time
  `ResourceEstimates` (frames/events/peak voices/memory) pinned against
  real renders in `tests/estimates.rs`; budgets documented in
  docs/performance.md.

## API and packaging

- ✅ **Stable Rust and Python surfaces documented with examples** —
  docs/api-tiers.md, rustdoc at zero warnings, `examples/compose.rs`,
  `examples/night_drive.py`, quickstart's five-minute song, cookbook.
- ✅ **Python typed with py.typed** — `tono/__init__.pyi` +
  `tono/instruments.pyi` + `py.typed` ship in the wheel (verified in
  `python-smoke` and the clean-venv wheel check in the Wheels workflow).
- ✅ **Structured errors** — diagnostics with stable T-codes and
  remediation (`diag.rs`), `tono.CompileError.diagnostics`,
  `ProgramError` (T3001/T3002), `PerformanceError` (queue full, unknown
  position), stale/foreign handles inert and tested
  (`foreign_patch_and_param_handles_are_inert_never_panic`).
- ✅ **Wheels and CLI binaries install and execute on clean environments** —
  the Wheels workflow smokes each wheel on a clean venv (numpy only) on
  all three platforms; CLI binaries build per-tag with sha256 sidecars.
- ⛔ ~~WASM and C ABI smoke tests load and execute a compiled Program~~ —
  **descoped by owner decision before release**: the `tono-capi` and
  `tono-wasm` faces were built and smoke-tested during the beta slices
  (C smoke + headless Node smoke, byte-identical), then dropped as
  overkill — native hosts embed `tono-core` directly. The gate returns
  if the faces do.
- ✅ **Release artifacts include hashes and capability metadata** — sha256
  sidecars for binaries and wheels on the GitHub Release; Programs carry
  target + capabilities (`tono compile --inspect`).

## Engineering

- ✅ **Required CI green and risk-routed** — one draft-gated Ubuntu PR job
  shares its checkout, toolchain, cache, and build artifacts across Rust,
  affected installed-wheel Python tests, and dependency policy. Native checks keep a
  path-filtered macOS job; benchmarks are explicit manual dispatches.
- ✅ **Golden, property, compatibility, Python-equivalence, and
  runtime-safety suites pass** — golden corpus, fuzz_validation +
  fuzz_bundle + fuzz_command_stream + fuzz_pattern_properties + fuzz_midi
  + fuzz_time, compat fixtures, the cross-language equivalence pins,
  rt_alloc + soak + performance determinism tests.
- ✅ **Security/license checks have no unresolved blockers** — CI runs
  cargo-deny for pull requests and on a weekly schedule against the committed
  lockfile.
- ✅ **Performance within documented budgets** — docs/performance.md
  (rc.1 references for all eight benches; the engine-5 premium documented
  and accepted).
- ✅ **Changelog, migration guide, architecture docs, release notes
  complete** — CHANGELOG.md through rc.1, docs/migration.md,
  docs/performance.md, site/architecture.html (v1.10 update), and this
  walk.

## The freeze

With every gate ✅, rc.1 freezes: the public Rust and Python API surface
(stable per docs/api-tiers.md), the document schema (SCHEMA_VERSION 2),
the Program bundle format (PROGRAM_VERSION 2), and the DSP engine
(ENGINE_VERSION 5). From here to v1.10.0: release-blocking fixes only.
