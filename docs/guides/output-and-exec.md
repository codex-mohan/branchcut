---
title: Output and command execution
description: Stream paths, count, sort, serialize JSON Lines, or execute commands safely.
icon: Terminal
---

## Streaming paths

The default traversal writes one relative path per line as soon as it is discovered. Ordinary queries use adaptive bounded workers; without an explicit `--threads`, `--limit`, `--first`, and per-match `--exec` use sequential traversal. An explicit multi-worker count is honored for limits, so those matches arrive in nondeterministic order and additional work may already be in flight. `--sort --limit N` selects the globally first N but must scan the full selection.

```bash
branchcut --glob '**/*.rs'
```

Enumeration order depends on the platform and filesystem.

## Pipeline-safe paths

Use NUL delimiters when filenames may contain spaces or newlines:

```bash
branchcut --glob '**/*.rs' -0
branchcut --glob '**/*.rs' --print0
```

Each native path is followed by one NUL byte. Unix output preserves native path bytes. Windows plain output uses the same display encoding as ordinary line output, so JSON Lines remains the lossless option for unpaired UTF-16 surrogates. `--print0` cannot be combined with `--json` or `--count`.

Use `--absolute` when a downstream program should not inherit the query root as its working directory:

```bash
branchcut --glob '**/*.rs' --absolute --print0
```

Absolute output resolves `--cwd` once and joins each selected path to that resolved root. It also controls the path inserted into `--exec` and `--exec-batch`.

## Deterministic sorting

```bash
branchcut --glob '**/*.rs' --sort
```

Unlimited sorting collects all matches. With `--limit N` or `--first`, Branchcut retains only the smallest N paths in native path order and sorts that selection. Both modes complete the full search before printing results; a sorted limit reduces retained paths, not filesystem work. Count mode may stop as soon as it has counted N matches because order does not affect a count.

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
- with `--sort`, commands are selected from the sorted result set; sorted limits run after full traversal;
- `--exec-jobs` defaults to one child process at a time. Values greater than one run commands concurrently, so side-effect and completion order is unspecified, including for sorted selections.

If `--threads N` greater than one is used with per-match `--exec`, also specify `--exec-jobs N` to opt into parallel traversal with command execution. More than one job runs commands concurrently; one job runs them serially in the unordered match-delivery order. A command can begin only after its matching path is delivered.

Use `--exec-preview` to print the parsed command argument vector as JSON Lines without launching the child. It requires `--exec` or `--exec-batch` and cannot be combined with `--count`, `--print0`, or `--json-metadata`.

## Batched commands

`--exec-batch` runs one shell-free command for a bounded set of matches:

```bash
branchcut --glob '**/*.rs' --exec-batch 'rustfmt {}'
branchcut --glob '**/*.rs' --sort --exec-batch 'rustfmt {}' --batch-size 50
```

The single `{}` placeholder must be its own argument. It expands to one native argument per selected path. A template without a whole `{}` argument, more than one placeholder, or a placeholder in the executable position is rejected before traversal.

`--batch-size` limits matches per invocation and defaults to 128. Branchcut also enforces a conservative 28 KiB command budget so long path sets are split before reaching platform command-line limits. A single path that cannot fit is reported as an error.

Fewer than `--batch-size` paths may be passed when the byte budget is reached first. With `--sort`, paths are selected and grouped in global path order, though multiple command jobs can complete out of order. With unordered parallel traversal, batch composition is unspecified. Commands run with the query root as their working directory. Any unsuccessful batch fails the query.

## Broken pipes

If a downstream consumer closes the pipe early, Branchcut exits cleanly instead of reporting an ordinary failure.
