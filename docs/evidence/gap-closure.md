---
title: Filesystem gap closure
description: Implemented capabilities, validation, and outstanding release gates.
icon: ListChecks
---

This record describes the development tree reviewed on 18 November 2026. It does not change the preserved `submission-v1` artifact. The [feature inventory](../reference/feature-status.md) defines current behavior; performance snapshots retain their own source identities.

## Phase status

| Phase | Implemented work | Verification boundary |
|---|---|---|
| 1. Ignores | Nested and above-cwd inheritance; repository/global excludes; linked worktrees; supported conditional includes; `.ignore`; explicit override files; BOM/escaping/classes; native Unix pattern bytes; provenance | Git-oracle tests on Windows and Linux; full Git configuration parity is not claimed |
| 2. Matching | Composed/nested component-local negated extglobs; POSIX ASCII classes; opt-in backslash quoting; Unicode scalar literals, wildcards and classes; matching-pattern IDs | Regression and supported-semantics differential tests; no normalization, grapheme matching, or multi-character folding |
| 3. Traversal | Followed symlinks with ancestor-cycle checks; multi-root unions; Unix filesystem boundary; handle budgets; post-order; iterative overflow; exact parallel limits; cooperative cancellation | Native Windows, Linux and Intel macOS CI; separate release artifact builds remain required |
| 4. Filters | Entry-type unions; empty/executable selection; Unix numeric ownership/group and special types; required terms; exact fractional sizes; offset/fractional timestamps | Candidate-first metadata checks and public-API integration regressions |
| 5. Workflows | Metadata JSON; formatting/component placeholders; quiet exit status; bounded child concurrency; lossless command preview; terminal decorations isolated from machine output | CLI integration tests, failure propagation and native-name regressions |
| 6. Rust APIs | PathMatcher, MatchOptions, escape_path_pattern; MultiRootQuery; EntryRecord visitors; CancellationToken | External-consumer tests; metadata-rich visits explicitly request metadata |
| 7. Distribution | Source bootstrap installers, four-shell completions and four prebuilt archives | [v0.2.1 release build](https://github.com/codex-mohan/branchcut/actions/runs/44356821678) passed. Downloaded archives match their published checksums. |
| 8. Documentation and evidence | General project README; current references; benchmark scope and coverage labels corrected; dated raw comparisons retained | New measurements apply only to their recorded source manifest; earlier snapshots remain historical |

## Validation

Required checks are `cargo fmt -- --check`, `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo tree -e normal`. The core dependency table remains empty.

Local Windows tests run natively. Local Linux tests run in Fedora under WSL with an independent Linux Rust toolchain and a Linux target directory. The subsequent [native validation matrix](https://github.com/codex-mohan/branchcut/actions/runs/44349839187) passed on hosted Windows, Ubuntu and Intel macOS at source `34e9e8e`. Each job passed formatting, tests, Clippy, pinned-oracle differential checks and install/query/uninstall. This verifies those workflows; it is not a Linux/macOS performance claim.

The first hosted matrix exposed fixture assumptions: Git resolves Windows short-name aliases before testing conditional includes, and the macOS fixture filesystem rejects invalid UTF-8 names with EILSEQ. Conditional-include fixtures now use a canonical Git directory. Native-byte filesystem assertions run where such names can be created; the helper permits only that specific macOS creation failure, and all other creation failures remain test errors. Pure matcher byte-path tests still run independently of filesystem filename restrictions. These corrections do not change production behavior or the measured binary.

The supported-semantics differential suite uses pinned `fast-glob@3.3.3` as a development-only oracle. Git is used only by the independent ignore tests; production ignore handling does not launch Git.

The expanded runner checks 28 result-equivalent cases in sequential and parallel modes and retains one explicit negated-extglob dialect difference. Branchcut and Bash match `aaa.rs` for `a!(a).rs`; fast-glob/micromatch does not. That case is not counted as compatibility with fast-glob.

## Defined costs and remaining boundaries

- Symlink following, post-order traversal, filesystem boundaries, and tight handle budgets can select sequential execution even through a parallel API.
- Handle-budget spills buffer unvisited ancestor entries. Global sorting and multi-root duplicate suppression also retain state proportional to the requested operation. Ordinary single-root unsorted output still streams.
- Unicode mode uses scalar matching and single-scalar lowercase comparisons. POSIX named classes keep ASCII definitions; byte mode remains the default.
- `includeIf hasconfig` and some Git environment/system configuration discovery are unsupported and documented. Root/nested ignore support itself is implemented.
- Exact one-filesystem enforcement, ownership IDs and Unix special-file types report unsupported on platforms where the standard-library implementation cannot provide them.
- Windows plain/NUL text cannot preserve every unpaired surrogate; native APIs, command arguments and JSON companion fields provide lossless alternatives.
- As of this historical gap-closure pass, filename regexes were absent. v0.3.0 adds optional filename/path regex in `branchcut-regex`. Capture-group output, custom filesystem adapters, foreign-language bindings, and POSIX/glibc ABI compatibility remain outside the implementation. These are independent of CLI timing results.
- The later v0.3.0 content expansion adds richer line selection, inputs, encodings, result formats, and optional line-oriented regex. See the [current content guide](../guides/content-search.md). The ripgrep benchmark in this gap-closure record measures `rg --files`.
- Package-manager publication and native Apple Silicon execution testing remain open. The Apple Silicon release archive was cross-built using Apple's SDK and passed architecture and signature verification. The regular native macOS checks use Intel runners.

## Benchmark scope

The original [direct CLI comparison](./cli-comparison.md) remains a historical Windows/NTFS snapshot. Missing cells mean the runner did not measure an equivalent invocation, not necessarily that the tool lacks the feature. A filesystem listing benchmark is not a full replacement assessment.

The final [current CLI comparison](./current-cli-comparison.md) uses committed runtime source `8035ad6`, ten workloads and all six CLI/API targets. It reports default and tuned configurations separately and adds same-query observed working-set samples. The preflight is retained separately. Local validation passed 209 Windows tests and 226 Linux tests, including doctests, with Clippy on both toolchains. Both Apple targets passed all-target cross-checks; the linked native Intel macOS matrix subsequently passed as well.

Final review added regressions for config-relative override files, cached target metadata error handling/counting, and a Linux `--max-open 1` search under a four-descriptor process limit.

The hosted runs also corrected command-cwd assertions to compare resolved directory identity (`/var` and `/private/var` can name the same directory), and constrained deep-tree fixtures to the macOS full-path budget. These were test-only corrections. Historical failing runs are retained in Actions; the successful matrix is linked above.
