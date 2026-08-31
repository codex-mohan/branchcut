---
title: Benchmarking
description: Rules for fair, reproducible, and interpretable performance evidence.
icon: TimerReset
---

## Correctness gate

Never time a query until result sets agree for the claimed semantics.

```text
baseline − Branchcut = {}
Branchcut − baseline = {}
```

## Record the environment

Every published run must identify:

- Branchcut revision and build mode;
- competitor and version;
- Rust, Node, Zig, or other relevant runtime version;
- OS, CPU, RAM, and filesystem;
- dataset generator or public-repository revision;
- pattern and semantic options;
- warmups, sample count, output mode, and timing boundary.

## Separate hot and cold

Cold CLI measurements include startup, imports, traversal, and requested output. Hot engine measurements keep runtimes loaded and exclude startup. Never label one as the other.

## Match the work

- sort both sides or neither;
- serialize equivalent output or disable serialization on both;
- use the same hidden and symlink policy;
- consume returned arrays when a library would otherwise optimize work away;
- retain failures and losses.

## Report distributions

Prefer median, P90, P95, and dispersion over a single best sample. Use a geometric mean only across defensibly comparable workloads.
