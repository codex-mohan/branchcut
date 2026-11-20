---
title: Roadmap
description: What is implemented, what is being validated, and what remains demand-driven.
icon: Map
---

Branchcut preserves the tagged competition submission while developing a maintainable filesystem-query engine. This page describes the current development tree. See [Feature status](../reference/feature-status) for exact behavior and [Compatibility](../reference/compatibility) for deliberate gaps.

## Status at a glance

| Area | Status | Meaning |
|---|---|---|
| Single-file competition artifact | Preserved | Tagged and retained as historical evidence |
| Modular Rust library and thin CLI | Implemented | Core traversal is independent of terminal presentation |
| Everyday filters | Implemented | Type, extension, size, time, case, depth, and hidden policy |
| Ignore policy | Implemented subset | Git sources and hierarchical `.gitignore`; optional `.ignore` and explicit higher-priority ignore files |
| Explain and why diagnostics | Implemented | Plan inspection and candidate-path explanation |
| Machine output and commands | Implemented | JSON Lines, NUL output, absolute paths, shell-free execution, batching, preview, and completions |
| Rust integrations | Implemented | Standalone path matcher, multi-root search, metadata-record visitors, and cooperative cancellation |
| Parallel traversal | Implemented | Bounded workers, streaming coordinator, sequential compatibility fallback |
| Performance work | Measured on Windows | Raw results and losses retained; broader platforms remain separate evidence |
| Cross-platform confidence | Native CI passes | [Windows, Linux and Intel macOS](https://github.com/codex-mohan/branchcut/actions/runs/44349839187); release artifact builds remain separate gates |
| Release artifacts and packaging | Published | [v0.2.1](https://github.com/codex-mohan/branchcut/releases/tag/v0.2.1) provides four prebuilt archives and SHA-256 checksums |

## Completed foundations

1. Reproducible baseline and preserved competition artifact.
2. Matching, sorting, ignore, path, command, and error-semantics corrections.
3. Library extraction and focused source boundaries.
4. Everyday filters with metadata-aware evaluation.
5. Query explanations, candidate explanations, and named project profiles.
6. Dependable pipeline output and shell-free command workflows.
7. Measured sequential and parallel Rust-level optimization passes.
8. Opt-in literal content search integrated with path selection, exclusions, filters, and early stop.

## Current adoption work

The active phase is release quality and onboarding:

- validate everyday, developer, and power-user workflows with real users;
- expand public-repository and cross-platform benchmark evidence;
- keep tutorials and installation paths synchronized with tested behavior.

## Shipped: File-content search

`--content PATTERN` (alias `--grep`) searches the regular files already selected by Branchcut's globs, ignore rules, type, extension, size, and time filters. It is literal and line-oriented: patterns combine with OR, output is `path:line:column:text`, `-l` prints paths once, `--count` counts matching lines, and `--limit`/`--first` stop scanning early. Smart-case inspects the content patterns; binary-looking files are skipped unless `--text` is passed.

Filename and metadata queries keep their existing semantics and read no file contents. Regex, captures, multiline matching, and replacement remain demand-driven candidates rather than shipped behavior. See the [CLI reference](../reference/cli.md#content-search) and [compatibility](../reference/compatibility.md) for exact semantics.

## Demand-driven candidates

These are not promises or current features:

- captures and richer result explanations;
- additional Git config conditions and advanced ignore escaping;
- richer command templates;
- indexing, watchers, or a GUI where a concrete workflow calls for them;
- optional integration crates outside the zero-crate core;
- specialized editor, harness, or agent adapters when a concrete service need exists.

The ordinary CLI remains a normal executable. Agents and scripts can consume its stdout, stderr, exit codes, JSON Lines, and NUL output without a separate protocol.

## Scope guardrails

- Keep the core engine and ordinary CLI free of third-party crates.
- Do not trade documented semantics for new syntax.
- Keep losing benchmark results and environment limitations public.
- Preserve simple filename search while adding advanced capability.
