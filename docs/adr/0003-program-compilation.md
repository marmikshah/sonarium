# ADR 0003: Song compilation and the Program artifact

Status: accepted for v1.10.0-alpha.1

## Context

`Song::to_doc()` compiles a song to a `SoundDoc`, but the result is a
loose document: no diagnostics beyond the first error, no metadata for
transport or sections, no resource estimates, no identity. v1.10.0 makes
the compiler the product boundary — the compiled artifact is what
applications render, ship, and run.

## Decision

- **`Song::compile()` is the single validation + lowering entry point.**
  It returns an immutable **`Program`**, not a loosely transformed
  document: the resolved `SoundDoc`, musical metadata (tempo, bars,
  duration in beats/seconds/frames, track roster), resource estimates,
  streaming blockers/capabilities, and a canonical content hash.
- **Three versions evolve independently** and are all recorded in the
  Program: `SCHEMA_VERSION` (document semantics), `ENGINE_VERSION` (DSP
  kernels, per ADR 0001), and `PROGRAM_VERSION` (the bundle format,
  starting at 1). A loader rejects a Program newer than itself.
- **The canonical content hash is FNV-1a over canonical JSON**: UTF-8,
  object keys sorted (the serde_json default map), no insignificant
  whitespace, floats in shortest-round-trip form. It covers the
  *semantic program* (the resolved document plus its pins), not the
  authoring structure and not serialization formatting — so two
  equivalent songs, whether authored in Rust or Python, compile to the
  same hash. The same Rust code computes it for both.
- **Validation collects all diagnostics in one pass.** Diagnostics carry
  a stable code, severity, the object path, a message, and remediation
  text; streaming blockers surface as warnings, not surprises.
- **A Program loads without recompiling authoring structures.** It
  serializes as versioned JSON with the resolved document embedded —
  `Program::load` validates versions and re-derives nothing musical.
- Compilation reuses the existing lowering (`Song::to_doc` → `SoundDoc`)
  so the render path is untouched and the golden corpus keeps its
  meaning. New correctness lives in validation and the artifact, not in
  new render math.

## Consequences

- Rust/Python equivalence is testable: the same song compiles to the
  same hash from either language (a fixture pins this).
- Programs are the runtime's input for 1.10.0-alpha.3 (transport,
  sections, transitions) — metadata needed there is preserved at compile
  time, not reconstructed.
- Estimates (frames, events, voices, memory) are bounded and documented;
  the runtime preallocates from them (ADR 0005).
