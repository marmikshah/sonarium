//! tono-wasm — the WebAssembly face of tono: the pure engine compiled to
//! `wasm32-unknown-unknown` behind a small `wasm-bindgen` API a browser (or
//! any JS host) drives. Render a SoundDoc, compile a Song into an immutable
//! [`Program`], and run it live through a [`PerformanceHandle`] — the same
//! artifact the CLI renders and the Python face plays, through the exact same
//! engine.
//!
//! The build is lean by construction (`tono-core` with
//! `default-features = false`): serde + schemars + tracing + rustfft — all
//! pure Rust. The `analysis` feature's PNG surface (the `image` crate) and the
//! `sampler` feature's SoundFont file I/O (`rustysynth`) stay out of the
//! browser sandbox; sampler tracks render silence, exactly like a native lean
//! build.
//!
//! # The JS surface (camelCase)
//!
//! - `renderDoc(docJson) -> Float32Array` — a mono bounce of a SoundDoc
//!   (throws a JS `Error` on a parse/validation failure).
//! - `compileSong(songJson, sampleRate?) -> Program` — throws a JS `Error`
//!   whose `message` is the compile diagnostics serialized as a JSON array
//!   (`JSON.parse(err.message)` recovers them).
//! - `class Program` — `hashHex()`, `frames()`, `sampleRate()`,
//!   `isStreamable()`, `render()`, `renderStems()`, `toJson()`,
//!   `Program.fromJson(json)`, `play()`.
//! - `class PerformanceHandle` — `schedule(commandJson, atJson?)`,
//!   `fill(frames)`, `state()`, `positionBeats()`, `metricsJson()`.
//!
//! All audio crosses the boundary as **stereo-interleaved** `Float32Array`s
//! (`[L0, R0, L1, R1, …]`) — one copy per call, the layout
//! [`PerformanceHandle::fill`] emits and WAV interleave wants. Counts that can
//! exceed 2^53 (`frames()`, the schedule sequence id) arrive as JS `BigInt`s.
//!
//! The AudioWorklet runtime (`js/tono-worklet.js` + `js/tono.js`) and a
//! runnable player (`js/example.html`) ship with the crate; see the README.

#![warn(missing_docs)]

use std::sync::Arc;

use js_sys::{Array, Float32Array, Object, Reflect};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use tono_core::diag::Diagnostic;
use tono_core::program::Program as CoreProgram;
use tono_core::runtime::{At, AudioSource, Command, Performance, TransportState};
use tono_core::song::{CompileOptions, CompileTarget, Song};

/// A JS `Error` carrying the message (compile diagnostics serialize to JSON,
/// so `JSON.parse(err.message)` recovers the structured form).
fn js_error(message: impl Into<String>) -> JsValue {
    js_sys::Error::new(&message.into()).into()
}

/// Compile diagnostics as a JSON array string — the `Error.message` a failed
/// `compileSong` throws. Pure (no JS calls) so the grammar stays unit-testable
/// off-wasm.
fn diagnostics_json(diags: &[Diagnostic]) -> String {
    serde_json::to_string(diags).unwrap_or_else(|_| "[]".to_string())
}

/// Render a SoundDoc JSON document to mono f32 samples (throws on an invalid
/// document — a JSON parse failure or a validation error).
#[wasm_bindgen(js_name = renderDoc)]
pub fn render_doc(doc_json: &str) -> Result<Vec<f32>, JsValue> {
    let doc: tono_core::dsl::SoundDoc =
        serde_json::from_str(doc_json).map_err(|e| js_error(format!("document JSON: {e}")))?;
    doc.validate().map_err(|e| js_error(e.to_string()))?;
    Ok(tono_core::render::render(&doc))
}

/// Compile a Song JSON document into an immutable [`Program`] at `sample_rate`
/// (Hz; omitted = the document default, 44 100). Throws a JS `Error` whose
/// message is the compile diagnostics as a JSON array.
#[wasm_bindgen(js_name = compileSong)]
pub fn compile_song(song_json: &str, sample_rate: Option<u32>) -> Result<Program, JsValue> {
    let song: Song =
        serde_json::from_str(song_json).map_err(|e| js_error(format!("song JSON: {e}")))?;
    let opts = CompileOptions {
        sample_rate,
        target: CompileTarget::Runtime,
    };
    song.compile(&opts)
        .map(|p| Program { inner: Arc::new(p) })
        .map_err(|diags| js_error(diagnostics_json(&diags.0)))
}

