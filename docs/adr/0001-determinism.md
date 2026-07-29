# ADR 0001: Determinism and engine revisions

Status: accepted (long-standing; written down for v1.10.0)

## Context

tono's product guarantee is that rendering is a pure function of
`(graph, seed, sample_rate)` — the same document renders byte-identical
audio, forever, on every face (offline bounce, streaming renderer,
runtime engine, Python, CLI). Users pin sounds by committing documents;
a kernel change that silently shifts samples would break every saved
project at once.

## Decision

- Rendering stays a pure function of `(graph, seed, sample_rate)` for a
  given **engine revision**. Byte-changing kernel work lands only behind
  a new document `engine` revision; every supported historical revision
  keeps its exact behavior, and documents are pinned at creation.
- The golden corpus (`crates/tono-core/tests/golden.rs`) pins rendered
  hashes of representative documents and every deterministic node and
  voice. A change that shifts both the offline and streaming paths
  together fails CI.
- The real-time path stays byte-identical to the offline bounce. A
  construct that cannot stream is *reported* (`StreamGraph::blockers`),
  never silently approximated; the buffer-backed `Player` is the
  explicit fallback.
- Until the deterministic-transcendental engine revision lands (planned
  for v1.10.0-beta.1), byte-identity is **per platform**: platform libm
  last bits differ between macOS-arm64 and linux-x86_64, so the corpus
  carries per-platform pins. That revision replaces libm calls in the
  deterministic path and fixes convolution/FFT operation ordering, after
  which the corpus collapses to one cross-platform pin set. Existing
  documents keep their per-platform revisions.

## Consequences

- Every new deterministic node, voice, and routing mode must join the
  golden corpus in the same change that ships it.
- A new engine revision is the *only* sanctioned way to change the
  samples an existing document produces.
- Cross-platform pins are accepted as a temporary cost; the beta.1
  revision is scoped and gated, not rushed.
