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
| Release artifacts and packaging | Planned | One-command source install exists; prebuilt distribution is not claimed |

## Completed foundations

1. Reproducible baseline and preserved competition artifact.
2. Matching, sorting, ignore, path, command, and error-semantics corrections.
3. Library extraction and focused source boundaries.
4. Everyday filters with metadata-aware evaluation.
5. Query explanations, candidate explanations, and named project profiles.
6. Dependable pipeline output and shell-free command workflows.
7. Measured sequential and parallel Rust-level optimization passes.

## Current adoption work

The active phase is release quality and onboarding:

- publish prebuilt binaries for supported targets;
- validate everyday, developer, and power-user workflows with real users;
- expand public-repository and cross-platform benchmark evidence;
- keep tutorials and installation paths synchronized with tested behavior.

## Demand-driven candidates

These are not promises or current features:

- captures and richer result explanations;
- additional Git config conditions and advanced ignore escaping;
- richer command templates;
- optional integration crates outside the zero-crate core;
- specialized editor, harness, or agent adapters when a concrete service need exists.

The ordinary CLI remains a normal executable. Agents and scripts can consume its stdout, stderr, exit codes, JSON Lines, and NUL output without a separate protocol.

## Scope guardrails

- Keep the core engine and ordinary CLI free of third-party crates.
- Do not trade documented semantics for new syntax.
- Do not add content search, indexing, a GUI, or a watcher without user evidence.
- Keep losing benchmark results and environment limitations public.
- Preserve simple filename search while adding advanced capability.
