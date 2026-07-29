//! tono-capi — a stable C ABI for tono (issue #52; 1.10.0-beta.1).
//!
//! Native hosts (game engines, embedded players, anything that can call a C
//! function) drive tono through this crate: validate a [`SoundDoc`] JSON,
//! load a compiled [`Program`] bundle, render it offline, and run it live
//! through a [`Performance`] — all behind opaque handles, C strings, and
//! plain return codes. The C header matching this file is `capi.h` in the
//! crate root; the C smoke test is `tests/smoke.c` (run via `make capi`).
//!
//! # Handles and ownership
//!
//! - `ProgramHandle` / `PerformanceHandle` are opaque: the host sees only
//!   pointers and must never dereference, copy, or forge them.
//! - `tono_program_from_json` / `tono_performance_new` return an owned
//!   handle; the matching `tono_*_free` consumes it exactly once.
//!   `tono_*_free(NULL)` is a no-op, like `free(3)`. Double-free is UB, as
//!   in C.
//! - Strings returned by value (`tono_program_hash_hex`,
//!   `tono_performance_metrics_json`) are owned by the caller and released
//!   with `tono_free_string`. `tono_last_error` returns a *borrowed* string,
//!   valid until the next `tono_*` call on the same thread.
//! - `tono_performance_new` clones the program's internal `Arc`: the caller
//!   keeps ownership of the program handle and must still free it.
//! - Handles are not thread-safe: confine each handle to one thread at a
//!   time (different handles may live on different threads).
//!
//! # Errors
//!
//! Every fallible call reports through a thread-local last-error string: it
//! returns the error value (NULL, -1, or 0 — stated per function) and sets
//! the cause, readable via `tono_last_error()`. A successful call leaves it
//! empty. Every `tono_*` call except `tono_last_error` itself refreshes the
//! slot.
//!
//! # The command / at JSON grammars
//!
//! `tono_performance_schedule_json` takes the command and its position as
//! two single-key JSON objects.
//!
//! Command — exactly one of:
//! `{"play":true}` · `{"pause":true}` · `{"stop":true}` · `{"seek_bar":3}` ·
//! `{"seek_beat":8.5}` · `{"seek_section":"chorus"}` ·
//! `{"set_loop_bars":[1,4]}` · `{"clear_loop":true}` · `{"set_gain":0.8}`
//!
//! At — exactly one of:
//! `{"immediate":true}` · `{"next_bar":true}` · `{"next_beat":true}` ·
//! `{"frame":96000}` · `{"beat":4.0}` · `{"bar":2}` · `{"marker":"drop"}` ·
//! `{"section":"chorus"}`
//!
//! Anything else is rejected with the accepted grammar in the error message.
//!
//! # SAFETY — the crate's one unsafe boundary
//!
//! Every exported function is `extern "C"` and defensive at the boundary:
//!
//! - NULL inputs return the error value with last-error set; a NULL pointer
//!   is never dereferenced. Buffer pointers (`tono_program_render`,
//!   `tono_performance_fill`) must point to the stated count of writable
//!   `f32` — that part is the caller's contract, as in any C API.
//! - No panic escapes: each entry point runs under `catch_unwind`; a panic
//!   becomes last-error "internal panic: …" plus the error value.
//! - Incoming C strings are read with `CStr::from_ptr` behind a NULL check
//!   and must be NUL-terminated UTF-8; non-UTF-8 is an error, never UB.

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{UnwindSafe, catch_unwind};
use std::ptr;
use std::sync::Arc;

use tono_core::dsl::SoundDoc;
use tono_core::program::Program;
use tono_core::runtime::{At, AudioSource, Command, Performance};

/// The accepted command grammar, quoted in every command-parse error (and
/// mirrored in `capi.h` and the module docs above).
const COMMAND_GRAMMAR: &str = "{\"play\":true} | {\"pause\":true} | {\"stop\":true} | {\"seek_bar\":BAR} | {\"seek_beat\":BEAT} | {\"seek_section\":\"NAME\"} | {\"set_loop_bars\":[START,END]} | {\"clear_loop\":true} | {\"set_gain\":GAIN}";

