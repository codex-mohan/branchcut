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
| 3. Traversal | Followed symlinks with ancestor-cycle checks; multi-root unions; Unix filesystem boundary; handle budgets; post-order; iterative overflow; exact parallel limits; cooperative cancellation | Windows and Linux tests; macOS native CI remains required |
| 4. Filters | Entry-type unions; empty/executable selection; Unix numeric ownership/group and special types; required terms; exact fractional sizes; offset/fractional timestamps | Candidate-first metadata checks and public-API integration regressions |
| 5. Workflows | Metadata JSON; formatting/component placeholders; quiet exit status; bounded child concurrency; lossless command preview; terminal decorations isolated from machine output | CLI integration tests, failure propagation and native-name regressions |
| 6. Rust APIs | PathMatcher, MatchOptions, escape_path_pattern; MultiRootQuery; EntryRecord visitors; CancellationToken | External-consumer tests; metadata-rich visits explicitly request metadata |
| 7. Distribution | Source bootstrap installers; four-shell completion generation; four-platform release/archive/checksum workflow | Release automation is prepared; actual prebuilt assets and package-manager publication require a release |
| 8. Documentation and evidence | General project README; current references; benchmark scope and coverage labels corrected; dated raw comparisons retained | New measurements apply only to their recorded source manifest; earlier snapshots remain historical |

## Validation

Required checks are `cargo fmt -- --check`, `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo tree -e normal`. The core dependency table remains empty.

Windows tests run natively. Linux tests run in Fedora under WSL with an independent Linux Rust toolchain and a Linux target directory. This exercises Unix symlinks, file types, permissions, raw filenames and Git behavior, but is not a native macOS run or a Linux performance claim.

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
- Filename regexes, captures, custom filesystem adapters, foreign-language bindings, and POSIX/glibc ABI compatibility are not implemented. These are not implied by glob support or CLI timing results.
- Content search, indexing, watchers, GUI/TUI and service protocols remain separate product work. The ripgrep benchmark measures `rg --files`, not content search.
- macOS execution, a tagged prebuilt release and package-manager publication remain release gates. A local passing build does not establish those outcomes.

## Benchmark scope

The original [direct CLI comparison](cli-comparison.md) remains a historical Windows/NTFS snapshot. Missing cells mean the runner did not measure an equivalent invocation, not necessarily that the tool lacks the feature. A filesystem listing benchmark is not a full replacement assessment.

The final [current CLI comparison](current-cli-comparison.md) uses committed runtime source `8035ad6`, ten workloads and all six CLI/API targets. It reports default and tuned configurations separately and adds same-query observed working-set samples. The preflight is retained separately. Local validation passed 209 Windows tests and 226 Linux tests, including doctests, with Clippy on both toolchains. Both Apple targets passed all-target cross-checks; native macOS CI remains pending.

Final review added regressions for config-relative override files, cached target metadata error handling/counting, and a Linux `--max-open 1` search under a four-descriptor process limit.
