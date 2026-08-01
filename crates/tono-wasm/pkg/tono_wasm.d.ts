/* tslint:disable */
/* eslint-disable */

/**
 * A running [`Program`]: sample-accurate transport, a bounded
 * scheduled-command queue, ramped gain, loops — driven by [`fill`](Self::fill)
 * (the AudioWorklet calls it once per 128-frame quantum).
 */
export class PerformanceHandle {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Render `frames` frames of live audio, executing due commands at their
     * exact frames, as a stereo-interleaved `Float32Array`
     * (`frames * 2` samples) — the quantum the AudioWorklet consumes.
     */
    fill(frames: number): Float32Array;
    /**
     * The health snapshot as a JSON object string: `frames_rendered`,
     * `commands_executed`, `commands_dropped`, `queue_depth_max`, `swaps`,
     * `stingers_fired`, and the `queue_depth` sampled now.
     */
    metricsJson(): string;
    /**
     * The transport position in beats (through the tempo map).
     */
    positionBeats(): number;
    /**
     * Schedule a command (see the grammars on [`parse_command`] /
     * [`parse_at`]). Returns the command's sequence id (a `BigInt` in JS);
     * throws on a grammar error, a full queue, or an unknown marker/section.
     */
    schedule(command_json: string, at_json?: string | null): bigint;
    /**
     * The transport state: `"playing"`, `"paused"`, or `"stopped"`. Reads the
     * render-side transport, so a just-scheduled command shows after the next
     * `fill`.
     */
    state(): string;
}

/**
 * A compiled song: validated, resolved, hashed — the immutable artifact the
 * runtime plays. Construct with [`compile_song`] or [`Program::from_json`].
 */
export class Program {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The program length in frames (a `BigInt` in JS).
     */
    frames(): bigint;
    /**
     * Reload a bundle from JSON (throws on a parse failure, a newer bundle
     * revision, or a hash mismatch — the structured `ProgramError` text).
     */
    static fromJson(json: string): Program;
    /**
     * The canonical content hash, hex (`"0x…"`, 16 digits).
     */
    hashHex(): string;
    /**
     * Whether the program streams natively (no streaming blockers). Either
     * way a [`PerformanceHandle`] plays it — a blocked program runs from its
     * pre-rendered bounce instead of the streaming renderer.
     */
    isStreamable(): boolean;
    /**
     * Start a live performance of this program, stopped at frame 0 — schedule
     * `{"play":true}` to start it (the AudioWorklet runtime does this for
     * you).
     */
    play(): PerformanceHandle;
    /**
     * The full bounce as ONE stereo-interleaved `Float32Array`
     * (`[L0, R0, L1, R1, …]`, `frames() * 2` samples) — the same layout
     * [`PerformanceHandle::fill`] emits, so a host deinterleaves both paths
     * the same way. One array is also one copy across the wasm/JS boundary
     * (two planar arrays would be two) and the layout WAV interleave wants.
     */
    render(): Float32Array;
    /**
     * Per-track and per-bus stereo stems (pre-master-chain) as a JS array of
     * `{ id, isBus, left, right }` objects with planar `Float32Array`
     * channels, in declaration order. Costs a full extra render per call.
     */
    renderStems(): Array<any>;
    /**
     * The sample rate the program was compiled for (Hz).
     */
    sampleRate(): number;
    /**
     * The program bundle as compact JSON — the portable form
     * [`Program::from_json`] reloads (hash re-verified on load).
     */
    toJson(): string;
}

/**
 * Compile a Song JSON document into an immutable [`Program`] at `sample_rate`
 * (Hz; omitted = the document default, 44 100). Throws a JS `Error` whose
 * message is the compile diagnostics as a JSON array.
 */
export function compileSong(song_json: string, sample_rate?: number | null): Program;

/**
 * Render a SoundDoc JSON document to mono f32 samples (throws on an invalid
 * document — a JSON parse failure or a validation error).
 */
export function renderDoc(doc_json: string): Float32Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_performancehandle_free: (a: number, b: number) => void;
    readonly __wbg_program_free: (a: number, b: number) => void;
    readonly compileSong: (a: number, b: number, c: number) => [number, number, number];
    readonly performancehandle_fill: (a: number, b: number) => [number, number];
    readonly performancehandle_metricsJson: (a: number) => [number, number];
    readonly performancehandle_positionBeats: (a: number) => number;
    readonly performancehandle_schedule: (a: number, b: number, c: number, d: number, e: number) => [bigint, number, number];
    readonly performancehandle_state: (a: number) => [number, number];
    readonly program_frames: (a: number) => bigint;
    readonly program_fromJson: (a: number, b: number) => [number, number, number];
    readonly program_hashHex: (a: number) => [number, number];
    readonly program_isStreamable: (a: number) => number;
    readonly program_play: (a: number) => number;
    readonly program_render: (a: number) => [number, number];
    readonly program_renderStems: (a: number) => any;
    readonly program_sampleRate: (a: number) => number;
    readonly program_toJson: (a: number) => [number, number];
    readonly renderDoc: (a: number, b: number) => [number, number, number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
