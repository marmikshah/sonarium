# ADR 0004: Python bindings

Status: accepted for v1.10.0-alpha.1

## Context

The Python package began as a JSON-string bridge (`tono.render(doc_json)`)
plus a live `Engine`. That made Python a demonstration surface: callers
hand-wrote JSON, validation feedback arrived as plain strings, and the
typing toolchain saw nothing. v1.10.0 makes Python a primary authoring
and runtime surface.

## Decision

- **Typed, native-held objects — no JSON strings in the normal path.**
  `tono.Song`, `tono.Pattern`, `tono.Program` (and later runtime
  handles) wrap the Rust model directly. Building, compiling, and
  rendering a song never crosses a JSON boundary.
- **Rust owns semantics.** Validation, compilation, DSP, and scheduling
  live in `tono-core` alone; Python never reimplements them. Equivalent
  songs compile to the same Program hash from either language (a test
  fixture pins this).
- **The GIL is released for bounded native work** (compile, render,
  load). All Python interaction stays off the audio callback.
- **Structured errors.** Rust diagnostics surface as typed Python
  exceptions carrying code, severity, path, and remediation; invalid
  commands are never silently ignored.
- **The package is typed**: `py.typed` plus hand-written stubs covering
  the public surface.
- The legacy JSON-string API keeps working through v1.10 — per
  docs/api-tiers.md it is *deprecated* (documented successor: the typed
  API), not removed.
- **Import name stays `tono`.** The PyPI *distribution* identity (the
  name on PyPI is taken) is decided at 1.10.0-beta.1, before any
  publish; it is recorded here as an open decision so no one treats it
  as settled.

## Consequences

- Normal user code contains no JSON and no `time.sleep`-based musical
  scheduling (the runtime API, alpha.3, schedules through the engine).
- NumPy buffers returned by renders are documented as owned copies with
  a defined shape, channel order, and dtype.
- Wheels stay ABI3 (one wheel per platform, CPython 3.9+).