/// The accepted at grammar, quoted in every at-parse error.
const AT_GRAMMAR: &str = "{\"immediate\":true} | {\"next_bar\":true} | {\"next_beat\":true} | {\"frame\":FRAME} | {\"beat\":BEAT} | {\"bar\":BAR} | {\"marker\":\"NAME\"} | {\"section\":\"NAME\"}";

/// An owned compiled program. The `Arc` is what `Performance::new` clones —
/// freeing the handle while a performance runs is safe and sound.
pub struct ProgramHandle {
    program: Arc<Program>,
}

/// An owned running program.
pub struct PerformanceHandle {
    performance: Performance,
}

thread_local! {
    /// The per-thread error slot behind `tono_last_error`. Always holds a
    /// valid C string; empty means "no error".
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::default());
}

/// Record the cause of a failed call. Interior NULs are blanked so the slot
/// always holds a valid C string; a re-entrant borrow drops the message
/// rather than panicking across the ABI.
fn set_last_error(message: impl Into<String>) {
    let message = message.into().replace('\0', " ");
    LAST_ERROR.with(|slot| {
        if let Ok(mut slot) = slot.try_borrow_mut()
            && let Ok(message) = CString::new(message)
        {
            *slot = message;
        }
    });
}

/// Run one entry point: clear the slot, catch any panic, map a `Result`
/// onto (error value + last-error). This is the panic firewall every
/// `extern "C"` body funnels through.
fn entry<T>(error_value: T, f: impl FnOnce() -> Result<T, String> + UnwindSafe) -> T {
    set_last_error("");
    match catch_unwind(f) {
        Ok(Ok(value)) => value,
        Ok(Err(message)) => {
            set_last_error(message);
            error_value
        }
        Err(payload) => {
            let detail = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown".into());
            set_last_error(format!("internal panic: {detail}"));
            error_value
        }
    }
}

/// Read an incoming C string behind a NULL check, as UTF-8.
fn c_str<'a>(ptr: *const c_char, what: &str) -> Result<&'a str, String> {
    if ptr.is_null() {
        return Err(format!("{what}: NULL string pointer"));
    }
    // SAFETY: non-NULL, checked above; the caller guarantees a live,
    // NUL-terminated string for the duration of the call.
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| format!("{what}: not valid UTF-8"))
}

/// Borrow the program handle behind a NULL check.
fn program_ref<'a>(handle: *const ProgramHandle) -> Result<&'a ProgramHandle, String> {
    if handle.is_null() {
        return Err("NULL program handle".into());
    }
    // SAFETY: non-NULL, checked above; the caller guarantees a live handle.
    Ok(unsafe { &*handle })
}

/// Borrow the performance handle behind a NULL check.
fn performance_ref<'a>(handle: *const PerformanceHandle) -> Result<&'a Performance, String> {
    if handle.is_null() {
        return Err("NULL performance handle".into());
    }
    // SAFETY: non-NULL, checked above; the caller guarantees a live handle.
    Ok(unsafe { &(*handle).performance })
}

/// Mutably borrow the performance handle behind a NULL check.
fn performance_mut<'a>(handle: *mut PerformanceHandle) -> Result<&'a mut Performance, String> {
    if handle.is_null() {
        return Err("NULL performance handle".into());
    }
    // SAFETY: non-NULL, checked above; the caller guarantees a live handle
    // confined to this thread (handles are not thread-safe — see module docs).
    Ok(unsafe { &mut (*handle).performance })
}

/// Hand an owned string to the caller (`tono_free_string` releases it).
fn into_c_string(s: String) -> Result<*mut c_char, String> {
    CString::new(s)
        .map(CString::into_raw)
        .map_err(|_| "internal: string contained a NUL byte".to_string())
}

/// The last error on this thread, or an empty string when there is none.
///
/// The returned pointer is borrowed: it stays valid until the next `tono_*`
/// call on the same thread. Never free it.
#[unsafe(no_mangle)]
pub extern "C" fn tono_last_error() -> *const c_char {
    match LAST_ERROR.try_with(|slot| slot.try_borrow().map(|s| s.as_ptr())) {
        Ok(Ok(ptr)) => ptr,
        // Thread teardown or a re-entrant borrow: still never NULL.
        _ => c"".as_ptr(),
    }
}

