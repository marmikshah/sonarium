# Byte-identity, engine revisions, and streaming

The guarantees behind "audio as a pure function": what makes a render
reproducible, how engine revisions let fidelity improve without ever
changing an older sound, and which documents can stream live.

## Determinism

- Rendering is a pure function of `(graph, seed, sample_rate)` — a document
  renders **byte-identical** every time. With `engine: 5` (the default for
  new documents and songs) that identity holds **across platforms**;
  engine ≤ 4 documents keep their historical per-platform renders
  bit-for-byte (platform libm's last bits differ between macOS-arm64 and
  linux-x86_64, though integer-RNG, PolyBLEP, and rational-filter content
  is identical everywhere).
- The document's top-level `seed` drives every noise source, `dust` train,
  and Karplus-Strong pluck burst, so takes are reproducible; change `seed`
  for a different-but-equivalent roll.
- Because the document *is* the artifact, version your `.json` files and
  you can always reproduce the exact WAV — no separate session log needed.

## Pick an engine revision

A document carries two independent version numbers: `version` is the
**schema** version (document structure); `engine` is the **DSP-kernel**
revision (which audio kernels render it). They are split so a fidelity
upgrade never changes the bytes of an older sound.

| `engine` | what changed |
|----------|--------------|
| omitted (0) | the original kernels — byte-for-byte forever |
| 1 | anti-aliased `drive` (ADAA) |
| 2 | per-node structurally-seeded RNG for `noise`/`dust` (decorrelated siblings; byte-identical streaming randomness) |
| 3 | the inharmonic additive `piano` voice (stretched partials, per-partial decay, hammer spectrum, detuned unison pair) |
| 4 | corrected mixer output stage (joint stereo loudness normalization, gated BS.1770, oversampled true-peak) and per-note humanize jitter |
| 5 | deterministic transcendental kernels (`det`: fdlibm-grade sin/cos/exp/ln/powf/tanh in pure f64) replace platform libm everywhere in the render path; `convolve` runs a fixed-order radix-2 FFT with deterministic twiddles ⇒ **renders byte-identically on every platform** |

- To modernise an existing sound, set `"engine": 5` — its output will
  change; that's the point.
- To keep a legacy sound bit-exact, leave `engine` off.
- New documents and songs stamp 5 by default.

## What streams live

The streaming renderer pulls a document block-by-block, byte-identical to
the offline render. `StreamGraph::blockers` reports what a document trips,
each blocker naming the fix:

| streams natively | falls back to the buffer-backed `Player` (byte-identical, whole-buffer) |
|------------------|---------------------------------------------------------------|
| every node with constant filter/EQ cutoffs and gain amounts | a `normalize` output stage (whole-buffer op) |
| all modulators on source params (closed forms of the sample index; `rand` carries its walk) | a filter/EQ/gain carrying a modulated cutoff or amount |
| `tremolo` (a closed form of the sample index) | `loop` playback; a `stereo` (Haas/Wide) treatment (write-time ops) |
| `noise`/`dust`/`seq` under engine ≥ 2 (structurally-seeded RNG) | RNG nodes under engine < 2; the `sampler` seq |
| a schema-v2 `tracks` root — sidechains, buses, automation | a schema-v1 `tracks` root |

`convolve` / `granular` are offline-only whole-buffer effects — no
streaming form exists; bounce them offline and keep the streamed graph
causal.
