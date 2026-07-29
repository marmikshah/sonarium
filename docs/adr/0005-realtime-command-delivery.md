# ADR 0005: Real-time command delivery

Status: accepted direction for v1.10.0-alpha.3 (recorded early — it
constrains what alpha.1's Program must preserve)

## Context

Live control today is immediate-mode: `set_gain`/`set_param` take effect
on the next block, and musical timing (playing a section *on the bar*)
is the caller's problem — a game loop or Python thread waking up and
hoping. That puts musical accuracy at the mercy of scheduler jitter,
and any lock or allocation on the audio callback risks a dropout.

## Decision

- **Musical scheduling is sample-accurate and owned by Rust.** Commands
  carry frame timestamps (or musical positions the Program's metadata
  converts once, per ADR 0002) and execute when the transport reaches
  them. No Python, game loop, or OS timer ever needs to wake on a
  musical boundary.
- **Delivery is one bounded SPSC queue** (the existing wait-free split
  direction: control thread produces, audio callback drains). The
  callback performs no allocation, locking, filesystem access, blocking
  calls, or formatted logging.
- **Preallocation from compile-time estimates.** Voices, events,
  automation lanes, and scratch DSP storage are sized from the Program's
  resource estimates (ADR 0003) or explicit budgets.
- **Exhaustion is defined, never silent.** A full command queue rejects
  (or drops oldest by policy) and counts it; a full voice pool steals or
  denies by priority and counts it. Counts surface in runtime metrics,
  readable off the callback.
- **Deterministic tie-break.** Commands with identical frame timestamps
  execute in submission order (a monotonically increasing sequence
  number), so a recorded command stream replays exactly.
- **Swap safety.** Program swaps happen at immediate/block/beat/bar/
  section boundaries with crossfades; a replacement that fails
  validation never displaces the last valid Program.

## Consequences

- The alpha.1 Program preserves the musical metadata (tempo, bars,
  sections later) that frame-stamped scheduling needs — this is why ADR
  0003 puts metadata in the artifact from the start.
- Hosts are told how far ahead to schedule (a documented lookahead
  budget) instead of guessing.
- Soak tests and allocation/lock assertions on the callback become
  release gates, not aspirations.
