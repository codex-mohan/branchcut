---
title: Documentation map
description: The canonical index for Branchcut documentation in this repository.
icon: Map
---

This directory is the canonical source for Branchcut documentation. The Fumadocs application in [`website/`](../website/) renders these files directly; prose should not be duplicated inside the web application.

## Start here

- [Documentation home](index.mdx)
- [Installation](getting-started/installation.md)
- [Quick start](getting-started/quick-start.md)
- [Mental model](getting-started/mental-model.md)

## Use Branchcut

- [Simple search](guides/simple-search.md)
- [Globs and exclusions](guides/globs-and-exclusions.md)
- [Query planning](guides/query-planning.md)
- [Ignore and hidden paths](guides/ignore-and-hidden.md)
- [Output and command execution](guides/output-and-exec.md)

## Understand the engine

- [Query compiler](concepts/query-compiler.md)
- [Traversal and pruning](concepts/traversal-and-pruning.md)
- [Parallel traversal](concepts/parallelism.md)
- [Path semantics](concepts/path-semantics.md)

## Look things up

- [CLI reference](reference/cli.md)
- [Glob syntax](reference/glob-syntax.md)
- [Exit behavior](reference/exit-behavior.md)
- [Compatibility](reference/compatibility.md)

## Inspect the evidence

- [Benchmarks](evidence/benchmarks.md)
- [Correctness](evidence/correctness.md)
- [Standard-library ledger](evidence/stdlib.md)

## Contribute

- [Verification](contributing/verification.md)
- [Benchmarking](contributing/benchmarking.md)
- [Documentation](contributing/documentation.md)

## Documentation rules

1. Keep claims aligned with the current CLI and [`COMPATIBILITY.md`](../COMPATIBILITY.md).
2. Use runnable commands and state the shell when syntax differs.
3. Explain planner behavior in terms of filesystem work avoided.
4. Keep benchmark claims tied to exact datasets, competitors, versions, and result-set checks.
5. Prefer small ASCII diagrams when they clarify traversal or state flow.