/// A compiled song: validated, resolved, hashed — the immutable artifact the
/// runtime plays. Construct with [`compile_song`] or [`Program::from_json`].
#[wasm_bindgen]
pub struct Program {
    inner: Arc<CoreProgram>,
}

#[wasm_bindgen]
impl Program {
    /// The canonical content hash, hex (`"0x…"`, 16 digits).
    #[wasm_bindgen(js_name = hashHex)]
    pub fn hash_hex(&self) -> String {
        format!("{:#018x}", self.inner.hash)
    }

    /// The program length in frames (a `BigInt` in JS).
    pub fn frames(&self) -> u64 {
        self.inner.meta.duration_frames
    }

    /// The sample rate the program was compiled for (Hz).
    #[wasm_bindgen(js_name = sampleRate)]
    pub fn sample_rate(&self) -> u32 {
        self.inner.meta.sample_rate
    }

    /// Whether the program streams natively (no streaming blockers). Either
    /// way a [`PerformanceHandle`] plays it — a blocked program runs from its
    /// pre-rendered bounce instead of the streaming renderer.
    #[wasm_bindgen(js_name = isStreamable)]
    pub fn is_streamable(&self) -> bool {
        self.inner.is_streamable()
    }

    /// The full bounce as ONE stereo-interleaved `Float32Array`
    /// (`[L0, R0, L1, R1, …]`, `frames() * 2` samples) — the same layout
    /// [`PerformanceHandle::fill`] emits, so a host deinterleaves both paths
    /// the same way. One array is also one copy across the wasm/JS boundary
    /// (two planar arrays would be two) and the layout WAV interleave wants.
    pub fn render(&self) -> Vec<f32> {
        let (left, right) = self.inner.render_stereo();
        let mut out = Vec::with_capacity(left.len() * 2);
        for (l, r) in left.into_iter().zip(right) {
            out.push(l);
            out.push(r);
        }
        out
    }

    /// Per-track and per-bus stereo stems (pre-master-chain) as a JS array of
    /// `{ id, isBus, left, right }` objects with planar `Float32Array`
    /// channels, in declaration order. Costs a full extra render per call.
    #[wasm_bindgen(js_name = renderStems)]
    pub fn render_stems(&self) -> Array {
        let out = Array::new();
        for stem in self.inner.render_stems() {
            let obj = Object::new();
            let set = |key: &str, value: &JsValue| {
                Reflect::set(&obj, &JsValue::from_str(key), value).expect("a plain object");
            };
            set("id", &JsValue::from_str(&stem.id));
            set("isBus", &JsValue::from_bool(stem.is_bus));
            set("left", &Float32Array::from(stem.left.as_slice()));
            set("right", &Float32Array::from(stem.right.as_slice()));
            out.push(&obj);
        }
        out
    }

    /// The program bundle as compact JSON — the portable form
    /// [`Program::from_json`] reloads (hash re-verified on load).
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> String {
        self.inner.to_json()
    }

    /// Reload a bundle from JSON (throws on a parse failure, a newer bundle
    /// revision, or a hash mismatch — the structured `ProgramError` text).
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<Program, JsValue> {
        CoreProgram::from_json(json)
            .map(|p| Program { inner: Arc::new(p) })
            .map_err(|e| js_error(e.to_string()))
    }

    /// Start a live performance of this program, stopped at frame 0 — schedule
    /// `{"play":true}` to start it (the AudioWorklet runtime does this for
    /// you).
    pub fn play(&self) -> PerformanceHandle {
        PerformanceHandle {
            inner: Performance::new(Arc::clone(&self.inner)),
        }
    }
}

