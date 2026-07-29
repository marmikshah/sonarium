# tono — repo guide

A deterministic synthesis-graph engine that renders audio and feeds back
analysis (a spectrogram + waveform + numeric stats), so a sound can be authored
by inspection — from Rust, the `tono render` CLI, or a live keyboard. The same
engine powers a real-time streaming renderer, a playable-instrument layer, a
song/arrangement layer, adaptive game music, a native desktop studio, and a
programmatic playground.

## Product voice & versioning

- tono stands on its own: a **developer-friendly audio engine with live
  playback at runtime**. Docs, changelogs, and PRs describe it in its own
  vocabulary (SoundDoc, Patch, Engine, layers/sections) — never reference
  other products by name or by analogy.
- **Docs are split by audience.** User-facing text (README, docs/, crate
  READMEs, example headers) answers with cargo/maturin/pip commands and
  runnable examples — the `Makefile` is the *contributor* interface and its
  targets appear only here (CLAUDE.md) and in the architecture guide.
- **There is never a 2.0.** Breaking changes land in ordinary 1.x minors, and
  deprecated surface is removed directly in the next minor — no long-lived
  deprecation shims. The byte-identity promise below is a product guarantee,
  independent of version numbers. Public API stability is **tiered**
  (docs/api-tiers.md): the *stable* surface follows SemVer and does not
  break on the 1.x line — the removal policy here applies to deprecated and
  *experimental* surface. Architecture decisions live in `docs/adr/`; a new
  or changed decision means an ADR update in the same commit.
- The Bevy face lives in the separate `bevy_tono` repo — update it there,
  don't grow a new adapter crate here.

## Workspace layout (one core, several faces)

The root is the `tono` crate (the CLI); the sub-crates live under `crates/`.

- **`crates/tono-core/`** — the pure engine: the `SoundDoc` graph DSL, DSP,
  deterministic renderer, analysis/critique, graph transforms, the byte-identical
  **streaming** real-time renderer, the **runtime** (`Engine`/`Mixer`/`AudioSource`),
  the **instrument** / **drum-kit** / **adaptive-music** layers, and the **song**
  arrangement layer. No I/O, no transport; pure compute.
- **`tono` (root crate, `src/`)** — a thin CLI shell: the `tono render` command,
  audio-file encoders, the analysis image writer, and MIDI export. Depends on and
  re-exports `tono-core`.
- **`crates/tono-desktop/`** — the native desktop studio (Tauri window + `cpal`
  real-time audio + MIDI keyboard input). Excluded from `default-members` and CI;
  built via `make desktop`. Heavy deps (webview/cpal/midir) never touch the default build.
- **`crates/tono-play/`** — the programmatic playground: a `cpal` speaker so a Rust
  program can build a sound/instrument and hear it in a couple of lines. Excluded
  from `default-members`/CI; run via `make play EXAMPLE=<name>`.
- **`crates/tono-py/`** — the PyO3 Python bindings (render + live `Engine` stream).
  Excluded from `default-members`/CI; built via `make python` / `make wheel`,
  smoke-tested by `make python-test`. Build-from-source only — never published to
  PyPI (the name is taken).
