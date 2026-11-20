---
title: Phase 7 measurements
description: Correctness-first before/after workloads, bounded sorted selection, and retained limitations.
icon: Timer
---

## Scope

The Phase 7 runner measures the expanded post-competition engine on reproducible 10,000- and 100,000-file trees. It covers broad count and streamed output, narrow queries, ASCII-insensitive prefixes, exclusions, nested ignores, metadata predicates, early limits, sequential and 2/4/8-worker runs, four-worker count, full sorting, small/large sorted limits, JSON/NUL output, and 1/10/100/1,000 shared or unrelated patterns.

The runner and warm-engine probe use only Node built-ins and Rust std plus Branchcut itself. No production dependency is added. The external pinned fast-glob oracle remains a separate development-only check.

## Reproduce

Run from the repository root. Preserve the before executables and source identity before editing the implementation:

```powershell
cargo build --release --locked --bins --examples
New-Item -ItemType Directory -Path target/phase7 -Force
Copy-Item target/release/branchcut.exe target/phase7/before.exe
Copy-Item target/release/examples/measure_engine.exe target/phase7/before-engine.exe
node scripts/phase7.mjs --snapshot target/phase7/before-source.json

# Apply the candidate change, then rebuild.
cargo build --release --locked --bins --examples
node scripts/phase7.mjs --before target/phase7/before.exe --after target/release/branchcut.exe --before-engine target/phase7/before-engine.exe --after-engine target/release/examples/measure_engine.exe --before-source target/phase7/before-source.json --sizes 10000,100000 --iterations 9 --warmups 2 --output benchmarks/results/local-phase7.json
```

On Unix, substitute native executable names without `.exe`. The query, fixture generation, path-set checks, timing, and warm-engine runs are portable. The separate process-resource probe is Windows-only; other platforms record missing resource metrics explicitly. `--only NAME` filters workloads for focused reruns. A smoke run can use `--sizes 1600 --iterations 1 --warmups 0`.

## Fixture and correctness

Each size has 20 packages. Generated entries cycle through source files, nested sources, TOML, documentation, build output, dependency output, uppercase-extension files, and logs. Files have known 32-byte or 4,096-byte sizes and fixed 2020/2024 timestamps. Root and nested ignore files are extra entries excluded by hidden policy. Case-distinct directories use genuinely different names so the tree is valid on case-insensitive filesystems.

Full CLI results are checked against the generator manifest before timing. Every timed count or path selection is checked again. Limited unordered selections must have the requested count, contain only valid paths, and have no duplicates; sorted limits must equal the globally first N paths. Timed library counts use the same expected selection. This manifest is independent of Branchcut's matcher.

The ignore benchmark uses explicit `**/target/` and `**/node_modules/` forms supported by both variants. Slashless directory-ignore correctness is checked separately because the before variant fails it; timing unequal selections would be invalid.

The run records binary hashes and per-source-file SHA-256 manifests. Both variants came from a dirty development tree, so the recorded HEAD is not their complete source identity. The before manifest was captured with the preserved executable before the candidate edits. Compiler, CPU, OS, RAM, Node version, fixture size, raw samples, and workload arguments are retained. The Windows `statfs` type can be 0 (unspecified); do not infer a filesystem name from that field.

## Measurement boundaries

- CLI samples include a fresh process, query compilation, traversal, requested output, and draining a stdout pipe. Variant order alternates on every iteration. The first stdout chunk includes process startup and buffering.
- Warm-engine samples compile a typed query once, then repeat executions within one process. Compilation is recorded separately. Sequential visitors record time to the first match; parallel collection has no first-match metric because its API buffers results.
- Each workload has two warmups and nine measured samples per variant. Raw times, median, P90, P95, standard deviation, and relative dispersion are retained. Filesystem cache state is uncontrolled.
- Opened/pruned directories, inspected entries, matches, and metadata requests accompany the timings. Explicit metadata counters are not a syscall profiler.
- Three separate Windows resource samples collect child CPU time and the greatest observed OS peak working set. The probe polls every 1 ms while the child is alive, so it can miss a final peak. These resource samples are not the headline timing samples. Process memory includes runtime/code overhead and is not an allocation count.

