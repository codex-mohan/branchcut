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

Sorting collects the complete result set, sorts globally, then applies any limit. This intentionally trades streaming and early termination for deterministic order.

## Count only

```bash
branchcut --glob '**/*.rs' --count
```

Count mode avoids path serialization. It is useful for measurement and automation.

## JSON Lines

```bash
branchcut --glob '**/*.rs' --json
```

Each result is emitted independently:

```json
{"path":"src/main.rs"}
```

The format is JSON Lines, not one enclosing array, so streaming remains possible.

## Command execution

```bash
branchcut --glob '**/*.tmp' --exec 'rm {}'
```

`{}` is replaced with the displayed path. Branchcut parses the command into arguments and calls `std::process::Command` directly. It never invokes a shell.

Consequences:

- quoting and escaped characters are supported by a small argument parser;
- shell pipelines, redirection, expansion, and operators are not supported;
- unsuccessful child commands make the query fail;
- use sequential mode when side-effect order matters.

Parallel traversal rejects `--exec` because worker completion order is not a safe command execution order.

## Broken pipes

If a downstream consumer closes the pipe early, Branchcut exits cleanly instead of reporting an ordinary failure.
