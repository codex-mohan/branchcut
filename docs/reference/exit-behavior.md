---
title: Exit behavior
description: Exit codes, filesystem errors, broken pipes, and child command failures.
icon: CircleAlert
---

## Exit codes

| Code | Meaning |
|---:|---|
| `0` | Successful filename query, a selected `grep` result, or a clean broken output pipe. |
| `1` | `grep` selected no result, or a quiet filename query found no result. |
| `2` | Invalid input, compilation failure, strict filesystem error, traversal failure, output failure, or unsuccessful child command. |

`branchcut grep` and `branchcut-regex grep` return 0 for a selected result, 1 for no result, and 2 for errors. With `--count` or `-c`, a zero result still returns 1. Ordinary filename queries succeed when the result is empty, while `--quiet` requests an existence status. Legacy flag-based content queries retain their previous exit-code behavior. Scan errors produce diagnostics and structured error records where applicable; `--strict` fails on any unreadable input.

## Best-effort filesystem behavior

By default, unreadable entries increment the filesystem error counter and traversal continues where possible.

```bash
branchcut --glob '**/*' --stats
```

Use `--strict` when incomplete traversal must fail:

```bash
branchcut --glob '**/*' --strict
```

## Invalid queries

Errors identify the invalid option or pattern and use the prefix `branchcut:`. Examples include:

- missing option values;
- `--limit 0`;
- an invalid `--type`;
- an invalid size, duration, UTC/RFC3339 timestamp, or Unix timestamp;
- conflicting case-mode or search-target options;
- `--size` combined with a non-file result type;
- `--min-depth` greater than `--max-depth`;
- unterminated character classes;
- unsupported brace structure;
- incompatible positional query modes;
- `--threads N` greater than one with per-match `--exec` unless `--exec-jobs` is also set.

`--one-file-system` requires Unix device identifiers. On Windows it fails during traversal with an unsupported-operation error and exits `2`.

## Broken output pipes

When a downstream reader stops early, Branchcut treats `BrokenPipe` as clean success. This supports ordinary pipelines without panic noise.

## Child commands

With `--exec`, a child process that exits unsuccessfully fails the Branchcut invocation. Shell syntax is never interpreted because no shell is launched.

Library visitors receive a `SearchReport` with `Completion::Complete`, `Completion::Stopped`, or `Completion::Incomplete`. Returning `ControlFlow::Break(())` or cancelling a query's `CancellationToken` reports `Stopped`; best-effort filesystem errors can complete as `Incomplete` while returning a report. A strict filesystem failure returns an `io::Error` instead.
