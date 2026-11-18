---
title: Documentation map
description: Choose the path that matches what you are trying to do with Branchcut.
icon: Map
---

The documentation is organized around jobs rather than source folders. Use search for a specific option, or start with one of the routes below.

## Choose by what you have

| You have | Start with | You will learn |
|---|---|---|
| Branchcut installed and want a first result | [Your first query](tutorials/first-query) | Literal search, globs, exclusions, plans, and statistics |
| A monorepo and repeated file-selection work | [Plan a monorepo search](tutorials/monorepo) | Prefix narrowing, pruning, limits, and saved profiles |
| Shell scripts, tools, or a coding agent | [Automate a workflow](tutorials/automation) | JSON Lines, NUL output, native paths, commands, and exit behavior |
| A Rust application that needs file selection | [Use Branchcut from Rust](tutorials/rust-library) | Query compilation, streaming visitors, cancellation, and parallelism |
| A command in mind and need the exact flag | [CLI reference](reference/cli) | Complete option meanings and interactions |
| A source tree that is behaving unexpectedly | [Query planning](guides/query-planning) | `--explain`, `--why`, and real work counters |

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

- [Mental model](getting-started/mental-model) explains the compiler-as-filesystem-machine idea.
- [Query compiler](concepts/query-compiler) covers parsing and shared pattern state.
- [Traversal and pruning](concepts/traversal-and-pruning) explains how plans change filesystem work.
- [Library and modules](concepts/library-and-modules) maps the Rust source boundaries.
- [Path semantics](concepts/path-semantics) covers native paths and platform differences.

## Use the command

| Job | Guide |
|---|---|
| Find one filename without glob syntax | [Simple search](guides/simple-search) |
| Combine size, time, depth, and case filters | [Everyday search](guides/everyday-search) |
| Compose patterns and exclusions | [Globs and exclusions](guides/globs-and-exclusions) |
| Understand hidden paths and `.gitignore` policy | [Ignore and hidden paths](guides/ignore-and-hidden) |
| Stream, count, sort, serialize, or execute commands | [Output and command execution](guides/output-and-exec) |
| Save a query for repeated project use | [Saved project queries](guides/project-queries) |
| Inspect why a path matched or was excluded | [Query planning](guides/query-planning) |

## Look up a fact

| Need | Reference |
|---|---|
| Every CLI option and allowed combination | [CLI reference](reference/cli) |
| Copy-ready common commands | [Cheat sheet](reference/cheatsheet) |
| Supported wildcard grammar | [Glob syntax](reference/glob-syntax) |
| Exit codes, strict mode, and broken pipes | [Exit behavior](reference/exit-behavior) |
| Implemented surface, alternatives, and gaps | [Feature status](reference/feature-status) |
| Compatibility boundaries | [Compatibility](reference/compatibility) |
| Product terminology | [Glossary](project/glossary) |

## Inspect the evidence

- [Benchmark guide](evidence/benchmarks) explains datasets, metrics, and limits.
- [Direct CLI comparison](evidence/cli-comparison) contains the cross-tool matrix.
- [Correctness](evidence/correctness) explains result equivalence and regression strategy.
- [Search performance investigation](evidence/search-performance) records profiling and losing results.
- [Rust-level optimization pass](evidence/rust-optimizations) records measured implementation changes.
- [Standard-library ledger](evidence/stdlib) maps the zero-crate core to normal dependencies.

## Work on Branchcut

1. Read [Verification](contributing/verification) before changing behavior.
2. Read [Benchmarking](contributing/benchmarking) before making a performance claim.
3. Read [Documentation](contributing/documentation) before adding a page or changing compatibility language.
4. Check the [Roadmap](project/roadmap) for delivery status rather than treating planned work as shipped.
5. Use the [FAQ](project/faq) for recurring scope and integration questions.

## Documentation conventions

- Runnable commands appear in code blocks with their shell noted when syntax differs.
- Performance claims identify the dataset, environment, exact query, and correctness check.
- “Planned,” “supported,” and “tested” retain their literal meanings.
- External comparisons describe behavior; they do not imply copied implementations.