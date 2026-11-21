---
title: Search performance investigation
description: Fresh native-tool comparisons, equal-worker controls, and source-backed bottleneck hypotheses.
icon: SearchCheck
---

## What the measurements establish

Branchcut is not uniformly slower than mature file finders. Its default worker policy explains much of the broad-search loss on the tested Windows tree: Branchcut defaults to one worker, while ripgrep and fd choose multiple workers. Equal-worker runs reverse the ranking on these fixtures. No result establishes a universal fastest engine.

The [initial comparison](https://github.com/codex-mohan/branchcut/blob/master/benchmarks/results/2026-11-11-current-tools.json) has 10,000- and 100,000-file trees, two warmups and nine samples. The [thread-controlled comparison](https://github.com/codex-mohan/branchcut/blob/master/benchmarks/results/2026-11-11-thread-matrix.json) repeats the 100,000-file tree with two warmups and seven samples, adding automatic/four-worker Branchcut and one/four-worker ripgrep/fd variants. Every complete output set is checked against the generator before timing and on every sample. All tools serialize all matching relative paths to a captured pipe; none is asked to sort.

The comparisons use Windows/NTFS, ripgrep 15.1.0, fd 10.4.2, and fast-glob 3.3.3. These are fresh-process timings including startup and output, with uncontrolled filesystem caches. They are not hot-engine or cold-disk measurements. The records include raw samples, executable hashes, dirty-tree source manifests, CPU/OS/runtime details, and commands. The measured source manifests distinguish snapshots from subsequent corrections. These trees have ordinary ASCII names, no symlinks, and no ignore files; they do not establish compatibility beyond those choices. Tools use their own default case modes, which do not affect this all-lowercase fixture.

## Equal-worker results

Median milliseconds; lower is better. The fixed-prefix query selects `packages/p00/src/**/*.rs`. The exclusion query selects Rust/TOML files outside build and dependency trees.

| 100,000-file workload | Branchcut 1 | ripgrep 1 | fd 1 | Branchcut 4 | ripgrep 4 | fd 4 |
|---|---:|---:|---:|---:|---:|---:|
| All 100,000 files | 216.27 | 302.81 | 287.21 | 104.34 | 157.99 | 156.07 |
| Fixed prefix: 1,250 matches | 42.40 | 202.64 | 57.76 | 37.19 | 112.95 | 59.20 |
| Extensions/exclusions: 37,500 matches | 146.22 | 218.59 | 261.65 | 82.40 | 116.47 | 119.91 |

For all files, the four-worker Branchcut median is about 1.51× faster than four-worker ripgrep and 1.50× faster than four-worker fd on this fixture. Broad-query P95 values are 117.06, 169.37, and 163.23 ms respectively. The default/automatic result still matters to users:

| Workload | Branchcut default | Branchcut auto | ripgrep default | fd default | fast-glob cold |
|---|---:|---:|---:|---:|---:|
| All files | 216.27 | 143.60 | 110.52 | 112.65 | 469.32 |
| Fixed prefix | 42.40 | 43.05 | 85.63 | 55.81 | 229.80 |
| Extensions/exclusions | 146.22 | 62.28 | 85.03 | 99.32 | 410.27 |

`--threads 0` requests available parallelism capped at 16; its exact Rust-reported availability is not recorded by the CLI. This machine reports 12 logical processors. Four workers beat automatic sizing on the high-output broad query, while automatic sizing wins on the lower-output exclusion query. This is evidence against simply choosing the largest worker count for every query. It is not enough evidence to change the product default across all machines and tree shapes.

## Source review

The owner explicitly requested studying competitor source. It is retained only under ignored `target/perf-reference` as research material; no implementation is copied into Branchcut and no third-party production dependency is introduced.

Pinned revisions:

- ripgrep 15.1.0: `af60c2de9d85e7f3d81c78601669468cf02dabab`.
- fd 10.4.2: `7027d45303b412be6fa9c09d689cc6276748fb38`.
- zlob 1.7.0: `5979979703ba21c2b47645aced50aac59ab1bca9`.

| Observation | Evidence | Interpretation |
|---|---|---|
| Default worker counts differ | [ripgrep high-level arguments](https://github.com/BurntSushi/ripgrep/blob/af60c2de9d85e7f3d81c78601669468cf02dabab/crates/core/flags/hiargs.rs#L168), [fd defaults](https://github.com/sharkdp/fd/blob/7027d45303b412be6fa9c09d689cc6276748fb38/src/cli.rs#L766), Branchcut `cli/args.rs` | Confirmed source difference with a measured effect in the controlled run |
| Broad positive matching allocates fresh state vectors | Branchcut `pattern/program.rs::advance_with_case` | Concrete extra work per entry; its share of elapsed time is not yet profiler-attributed. Empty vectors with capacity zero do not heap-allocate |
| Count output clones a path even when it need not serialize it | Branchcut `cli/output.rs::matched` and `output_path` | Removable work in count-only workloads; cannot explain full-output results by itself |
| Output handoff strategies differ | [fd batch sender](https://github.com/sharkdp/fd/blob/7027d45303b412be6fa9c09d689cc6276748fb38/src/walk.rs#L74), [ripgrep parallel file listing](https://github.com/BurntSushi/ripgrep/blob/af60c2de9d85e7f3d81c78601669468cf02dabab/crates/core/main.rs#L271) | fd batches results; Branchcut sends individual paths. Ripgrep also sends individual results, so a channel per path alone is not proof of the gap |
| Queue accounting and ignore context differ | [ripgrep walker](https://github.com/BurntSushi/ripgrep/blob/af60c2de9d85e7f3d81c78601669468cf02dabab/crates/ignore/src/walk.rs#L1518), [shared ignore context](https://github.com/BurntSushi/ripgrep/blob/af60c2de9d85e7f3d81c78601669468cf02dabab/crates/ignore/src/dir.rs#L94) | Branchcut uses globally synchronized queue accounting and cloned rule vectors. These need contention/ignore-heavy measurements; there are no ignore files in this comparison |
| Matcher strategies are specialized elsewhere too | [globset compilation](https://github.com/BurntSushi/ripgrep/blob/af60c2de9d85e7f3d81c78601669468cf02dabab/crates/globset/src/lib.rs#L472) | Literal/basename/extension/prefix/suffix strategies and reusable scratch are useful references. Their existence is not evidence that regex or dependencies explain the observed ranking |

Broad `**/*` queries provide little opportunity to prune: every eligible directory must be read and every returned path delivered. Branchcut's literal-root advantage matters more for scoped queries. The source identifies allocation, queue, and output costs to investigate after correcting the worker-count comparison, not a proven percentage breakdown of runtime.

## Current zlob native walker reproduction

The [native-walker record](https://github.com/codex-mohan/branchcut/blob/master/benchmarks/results/2026-11-11-zlob-native.json) adds the official zlob 1.7.0 ReleaseFast library, built with Zig 0.16.0 at the pinned revision above. The separately built [public-C-API probe](https://github.com/codex-mohan/branchcut/blob/master/benchmarks/probes/README.md) uses `zlob_walk_collect` for path output and the Windows `windows_ntdll` backend. It requests no optional metadata, no ignores or symlink following, excludes hidden paths, and emits only regular files. Every complete path set agrees with the generator and the other tools.

These are seven-sample fresh-process medians including full output on the same 100,000-file tree. Zlob collects before output while Branchcut streams, so this does not claim equivalent memory behavior. Other native tools were rerun in the same rotation; the earlier thread-controlled record remains separate rather than being averaged into this run.

| Query | Branchcut 1 | zlob 1 | Branchcut 4 | zlob 4 | Branchcut auto | zlob auto |
|---|---:|---:|---:|---:|---:|---:|
| All 100,000 files | 162.21 ms | 139.79 ms | 109.87 ms | 89.48 ms | 177.31 ms | 77.11 ms |
| Fixed prefix, 1,250 matches | 41.82 ms | 37.91 ms | 34.23 ms | 39.60 ms | 42.21 ms | 38.25 ms |

Zlob wins the broad query at equal worker counts: about 1.16× at one worker and 1.23× at four. Its automatic setting is about 1.42× faster than Branchcut's best tested setting on this query. The narrow-query timings are close and do not justify a broad superiority claim. The explicit-exclusion workload is skipped for zlob because this adapter accepts one include pattern and no excludes; it must not be timed with a different result set.

The directory batches and suffix specialization are confirmed in source. This experiment still does not isolate the percentage attributable to enumeration, allocation, scheduling, or conversion. The SDK's published hot-walker figures below are a different measurement boundary, platform, and corpus; do not divide them by these CLI/probe medians.

## Published landscape

These are author-published results and architecture descriptions, not independently reproduced rankings:

| Project | Relevant evidence | Boundary |
|---|---|---|
| [zlob 1.7.0](https://github.com/dmtrKovalenko/zlob/tree/v1.7.0) | Published warm-cache 185,737-entry plain walk: 27.4 ms versus walkdir 90.9 ms; parallel ignored walk 0.64 ms versus ignore 4.9 ms. Uses native enumeration and specialized matching | Published walker figures use best of five on another machine; CLI `match` and the newer parallel walker are distinct execution paths |
| [Ferralk](https://github.com/sebastian-software/ferralk) | Pattern-directed parallel traversal, raw/native paths, and Git-oracle validation; published 53,601-file macOS fixture compares portable/native walkers and zlob | Very close architectural comparison for Branchcut; its same-host numbers cannot be divided by our Windows CLI times |
| [ripgrep](https://github.com/BurntSushi/ripgrep), [fd](https://github.com/sharkdp/fd) | Mature traversal and filename filtering; directly measured above | `rg --files` is a listing path, not content searching. Their content-search speed is a different workload |
| [ugrep](https://github.com/Genivia/ugrep) | Author-maintained grep benchmarks and rich content searching | Grep/content results do not rank filename globbing |
| [hypergrep](https://github.com/p-ranav/hypergrep) | Hyperscan-based content search with SIMD/threading and published wins on selected patterns | Its README comparisons include older ripgrep/ugrep releases and Linux-specific assumptions |
| [fff](https://github.com/dmtrKovalenko/fff) | Repeated search over a resident index with watcher and content-index features | Warm indexed lookup excludes initial indexing/startup and is not an equivalent fresh filesystem query |

Zlob's current source selects native scanners in [`walker/scan.zig`](https://github.com/dmtrKovalenko/zlob/blob/5979979703ba21c2b47645aced50aac59ab1bca9/src/walker/scan.zig): Linux batched directory reads, macOS bulk attributes when requested, and Windows directory batches carrying metadata. Its [`compiled_pattern.zig`](https://github.com/dmtrKovalenko/zlob/blob/5979979703ba21c2b47645aced50aac59ab1bca9/src/compiled_pattern.zig) has specialized suffix and grouped-suffix strategies. These are credible reasons to benchmark the newer walker, not evidence that Branchcut needs unsafe code or copied platform layers.

## Next experiments

Keep defaults unchanged until representative public-repository and cross-platform evidence supports an adaptive worker policy. Then measure reusable positive-state buffers, count-only path avoidance, and bounded result batches independently. Preserve first-match latency, cancellation/backpressure behavior, result equality, and losing cases. Native filesystem backends remain a later option only if measurements show the standard-library boundary dominates.

Reproduce the thread matrix after building a release executable:

```powershell
node scripts/compare-current.mjs --binary target/release/branchcut.exe --oracle-dir target/oracle --sizes 100000 --warmups 2 --iterations 7 --thread-matrix --output benchmarks/results/local-thread-matrix.json
```

No Linux/macOS matrix is implied by these Windows tables. Public-repository measurements, a hot-process zlob/Branchcut comparison, and a zlob exclusion-capable adapter remain separate gates.