/// The schedule-command JSON grammar (the same grammar the C ABI speaks):
/// exactly one key — `{"play":true}`, `{"pause":true}`, `{"stop":true}`,
/// `{"seek_beat":4.0}`, `{"seek_bar":2}`, `{"seek_section":"chorus"}`,
/// `{"set_loop_bars":[1,3]}`, `{"clear_loop":true}`, `{"set_gain":0.8}`.
/// Flags must be `true` (`{"play":false}` is an error, not a no-op).
fn parse_command(json: &str) -> Result<Command, String> {
    /// Every field `None` unless set — serde rejects unknown keys, the count
    /// check below rejects ambiguity, so a command is exactly one key.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct CommandJson {
        play: Option<bool>,
        pause: Option<bool>,
        stop: Option<bool>,
        seek_beat: Option<f64>,
        seek_bar: Option<u32>,
        seek_section: Option<String>,
        set_loop_bars: Option<[u32; 2]>,
        clear_loop: Option<bool>,
        set_gain: Option<f32>,
    }
    const GRAMMAR: &str = "exactly one of play/pause/stop/seek_beat/seek_bar/\
                           seek_section/set_loop_bars/clear_loop/set_gain";
    let parsed: CommandJson = serde_json::from_str(json)
        .map_err(|e| format!("command JSON: {e} — expected {GRAMMAR}"))?;
    let mut commands: Vec<Command> = Vec::new();
    for (flag, name, command) in [
        (parsed.play, "play", Command::Play),
        (parsed.pause, "pause", Command::Pause),
        (parsed.stop, "stop", Command::Stop),
        (parsed.clear_loop, "clear_loop", Command::ClearLoop),
    ] {
        if let Some(on) = flag {
            if !on {
                return Err(format!(
                    "command JSON: \"{name}\" is a flag — set it to true or omit the key"
                ));
            }
            commands.push(command);
        }
    }
    if let Some(beat) = parsed.seek_beat {
        commands.push(Command::SeekBeat(beat));
    }
    if let Some(bar) = parsed.seek_bar {
        commands.push(Command::SeekBar(bar));
    }
    if let Some(name) = parsed.seek_section {
        commands.push(Command::SeekSection(name));
    }
    if let Some([start, end]) = parsed.set_loop_bars {
        commands.push(Command::SetLoopBars(start, end));
    }
    if let Some(gain) = parsed.set_gain {
        commands.push(Command::SetGain(gain));
    }
    match commands.len() {
        1 => Ok(commands.remove(0)),
        0 => Err(format!(
            "command JSON: expected {GRAMMAR} — got an empty object"
        )),
        n => Err(format!(
            "command JSON: expected {GRAMMAR} — got {n} keys in one object"
        )),
    }
}

/// The schedule-time JSON grammar: omitted / `null` / `{}` /
/// `{"immediate":true}` = the next frame; otherwise exactly one key —
/// `{"frame":96000}`, `{"beat":4.0}`, `{"bar":2}`, `{"next_beat":true}`,
/// `{"next_bar":true}`, `{"marker":"drop"}`, `{"section":"chorus"}`.
fn parse_at(json: Option<&str>) -> Result<At, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct AtJson {
        immediate: Option<bool>,
        frame: Option<u64>,
        beat: Option<f64>,
        bar: Option<u32>,
        next_beat: Option<bool>,
        next_bar: Option<bool>,
        marker: Option<String>,
        section: Option<String>,
    }
    const GRAMMAR: &str = "at most one of immediate/frame/beat/bar/next_beat/\
                           next_bar/marker/section";
    let Some(json) = json else {
        return Ok(At::Immediate);
    };
    let parsed: Option<AtJson> =
        serde_json::from_str(json).map_err(|e| format!("at JSON: {e} — expected {GRAMMAR}"))?;
    let Some(parsed) = parsed else {
        return Ok(At::Immediate); // explicit null
    };
    let mut ats: Vec<At> = Vec::new();
    for (flag, name, at) in [
        (parsed.immediate, "immediate", At::Immediate),
        (parsed.next_beat, "next_beat", At::NextBeat),
        (parsed.next_bar, "next_bar", At::NextBar),
    ] {
        if let Some(on) = flag {
            if !on {
                return Err(format!(
                    "at JSON: \"{name}\" is a flag — set it to true or omit the key"
                ));
            }
            ats.push(at);
        }
    }
    if let Some(frame) = parsed.frame {
        ats.push(At::Frame(frame));
    }
    if let Some(beat) = parsed.beat {
        ats.push(At::Beat(beat));
    }
    if let Some(bar) = parsed.bar {
        ats.push(At::Bar(bar));
    }
    if let Some(name) = parsed.marker {
        ats.push(At::Marker(name));
    }
    if let Some(name) = parsed.section {
        ats.push(At::Section(name));
    }
    match ats.len() {
        0 => Ok(At::Immediate), // an empty object schedules immediately
        1 => Ok(ats.remove(0)),
        n => Err(format!(
            "at JSON: expected {GRAMMAR} — got {n} keys in one object"
        )),
    }
}

