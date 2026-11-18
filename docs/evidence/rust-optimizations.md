---
title: Rust-level optimization pass
description: Measured allocation, matcher-state, ignore-context, and output-path improvements with retained losses.
icon: Gauge
---

## Scope

This pass follows the first planner-level benchmark round. It targets Rust-level work inside the established architecture:

- empty matcher programs now return empty state sets;
- universal `**/*` plans bypass unused positive and negative state work;
- count-only consumers can receive matches without materialized paths;
- ordinary output streams borrowed parent/name path parts through one reusable byte buffer;
- ignore checks reuse path-match scratch vectors and share inherited rule contexts;
- parallel workers batch pathless counts and avoid per-file relative path joins when no consumer needs them.
- metadata-free queries no longer materialize an absolute `DirEntry` path for a no-op metadata test.

These changes preserve matching, filtering, ignore, sorting, command, native-name, and limit semantics. The full matrix checks every generated path or count before timing and after each CLI sample.

## Record

The retained [final 52-workload record](../../benchmarks/results/2026-11-11-rust-language-final.json) contains 26 workloads on each of the 10,000- and 100,000-file trees. It uses two warmups and seven measured samples per variant, alternating variant order. The [focused no-metadata record](../../benchmarks/results/2026-11-11-rust-no-metadata-path.json) uses three warmups and nine samples to isolate the final `DirEntry::path()` removal. The [focused losing-case recheck](../../benchmarks/results/2026-11-11-rust-loss-recheck.json) uses three warmups and nine samples.

The earlier intermediate records are retained separately:

- [Earlier full matrix](../../benchmarks/results/2026-11-11-rust-language-full.json)
- [Matcher state and ignore components](../../benchmarks/results/2026-11-11-rust-state-ignore.json)
- [Pathless count visitor](../../benchmarks/results/2026-11-11-rust-pathless.json)
- [Borrowed output paths](../../benchmarks/results/2026-11-11-rust-output-paths.json)
- [Buffered output line](../../benchmarks/results/2026-11-11-rust-output-buffer.json)
- [Universal plan fast path](../../benchmarks/results/2026-11-11-rust-universal-plan.json)
- [Parallel pathless counts](../../benchmarks/results/2026-11-11-rust-parallel-count.json)
- [Reusable ignore state](../../benchmarks/results/2026-11-11-rust-ignore-scratch.json)
- [Shared ignore context](../../benchmarks/results/2026-11-11-rust-ignore-shared.json)

Filesystem cache state is uncontrolled. CLI measurements are fresh processes that include startup, planning, traversal, output, and pipe draining. Warm-engine measurements reuse one compiled plan.

## Results

Across all 26 comparable workloads, the final geometric-mean fresh-process ratio was 1.014x at 10,000 files and 1.166x at 100,000 files. The warm-engine geometric means were 1.135x and 1.305x. These are before/after comparisons against one preserved Windows build, not competitor claims.

Selected 100,000-file medians:

| Workload | Before | After | Ratio |
|---|---:|---:|---:|
| Broad count | 146.69 ms | 102.48 ms | 1.43x |
| Broad streamed output | 182.12 ms | 140.66 ms | 1.30x |
| Exclusion-heavy traversal | 109.92 ms | 94.66 ms | 1.16x |
| Ignore-heavy traversal | 175.02 ms | 129.79 ms | 1.35x |
| Four-worker count | 93.94 ms | 71.21 ms | 1.32x |
| Four-worker output | 73.37 ms | 53.88 ms | 1.36x |

The final matrix also measured JSON output at 1.32x, NUL output at 1.45x, and sorted-ten selection at 1.38x. Unlimited sorting was 0.98x, modification-time filtering 0.97x, size filtering 1.03x, and first-ten selection at 0.96x. The small-tree matrix had five losses of at least 2%, led by unrelated-pattern cases; these are not hidden or averaged into the highlighted results.

## Limits

Metadata predicates remain dominated by per-candidate filesystem metadata and improved little. Sorted full-result output still spends most of its time retaining and ordering every path; this pass does not change that algorithm. The synthetic tree has shallow ignore nesting, so shared ignore contexts show architectural and allocation benefit without a distinct timing claim on this fixture.

Public-repository, million-file, Linux, macOS, cold-disk, and profiler-attributed allocation measurements remain open. No result establishes universal performance superiority.
