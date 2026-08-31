---
title: Query planning
description: Read --explain output and connect it to filesystem work avoided.
icon: Route
---

`--explain` compiles the query and prints its plan without traversing the filesystem.

```bash
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**' \
  --limit 100 \
  --explain
```

## Plan anatomy

```text
QUERY PLAN

ROOT
  ./packages

SHARED LITERAL PREFIX
  packages

POSITIVE PATTERNS
  packages/**/src/**/*.rs [FixedPrefixRecursive]
  packages/**/src/**/*.ts [FixedPrefixRecursive]

METADATA
  not required

TERMINATION
  first 100 matches
```

### Root

The physical directory where traversal begins. A narrow root is often the largest single source of avoided work.

### Shared literal prefix

The common literal components found before patterns diverge. This is both a diagnostic and evidence that the compiler understood shared structure.

### Pattern classification

Patterns are classified as `Literal`, `SingleDirectory`, `FixedPrefixRecursive`, or `UnboundedRecursive`. Classification selects execution behavior; it is not decorative metadata.

### Leaf filters

The requested file type and extension set. Specialized suffix matching handles common extension filters without invoking the general wildcard matcher.

### Metadata

Current supported filters do not require per-entry `metadata()` calls. The root is inspected once; directory entries use `DirEntry::file_type()`.

### Termination

`--first` and `--limit` become traversal stop conditions in sequential mode. With `--sort`, the full result set must be collected before the limit is applied.

### Strategy

Sequential mode uses depth-first traversal with positive and exclusion pruning. `--threads N` selects bounded parallel root-task traversal with buffered output.

## Prove the plan with stats

Use `--explain` to see intended decisions and `--stats` to see what actually happened:

```bash
branchcut --glob '**/*.rs' --exclude '**/target/**' --stats
```
