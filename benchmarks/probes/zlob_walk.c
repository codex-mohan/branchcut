/* Branchcut development probe for zlob 1.7.0's public native walker API.
 * No zlob implementation is included here. Link the pinned official library.
 * Usage: zlob-walk ROOT PATTERN THREADS [paths|count|collect-count]
 * PATTERN '-' disables glob filtering. Default mode is streaming count.
 */
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdatomic.h>
#include <stdlib.h>
#include <string.h>
#include "zlob.h"
#ifdef _WIN32
#include <fcntl.h>
#include <io.h>
#endif

struct counter_shard {
    uint64_t value;
    unsigned char padding[120];
};

struct counters {
    struct counter_shard *workers;
    size_t worker_count;
    _Atomic int invalid_worker;
};

static int count_entry(const zlob_walk_entry_t *entry, void *context) {
    struct counters *counts = (struct counters *)context;
    if ((size_t)entry->worker_id >= counts->worker_count) {
        /* The public API guarantees this cannot occur. Stop on a violation. */
        atomic_store_explicit(&counts->invalid_worker, 1, memory_order_relaxed);
        return 2;
    }
    /* Each worker_id is used by exactly one worker at a time. */
    if (entry->kind == ZLOB_WALK_KIND_FILE)
        counts->workers[entry->worker_id].value++;
    return 0;
}

static int report_error(const char *path, int reason) {
    fprintf(stderr, "zlob-walk: cannot read %s (error %d)\n", path, reason);
    return 1;
}

int main(int argc, char **argv) {
    if (argc < 4 || argc > 5) {
        fprintf(stderr, "usage: zlob-walk ROOT PATTERN THREADS [paths|count|collect-count]\n");
        return 2;
    }
    char *end = NULL;
    errno = 0;
    unsigned long requested = strtoul(argv[3], &end, 10);
    if (errno || !argv[3][0] || *end || requested > 1024) {
        fprintf(stderr, "zlob-walk: threads must be 0 through 1024\n");
        return 2;
    }
    const char *mode = argc == 5 ? argv[4] : "count";
    int paths = strcmp(mode, "paths") == 0;
    int collect = paths || strcmp(mode, "collect-count") == 0;
    if (!collect && strcmp(mode, "count") != 0) {
        fprintf(stderr, "zlob-walk: unknown output mode\n");
        return 2;
    }
#ifdef _WIN32
    if (_setmode(_fileno(stdout), _O_BINARY) == -1) {
        perror("zlob-walk: stdout mode");
        return 2;
    }
#endif
    zlob_walk_options_t options = {0};
    options.flags = ZLOB_WALK_SKIP_HIDDEN | ZLOB_WALK_NO_REPORT_DIRS |
                    ZLOB_WALK_ABORT_ON_ERROR;
    options.threads = (uint16_t)requested;
    options.pattern = strcmp(argv[2], "-") == 0 ? NULL : argv[2];
    options.errfunc = report_error;
    uint64_t matched = 0;
    if (collect) {
        zlob_walk_result_t result = {0};
        int status = zlob_walk_collect(argv[1], &options, &result);
        if (status != 0) {
            fprintf(stderr, "zlob-walk: collection failed (%d)\n", status);
            return 2;
        }
        int failed = 0;
        for (size_t i = 0; i < result.count; i++) {
            const zlob_walk_entry_t *entry = &result.entries[i];
            if (entry->kind != ZLOB_WALK_KIND_FILE) continue;
            matched++;
            if (paths) {
                size_t length = entry->path_len - entry->relative_offset;
                if (fwrite(entry->path + entry->relative_offset, 1, length, stdout) != length ||
                    fputc('\n', stdout) == EOF) {
                    failed = 1;
                    break;
                }
            }
        }
        zlob_walk_result_free(&result);
        if (failed) return 2;
    } else {
        struct counters counts = {0};
        counts.worker_count = zlob_walk_max_workers(&options);
        counts.workers = (struct counter_shard *)calloc(counts.worker_count, sizeof(struct counter_shard));
        if (!counts.workers) return 2;
        void *rules = NULL;
        int status = zlob_walk(argv[1], &options, count_entry, &counts, &rules);
        zlob_ignore_rules_free(rules);
        for (size_t i = 0; i < counts.worker_count; i++) matched += counts.workers[i].value;
        free(counts.workers);
        if (status != 0 || atomic_load_explicit(&counts.invalid_worker, memory_order_relaxed)) {
            fprintf(stderr, "zlob-walk: streaming walk failed (%d)\n", status);
            return 2;
        }
    }
    if (!paths && printf("%llu\n", (unsigned long long)matched) < 0) return 2;
    return fflush(stdout) == 0 ? 0 : 2;
}
