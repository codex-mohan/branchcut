---
title: Feature status and replacements
description: Implemented capabilities, dependable alternatives, and remaining filesystem-query gaps.
icon: ListChecks
---

This page covers Branchcut v0.2.1. See the [roadmap](../project/roadmap.md) for work in development and the [validation record](../evidence/gap-closure.md) for platform checks.

## Complete implemented surface

| Area | Available behavior |
|---|---|
| Everyday search | Literal filename/path containment; ordinary terms form an OR group; repeatable `--and` terms are also required; `--` for dash-prefixed terms |
| Glob selection | `*`, `?`, component-level `**`, classes/ranges/POSIX ASCII classes, bounded nested braces/ranges, composable component-local extglobs including negation, repeated patterns, opt-in portable backslash quoting |
| Composition | Positive patterns form a union; explicit exclusions remove matches; different filter families combine with AND; repeated extensions form a union; repeated size predicates combine with AND |
| Entry types | Regular files by default; repeatable file/directory/symlink union; Unix sockets/FIFOs/devices; empty/executable filters and numeric Unix ownership/group filters |
| Case | Sensitive by default; ASCII-insensitive/smart modes; opt-in Unicode scalar wildcards/classes and single-scalar lowercase comparisons; native invalid-UTF-8 fallback |
| Extension | Repeatable extension filters, optional leading dot, selected case mode |
| Content search | Opt-in literals over selected regular files; any/all/none patterns; word/line bounds; inversion; context; case modes; BOM and explicit encodings; stdin and path lists; bounded workers; exact limits |
| Regex integration | Separate `branchcut-regex` executable with filename/path regex via `find -E` and line-oriented content regex via `grep -E` |
| Content results | Text, JSONL, JSON, CSV, TSV, templates, matching-file/no-match-file lists, line counts, spans with pattern IDs, selection explanations, and summaries |
| Depth | Inclusive minimum/maximum depth measured from cwd; maximum depth also bounds descent |
| Size | Equality and ordered comparisons; decimal/binary units; exact decimal fractions that resolve to whole bytes; regular files only |
| Time | Inclusive modified-within; strict modified-before; fixed durations, RFC3339 offsets/fractions, UTC dates, signed Unix instants; clock captured per execution |
| Roots and hidden paths | `--cwd`; repeatable `--search-path` union with duplicate prevention; hidden exclusion by default; opt-in symlink following with ancestor-cycle checks; Unix one-filesystem control |
| Ignore files | Root/nested/above-cwd `.gitignore`, repository excludes, global excludes, linked worktrees, supported conditional includes; `--ignore` adds `.ignore`; explicit repeatable `--ignore-file` overrides discovered sources; raw Unix bytes and rule provenance |
| Saved queries | Explicit config file; independent named profiles; config-relative default root and relative `cwd`; CLI scalar overrides and repeatable query additions |
| Explanation | Compiled plan without traversal; candidate checks without scanning unrelated trees; decisive ignore rule with source path and line |
| Streaming and limits | Bounded parallel streaming by default; exact first/N cancellation also with explicit workers; library visitors and shared cancellation tokens can stop execution |
| Sorting | Global native-path order; a sorted limit scans the full tree and retains at most N paths; unlimited sorting collects the full selection |
| Output | Plain/count/JSON Lines/NUL/absolute output; opt-in metadata JSON; format templates; quiet existence status; terminal colors/hyperlinks suppressed in machine modes |
| Native names | Native filesystem matching and command arguments; raw Unix path output; JSON companion `path_bytes` or `path_utf16` when text cannot preserve a name |
| Commands | Shell-free native templates with path/basename/parent/stem placeholders; query-root child cwd; per-match/batch execution; bounded child concurrency; lossless preview without launching commands |
| Batching | One whole `{}` argument expands into selected paths; default 128-path batches; configurable count and conservative 28 KiB command budget; child failures fail the query |
| Parallel traversal | Bounded workers/channels; iterative overflow traversal; handle-budget spilling; post-order directories; sequential policy fallback for symlink-following, filesystem boundaries, post-order, and tight handle budgets |
| Errors | Invalid query diagnostics; reported best-effort traversal errors; strict failure mode; failing commands fail the query; broken output pipes end quietly |
| Statistics | Matches, opened/considered/pruned directories, entries, candidates, metadata calls, filesystem errors, content files/bytes/binary skips, elapsed time |
| Library | Typed reusable queries; path/metadata visitors; sequential/parallel/cancellable execution; multi-root unions; filesystem-independent PathMatcher with matching input IDs; path escaping; reports and candidate explanations |
| Structure | One Cargo package; focused library modules and thin binary; no third-party production crates |
| Planner | Shared pattern states; case-sensitive common-literal-root narrowing; positive/exclusion/ignore/depth pruning; type/extension/name checks before requested metadata |

