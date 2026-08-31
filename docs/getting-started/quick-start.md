---
title: Quick start
description: Build Branchcut and run your first planned filesystem query.
icon: Zap
---

## Build the release binary

```bash
cargo build --release
```

On Unix, the binary is written to `target/release/branchcut`. On Windows it is written to `target/release/branchcut.exe`.

## Find Rust files

```bash
branchcut --glob '**/*.rs'
```

## Exclude generated trees

```bash
branchcut \
  --glob '**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**'
```

## Inspect the plan

Add `--explain` to print compiler decisions without traversing:

```bash
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**' \
  --limit 100 \
  --explain
```

## Measure the traversal

Add `--stats` to write real counters to standard error:

```bash
branchcut --glob '**/*.rs' --exclude '**/target/**' --stats
```

Branchcut reports matches, directories considered and opened, pruning decisions, entries inspected, candidate files, metadata calls, filesystem errors, and elapsed time.
