---
title: Parallel traversal
description: Bounded workers, buffered output, and the reasons some options remain sequential.
icon: Workflow
---

Sequential streaming is the default. Parallel traversal is opt-in:

```bash
branchcut --glob '**/*.rs' --threads 4
```

`--threads 0` selects the available parallelism, capped at 16 workers.

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

## Buffered output

Workers do not write directly to standard output. They return paths to a coordinator, which optionally sorts and serializes them. This avoids interleaved output but changes the memory profile relative to sequential streaming.

## Intentional incompatibilities

Parallel mode rejects:

- `--limit` and `--first`, because exact global early-stop ordering is not defined across workers;
- `--exec`, because side effects must not occur in nondeterministic worker order.

Use sequential mode for these workflows.

## When to use it

Parallel traversal can help wide trees and slower storage. Narrow fixed-prefix queries may already have too little work to amortize worker coordination. Measure representative workloads rather than assuming more threads are always faster.
