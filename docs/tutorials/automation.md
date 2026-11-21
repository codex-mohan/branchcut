---
title: Automate a workflow
description: Keep selected paths intact across scripts, tools, commands, and coding agents.
icon: Workflow
---

Branchcut is useful at process boundaries because its default output is ordinary text and its machine formats preserve native path information when text is insufficient.

## Feed paths to another process

Newline output is convenient when names are known not to contain newlines:

```bash
branchcut --glob '**/*.rs'
```

Use NUL delimiters for a general-purpose Unix pipeline:

```bash
branchcut --glob '**/*.rs' --print0 | xargs -0 command
```

NUL output writes one native path plus one NUL byte. It cannot be combined with `--count` or `--json`.

## Consume JSON Lines

```bash
branchcut --glob '**/*.rs' --json
```

Each stdout line is one independent object:

```json
{"path":"src/main.rs"}
{"path":"src/cli/mod.rs"}
```

A path that is not representable in the selected text encoding gains `path_bytes` on Unix or `path_utf16` on Windows. Use the companion field when lossless machine handling matters.

PowerShell can consume the stream without loading the complete result:

```powershell
branchcut --glob "**/*.rs" --json |
  ForEach-Object { ($_ | ConvertFrom-Json).path }
```

## Execute commands without a shell

Run one command per selected path:

```bash
branchcut --glob '**/*.tmp' --exec rm '{}'
```

`{}` is substituted with a native path. Shell interpolation, pipes, redirects, and expansions are not interpreted. Use `--exec-batch` when one invocation should receive several paths:

```bash
branchcut --glob '**/*.rs' --exec-batch rustfmt '{}'
```

The batch placeholder must be its own argument. Branchcut partitions batches by `--batch-size` and a conservative command-size budget.

## Choose a stable path form

```bash
branchcut --glob '**/*.rs' --absolute --print0
```

Relative output is usually right for tools that inherit the query root. Use `--absolute` when the consumer runs from another directory.

## Return reliable failures

- Exit `0`: completed successfully, including no matches and a clean broken output pipe.
- Exit `2`: invalid query, strict filesystem failure, output failure, or unsuccessful child command.

Use `--strict` when an incomplete traversal must fail instead of continuing best-effort.

## Give an agent a stable interface

A coding agent or harness can treat Branchcut as a normal system executable. Prefer `--json` or `--print0` for paths, `--stats` for work counters, and `--explain` for intended behavior. No service protocol or MCP server is required because the command consumes argv and returns streamed stdout/stderr.

## Next

- Review every option in the [CLI reference](../reference/cli.md).
- Use the one-page [Cheat sheet](../reference/cheatsheet.md).
- Read the exact failure contract in [Exit behavior](../reference/exit-behavior.md).