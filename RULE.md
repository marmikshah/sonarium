# RULE — the one contributor surface

**Less is more. Explicit is always better than implicit.**

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
  READMEs, example headers) answers with direct cargo/maturin/pip commands and
  runnable examples. Contributor gates use those same direct commands.
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

The root is a pure virtual workspace manifest; every package lives under
`crates/`.

- **`crates/tono-core/`** — the platform-independent engine: the `SoundDoc` graph DSL, DSP,
  deterministic renderer, analysis/critique, graph transforms, the byte-identical
  **streaming** real-time renderer, the **runtime** (`Engine`/`Mixer`/`AudioSource`),
  the **instrument** / **drum-kit** / **adaptive-music** layers, and the **song**
  arrangement layer. No platform I/O; deterministic compute and transport.
- **`crates/tono-cli/`** — the `tono` crate (published to crates.io under that
  name): a thin CLI shell — the `tono render` command, audio-file encoders,
  the analysis image writer, and MIDI export. Depends on and re-exports
  `tono-core`.
- **`crates/tono-desktop/`** — the native desktop studio (Tauri window + `cpal`
  real-time audio + MIDI keyboard input). Excluded from `default-members`;
  build with `cargo build -p tono-desktop --release`. The Native workflow
  checks it explicitly. Heavy deps
  (webview/cpal/midir) never touch the default build.
- **`crates/tono-play/`** — the programmatic playground: a `cpal` speaker so a Rust
  program can build a sound/instrument and hear it in a couple of lines. Excluded
  from `default-members`; run with `cargo run -p tono-play --example <name>`.
- **`crates/tono-py/`** — the PyO3 Python bindings (render + live `Engine` stream).
  Excluded from `default-members`; build with `maturin develop -m
  crates/tono-py/Cargo.toml` and run its two test scripts directly.
  Build-from-source only — never published to PyPI (the name is taken).
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

- Lint: `cargo fmt --all -- --check`, then
  `cargo clippy --locked --all-targets -- -D warnings`.
- Test: `cargo test --locked`.
- Native crates: `cargo clippy --locked -p tono-desktop -p tono-play -p
  tono-py --all-targets -- -D warnings`, then
  `cargo test --locked -p tono-desktop -p tono-play`.
- Python: build/install the wheel, then run
  `python3 crates/tono-py/tests/smoke.py` and
  `python3 crates/tono-py/tests/test_typed.py`.
- Dependencies: `cargo deny check advisories licenses`.
- Hooks: `git config core.hooksPath .githooks`. Pre-commit runs the lint
  commands; pre-push refuses `master` and runs lint plus tests.

## Release checklist

Every release starts from clean protected `master`. Pushing the tag triggers
both release workflows: `release.yml` creates the GitHub Release and builds
its CLI binaries, `publish.yml` pushes the crates to crates.io. Wheels remain
manual-only for budget reasons.

1. Bump **both** version fields in the root `Cargo.toml` together:
   `workspace.package.version` and `workspace.dependencies.tono-core`
   (cargo strips `path` at publish time and pins the crates.io dep to the
   version field — a mismatch ships a CLI built against last release's core).
2. Retitle the CHANGELOG's `## Unreleased` to `## X.Y.Z — <date>` (the
   Release workflow extracts the notes by that exact header).
3. Run the lint/test commands above and confirm
   `cargo publish --dry-run -p tono-core` passes.
4. Tag and push directly: `git tag -a vX.Y.Z -m "vX.Y.Z"`, then
   `git push origin vX.Y.Z`. `release.yml` creates the GitHub Release and
   binary assets; `publish.yml` publishes `tono-core`, waits for crates.io to
   index it, then publishes `tono`. Both are idempotent — re-running a
   released tag skips whatever already exists.
5. `publish.yml` authenticates with the `CARGO_REGISTRY_TOKEN` repo secret (a
   crates.io API token with the publish-update scope). If the workflow is ever
   unavailable, the manual fallback is: `cargo publish -p tono-core`, wait for
   the index, then `cargo publish -p tono`.

Before the tag, the release-candidate gates: the direct lint/test commands on
the pinned toolchain and latest stable (`cargo +stable clippy --locked
--all-targets -- -D warnings`, then `cargo +stable test --locked`);
`cargo doc -p tono-core --no-deps` warning-free (the rustdoc gate — keep it at
zero); the API compatibility
review — `cargo public-api diff` against the last tag if the tool is
installed, otherwise a manual skim of `git diff <last-tag> --
crates/*/src/lib.rs` public items against docs/api-tiers.md (stable surface
must not break; experimental changes called out in the CHANGELOG);
`cargo deny check advisories licenses`; and the on-demand soak
(`cargo test -p tono-core --test soak -- --include-ignored`) clean.

## Conventions

- Clippy clean at `-D warnings`; `cargo fmt` before committing. No dead code.
- Small, focused commits; commit and push as work lands (one concern per commit).
- `tono-core` stays decoupled — no transport/file-IO leaks into it.
- New capabilities should be expressible across the faces (CLI + code + UI)
  over the same `SoundDoc`, not bolted onto one.
