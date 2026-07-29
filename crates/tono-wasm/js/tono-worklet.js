// tono-worklet.js — the AudioWorkletProcessor half of the tono WASM face.
//
// The processor owns one `PerformanceHandle` (the core `Performance` running
// in this worklet's own wasm instance) and renders 128-frame quanta by calling
// `fill(frames)` on it — the same `fill` the Rust tests and the Python face's
// headless mode drive, byte-identical to the offline bounce of the program.
//
// The AudioWorkletGlobalScope has no `fetch`, so the main thread fetches the
// .wasm bytes once (tono.js `init`) and transfers a copy with the `load`
// message; this module imports only the wasm-bindgen glue — which touches no
// DOM outside its fetch path — and instantiates synchronously (`initSync`).
// Keep the relative layout `js/` + `pkg/` this import assumes (`make wasm`
// produces exactly that).
//
// Port protocol (→ from the main thread, ← to it):
//   → { type: "load", wasmBytes, programJson, autoplay? }   instantiate + load (+ play)
//   ← { type: "ready" } | { type: "error", message }
//   → { type: "schedule", command, at? }                    the JSON grammar (see README)
//   ← { type: "scheduled", seq } | { type: "error", message, command }
//   → { type: "metrics", id }
//   ← { type: "metrics", id, metrics, state, positionBeats }

import { initSync, Program } from "../pkg/tono_wasm.js";

class TonoPerformanceProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    /** @type {import("../pkg/tono_wasm.js").PerformanceHandle | null} */
    this.handle = null;
    this.dead = false;
    // Serialize message handling: an async `load` must finish before any later
    // `schedule` runs.
    this.queue = Promise.resolve();
    this.port.onmessage = (event) => {
      this.queue = this.queue.then(() => this._onMessage(event.data)).catch(() => {});
    };
  }

  async _onMessage(msg) {
    switch (msg.type) {
      case "load":
        try {
          initSync({ module: msg.wasmBytes });
          const program = Program.fromJson(msg.programJson);
          this.handle = program.play();
          program.free(); // the handle holds its own reference (an Arc clone)
          if (msg.autoplay) {
            this.handle.schedule(JSON.stringify({ play: true }));
          }
          this.port.postMessage({ type: "ready" });
        } catch (err) {
          this.port.postMessage({ type: "error", message: String((err && err.message) || err) });
        }
        break;
      case "schedule":
        if (!this.handle) break;
        try {
          const seq = this.handle.schedule(
            JSON.stringify(msg.command),
            msg.at === undefined ? undefined : JSON.stringify(msg.at),
          );
          this.port.postMessage({ type: "scheduled", seq });
        } catch (err) {
          this.port.postMessage({
            type: "error",
            message: String((err && err.message) || err),
            command: msg.command,
          });
        }
        break;
      case "metrics":
        this.port.postMessage({
          type: "metrics",
          id: msg.id,
          metrics: this.handle ? JSON.parse(this.handle.metricsJson()) : null,
          state: this.handle ? this.handle.state() : "stopped",
          positionBeats: this.handle ? this.handle.positionBeats() : 0,
        });
        break;
    }
  }

  process(_inputs, outputs) {
    const out = outputs[0];
    if (!out || out.length === 0) return true;
    const frames = out[0].length;
    if (!this.handle || this.dead) {
      for (const ch of out) ch.fill(0);
      return true;
    }
    let interleaved;
    try {
      interleaved = this.handle.fill(frames);
    } catch (err) {
      // A failure on the audio path must not kill the worklet: go silent and
      // report once.
      this.dead = true;
      this.port.postMessage({
        type: "error",
        message: `fill failed: ${String((err && err.message) || err)}`,
      });
      for (const ch of out) ch.fill(0);
      return true;
    }
    const left = out[0];
    const right = out.length > 1 ? out[1] : out[0];
    for (let i = 0; i < frames; i++) {
      left[i] = interleaved[i * 2];
      right[i] = interleaved[i * 2 + 1];
    }
    return true;
  }
}

registerProcessor("tono-performance", TonoPerformanceProcessor);