/// A running [`Program`]: sample-accurate transport, a bounded
/// scheduled-command queue, ramped gain, loops — driven by [`fill`](Self::fill)
/// (the AudioWorklet calls it once per 128-frame quantum).
#[wasm_bindgen]
pub struct PerformanceHandle {
    inner: Performance,
}

#[wasm_bindgen]
impl PerformanceHandle {
    /// Schedule a command (see the grammars on [`parse_command`] /
    /// [`parse_at`]). Returns the command's sequence id (a `BigInt` in JS);
    /// throws on a grammar error, a full queue, or an unknown marker/section.
    pub fn schedule(
        &mut self,
        command_json: &str,
        at_json: Option<String>,
    ) -> Result<u64, JsValue> {
        let command = parse_command(command_json).map_err(js_error)?;
        let at = parse_at(at_json.as_deref()).map_err(js_error)?;
        self.inner
            .schedule(command, at)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Render `frames` frames of live audio, executing due commands at their
    /// exact frames, as a stereo-interleaved `Float32Array`
    /// (`frames * 2` samples) — the quantum the AudioWorklet consumes.
    pub fn fill(&mut self, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames * 2];
        self.inner.fill(&mut out);
        out
    }

    /// The transport state: `"playing"`, `"paused"`, or `"stopped"`. Reads the
    /// render-side transport, so a just-scheduled command shows after the next
    /// `fill`.
    pub fn state(&self) -> String {
        match self.inner.transport().state() {
            TransportState::Playing => "playing",
            TransportState::Paused => "paused",
            TransportState::Stopped => "stopped",
        }
        .to_string()
    }

    /// The transport position in beats (through the tempo map).
    #[wasm_bindgen(js_name = positionBeats)]
    pub fn position_beats(&self) -> f64 {
        self.inner.transport().position_beats()
    }