/// Free a string tono returned ownership of (`tono_program_hash_hex`,
/// `tono_performance_metrics_json`). NULL is a no-op. Passing any other
/// pointer is UB, as with `free(3)`.
///
/// # Safety
///
/// `s` must be NULL or a pointer previously returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tono_free_string(s: *mut c_char) {
    entry((), || {
        if !s.is_null() {
            // SAFETY: the caller's contract — a string this library returned,
            // freed exactly once.
            drop(unsafe { CString::from_raw(s) });
        }
        Ok(())
    });
}

/// Validate a SoundDoc JSON document: 1 when it parses and passes
/// validation, 0 otherwise (last-error names the problem).
///
/// # Safety
///
/// `json` must be NULL or a NUL-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tono_doc_validate(json: *const c_char) -> i32 {
    entry(0, || {
        let json = c_str(json, "json")?;
        let doc: SoundDoc =
            serde_json::from_str(json).map_err(|e| format!("invalid document JSON: {e}"))?;
        doc.validate()
            .map_err(|e| format!("invalid document: {e}"))?;
        Ok(1)
    })
}

/// Load a compiled Program bundle (the JSON `tono compile` writes). Returns
/// an owned handle, or NULL on error — malformed JSON, a bundle newer than
/// this binary (T3001), or a hash mismatch (T3002); last-error names which.
///
/// # Safety
///
/// `json` must be NULL or a NUL-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tono_program_from_json(json: *const c_char) -> *mut ProgramHandle {
    entry(ptr::null_mut(), || {
        let json = c_str(json, "json")?;
        let program = Program::from_json(json).map_err(|e| e.to_string())?;
        Ok(Box::into_raw(Box::new(ProgramHandle {
            program: Arc::new(program),
        })))
    })
}

/// Free a program handle. NULL is a no-op. Performances created from the
/// program keep it alive (they hold their own `Arc`), so freeing a program
/// with live performances is sound.
///
/// # Safety
///
/// `program` must be NULL or a live handle from `tono_program_from_json`,
/// freed exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tono_program_free(program: *mut ProgramHandle) {
    entry((), || {
        if !program.is_null() {
            // SAFETY: the caller's contract — a live handle, freed once.
            drop(unsafe { Box::from_raw(program) });
        }
        Ok(())
    })
}

/// The program's canonical content hash as an owned `"0x…"` hex string
/// (free with `tono_free_string`), or NULL on error.
///
/// # Safety
///
/// `program` must be NULL or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tono_program_hash_hex(program: *const ProgramHandle) -> *mut c_char {
    entry(ptr::null_mut(), || {
        let program = program_ref(program)?;
        into_c_string(format!("{:#018x}", program.program.hash))
    })
}

/// Render the full program to stereo: `out_l` / `out_r` are caller buffers
/// of `capacity` frames each. Returns the frames written, or -1 when
/// `capacity` is smaller than the program needs (query
/// `tono_program_frames` first; last-error repeats the number).
///
/// # Safety
///
/// `out_l` / `out_r` must each point to `capacity` writable `f32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tono_program_render(
    program: *const ProgramHandle,
    out_l: *mut f32,
    out_r: *mut f32,
    capacity: usize,
) -> i64 {
    entry(-1, || {
        let program = program_ref(program)?;
        if out_l.is_null() || out_r.is_null() {
            return Err("NULL output buffer".into());
        }
        let (left, right) = program.program.render_stereo();
        let frames = left.len();
        if capacity < frames {
            return Err(format!(
                "buffer capacity {capacity} frames < the program's {frames} frames — \
                 call tono_program_frames for the needed capacity"
            ));
        }
        // SAFETY: the caller's contract — `capacity` (≥ frames) writable
        // floats at each pointer.
        unsafe {
            ptr::copy_nonoverlapping(left.as_ptr(), out_l, frames);
            ptr::copy_nonoverlapping(right.as_ptr(), out_r, frames);
        }
        Ok(frames as i64)
    })
}