- **`crates/tono-capi/`** — the stable C ABI for native hosts (issue #52):
  opaque handles over a compiled `Program` and its `Performance` runtime,
  cdylib + staticlib artifacts, a hand-written `capi.h`, and a C smoke test.
  Excluded from `default-members`/CI; built and smoke-tested via `make capi`.
- **`crates/tono-wasm/`** — the WebAssembly face for the browser (issue #52):
  the lean engine (`tono-core` with default features off — no analysis PNGs,
  no sampler) as a wasm32 cdylib behind a `wasm-bindgen` API (`renderDoc` /
  `compileSong` / `Program` / `PerformanceHandle`), an AudioWorklet runtime
  (`js/`), and a player example. Excluded from `default-members`; built via
  `make wasm`, build-gated in CI (the `build-wasm` job in ci.yml).

## The invariant that matters

Rendering is a pure function of `(graph, seed, sample_rate)` → **byte-identical**
audio. A golden corpus (`crates/tono-core/tests/golden.rs`) pins the exact
rendered hashes of representative documents — and the docs/examples recipes —
in CI, so a kernel change that shifts the offline and streaming paths together
still fails loudly. Do not change synthesis math in a way that breaks existing
renders — gate byte-changing kernel upgrades behind the document `engine`
revision. The real-time audition path must stay byte-identical to an offline
bounce.

Known limitation, retired: byte-identity **was** per-platform for documents
pinned at engine revisions ≤ 4 (platform libm's last bits differ between
macOS-arm64 and linux-x86_64), so the corpus pins those revisions
per-platform. **Engine revision 5** renders through the deterministic `det`
kernels (`crates/tono-core/src/det.rs`) and a fixed-order FFT — documents
stamped `engine: 5` are byte-identical on every supported target, and the
corpus asserts one shared pin set for them in CI. New documents and songs
stamp 5 by default; older documents keep their historical per-platform
renders forever.

## Build / test

- `make verify` — exactly what CI runs: `fmt --check` + clippy (`-D warnings`) +
  tests. The pre-push hook runs this. `make check` is the mutating version.
- `make pre-commit-checks` — the lint gate (fmt + clippy) alone.
- `make verify-native` — the gate for the off-CI crates: touching tono-desktop /
  tono-play / tono-py? This is your gate — plain `make verify` does not compile
  them (they are non-default workspace members). CI runs it via the Native
  workflow when those crates change.
- `make desktop` / `make play` — the native faces (heavy deps, off the default build).
- `make capi` / `make wasm` — the C ABI and the browser face (off the default
  build; wasm32 builds are also gated by the `build-wasm` job in CI).
- `make hooks` — install the git hooks (`.githooks/pre-commit`, `pre-push`).

## Release checklist

Every release, in order (the `release` target enforces clean master + tags
from `Cargo.toml`; CI publishes to crates.io and builds wheels on the tag):

1. Bump **both** version fields in the root `Cargo.toml` together:
   `workspace.package.version` and `workspace.dependencies.tono-core`
   (cargo strips `path` at publish time and pins the crates.io dep to the
   version field — a mismatch ships a CLI built against last release's core).
2. Retitle the CHANGELOG's `## Unreleased` to `## X.Y.Z — <date>` (the
   Release workflow extracts the notes by that exact header).
3. Confirm `cargo publish --dry-run -p tono-core` passes. The `-p tono`
   dry-run only resolves once `tono-core` X.Y.Z is on crates.io, so it runs
   after step 4, not before.
4. `make release` (tags `vX.Y.Z`, pushes; CI publishes `tono-core` then
   `tono`, creates the GitHub Release, and builds the tag-only wheels and
   CLI binaries).

Before the tag, the release-candidate gates: `make verify` on the pinned
toolchain plus `stable-compat` green; `cargo doc -p tono-core --no-deps`
warning-free (the rustdoc gate — keep it at zero); the API compatibility
review — `cargo public-api diff` against the last tag if the tool is
installed, otherwise a manual skim of `git diff <last-tag> -- */src/lib.rs
crates/*/src/lib.rs` public items against docs/api-tiers.md (stable surface
must not break; experimental changes called out in the CHANGELOG); the
Audit workflow green (licenses + advisories); and the on-demand soak
(`cargo test -p tono-core --test soak -- --include-ignored`) clean.

## Conventions

- Clippy clean at `-D warnings`; `cargo fmt` before committing. No dead code.
- Small, focused commits; commit and push as work lands (one concern per commit).
- `tono-core` stays decoupled — no transport/file-IO leaks into it.
- New capabilities should be expressible across the faces (CLI + code + UI)
  over the same `SoundDoc`, not bolted onto one.