    /// The health snapshot as a JSON object string: `frames_rendered`,
    /// `commands_executed`, `commands_dropped`, `queue_depth_max`, `swaps`,
    /// `stingers_fired`, and the `queue_depth` sampled now.
    #[wasm_bindgen(js_name = metricsJson)]
    pub fn metrics_json(&self) -> String {
        let metrics = self.inner.metrics();
        serde_json::json!({
            "frames_rendered": metrics.frames_rendered,
            "commands_executed": metrics.commands_executed,
            "commands_dropped": metrics.commands_dropped,
            "queue_depth_max": metrics.queue_depth_max,
            "swaps": metrics.swaps,
            "stingers_fired": metrics.stingers_fired,
            "queue_depth": self.inner.queue_depth(),
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_grammar_accepts_the_documented_forms() {
        assert!(matches!(
            parse_command(r#"{"play":true}"#).unwrap(),
            Command::Play
        ));
        assert!(matches!(
            parse_command(r#"{"pause":true}"#).unwrap(),
            Command::Pause
        ));
        assert!(matches!(
            parse_command(r#"{"stop":true}"#).unwrap(),
            Command::Stop
        ));
        assert!(matches!(
            parse_command(r#"{"seek_beat":4.5}"#).unwrap(),
            Command::SeekBeat(b) if b == 4.5
        ));
        assert!(matches!(
            parse_command(r#"{"seek_bar":2}"#).unwrap(),
            Command::SeekBar(2)
        ));
        assert!(matches!(
            parse_command(r#"{"seek_section":"hook"}"#).unwrap(),
            Command::SeekSection(n) if n == "hook"
        ));
        assert!(matches!(
            parse_command(r#"{"set_loop_bars":[1,3]}"#).unwrap(),
            Command::SetLoopBars(1, 3)
        ));
        assert!(matches!(
            parse_command(r#"{"clear_loop":true}"#).unwrap(),
            Command::ClearLoop
        ));
        assert!(matches!(
            parse_command(r#"{"set_gain":0.8}"#).unwrap(),
            Command::SetGain(g) if g == 0.8
        ));
    }

    #[test]
    fn command_grammar_rejects_ambiguity_unknown_keys_and_false_flags() {
        assert!(parse_command(r#"{"play":true,"stop":true}"#).is_err());
        assert!(parse_command(r#"{"play":false}"#).is_err());
        assert!(parse_command(r#"{"explode":true}"#).is_err());
        assert!(parse_command(r#"{}"#).is_err());
        assert!(parse_command("not json").is_err());
    }

    #[test]
    fn at_grammar_accepts_the_documented_forms() {
        assert_eq!(parse_at(None).unwrap(), At::Immediate);
        assert_eq!(parse_at(Some("null")).unwrap(), At::Immediate);
        assert_eq!(parse_at(Some("{}")).unwrap(), At::Immediate);
        assert_eq!(
            parse_at(Some(r#"{"immediate":true}"#)).unwrap(),
            At::Immediate
        );
        assert_eq!(
            parse_at(Some(r#"{"frame":96000}"#)).unwrap(),
            At::Frame(96_000)
        );
        assert_eq!(parse_at(Some(r#"{"beat":4.0}"#)).unwrap(), At::Beat(4.0));
        assert_eq!(parse_at(Some(r#"{"bar":2}"#)).unwrap(), At::Bar(2));
        assert_eq!(
            parse_at(Some(r#"{"next_beat":true}"#)).unwrap(),
            At::NextBeat
        );
        assert_eq!(parse_at(Some(r#"{"next_bar":true}"#)).unwrap(), At::NextBar);
        assert_eq!(
            parse_at(Some(r#"{"marker":"drop"}"#)).unwrap(),
            At::Marker("drop".to_string())
        );
        assert_eq!(
            parse_at(Some(r#"{"section":"hook"}"#)).unwrap(),
            At::Section("hook".to_string())
        );
        assert!(parse_at(Some(r#"{"next_bar":false}"#)).is_err());
        assert!(parse_at(Some(r#"{"frame":1,"bar":2}"#)).is_err());
    }

    #[test]
    fn diagnostics_serialize_as_a_json_array() {
        let song: Song = serde_json::from_str(
            r#"{"name":"bad","bpm":120.0,"tracks":[],"patterns":[],"arrangement":[]}"#,
        )
        .unwrap();
        let diags = song.compile(&CompileOptions::default()).unwrap_err();
        let json = diagnostics_json(&diags.0);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed[0]["code"], "T1000");
        assert_eq!(parsed[0]["severity"], "error");
    }

    /// The smoke test a browser run reproduces: compile → play → schedule →
    /// fill renders the song live, and the first block equals the bounce's
    /// head (the runtime is byte-identical to the offline render).
    #[test]
    fn compile_play_fill_matches_the_bounce_head() {
        let song = r#"{
            "name": "wasm-test", "bpm": 120.0, "version": 2,
            "tracks": [
                { "name": "bass", "wave": "bass",
                  "env": { "a": 0.005, "d": 0.1, "s": 0.8, "r": 0.2, "punch": 0.0 } }
            ],
            "patterns": [
                { "name": "riff", "bars": 1,
                  "notes": [ { "step": 0, "len": 4, "pitch": "C2" },
                             { "step": 8, "len": 4, "pitch": "G2" } ] }
            ],
            "arrangement": [ { "track": "bass", "pattern": "riff", "bar": 0 } ]
        }"#;
        let program = compile_song(song, Some(44_100)).unwrap();
        assert_eq!(program.sample_rate(), 44_100);
        assert!(program.frames() > 0);
        assert!(program.is_streamable());
        assert!(program.hash_hex().starts_with("0x"));
        // The bundle round-trips.
        let reloaded = Program::from_json(&program.to_json()).unwrap();
        assert_eq!(reloaded.hash_hex(), program.hash_hex());
        // Live: schedule play immediately and fill one quantum.
        let mut handle = program.play();
        assert_eq!(handle.state(), "stopped");
        handle
            .schedule(r#"{"play":true}"#, None)
            .unwrap_or_else(|_| panic!("a valid schedule parses"));
        let live = handle.fill(512);
        assert_eq!(live.len(), 1024);
        assert_eq!(handle.state(), "playing");
        assert!(handle.position_beats() > 0.0);
        assert!(live.iter().any(|s| *s != 0.0), "the song sounds");
        // Byte-identical to the offline bounce's head (interleaved).
        assert_eq!(live, program.render()[..1024]);
        let metrics: serde_json::Value = serde_json::from_str(&handle.metrics_json()).unwrap();
        assert_eq!(metrics["frames_rendered"], 512);
        assert_eq!(metrics["commands_executed"], 1);
    }
}