/// The program's length in frames — the buffer capacity
/// `tono_program_render` needs. 0 on error.
///
/// # Safety
///
/// `program` must be NULL or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tono_program_frames(program: *const ProgramHandle) -> u64 {
    entry(0, || Ok(program_ref(program)?.program.estimates.frames))
}

/// Whether the program streams natively through the real-time renderer
/// (byte-identical to the offline bounce): 1 yes, 0 no or on error. Either
/// way a `Performance` plays it — non-streamable programs play their
/// pre-rendered bounce.
///
/// # Safety
///
/// `program` must be NULL or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tono_program_is_streamable(program: *const ProgramHandle) -> i32 {
    entry(0, || {
        Ok(i32::from(program_ref(program)?.program.is_streamable()))
    })
}

/// Start a performance of a program, stopped at frame 0. Takes a clone of
/// the program's internal `Arc` — the caller keeps ownership of the program
/// handle and must still free it. Returns NULL on error. Building the
/// playback source renders the bounce up front for non-streamable programs.
///
/// # Safety
///
/// `program` must be NULL or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tono_performance_new(
    program: *mut ProgramHandle,
) -> *mut PerformanceHandle {
    entry(ptr::null_mut(), || {
        let program = program_ref(program)?;
        Ok(Box::into_raw(Box::new(PerformanceHandle {
            performance: Performance::new(Arc::clone(&program.program)),
        })))
    })
}

/// Free a performance handle. NULL is a no-op.
///
/// # Safety
///
/// `performance` must be NULL or a live handle from `tono_performance_new`,
/// freed exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tono_performance_free(performance: *mut PerformanceHandle) {
    entry((), || {
        if !performance.is_null() {
            // SAFETY: the caller's contract — a live handle, freed once.
            drop(unsafe { Box::from_raw(performance) });
        }
        Ok(())
    })
}

/// Schedule a command (see the module docs for the two grammars), e.g.
/// `tono_performance_schedule_json(p, "{\"play\":true}", "{\"next_bar\":true}")`.
/// Returns the scheduled sequence id (> 0), or -1 on error — off-grammar
/// JSON (last-error quotes the accepted grammar), an unknown marker/section,
/// or a full queue.
///
/// # Safety
///
/// `performance` must be NULL or a live handle; `command_json` / `at_json`
/// must be NULL or NUL-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tono_performance_schedule_json(
    performance: *mut PerformanceHandle,
    command_json: *const c_char,
    at_json: *const c_char,
) -> i64 {
    entry(-1, || {
        let performance = performance_mut(performance)?;
        let command = parse_command(c_str(command_json, "command_json")?).map_err(|why| {
            format!("bad command JSON: {why} — accepted grammar: {COMMAND_GRAMMAR}")
        })?;
        let at = parse_at(c_str(at_json, "at_json")?)
            .map_err(|why| format!("bad at JSON: {why} — accepted grammar: {AT_GRAMMAR}"))?;
        performance
            .schedule(command, at)
            .map(|seq| seq as i64)
            .map_err(|e| e.to_string())
    })
}

/// Render `frames` frames of stereo-interleaved audio into `out` (`frames *
/// 2` floats), executing due scheduled commands at their exact frames.
/// Returns the frames written (always `frames` on success), 0 on error.
///
/// # Safety
///
/// `out` must point to `frames * 2` writable `f32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tono_performance_fill(
    performance: *mut PerformanceHandle,
    out: *mut f32,
    frames: usize,
) -> usize {
    entry(0, || {
        let performance = performance_mut(performance)?;
        if out.is_null() {
            return Err("NULL output buffer".into());
        }
        if frames == 0 {
            return Ok(0);
        }
        let len = frames.checked_mul(2).ok_or("frames too large")?;
        // SAFETY: the caller's contract — `frames * 2` writable floats.
        let out = unsafe { std::slice::from_raw_parts_mut(out, len) };
        Ok(performance.fill(out))
    })
}

