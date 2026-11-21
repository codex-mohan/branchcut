---
title: Documentation map
description: Installation, tutorials, command reference, and contributor guides.
icon: Map
---

Start with [installation](./getting-started/installation.md) or use the table to find a guide.

## Choose by what you have

| You have | Start with | You will learn |
|---|---|---|
| Branchcut installed and want a first result | [Your first query](./tutorials/first-query.md) | Literal search, globs, exclusions, plans, and statistics |
| A monorepo and repeated file-selection work | [Plan a monorepo search](./tutorials/monorepo.md) | Prefix narrowing, pruning, limits, and saved profiles |
| Shell scripts, tools, or a coding agent | [Automate a workflow](./tutorials/automation.md) | JSON Lines, NUL output, native paths, commands, and exit behavior |
| A Rust application that needs file selection | [Use Branchcut from Rust](./tutorials/rust-library.md) | Query compilation, streaming visitors, cancellation, and parallelism |
| A command in mind and need the exact flag | [CLI reference](./reference/cli.md) | Complete option meanings and interactions |
| A source tree that is behaving unexpectedly | [Query planning](./guides/query-planning.md) | `--explain`, `--why`, and real work counters |

## Learn the model

```text
query input
    |
    v
compile patterns + exclusions + policy + filters + termination
    |
    v
narrow the root and prune impossible subtrees
    |
    v
stream native paths, diagnostics, and commands
```

- [Mental model](./getting-started/mental-model.md) explains the compiler-as-filesystem-machine idea.
- [Query compiler](./concepts/query-compiler.md) covers parsing and shared pattern state.
- [Traversal and pruning](./concepts/traversal-and-pruning.md) explains how plans change filesystem work.
- [Library and modules](./concepts/library-and-modules.md) maps the Rust source boundaries.
- [Path semantics](./concepts/path-semantics.md) covers native paths and platform differences.

## Use the command

| Job | Guide |
|---|---|
| Find one filename without glob syntax | [Simple search](./guides/simple-search.md) |
| Combine size, time, depth, and case filters | [Everyday search](./guides/everyday-search.md) |
| Compose patterns and exclusions | [Globs and exclusions](./guides/globs-and-exclusions.md) |
| Understand hidden paths and `.gitignore` policy | [Ignore and hidden paths](./guides/ignore-and-hidden.md) |
| Stream, count, sort, serialize, or execute commands | [Output and command execution](./guides/output-and-exec.md) |
| Save a query for repeated project use | [Saved project queries](./guides/project-queries.md) |
| Inspect why a path matched or was excluded | [Query planning](./guides/query-planning.md) |

## Look up a fact

| Need | Reference |
|---|---|
| Every CLI option and allowed combination | [CLI reference](./reference/cli.md) |
| Copy-ready common commands | [Cheat sheet](./reference/cheatsheet.md) |
| Supported wildcard grammar | [Glob syntax](./reference/glob-syntax.md) |
| Exit codes, strict mode, and broken pipes | [Exit behavior](./reference/exit-behavior.md) |
| Implemented surface, alternatives, and gaps | [Feature status](./reference/feature-status.md) |
| Compatibility boundaries | [Compatibility](./reference/compatibility.md) |
| Product terminology | [Glossary](./project/glossary.md) |

## Inspect the evidence

- [Benchmark guide](./evidence/benchmarks.md) explains datasets, metrics, and limits.
- [Current CLI comparison](./evidence/current-cli-comparison.md) shows default timings, tuned runs, and working-set memory for all six tools.
- [Correctness](./evidence/correctness.md) explains result equivalence and regression strategy.
- [Search performance investigation](./evidence/search-performance.md) records profiling and losing results.
- [Rust-level optimization pass](./evidence/rust-optimizations.md) records measured implementation changes.
- [Standard-library ledger](./evidence/stdlib.md) maps the zero-crate core to normal dependencies.

## Work on Branchcut

1. Read [Verification](./contributing/verification.md) before changing behavior.
2. Read [Benchmarking](./contributing/benchmarking.md) before making a performance claim.
3. Read [Documentation](./contributing/documentation.md) before adding a page or changing compatibility language.
4. Check the [Roadmap](./project/roadmap.md) for features in development.
5. Use the [FAQ](./project/faq.md) for recurring scope and integration questions.
