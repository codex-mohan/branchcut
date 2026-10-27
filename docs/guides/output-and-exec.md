---
title: Output and command execution
description: Stream paths, count, sort, serialize JSON Lines, or execute commands safely.
icon: Terminal
---

## Streaming paths

The default sequential mode writes one relative path per line as soon as it is discovered.

```bash
branchcut --glob '**/*.rs'
```

Enumeration order depends on the platform and filesystem.

## Deterministic sorting

```bash
branchcut --glob '**/*.rs' --sort
```

Sorting collects the complete matching result set and sorts it globally. If `--limit N` or `--first` is also specified, Branchcut returns the first N paths from that global order. This requires completing the search before printing results. Count mode may stop as soon as it has counted N matches because order does not affect a count.

## Count only

```bash
branchcut --glob '**/*.rs' --count
```

Count mode avoids path serialization. It is useful for measurement and automation.

## JSON Lines

```bash
branchcut --glob '**/*.rs' --json
```

Each result is emitted independently. UTF-8 paths have one `path` field:

```json
{"path":"src/main.rs"}
```

The format is JSON Lines, not one enclosing array, so streaming remains possible. When a relative path is not UTF-8, Unix results also include `path_bytes` with the complete native path bytes encoded as lowercase hexadecimal. Windows results include `path_utf16` with the complete relative path as UTF-16 code units. Use those companion fields for lossless machine handling; `path` is the readable display value.

## Command execution

```bash
branchcut --glob '**/*.tmp' --exec 'rm {}'
```

`{}` is replaced with the relative path rooted at `--cwd`. Branchcut builds each argument as an `OsString`, preserving native path data, and sets the child process working directory to the query root. It parses the command into arguments and calls `std::process::Command` directly; it never invokes a shell.

Consequences:

- quoting and escaped characters are supported by a small argument parser;
- shell pipelines, redirection, expansion, and operators are not supported;
- unsuccessful child commands make the query fail;
- with `--sort`, commands run for the final selected paths in sorted order; sorted limits run after full traversal;
- use sequential mode when side-effect order matters.

Parallel traversal rejects `--exec` because worker completion order is not a safe command execution order.

## Broken pipes

If a downstream consumer closes the pipe early, Branchcut exits cleanly instead of reporting an ordinary failure.
