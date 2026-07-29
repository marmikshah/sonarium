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
- Until the deterministic-transcendental engine revision landed, byte-identity
  was **per platform**: platform libm last bits differ between macOS-arm64
  and linux-x86_64, so older revisions keep per-platform pins. **Engine
  revision 5** closes that: the `det` kernels replace libm in the render
  path, and convolution uses a fixed-order radix-2 FFT with a documented
  power-of-two sizing rule. Engine ≥ 5 documents render identically on
  every supported target; the corpus pins one value per engine-5 case and
  asserts both platforms reproduce it. Documents pinned at older revisions
  keep their historical per-platform renders.

## Consequences

- Every new deterministic node, voice, and routing mode must join the
  golden corpus in the same change that ships it.
- A new engine revision is the *only* sanctioned way to change the
  samples an existing document produces.
- Cross-platform pins are accepted as a temporary cost; the beta.1
  revision is scoped and gated, not rushed.
