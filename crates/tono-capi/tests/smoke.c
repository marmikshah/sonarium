/*
 * smoke.c — the tono-capi smoke test: exercises every ABI function end to
 * end against a real compiled Program. Usage:
 *
 *   smoke path/to/program.json
 *
 * `make capi` generates the fixture with the emit_program example and runs
 * this against the freshly built staticlib.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "capi.h"

static void check(int ok, const char *what) {
    if (!ok) {
        fprintf(stderr, "FAIL: %s — last_error: %s\n", what, tono_last_error());
        exit(1);
    }
    printf("ok: %s\n", what);
}

static char *read_file(const char *path) {
    FILE *f = fopen(path, "rb");
    long size;
    char *buf;
    if (!f) {
        fprintf(stderr, "cannot open %s\n", path);
        exit(2);
    }
    fseek(f, 0, SEEK_END);
    size = ftell(f);
    fseek(f, 0, SEEK_SET);
    buf = malloc((size_t)size + 1);
    if (!buf || fread(buf, 1, (size_t)size, f) != (size_t)size) {
        fprintf(stderr, "cannot read %s\n", path);
        exit(2);
    }
    buf[size] = '\0';
    fclose(f);
    return buf;
}

int main(int argc, char **argv) {
    static const char *GOOD_DOC =
        "{\"name\":\"blip\",\"duration\":0.2,\"root\":{\"type\":\"mul\",\"inputs\":["
        "{\"type\":\"sawtooth\",\"freq\":880},"
        "{\"type\":\"env\",\"a\":0.0,\"d\":0.05,\"s\":0.0,\"r\":0.01}]}}";
    static const char *BAD_DOC = "{\"name\":\"x\",\"root\":{\"type\":\"nope\"}}";

    TonoProgram *program;
    TonoPerformance *performance;
    char *program_json;
    char *hash;
    char *metrics;
    float *left, *right, *interleaved;
    uint64_t frames;
    int64_t seq;
    uint64_t i;
    int sounded = 0;

    if (argc != 2) {
        fprintf(stderr, "usage: %s path/to/program.json\n", argv[0]);
        return 2;
    }

    /* Errors: empty at first, set by a failure, cleared by a success. */
    check(tono_last_error() != NULL && tono_last_error()[0] == '\0', "last_error starts empty");
    check(tono_doc_validate(NULL) == 0, "validate(NULL) is 0");
    check(tono_last_error()[0] != '\0', "a failure sets last_error");

    /* Documents. */
    check(tono_doc_validate(GOOD_DOC) == 1, "a good document validates");
    check(tono_last_error()[0] == '\0', "a success clears last_error");
    check(tono_doc_validate(BAD_DOC) == 0, "a bad document is rejected");
    check(strstr(tono_last_error(), "invalid document") != NULL, "the validation error is named");

    /* Programs. */
    check(tono_program_from_json(NULL) == NULL, "from_json(NULL) is NULL");
    check(tono_program_from_json("{}") == NULL, "from_json(\"{}\") is NULL");
    check(tono_program_from_json(BAD_DOC) == NULL, "a SoundDoc is not a Program bundle");
    program_json = read_file(argv[1]);
    program = tono_program_from_json(program_json);
    check(program != NULL, "a compiled Program bundle loads");

    hash = tono_program_hash_hex(program);
    check(hash != NULL && strncmp(hash, "0x", 2) == 0 && strlen(hash) == 18,
          "the content hash is an owned 0x-prefixed hex string");
    tono_free_string(hash);

    frames = tono_program_frames(program);
    check(frames > 0, "the program has frames");
    printf("ok: is_streamable = %d\n", tono_program_is_streamable(program));

    /* Offline render: too-small capacity is -1; exact capacity renders. */
    left = malloc(sizeof(float) * frames);
    right = malloc(sizeof(float) * frames);
    check(left && right, "host buffers allocated");
    check(tono_program_render(program, left, right, frames - 1) == -1,
          "a too-small render buffer is -1");
    check(strstr(tono_last_error(), "capacity") != NULL, "the capacity error names the fix");
    check(tono_program_render(program, NULL, right, frames) == -1, "render(NULL, …) is -1");
    check(tono_program_render(program, left, right, frames) == (int64_t)frames,
          "the program renders to stereo");
    for (i = 0; i < frames; i++) {
        if (left[i] != 0.0f) {
            sounded = 1;
            break;
        }
    }
    check(sounded, "the render is not silence");

    /* Performances. */
    check(tono_performance_new(NULL) == NULL, "performance_new(NULL) is NULL");
    performance = tono_performance_new(program);
    check(performance != NULL, "a performance starts");
    check(tono_program_frames(program) == frames,
          "the program handle is still owned by the caller (Arc-cloned, not moved)");

    /* Scheduling: off-grammar JSON names the grammar; on-grammar schedules. */
    check(tono_performance_schedule_json(performance, "{\"bogus\":true}", "{\"immediate\":true}") == -1,
          "an unknown command is -1");
    check(strstr(tono_last_error(), "accepted grammar") != NULL,
          "the grammar error quotes the accepted grammar");
    check(tono_performance_schedule_json(performance, "{\"play\":true}", NULL) == -1,
          "a NULL at is -1");
    seq = tono_performance_schedule_json(performance, "{\"play\":true}", "{\"immediate\":true}");
    check(seq > 0, "play @ immediate schedules");
    check(tono_performance_schedule_json(performance, "{\"set_gain\":0.9}", "{\"next_bar\":true}") > seq,
          "set_gain @ next_bar schedules with a later seq");

    /* Rendering audio through the performance. */
    interleaved = calloc(512 * 2, sizeof(float));
    check(interleaved != NULL, "interleaved buffer allocated");
    check(tono_performance_fill(performance, NULL, 512) == 0, "fill(NULL) is 0");
    check(tono_performance_fill(performance, interleaved, 512) == 512, "512 frames render");

    /* Metrics: the JSON parses (object with the documented keys) and the
     * fill above is visible in it. */
    metrics = tono_performance_metrics_json(performance);
    check(metrics != NULL, "metrics JSON is returned");
    check(metrics[0] == '{' && strstr(metrics, "\"frames_rendered\":512") != NULL &&
              strstr(metrics, "\"commands_executed\":1") != NULL &&
              strstr(metrics, "\"queue_depth_max\":") != NULL &&
              strstr(metrics, "\"swaps\":") != NULL &&
              strstr(metrics, "\"stingers_fired\":") != NULL,
          "metrics JSON parses with the expected counters");
    printf("ok: metrics = %s\n", metrics);
    tono_free_string(metrics);

    /* Teardown: frees consume the handles; NULL frees are no-ops. */
    tono_performance_free(performance);
    tono_program_free(program);
    tono_performance_free(NULL);
    tono_program_free(NULL);
    tono_free_string(NULL);

    free(interleaved);
    free(left);
    free(right);
    free(program_json);
    printf("PASS: tono-capi smoke test\n");
    return 0;
}