/// A point-in-time metrics snapshot as owned JSON — `{"frames_rendered":…,
/// "commands_executed":…, "commands_dropped":…, "queue_depth_max":…,
/// "swaps":…, "stingers_fired":…}` (free with `tono_free_string`), or NULL
/// on error.
///
/// # Safety
///
/// `performance` must be NULL or a live handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tono_performance_metrics_json(
    performance: *const PerformanceHandle,
) -> *mut c_char {
    entry(ptr::null_mut(), || {
        let metrics = performance_ref(performance)?.metrics();
        let json = serde_json::json!({
            "frames_rendered": metrics.frames_rendered,
            "commands_executed": metrics.commands_executed,
            "commands_dropped": metrics.commands_dropped,
            "queue_depth_max": metrics.queue_depth_max,
            "swaps": metrics.swaps,
            "stingers_fired": metrics.stingers_fired,
        });
        into_c_string(json.to_string())
    })
}

/// Parse a single-key JSON object into `(key, value)`.
fn single_key_object(json: &str) -> Result<(String, serde_json::Value), String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("not JSON: {e}"))?;
    let object = value.as_object().ok_or("expected a JSON object")?;
    if object.len() != 1 {
        return Err(format!(
            "expected a single-key object, got {} keys",
            object.len()
        ));
    }
    let (key, value) = object.iter().next().expect("len checked");
    Ok((key.clone(), value.clone()))
}

/// A flag value must be exactly `true` — `{"play":false}` is off-grammar,
/// never a silent no-op.
fn want_flag(value: &serde_json::Value) -> Result<(), String> {
    if *value == serde_json::Value::Bool(true) {
        Ok(())
    } else {
        Err("expected true".into())
    }
}

fn want_u32(value: &serde_json::Value, what: &str) -> Result<u32, String> {
    let n = value
        .as_u64()
        .ok_or_else(|| format!("{what}: expected a non-negative integer"))?;
    u32::try_from(n).map_err(|_| format!("{what}: too large"))
}

fn want_f64(value: &serde_json::Value, what: &str) -> Result<f64, String> {
    value
        .as_f64()
        .ok_or_else(|| format!("{what}: expected a number"))
}

fn want_string<'v>(value: &'v serde_json::Value, what: &str) -> Result<&'v str, String> {
    value
        .as_str()
        .ok_or_else(|| format!("{what}: expected a string"))
}

/// The command side of the scheduling grammar (see the module docs).
fn parse_command(json: &str) -> Result<Command, String> {
    let (key, value) = single_key_object(json)?;
    match key.as_str() {
        "play" => want_flag(&value).map(|()| Command::Play),
        "pause" => want_flag(&value).map(|()| Command::Pause),
        "stop" => want_flag(&value).map(|()| Command::Stop),
        "seek_bar" => want_u32(&value, "seek_bar").map(Command::SeekBar),
        "seek_beat" => want_f64(&value, "seek_beat").map(Command::SeekBeat),
        "seek_section" => {
            want_string(&value, "seek_section").map(|name| Command::SeekSection(name.to_string()))
        }
        "set_loop_bars" => {
            let pair = value
                .as_array()
                .ok_or("set_loop_bars: expected [start, end]")?;
            if pair.len() != 2 {
                return Err(format!(
                    "set_loop_bars: expected 2 elements, got {}",
                    pair.len()
                ));
            }
            let start = want_u32(&pair[0], "set_loop_bars[0]")?;
            let end = want_u32(&pair[1], "set_loop_bars[1]")?;
            Ok(Command::SetLoopBars(start, end))
        }
        "clear_loop" => want_flag(&value).map(|()| Command::ClearLoop),
        "set_gain" => {
            let gain = want_f64(&value, "set_gain")? as f32;
            if !gain.is_finite() {
                return Err("set_gain: out of range".into());
            }
            Ok(Command::SetGain(gain))
        }
        _ => Err(format!("unknown command key '{key}'")),
    }
}

