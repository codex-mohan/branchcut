---
title: Your first query
description: Go from an installed command to a useful, explainable filesystem search.
icon: Terminal
---

This tutorial takes about five minutes. It assumes Branchcut is installed and the current terminal is inside a project you want to inspect.

## 1. Run a simple filename search

```bash
branchcut config
```

This searches file names for `config`. Files are the default result type; use `--type dir` to select directory names. Hidden paths are excluded by default, and the command streams matches as they are found.

Add an explicit glob when the shape of the path matters:

```bash
branchcut --glob '**/*config*' --type file --limit 10
```

`**` spans complete directory components, `*` stays inside one component, and `--type file` excludes matching directories.

## 2. Exclude generated trees

```bash
branchcut \
  --glob '**/*.{rs,toml}' \
  --exclude '**/target/**' \
  --exclude '**/node_modules/**' \
  --type file
```

Positive patterns form a union. Exclusions remove matches, and filters such as type and extension narrow the final selection. An excluded subtree can be skipped before its contents are opened.

## 3. Inspect the compiled plan

```bash
branchcut \
  --glob 'src/**/*.{rs,toml}' \
  --exclude '**/target/**' \
  --limit 25 \
  --explain
```

`--explain` prints the plan without traversing. Read four fields first:

- **root:** where the physical search begins;
- **shared literal prefix:** path components multiple patterns have in common;
- **metadata:** whether the query needs file metadata;
- **termination:** when traversal is allowed to stop.

The exact plan text follows the current query and may evolve as planning improves.

## 4. Compare intent with work

Run the same query with statistics:

```bash
branchcut \
  --glob 'src/**/*.{rs,toml}' \
  --exclude '**/target/**' \
  --limit 25 \
  --stats
```

`--stats` reports actual filesystem work, including directories considered, opened, and pruned; entries inspected; metadata calls; matches; and errors. Counter values depend on the tree and filesystem order.

## 5. Understand one path

```bash
branchcut \
  --glob 'src/**/*.rs' \
  --exclude '**/generated/**' \
  --why src/generated/client.rs
```

`--why` evaluates one candidate against the ordered checks and reports where it matched, failed, or was skipped.

## Next

- Build a reusable query in [Plan a monorepo search](./monorepo.md).
- Keep machine paths intact in [Automate a workflow](./automation.md).
- Embed the engine in [Use Branchcut from Rust](./rust-library.md).
