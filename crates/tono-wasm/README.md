# tono-wasm — the tono engine for the browser

The WebAssembly face of [tono](https://github.com/marmikshah/tono): the pure,
deterministic engine compiled to `wasm32-unknown-unknown` behind a small
`wasm-bindgen` API, plus an AudioWorklet runtime for live playback. Render a
SoundDoc, compile a Song into an immutable `Program`, and play it live through
a `PerformanceHandle` — the same artifact the CLI renders and the Python face
plays, through the exact same engine, byte-identical to the offline bounce.

This is a **player** surface, not an editor.

## The feature set (lean by construction)

`tono-core` with `default-features = false`. What remains — serde, serde_json,
schemars, tracing, rustfft — is pure Rust and compiles cleanly to wasm32:

- **`analysis` off** — the `image` crate (spectrogram/waveform PNGs) is heavy
  and is an authoring-feedback surface, not a player surface.
- **`sampler` off** — the SoundFont sampler (`rustysynth`) loads `.sf2` files
  by path, which doesn't exist in a browser sandbox. Sampler tracks render
  silence, exactly like a native lean build; the streaming-blocker warning
  (`T1506`) says so at compile time.
- **`rustfft` stays** — it's a full (non-optional) dependency of the engine's
  `convolve` node and is pure Rust, so every document compiles and renders.

## Build

```sh
make wasm        # from the repo root — the documented path
```

The target builds the crate in release for `wasm32-unknown-unknown` (adding
the rustup target if missing), then — when `wasm-bindgen-cli` is installed —
generates the JS bindings into `crates/tono-wasm/pkg/`. Without the CLI it
still builds the `.wasm` and prints the exact two commands to finish:

```sh
cargo install wasm-bindgen-cli --version <version from Cargo.lock>
wasm-bindgen --target web --out-dir crates/tono-wasm/pkg \
    target/wasm32-unknown-unknown/release/tono_wasm.wasm
```

(`wasm-pack build --target web` works too; point its `--out-dir` at
`crates/tono-wasm/pkg`. The hand-rolled path above is what CI and the Makefile
use — one less tool.)

The bindings version is coupled to the `wasm-bindgen` crate version — install
the CLI at the version `Cargo.lock` resolved (the Makefile prints it).

## The API

All names are camelCase in JS. Audio crosses the boundary as
**stereo-interleaved** `Float32Array`s (`[L0, R0, L1, R1, …]`) — one copy per
call, the layout `fill` emits and WAV interleave wants. Counts that can
exceed 2³² (`frames()`, the schedule sequence id) arrive as `BigInt`s.
Failures throw a JS `Error`; where structured diagnostics exist (compile) the
`Error.message` **is** the JSON array — `JSON.parse(err.message)`.

| Export | Signature | Notes |
| --- | --- | --- |
| `renderDoc` | `(docJson: string) → Float32Array` | Mono bounce of a SoundDoc; throws on parse/validation errors. |
| `compileSong` | `(songJson: string, sampleRate?: number) → Program` | Throws the compile diagnostics as a JSON array string. |
| `Program.hashHex()` | `→ string` | Canonical content hash, `"0x…"`. |
| `Program.frames()` | `→ bigint` | Length in frames at the compiled rate. |
| `Program.sampleRate()` | `→ number` | Hz. |
| `Program.isStreamable()` | `→ boolean` | No streaming blockers (plays either way — blocked programs run from the pre-rendered bounce). |
| `Program.render()` | `→ Float32Array` | The full bounce, stereo-interleaved (`frames() * 2` samples). |
| `Program.renderStems()` | `→ Array<{id, isBus, left, right}>` | Per-track + per-bus planar stems (pre-master-chain). Costs a full extra render per call. |
| `Program.toJson()` / `Program.fromJson(json)` | `→ string` / `→ Program` | The portable bundle; `fromJson` re-verifies the hash (T3002) and rejects newer revisions (T3001). |
| `Program.play()` | `→ PerformanceHandle` | A live performance, stopped at frame 0. |
| `PerformanceHandle.schedule(commandJson, atJson?)` | `→ bigint` | The command's sequence id; throws on a grammar error, a full queue, or an unknown `at` position. |
| `PerformanceHandle.fill(frames)` | `→ Float32Array` | Renders live audio, executing due commands at their exact frames. The AudioWorklet calls this once per 128-frame quantum. |
| `PerformanceHandle.state()` | `→ "playing" \| "paused" \| "stopped"` | Reads the render-side transport. |
| `PerformanceHandle.positionBeats()` | `→ number` | Through the tempo map. |
| `PerformanceHandle.metricsJson()` | `→ string` | Health snapshot as a JSON object string (frames rendered, commands executed/dropped, queue depths, swaps, stingers). |

### The schedule JSON grammar

The same grammar the C ABI speaks — one JSON object, exactly one key.

Commands (`schedule(commandJson)`):

```json
{"play":true}  {"pause":true}  {"stop":true}  {"seek_beat":4.0}
{"seek_bar":2}  {"seek_section":"chorus"}  {"set_loop_bars":[1,3]}
{"clear_loop":true}  {"set_gain":0.8}
```

Times (`schedule(commandJson, atJson)` — omitted / `null` / `{}` /
`{"immediate":true}` = the next frame):

```json
{"frame":96000}  {"beat":4.0}  {"bar":2}  {"next_beat":true}
{"next_bar":true}  {"marker":"drop"}  {"section":"chorus"}
```

Flags must be `true` (`{"play":false}` is an error, not a no-op); more than
one key in an object is an error. Musical positions resolve to exact frames at
schedule time, so JS never wakes on a musical boundary — a quantized section
transition is `schedule({"seek_section":"hook"}, {"next_bar":true})`.

## The AudioWorklet runtime

Two vanilla-JS ES modules ship with the crate (no bundler, no npm):

- `js/tono-worklet.js` — an `AudioWorkletProcessor` that holds one
  `PerformanceHandle` and fills 128-frame quanta via `fill`. The
  AudioWorkletGlobalScope has no `fetch`, so the main thread transfers the
  `.wasm` bytes with the `load` message and the worklet instantiates
  synchronously (`initSync`). A `fill` failure reports and goes silent rather
  than killing the worklet.
- `js/tono.js` — the main-thread wrapper: `init(wasmUrl?)` (fetch +
  instantiate once; the fetched bytes are cached so every worklet gets its own
  copy), `renderDoc`, `compileSong`, `deinterleave`, and
  `playSong(program, audioContext?)`.

```js
import { init, compileSong, playSong } from "./js/tono.js";

await init();                                  // once
const program = compileSong(songJson, 44100);  // the rate is authoritative
const node = await playSong(program);          // creates an AudioContext at 44 100 Hz
node.connect(node.context.destination);

node.performance.schedule({ seek_section: "hook" }, { next_bar: true });
const { state, positionBeats, metrics } = await node.performance.metrics();
```

The program's compiled sample rate is authoritative — the performance renders
at exactly that rate, and Web Audio gives no resampling guarantee — so
`playSong` rejects a context at any other rate: recompile with
`compileSong(songJson, audioContext.sampleRate)`. `playSong` returns the node
unconnected; the host owns the routing.

Keep the `js/` + `pkg/` relative layout `make wasm` produces — both modules
import the wasm-bindgen glue from `../pkg/`.

## The example

`js/example.html` — a player, not an editor: compiles an embedded Song JSON
in-page, shows the program facts, bounces a WAV download, and plays the same
program live through the worklet (with a quantized section jump).

```sh
make wasm && cd crates/tono-wasm && python3 -m http.server 8000
# open http://localhost:8000/js/example.html
```

(Any static file server works; the `.wasm` just needs HTTP — `file://` won't
load it.)

## Testing state — honest version

- **Build-verified in CI**: the fast path compiles the crate to
  `wasm32-unknown-unknown` on every push/PR (`.github/workflows/ci.yml`,
  `build-wasm` job). No browser tests run in CI.
- **Unit-tested off-wasm**: `cargo test -p tono-wasm` covers the schedule/at
  JSON grammars, the diagnostics serialization, and a compile → play → fill
  smoke that pins the live `fill` byte-identical to the offline bounce head
  (the engine's own determinism invariant).
- **Smoke-tested on the real wasm**: during development the compiled `.wasm`
  was driven headlessly through the wasm-bindgen glue (Node 22): the full API
  surface, BigInt returns, error JSON, and the byte-identity check all passed.
- **Browser smoke is manual**: serve the crate dir and open
  `js/example.html` (above). That's the intended end-to-end check — it
  exercises the AudioWorklet path no headless runner can.
