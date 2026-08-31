---
title: Globs and exclusions
description: Compose positive patterns and prune excluded subtrees.
icon: GitFork
---

## Positive patterns

Repeat `--glob` to compile several positive patterns into one shared program:

```bash
branchcut \
  --glob 'src/**/*.rs' \
  --glob 'tests/**/*.rs'
```

Overlapping positives do not produce duplicate output. Common segments are represented once in the pattern program.

```text
src/
└── **
    ├── *.rs
    ├── *.toml
    └── test*.rs
```

## Exclusions

Use `--exclude`; a leading `!` is not required:

```bash
branchcut \
  --glob '**/*.{rs,ts}' \
  --exclude '**/target/**' \
  --exclude '**/node_modules/**'
```

Exclusions ending in a complete subtree globstar can prune the directory before it is opened.

```text
repository/
├── src/ ─────────────────────▶ inspect
├── tests/ ───────────────────▶ inspect
├── target/ ──────────────────╳ prune
└── node_modules/ ────────────╳ prune
```

Negative syntax passed to `--glob` is not interpreted as exclusion syntax. Always use `--exclude`.

## Brace alternatives

One flat, non-nested brace group is supported:

```bash
branchcut --glob '**/*.{rs,toml}'
branchcut --exclude '**/{target,dist}/**'
```

Nested braces, brace ranges, and multiple brace groups are deliberately outside the current compatibility surface.

## Narrow roots

A literal prefix changes where traversal starts:

```bash
branchcut --glob 'packages/core/src/**/*.rs'
```

The planner starts at `packages/core/src` instead of repository root. Use `--explain` to verify the selected root.
