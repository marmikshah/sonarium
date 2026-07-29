// tono.js — the main-thread wrapper of the tono WASM face. A tiny ES module,
// no bundler, no npm:
//
//   import { init, compileSong, renderDoc, playSong } from "./js/tono.js";
//   await init();                       // fetch + instantiate the .wasm once
//   const program = compileSong(songJson, 44100);
//   const node = await playSong(program, audioContext);   // live, via AudioWorklet
//   node.performance.schedule({ seek_section: "hook" }, { next_bar: true });
//
// Keep the `js/` + `pkg/` relative layout `make wasm` produces — both this
// module and the worklet import the wasm-bindgen glue from `../pkg/`.

import initWasm, {
  renderDoc as wasmRenderDoc,
  compileSong as wasmCompileSong,
  Program,
} from "../pkg/tono_wasm.js";

let readyPromise = null;
let wasmBytes = null;

/**
 * Fetch and instantiate the wasm module. Safe to call more than once — only
 * the first call does the work; later calls return the same promise.
 * @param {string | URL} [wasmUrl] defaults to the `pkg/` output of `make wasm`.
 */
export function init(wasmUrl = new URL("../pkg/tono_wasm_bg.wasm", import.meta.url)) {
  if (!readyPromise) {
    readyPromise = (async () => {
      const response = await fetch(wasmUrl);
      if (!response.ok) {
        throw new Error(`tono: fetching the wasm module failed: ${response.status} ${response.statusText}`);
      }
      wasmBytes = await response.arrayBuffer();
      await initWasm({ module_or_path: wasmBytes });
    })();
    // A failed init must not poison later retries.
    readyPromise.catch(() => {
      readyPromise = null;
      wasmBytes = null;
    });
  }
  return readyPromise;
}

/**
 * Render a SoundDoc JSON document to mono samples.
 * @returns {Float32Array}
 */
export function renderDoc(docJson) {
  return wasmRenderDoc(docJson);
}

/**
 * Compile a Song JSON document into a Program. Throws a JS `Error` whose
 * `message` is the compile diagnostics as a JSON array
 * (`JSON.parse(err.message)`).
 * @returns {Program}
 */
export function compileSong(songJson, sampleRate) {
  return wasmCompileSong(songJson, sampleRate);
}

/** Split a stereo-interleaved buffer into `{ left, right }` planar channels. */
export function deinterleave(interleaved) {
  const frames = interleaved.length >> 1;
  const left = new Float32Array(frames);
  const right = new Float32Array(frames);
  for (let i = 0; i < frames; i++) {
    left[i] = interleaved[i * 2];
    right[i] = interleaved[i * 2 + 1];
  }
  return { left, right };
}

/**
 * Play a Program live through an AudioWorklet. The program's compiled sample
 * rate is authoritative — the performance renders at exactly that rate, so a
 * context at any other rate is rejected (recompile with
 * `compileSong(songJson, audioContext.sampleRate)`); omit `audioContext` to
 * have one created at the program's rate. Resolves to the AudioWorkletNode
 * (connect it to `audioContext.destination` — playSong does NOT connect it,
 * so a host owns the routing) with a `performance` convenience surface:
 *   node.performance.schedule(command, at?)  — fire-and-forget, the JSON
 *                                              grammar as JS objects
 *   node.performance.metrics()               — Promise of the health snapshot
 * @param {Program} program
 * @param {AudioContext} [audioContext]
 * @param {{ workletUrl?: string | URL }} [options]
 */
export async function playSong(program, audioContext, options = {}) {
  const rate = program.sampleRate();
  if (audioContext === undefined) {
    audioContext = new AudioContext({ sampleRate: rate });
  }
  if (audioContext.sampleRate !== rate) {
    throw new Error(
      `tono: the program is compiled for ${rate} Hz but the AudioContext runs at ` +
        `${audioContext.sampleRate} Hz — recompile with compileSong(songJson, ${audioContext.sampleRate})`,
    );
  }
  await init(); // guarantees the fetched bytes are cached for the worklet
  const workletUrl = options.workletUrl ?? new URL("./tono-worklet.js", import.meta.url);
  await audioContext.audioWorklet.addModule(workletUrl);

  const node = new AudioWorkletNode(audioContext, "tono-performance", {
    numberOfInputs: 0,
    numberOfOutputs: 1,
    outputChannelCount: [2],
  });

  const ready = new Promise((resolve, reject) => {
    const onMessage = (event) => {
      const msg = event.data;
      if (msg.type === "ready" || msg.type === "error") {
        node.port.removeEventListener("message", onMessage);
        if (msg.type === "ready") resolve();
        else reject(new Error(msg.message));
      }
    };
    node.port.addEventListener("message", onMessage);
    node.port.start();
  });
  // A fresh copy per worklet (the transfer detaches it): the cached bytes
  // stay intact for the next worklet.
  const bytes = wasmBytes.slice(0);
  node.port.postMessage(
    { type: "load", wasmBytes: bytes, programJson: program.toJson(), autoplay: true },
    [bytes],
  );
  await ready;

  let metricId = 0;
  const metricWaiters = new Map();
  node.port.addEventListener("message", (event) => {
    const msg = event.data;
    if (msg.type === "metrics" && metricWaiters.has(msg.id)) {
      metricWaiters.get(msg.id)(msg);
      metricWaiters.delete(msg.id);
    }
  });
  node.performance = {
    schedule: (command, at) => node.port.postMessage({ type: "schedule", command, at }),
    metrics: () =>
      new Promise((resolve) => {
        const id = metricId++;
        metricWaiters.set(id, resolve);
        node.port.postMessage({ type: "metrics", id });
      }),
  };
  return node;
}

export { Program };