The [CLI reference](./cli.md) describes option interactions. Ordinary name-only queries avoid per-entry metadata fetches; root/ancestor validation still uses metadata. Statistics count explicit metadata requests, not every operating-system call inside the standard library.

## Reliable alternatives for missing behavior

An alternative is useful only within its stated boundary. These do not establish full compatibility with other tools.

| Gap | Available alternative | Remaining work |
|---|---|---|
| Every glob dialect edge case | Use documented component-local language semantics or explicit exclusions | Cross-component extglobs and every shell/micromatch corner case are not claimed |
| Full Git configuration parity | Direct sources, supported includes, and `--ignore-file`; inspect provenance with `--why` | `includeIf hasconfig` reports unsupported; Git command-line environment overrides and implicit Windows Git-installation system config are not resolved |
| Full Unicode normalization/case folding | Opt-in Unicode scalar mode; exact spelling or explicit variants | No grapheme matching, normalization, or multi-character case folding; POSIX named classes use ASCII definitions |
| Lossless unusual Windows names in plain/NUL output | Use JSON Lines and consume `path_utf16`, the Rust library's native paths, or native command substitution | Plain/NUL Windows output is text and can replace unpaired surrogates; NUL alone does not fix encoding |
| Newlines or invalid UTF-8 in Unix names | Use `--print0`; JSON `path_bytes` also preserves invalid UTF-8 | Consumers must decode the companion bytes rather than rely on the display string |
| Fully bounded memory for every operation | Stream paths without sorting, use explicit limits and ordinary single-root queries | Sorting collects selections; multi-root deduplication retains a set; handle spills buffer ancestor entries; OS limits still apply |
| Shell expressions in command templates | Invoke an explicit wrapper program when shell behavior is actually needed | Quoting, pipes, redirects, and expansion are not interpreted by Branchcut; paths must remain separate arguments |
| Very large CLI pattern lists | Saved query files or the typed Rust query API avoid operating-system command-line length limits | No dedicated response-file flag is implemented |
| Full fast-glob API compatibility | Use the supported glob subset and the Rust query API | This is a filesystem query tool, not an npm API replacement |
| Capture-group output | Full-expression spans and zero-based pattern IDs in structured results | Individual capture-group serialization remains open |
| Multiline content, PCRE lookaround/backreferences, and replacement | Line-oriented Rust regex in the optional executable, or a specialized external processor | Cross-line matches, PCRE-specific syntax, capture-group output, and rewriting files remain outside this release |
| Cross-platform ownership/device boundaries | Numeric ownership and filesystem IDs on Unix; explicit unsupported errors elsewhere | Windows ownership and exact one-filesystem enforcement need a separately approved platform implementation |
| Bindings, virtual filesystem adapters, POSIX glob ABI | Invoke the executable or Rust API | JavaScript/C/Zig bindings, custom filesystem adapters, and POSIX/glibc API compatibility are not shipped |
| Package-manager installation | [GitHub release archives](https://github.com/codex-mohan/branchcut/releases/latest), source installers and Cargo installation | Package-manager submissions remain open |

Content search reads only selected regular files. The ordinary CLI uses literals, and the optional `branchcut-regex` executable adds filename/path and line-oriented content regex. See the [content guide](../guides/content-search.md) for the complete input, encoding, result-schema, and resource-bound contract. Multiline matching, capture-group output, replacement, indexing, GUI/TUI, watchers, and service protocols remain outside this release.

## Recent reliability correction

The Phase 7 fixture exposed a supported-ignore regression: appending `/**` before checking separators incorrectly anchored a slashless directory rule such as `target/` to the ignore file's directory. Separator detection now precedes that transformation. Directory-only rules also distinguish directories from regular files and symlinks with the same name. Sequential traversal, parallel traversal, narrowed roots, and candidate explanations have regression coverage.

The gap-closure work expands this coverage to inherited/global/linked-worktree sources, combined ignore precedence, BOM and malformed-pattern behavior, native byte patterns, composable negation, scalar matching, iterative traversal policies, and selected-file workflows. `generated/**` excludes contents while keeping its parent traversable, allowing an explicit child re-inclusion. Validation counts and source identities belong to the dated evidence record rather than this feature inventory.

## Evidence and limits

Every confirmed defect needs a regression. Performance changes need before/after measurements with equivalent selections and retained losses. See [Phase 7 evidence](../evidence/phase7.md) and [compatibility](./compatibility.md). This list does not turn unresolved gaps into supported behavior.