/// The position side of the scheduling grammar (see the module docs).
fn parse_at(json: &str) -> Result<At, String> {
    let (key, value) = single_key_object(json)?;
    match key.as_str() {
        "immediate" => want_flag(&value).map(|()| At::Immediate),
        "next_bar" => want_flag(&value).map(|()| At::NextBar),
        "next_beat" => want_flag(&value).map(|()| At::NextBeat),
        "frame" => value
            .as_u64()
            .ok_or_else(|| "frame: expected a non-negative integer".to_string())
            .map(At::Frame),
        "beat" => want_f64(&value, "beat").map(At::Beat),
        "bar" => want_u32(&value, "bar").map(At::Bar),
        "marker" => want_string(&value, "marker").map(|name| At::Marker(name.to_string())),
        "section" => want_string(&value, "section").map(|name| At::Section(name.to_string())),
        _ => Err(format!("unknown at key '{key}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tono_core::dsl::{Adsr, SeqWave};
    use tono_core::song::{CompileOptions, Song, note};

    #[test]
    fn command_grammar_round_trips() {
        assert!(matches!(
            parse_command(r#"{"play":true}"#),
            Ok(Command::Play)
        ));
        assert!(matches!(
            parse_command(r#"{"pause":true}"#),
            Ok(Command::Pause)
        ));
        assert!(matches!(
            parse_command(r#"{"stop":true}"#),
            Ok(Command::Stop)
        ));
        assert!(matches!(
            parse_command(r#"{"seek_bar":3}"#),
            Ok(Command::SeekBar(3))
        ));
        assert!(matches!(
            parse_command(r#"{"seek_beat":8.5}"#),
            Ok(Command::SeekBeat(b)) if (b - 8.5).abs() < 1e-9
        ));
        assert!(matches!(
            parse_command(r#"{"seek_section":"chorus"}"#),
            Ok(Command::SeekSection(name)) if name == "chorus"
        ));
        assert!(matches!(
            parse_command(r#"{"set_loop_bars":[1,4]}"#),
            Ok(Command::SetLoopBars(1, 4))
        ));
        assert!(matches!(
            parse_command(r#"{"clear_loop":true}"#),
            Ok(Command::ClearLoop)
        ));
        assert!(matches!(
            parse_command(r#"{"set_gain":0.8}"#),
            Ok(Command::SetGain(g)) if (g - 0.8).abs() < 1e-6
        ));
    }

    #[test]
    fn command_grammar_rejects_off_grammar_json() {
        for bad in [
            "not json",
            "{}",
            r#"[]"#,
            r#"{"play":true,"pause":true}"#,
            r#"{"play":false}"#,
            r#"{"seek_bar":-1}"#,
            r#"{"seek_bar":2.5}"#,
            r#"{"seek_section":4}"#,
            r#"{"set_loop_bars":[1]}"#,
            r#"{"set_gain":"loud"}"#,
            r#"{"unknown":true}"#,
        ] {
            assert!(parse_command(bad).is_err(), "{bad} should fail");
        }
    }

    #[test]
    fn at_grammar_round_trips() {
        assert_eq!(parse_at(r#"{"immediate":true}"#), Ok(At::Immediate));
        assert_eq!(parse_at(r#"{"next_bar":true}"#), Ok(At::NextBar));
        assert_eq!(parse_at(r#"{"next_beat":true}"#), Ok(At::NextBeat));
        assert_eq!(parse_at(r#"{"frame":96000}"#), Ok(At::Frame(96_000)));
        assert_eq!(parse_at(r#"{"beat":4.0}"#), Ok(At::Beat(4.0)));
        assert_eq!(parse_at(r#"{"bar":2}"#), Ok(At::Bar(2)));
        assert_eq!(
            parse_at(r#"{"marker":"drop"}"#),
            Ok(At::Marker("drop".into()))
        );
        assert_eq!(
            parse_at(r#"{"section":"chorus"}"#),
            Ok(At::Section("chorus".into()))
        );
    }

    #[test]
    fn at_grammar_rejects_off_grammar_json() {
        for bad in [
            "{}",
            r#"{"immediate":1}"#,
            r#"{"frame":-1}"#,
            r#"{"bar":"2"}"#,
            r#"{"marker":4}"#,
            r#"{"bogus":true}"#,
        ] {
            assert!(parse_at(bad).is_err(), "{bad} should fail");
        }
    }

    /// A small compiled program, as JSON — the same shape `make capi`
    /// generates for the C smoke test via the emit_program example.
    fn demo_program_json() -> String {
        let mut song = Song::new("capi-test", 120.0);
        song.add_track(
            "bass",
            SeqWave::Bass,
            Adsr {
                a: 0.005,
                d: 0.1,
                s: 0.8,
                r: 0.2,
                punch: 0.0,
            },
        );
        song.add_pattern("riff", 1, vec![note(0, 4, "C2"), note(8, 4, "G2")]);
        song.arrange("bass", "riff", 0);
        song.compile(&CompileOptions::default())
            .expect("compiles")
            .to_json()
    }

    #[test]
    fn null_inputs_set_last_error_and_never_deref() {
        unsafe {
            assert_eq!(tono_doc_validate(ptr::null()), 0);
            let err = CStr::from_ptr(tono_last_error()).to_str().unwrap();
            assert!(!err.is_empty(), "a failure sets last-error");
            assert!(tono_program_from_json(ptr::null()).is_null());
            assert!(tono_program_hash_hex(ptr::null()).is_null());
            assert_eq!(
                tono_program_render(ptr::null(), ptr::null_mut(), ptr::null_mut(), 0),
                -1
            );
            assert_eq!(tono_program_frames(ptr::null()), 0);
            assert_eq!(tono_program_is_streamable(ptr::null()), 0);
            assert!(tono_performance_new(ptr::null_mut()).is_null());
            assert_eq!(
                tono_performance_schedule_json(ptr::null_mut(), ptr::null(), ptr::null()),
                -1
            );
            assert_eq!(
                tono_performance_fill(ptr::null_mut(), ptr::null_mut(), 64),
                0
            );
            assert!(tono_performance_metrics_json(ptr::null()).is_null());
            // Frees tolerate NULL (no-op, like free(3)).
            tono_program_free(ptr::null_mut());
            tono_performance_free(ptr::null_mut());
            tono_free_string(ptr::null_mut());
        }
    }

    #[test]
    fn end_to_end_over_the_abi() {
        let json = CString::new(demo_program_json()).unwrap();
        unsafe {
            let program = tono_program_from_json(json.as_ptr());
            assert!(
                !program.is_null(),
                "{}",
                CStr::from_ptr(tono_last_error()).to_string_lossy()
            );
            let frames = tono_program_frames(program);
            assert!(frames > 0);
            let hash = tono_program_hash_hex(program);
            assert!(!hash.is_null());
            assert!(CStr::from_ptr(hash).to_str().unwrap().starts_with("0x"));
            tono_free_string(hash);
            let mut left = vec![0.0f32; frames as usize];
            let mut right = vec![0.0f32; frames as usize];
            // Too small a capacity is -1; exactly right renders.
            assert_eq!(
                tono_program_render(
                    program,
                    left.as_mut_ptr(),
                    right.as_mut_ptr(),
                    (frames - 1) as usize
                ),
                -1
            );
            assert_eq!(
                tono_program_render(
                    program,
                    left.as_mut_ptr(),
                    right.as_mut_ptr(),
                    frames as usize
                ),
                frames as i64
            );
            assert!(left.iter().any(|s| *s != 0.0), "the render sounds");
            let performance = tono_performance_new(program);
            assert!(!performance.is_null());
            // The program handle is still ours (Arc-cloned, not moved).
            assert!(tono_program_frames(program) == frames);
            let play = CString::new(r#"{"play":true}"#).unwrap();
            let now = CString::new(r#"{"immediate":true}"#).unwrap();
            let seq = tono_performance_schedule_json(performance, play.as_ptr(), now.as_ptr());
            assert!(
                seq > 0,
                "{}",
                CStr::from_ptr(tono_last_error()).to_string_lossy()
            );
            let mut out = vec![0.0f32; 512 * 2];
            assert_eq!(
                tono_performance_fill(performance, out.as_mut_ptr(), 512),
                512
            );
            let metrics = tono_performance_metrics_json(performance);
            assert!(!metrics.is_null());
            let metrics = CStr::from_ptr(metrics).to_str().unwrap();
            assert!(metrics.contains(r#""frames_rendered":512"#), "{metrics}");
            assert!(metrics.contains(r#""commands_executed":1"#), "{metrics}");
            tono_performance_free(performance);
            tono_program_free(program);
        }
    }
}
