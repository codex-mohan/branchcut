---
title: Quick start
description: Install Branchcut, run a useful query, inspect the plan, and check its work.
icon: Zap
---

## 1. Install

Use the complete platform command from [Installation](installation). The recommended Windows and Unix commands run the checked-in installer and verify the resulting executable.

Confirm the command before searching:

```bash
branchcut --version
branchcut --help
```

## 2. Run a simple search

From a project directory:

```bash
branchcut config
```

`config` is a literal filename search. Hidden paths are excluded by default. Repeated terms match with OR:

```bash
branchcut readme license
```

Use an explicit glob when path structure matters:

```bash
branchcut --glob 'src/**/*.{rs,toml}' --type file
```

## 3. Exclude generated trees

```bash
branchcut \
  --glob '**/*.{rs,ts}' \
  --exclude '**/target/**' \
  --exclude '**/node_modules/**' \
  --exclude '**/dist/**' \
  --type file
```

Positive patterns form a union. Exclusions remove matches and can prune a safely excluded subtree.

## 4. Explain before walking

```bash
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**' \
  --limit 100 \
  --explain
```

The plan shows the root, shared pattern structure, exclusions, metadata requirement, and termination rule. `--explain` does not traverse.

## 5. Measure actual work

```bash
branchcut \
  --glob '**/*.rs' \
  --exclude '**/target/**' \
  --limit 100 \
  --stats
```

Statistics are written to standard error so stdout remains suitable for a pipeline. Results stream while useful directories are traversed.

## 6. Keep paths machine-safe

```bash
branchcut --glob '**/*.rs' --json
branchcut --glob '**/*.rs' --print0
```

JSON Lines preserves unusual Windows names through a companion `path_utf16` field. NUL output preserves native Unix path bytes and pairs with `xargs -0`.

## Where next

<Cards>
  <Card title="Your first query" description="A guided first pass with plan and statistics." href="/docs/tutorials/first-query" />
  <Card title="Monorepo search" description="Narrow roots, prune generated trees, and save the query." href="/docs/tutorials/monorepo" />
  <Card title="Cheat sheet" description="The options and semantic boundaries to keep nearby." href="/docs/reference/cheatsheet" />
</Cards>