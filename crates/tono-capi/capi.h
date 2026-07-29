/*
 * capi.h — the tono C ABI (crate `tono-capi`, issue #52).
 *
 * Native hosts drive tono through this surface: validate a SoundDoc JSON,
 * load a compiled Program bundle (what `tono compile` writes), render it
 * offline, and run it live through a Performance. Everything is an opaque
 * handle, a C string, or a plain return code — no Rust types cross the
 * boundary.
 *
 * Handles and ownership
 * ---------------------
 * - TonoProgram / TonoPerformance are opaque: never dereference, copy, or
 *   forge them.
 * - Every handle returned by this library is owned by the caller and
 *   released with the matching tono_*_free, exactly once. tono_*_free(NULL)
 *   is a no-op, like free(3). Double-free is UB, as in C.
 * - Strings returned by value (tono_program_hash_hex,
 *   tono_performance_metrics_json) are owned by the caller and released
 *   with tono_free_string. tono_last_error returns a BORROWED string,
 *   valid until the next tono_* call on the same thread — never free it.
 * - tono_performance_new clones the program's internal reference: the
 *   caller keeps ownership of the program handle and must still free it.
 *   Freeing a program while performances of it run is sound.
 * - Handles are not thread-safe: confine each handle to one thread at a
 *   time (different handles may live on different threads).
 *
 * Errors
 * ------
 * Every fallible call returns an error value (NULL, -1, or 0 — stated per
 * function) and sets a thread-local last-error string; read it with
 * tono_last_error(). A successful call leaves it empty. No panic crosses
 * the boundary: one is reported as last-error "internal panic: …" plus the
 * error value. NULL inputs yield the error value and are never
 * dereferenced.
 *
 * The command / at JSON grammars (tono_performance_schedule_json)
 * ---------------------------------------------------------------
 * Two single-key JSON objects.
 *
 * Command — exactly one of:
 *   {"play":true}  {"pause":true}  {"stop":true}  {"seek_bar":3}
 *   {"seek_beat":8.5}  {"seek_section":"chorus"}  {"set_loop_bars":[1,4]}
 *   {"clear_loop":true}  {"set_gain":0.8}
 *
 * At — exactly one of:
 *   {"immediate":true}  {"next_bar":true}  {"next_beat":true}
 *   {"frame":96000}  {"beat":4.0}  {"bar":2}  {"marker":"drop"}
 *   {"section":"chorus"}
 *
 * Anything else is rejected with the accepted grammar in the error message.
 *
 * The Rust-side handle types are ProgramHandle / PerformanceHandle; the
 * symbols below are the whole ABI.
 */
#ifndef TONO_CAPI_H
#define TONO_CAPI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque compiled-program handle (Rust: ProgramHandle). */
typedef struct TonoProgram TonoProgram;
/* Opaque running-program handle (Rust: PerformanceHandle). */
typedef struct TonoPerformance TonoPerformance;

/*
 * The last error on this thread, or an empty string when there is none.
 * Borrowed: valid until the next tono_* call on the same thread; never
 * NULL; never free it.
 */
const char *tono_last_error(void);

/*
 * Free a string tono returned ownership of (tono_program_hash_hex,
 * tono_performance_metrics_json). NULL is a no-op. Passing any other
 * pointer is UB, as with free(3).
 */
void tono_free_string(char *s);

/*
 * Validate a SoundDoc JSON document: 1 when it parses and passes
 * validation, 0 otherwise (tono_last_error names the problem).
 */
int tono_doc_validate(const char *json);

/*
 * Load a compiled Program bundle (the JSON `tono compile` writes).
 * Returns an owned handle, or NULL on error — malformed JSON, a bundle
 * newer than this binary (T3001), or a hash mismatch (T3002);
 * tono_last_error names which. Free with tono_program_free.
 */
TonoProgram *tono_program_from_json(const char *json);

/* Free a program handle. NULL is a no-op. */
void tono_program_free(TonoProgram *program);

/*
 * The program's canonical content hash as an owned "0x…" hex string (free
 * with tono_free_string), or NULL on error.
 */
char *tono_program_hash_hex(const TonoProgram *program);

/*
 * Render the full program to stereo: out_l / out_r are caller buffers of
 * `capacity` frames each. Returns the frames written, or -1 when capacity
 * is smaller than the program needs (query tono_program_frames first;
 * tono_last_error repeats the number).
 */
int64_t tono_program_render(const TonoProgram *program, float *out_l, float *out_r, size_t capacity);

/*
 * The program's length in frames — the buffer capacity
 * tono_program_render needs. 0 on error.
 */
uint64_t tono_program_frames(const TonoProgram *program);

/*
 * Whether the program streams natively through the real-time renderer
 * (byte-identical to the offline bounce): 1 yes, 0 no or on error. Either
 * way a Performance plays it — non-streamable programs play their
 * pre-rendered bounce.
 */
int tono_program_is_streamable(const TonoProgram *program);

/*
 * Start a performance of a program, stopped at frame 0. Clones the
 * program's internal reference — the caller keeps ownership of the
 * program handle and must still free it. Returns NULL on error. Building
 * the playback source renders the bounce up front for non-streamable
 * programs. Free with tono_performance_free.
 */
TonoPerformance *tono_performance_new(TonoProgram *program);

/* Free a performance handle. NULL is a no-op. */
void tono_performance_free(TonoPerformance *performance);

/*
 * Schedule a command (grammars above), e.g.
 *   tono_performance_schedule_json(p, "{\"play\":true}", "{\"next_bar\":true}");
 * Returns the scheduled sequence id (> 0), or -1 on error — off-grammar
 * JSON (tono_last_error quotes the accepted grammar), an unknown
 * marker/section, or a full queue.
 */
int64_t tono_performance_schedule_json(TonoPerformance *performance, const char *command_json, const char *at_json);

/*
 * Render `frames` frames of stereo-interleaved audio into `out`
 * (frames * 2 floats), executing due scheduled commands at their exact
 * frames. Returns the frames written (always `frames` on success), 0 on
 * error.
 */
size_t tono_performance_fill(TonoPerformance *performance, float *out, size_t frames);

/*
 * A point-in-time metrics snapshot as owned JSON — {"frames_rendered":…,
 * "commands_executed":…, "commands_dropped":…, "queue_depth_max":…,
 * "swaps":…, "stingers_fired":…} — free with tono_free_string. NULL on
 * error.
 */
char *tono_performance_metrics_json(const TonoPerformance *performance);

#ifdef __cplusplus
}
#endif

#endif /* TONO_CAPI_H */
