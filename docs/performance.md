# Performance: the reference budgets (rc.1)

What "within the documented reference budgets" means for tono, and how the
numbers are produced:

- Benches live in `crates/tono-core/benches/render.rs` (criterion).
- The CI workflow can run them by explicit manual dispatch and publishes the
  results — **no per-commit gate**, by design: shared CI runners are too noisy
  for hard thresholds.
- Read reports against the budgets below: a sustained 2× regression on any
  of them is a release blocker; a few percent is runner weather.
- Reference numbers measured on Apple Silicon (macOS-aarch64, release); CI
  numbers differ in absolute terms and track in relative ones.

| Bench | What it measures | rc.1 reference |
|---|---|---|
| `render/blip_osc_env` | sine + ADSR micro-render (0.3 s) | ~0.31 ms |
| `render/tracks_mix_automation_master_reverb` | full mixer pass incl. lanes + master chain | ~3.5 ms |
| `render/seq_piano` | the additive piano voice (1 s) | ~8.8 ms |
| `render/fx_chain_reverb_delay_compress` | a wet FX chain | ~1.9 ms |
| `streaming/streamgraph_fill_512` | one 512-frame block through the streaming renderer | ~0.89 ms |
| `compile/song_to_program` | `Song::compile` of a representative multi-track song | ~0.21 ms |
| `scheduling/performance_fill_512` | one 512-frame block through `Performance::fill` | ~19 µs |
| `mixing/tracks_stems_8` | `render_stems` on an 8-track mix with a bus | ~72 ms |

## Check the real-time headroom

- A 512-frame block at 48 kHz lasts **10.7 ms** of wall time.
- `Performance::fill` renders it in **~19 µs** (~0.2% of block time); the
  full streaming render of the same block is **~0.89 ms** (~8%).
- The command-queue and voice budgets are static and documented (4096
  scheduled commands, estimates-driven preallocation), so the engine never
  draws on more than its headroom under legal load.
- The soak (`tests/soak.rs`, on-demand) proves it end to end with zero
  engine-originated underruns.
- Offline render cost is dominated by content, not the framework: a piano
  second is ~8.8 ms (~113× real time), a wet FX chain ~1.9 ms, an 8-track
  stem decomposition ~72 ms — all comfortably inside authoring-loop
  latency.

## The engine-5 premium, documented and accepted

- The deterministic kernels (`det`) are pure f64 polynomial evaluation, so
  transcendental-heavy micro-renders pay a measured premium over platform
  libm: the blip bench sits ~2× its pre-engine-5 baseline, the wet FX chain
  ~1.4×.
- This is the price of **cross-platform byte-identity**, accepted
  deliberately: it applies only to documents stamped `engine: 5` — older
  revisions keep their libm paths and their old numbers.
- The premium is bounded by the table above; no bench crosses its budget
  because of it. Engine ≤ 4 paths are byte- and speed-identical to before.

## Bound memory at compile time

Compile-time estimates bound the runtime:

- `estimates.frames` — the exact render length,
  `round(duration × sample_rate)`.
- `estimates.memory_bytes` — `frames × 8` (the stereo f32 output).
- `peak_voices` — per-track max overlap summed: a voice-pool size that
  never steals.

The estimates test (`tests/estimates.rs`) proves the bounds against real
renders.
