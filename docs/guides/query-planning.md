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

Size and time filters request metadata only after type, path, extension, and glob checks pass; one fetch is shared by the requested predicates. Queries without those filters avoid per-entry metadata calls. A size filter requires regular-file selection. A time filter on a directory affects that directory's result only and never removes its descendants from traversal.

Case-insensitive matching disables literal-prefix root narrowing because directory names may differ in ASCII case on case-sensitive filesystems. `--explain` reports the disabled prefix. Depth is always relative to `--cwd`, even when traversal begins at a deeper literal-prefix root.

### Termination

`--first` and `--limit` become traversal stop conditions in sequential mode. With `--sort`, the full result set must be collected before the limit is applied.

### Content stage

For `branchcut grep`, the plan adds a content stage after path selection. Globs, exclusions, ignores, type, extension, size, and time filters decide which files reach the scanner, so an excluded subtree is pruned before content reads. The scanner reports selected lines in file order and stops at the requested limit. `--explain` shows the content patterns, case mode, binary policy, and output shape.

### Strategy

Sequential mode uses depth-first directory frames with positive and exclusion pruning. Ordinary CLI queries use adaptive bounded parallel traversal by default; limits, `--first`, and per-match execution default to one worker unless you explicitly request multiple workers. Symlink following, one-filesystem traversal, contents-first output, and a `--max-open` below 64 select sequential traversal. In parallel mode, a full work queue falls back to a worker's iterative local directory stack.

`--search-path` compiles an independent plan for each additional root. The underlying `MultiRootQuery` visitor API emits absolute native paths and lexically deduplicates them; the CLI strips the primary `--cwd` prefix when possible, while outside-root paths can remain absolute. `--absolute` always requests absolute CLI output. Deduplication retains a set proportional to distinct results. Symlink aliases are not canonicalized.

## Explain one candidate

`--why PATH` evaluates one path against the compiled query without walking unrelated directories:

```bash
branchcut \
  --glob 'packages/**/src/**/*.rs' \
  --exclude '**/target/**' \
  --gitignore \
  --why packages/core/src/lib.rs
```

The output starts with `MATCHED` or `EXCLUDED`, then reports hidden-path, enabled ignore-source, positive-pattern, exclusion, type, depth, extension, simple-search, and metadata checks. Each check is marked `PASS`, `FAIL`, or `SKIP`.

Ignore explanations retain provenance:

```text
  [FAIL] ignore
         excluded by `generated/` at .gitignore:1
```

The candidate path is interpreted relative to `--cwd`; an absolute path is accepted when it is below the textual query root. `--explain` and `--why` are separate diagnostics and cannot be combined. A candidate that is correctly excluded still exits successfully, because the command completed the explanation.

## Prove the plan with stats

Use `--explain` to see intended decisions and `--stats` to see what actually happened:

```bash
branchcut --glob '**/*.rs' --exclude '**/target/**' --stats
```

`metadata_calls` includes query-required metadata checks and explicit metadata requested by the CLI output or library `visit_entries` APIs. A path-only library visitor avoids metadata unless a query predicate requires it.
