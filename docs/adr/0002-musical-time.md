# ADR 0002: Exact musical time

Status: accepted for v1.10.0-alpha.1

## Context

Musical positions (beats, bars, tuplet divisions) are exact fractions,
but floating-point seconds accumulate error: a 1/3-beat triplet at
127 BPM is not representable, and repeated pattern transforms
(stretch, rotate, concatenate) drift. On the other side of the engine,
audio is integer frames. A platform that composes music in code needs
one exact representation and one deterministic rule for crossing to
frames.

## Decision

- **Beat positions and durations are exact rationals** (`num/den`,
  reduced, positive denominators) in the composition model. Tuplets and
  repeated transformations never accumulate floating-point error.
- **Beat → frame conversion happens once, at the scheduling boundary**,
  and is defined exactly: seconds are computed in `f64`
  (`beats × 60 / bpm`), and frames are `round(seconds × sample_rate)`
  with halves rounded away from zero. All math before that boundary
  stays rational.
- The seq grid (`steps_per_beat`, integer steps) stays as the compiled
  representation; the exact types compile down onto it. 1.10.0-alpha.1
  ships the exact `Beat`/`Duration` types and typed units (frames,
  samples, hertz, decibels, tempo); a song still has one constant tempo.
- Tempo and time-signature maps (alpha.2) extend the rule without
  changing it: a map is a list of segments at exact beat positions; each
  segment converts its own local beats to frames and segments
  concatenate at exact frame boundaries — rounding happens per segment
  and never compounds. Notes or automation crossing a boundary are
  split at the boundary.

## Consequences

- Every placement, transition, and automation point lands on the same
  frame on every platform and in every process — the runtime never has
  to re-derive timing from Python, a game loop, or an OS timer.
- Rounding is specified, not emergent: tests pin the half-away-from-zero
  rule (e.g. the triplet grid at 120 BPM / 48 kHz).
- The compiled Program carries frames plus the musical grid, so both
  offline render and runtime transport agree without recomputation.