## Accepted algorithm and tradeoff

The retained [48-workload record](https://github.com/codex-mohan/branchcut/blob/master/benchmarks/results/2026-11-11-phase7.json) was captured on 2026-11-11 on Windows/NTFS before the subsequent syntax and traversal gap-closure work. Each row below uses the median of nine fresh-process samples; memory is the median of three separate observed peak working-set samples.

| Corpus | Sorted selection | Before median | Candidate median | Before memory | Candidate memory |
|---|---|---:|---:|---:|---:|
| 10,000 files | First 10 | 75.389 ms | 64.103 ms | 6.36 MiB | 5.32 MiB |
| 10,000 files | First 5,000 | 70.655 ms | 77.556 ms | 6.38 MiB | 5.84 MiB |
| 100,000 files | First 10 | 300.353 ms | 198.010 ms | 13.14 MiB | 5.39 MiB |
| 100,000 files | First 50,000 | 296.422 ms | 329.162 ms | 13.17 MiB | 9.41 MiB |

On the 100,000-file first-ten selection, the observed ratio is 1.52× and observed memory falls by about 59%. Selecting half the matches is about 11% slower. These are workload-specific results, not evidence that Branchcut is the fastest filesystem search tool. Every before/after full selection passed the generator checks; all raw winning and losing samples are retained.

The [A/A preflight record](https://github.com/codex-mohan/branchcut/blob/master/benchmarks/results/2026-11-11-phase7-preflight.json) runs the same preserved binary on both sides with five samples. Its first resource probe could not read peak memory after process exit, so those memory fields are null and are not used in memory claims. The final probe samples while the process is alive.

Sorted limits previously retained every matching path, sorted the full collection, and truncated it. They now maintain a max-heap of at most N native paths. A smaller candidate replaces the greatest retained path; final output sorts the retained selection. The algorithm completes the same full traversal, preserves native ordering and command selection, and bounds retained results by N. It does not make sorted limits an early-stop operation.

The bounded heap targets small sorted selections. Large limits can cost more time than full-vector sorting; retained raw records include that losing case. Unlimited sorting and parallel collection still retain complete result sets.

## Correctness defect found while measuring

The first independent ignore check failed: root `target/` rules left `packages/core/target/generated.rs` eligible. `IgnoreRule::parse` detected separators after appending `/**`, turning a slashless rule into a rooted pattern. The correction detects anchoring before adding the descendant suffix. Directory-only rules now check a non-directory candidate's parent so a regular file or symlink named `target` remains eligible.

The regression was demonstrated failing before the fix, then passing in sequential/parallel traversal, literal-prefix narrowing, and `--why`. The existing supported-semantics oracle remains a separate gate. This correction does not claim complete Git ignore compatibility.

## Rust-level follow-up

A later [52-workload pass](./rust-optimizations.md) measures pathless count delivery, borrowed output paths, reusable matcher scratch, shared ignore contexts, universal-plan short-circuiting, metadata-path avoidance, and parallel count batching. It reports a 1.166x fresh-process geometric mean and a 1.305x warm-engine geometric mean on the 100,000-file tree, with workload-specific losses retained. The default worker policy was subsequently changed to adaptive parallelism; the direct CLI comparison records the final default.

## Remaining measurements and platform gates

This suite is a before/after Branchcut experiment, not a competitor comparison. It does not measure a public repository, a million-file tree, batch child-process composition, hardware cold-cache behavior, or profiler-attributed allocations. Those remain follow-ups before broader performance claims. Linux/macOS CI was not run from the Windows checkout; cross-platform completion remains pending.
