---
title: Plan a monorepo search
description: Compose one query that narrows its root, prunes generated trees, and proves the work it avoided.
icon: Network
---

This tutorial builds a query useful in a JavaScript, Rust, or mixed monorepo. The counts and timings shown below are illustrative; your tree supplies the real values.

## Start with the desired files

Suppose the useful files live under package `src` directories and you want Rust and TypeScript only:

```bash
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --type file
```

Without planning, this can still scan unrelated package trees. Add the generated directories that should never contribute:

```bash
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/target/**' \
  --exclude '**/node_modules/**' \
  --exclude '**/dist/**' \
  --type file
```

## See the traversal strategy

```bash
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**' \
  --type file \
  --limit 100 \
  --explain
```

A representative plan looks like this:

```text
QUERY PLAN

ROOT
  ./packages

SHARED LITERAL PREFIX
  packages

POSITIVE PATTERNS
  packages/**/src/**/*.rs
  packages/**/src/**/*.ts

EXCLUSIONS
  **/target/**
  **/node_modules/**
  **/dist/**

METADATA
  not required

TERMINATION
  first 100 matches
```

The important result is the chosen root and prune rules, not the illustrative formatting.

## Prove the filesystem work

```bash
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**' \
  --type file \
  --limit 100 \
  --stats
```

Compare `directories opened`, `directories pruned`, `entries inspected`, and `metadata calls` with a broader query. A metadata-free name query should report no per-entry metadata work even though root setup and ignore validation may use metadata.

## Stop after the useful result set

```bash
branchcut \
  --glob 'packages/**/src/**/*.rs' \
  --exclude '**/target/**' \
  --limit 20 \
  --stats
```

Without an explicit `--threads` value, this query uses sequential traversal and stops after 20 delivered matches. If you explicitly request multiple workers, results are unordered and workers may inspect additional entries before the limit is delivered. A plain `| head` only stops the consumer; use Branchcut's own limit to ask the engine to stop.

## Save the query

Create `.branchcut-queries` at the repository root:

```text
[src]
glob packages/**/src/**/*.{rs,ts}
exclude **/target/**
exclude **/node_modules/**
exclude **/dist/**
type file
```

Run the profile from any directory:

```bash
branchcut --config path/to/.branchcut-queries --profile src --explain
branchcut --config path/to/.branchcut-queries --profile src
```

A profile is independent and does not inherit a `default` section. Command-line scalar options override profile values, while repeatable query options append. See [Saved project queries](../guides/project-queries.md) for every accepted line.

Keep generated query files under review like any other executable project configuration.

## Next

- Add size and time filters in [Everyday search](../guides/everyday-search.md).
- Explain a specific excluded file with [Query planning](../guides/query-planning.md).
- Verify the result set before making performance claims with [Correctness](../evidence/correctness.md).
