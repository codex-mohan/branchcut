---
title: Parallel traversal
description: Bounded workers, streaming delivery, and cancellation.
icon: Workflow
---

The library's `visit` method uses sequential streaming. The CLI chooses adaptive bounded parallelism for ordinary queries:

```bash
branchcut --glob '**/*.rs'
```

`--threads 1` forces sequential traversal. `--threads 0` requests available parallelism capped at 16 workers; the default adapts to the query.

## Worker model

```text
                 shared outstanding-task count
                            │
       ┌────────────────────┼────────────────────┐
       ▼                    ▼                    ▼
   worker 0             worker 1             worker 2
 local deque            local deque           local deque
       │                    │                    │
       └──────────── dynamic work stealing ──────┘
```

Each worker owns reusable traversal state and a bounded local queue. Idle workers steal available tasks. A condition variable sleeps workers instead of spinning, while atomics communicate cancellation.

## Bounded streaming output

Workers send results through a bounded channel to one coordinator, which invokes the visitor and writes output. The channel holds one event per worker, and path events are batched in groups of at most 32; a slow consumer applies backpressure. Cancelling or returning an error drops the receiver before joining workers, releasing blocked senders. Callbacks run on the coordinator, so the visitor need not implement `Send`. Sorting still collects results unless a sorted limit bounds the selection.

The public `visit_parallel(threads, strict, visitor)` API streams through one coordinator. `collect_parallel` remains an explicitly collected API for consumers that want all native paths in memory. Results are unordered in parallel mode. If the bounded work queue is full, the current worker processes that directory on its own iterative depth-first frame stack; it does not recurse through the call stack.

## Limits and command execution

When `--first`, `--limit`, or per-match `--exec` is used without an explicit worker count, the CLI selects one worker. An explicitly requested worker count greater than one is honored for limits; the first matches delivered are not a stable or globally sorted prefix, and workers may already have inspected additional entries when delivery stops. Use `--sort --limit N` when you need the globally first N paths; this scans the full selection.

Per-match `--exec` uses one job by default. Combining it with `--threads N` greater than one requires explicit `--exec-jobs N`. Values above one run child commands concurrently, so their side-effect and completion order is unspecified. With one job, commands run serially but follow the unordered match-delivery order. `--exec-batch` has its own bounded batch behavior; unordered parallel traversal does not promise stable batch composition.

## When to use it

Parallel traversal helps wide trees and slower storage. Narrow fixed-prefix queries may have too little work to amortize worker coordination. The CLI default favors parallel traversal for ordinary queries and falls back to sequential mode for policies that require it, including symlink following, one-filesystem traversal, contents-first directory output, and `--max-open` below 64.
