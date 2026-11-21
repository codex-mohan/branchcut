# Benchmarks

The competition-era results below are retained as historical evidence. Current development uses the checked-in [benchmark and oracle runners](docs/contributing/benchmarking.md). Raw post-competition records live in [benchmarks/results](benchmarks/results/).

The [18 November 2026 CLI comparison](docs/evidence/current-cli-comparison.md) measures six tools on filesystem-selection workloads. It separates default invocations from tuned worker settings and reports observed working set on the same query.

The [2026-11-11 native-tool investigation](docs/evidence/search-performance.md) includes fresh ripgrep/fd/fast-glob comparisons, equal-worker controls, and source references. That early snapshot preceded the adaptive-worker default and should not be used as the current out-of-box ranking.

The historical [12 November direct CLI comparison](docs/evidence/cli-comparison.md) uses official `zlob`, `rg`, `fd`, fast-glob, and tinyglobby entry points without a lower-level walker adapter. It records all tested worker configurations and verifies every selected path set.

## v0.3.0 literal content search

These medians are process-inclusive wall-clock milliseconds on Windows 11 and NTFS. The fixture is 96 MiB across 96 log files, or one 8 MiB shard in the last row. The runner interleaves two warmups and seven timed samples per configuration. It checks the exact normalized path, line, byte column, and text from every invocation. Cache state was uncontrolled. Ripgrep 15.1.0 ran with `-F --no-mmap --no-encoding` on ASCII input. The Branchcut baseline is the original local literal implementation at `55bf549`, rebuilt with the same release compiler as `34923d9`.

| Search | Branchcut baseline (1) | Branchcut v0.3.0 (1) | ripgrep (1) | Branchcut baseline (8) | Branchcut v0.3.0 (8) | ripgrep (8) |
|---|---:|---:|---:|---:|---:|---:|
| Absent literal | 115.89 | 95.75 | 61.21 | 120.44 | 47.49 | 40.87 |
| Sparse literal | 94.58 | 71.17 | 47.19 | 94.74 | 32.82 | 34.76 |
| Eight literals | 768.31 | 156.65 | 90.37 | 768.43 | 76.83 | 62.82 |
| Narrow shard | 62.11 | 59.84 | 50.29 | 62.61 | 52.03 | 54.73 |

Branchcut shares matching work across fixed patterns, borrows complete lines, and skips full lines in buffers with no candidate bytes. Ripgrep searches a buffered slice for a candidate before locating and confirming its line. Ripgrep led all single-worker rows and the eight-worker absent and eight-literal rows. [Ripgrep candidate-line implementation](https://github.com/BurntSushi/ripgrep/blob/15.1.0/crates/searcher/src/searcher/core.rs)

See the [runner](scripts/benchmark-content.mjs) and [raw samples](benchmarks/results/content-v0.3.0-windows.json) for commands, hashes, environment, and sample dispersion.

## Post-competition module extraction

For the later 48-workload matrix and bounded sorted-selection tradeoff, see [Phase 7 measurements](docs/evidence/phase7.md). The first-ten result improves on the tested synthetic tree while large sorted selections lose time. These before/after records do not establish a competitor ranking.

The subsequent [Rust-level optimization pass](docs/evidence/rust-optimizations.md) records 52 workloads across 10,000- and 100,000-file trees. On the larger tree, broad count improved 1.43x, broad streamed output 1.30x, ignore-heavy traversal 1.35x, and four-worker output 1.36x. Metadata filtering is effectively unchanged, and unlimited sorting remains slightly slower. The raw record includes every workload and both named count/output versions. The current CLI subsequently made adaptive worker selection the default; use the 18 November comparison for current out-of-box results.

On 2026-10-28, the same runner measured the preserved pre-module binary and the extracted-library binary on Windows/NTFS. Both returned the same 10,000 paths on a generated 16,000-file corpus before timing. Each engine used two fresh-process warmups and ten samples, alternating execution order. Filesystem cache state was uncontrolled.

| Branchcut build | Median | P90 | P95 |
|---|---:|---:|---:|
| Pre-module binary, source `28109b1` | 62.416 ms | 64.410 ms | 67.587 ms |
| Extracted-library binary | 63.290 ms | 67.952 ms | 68.683 ms |

The median difference is about 1.4%, and the distributions overlap. This run supports no performance-improvement claim. It also does not establish behavior on other workloads or operating systems. Raw samples, binary labels, hashes, dirty-tree state, oracle timings, and environment details are retained in [pre-module](benchmarks/results/2026-10-28-pre-modular.json) and [post-module](benchmarks/results/2026-10-28-post-modular.json) records. These snapshots preceded the follow-up parallel-error regression fixes.

## Historical competition measurements

These measurements compare Branchcut against the pinned `fast-glob@3.3.3` oracle. They are cold end-to-end measurements, not hot engine measurements.

## Environment

- OS: Windows 11 Pro for Workstations
- CPU: Intel Core Ultra 5 210H, x64
- Rust compiler: `rustc 1.96.0 (ac68faa20 2026-05-25)`
- Build: `cargo build --release`
- Artifact: 236,544 bytes
- Dataset: generated temporary corpus, 16,000 files
- Artifact SHA-256: `bf1747ad90e0b1f371a7930cdb663f5d4aa63fcc68c9893247f32e1babf62f64`
- Node oracle runtime: `v25.2.1`
- Timing: parent-process wall-clock duration, including process startup

## Dataset shape

Twenty package directories were generated. Each package contained:

- `src/`: 250 `.rs` and 250 `.toml` files
- `target/debug/`: 150 generated `.rs` files
- `node_modules/dep/`: 150 `.ts` files

Generation is intentionally not committed. The exact generator used 20 packages and those counts.

| Workload | Matches | Branchcut median | fast-glob median | Branchcut P90 | fast-glob P90 |
|---|---:|---:|---:|---:|---:|
| `**/*.{rs,toml}`, exclude `target` and `node_modules` | 10,000 | 24.99 ms | 148.53 ms | 27.23 ms | 168.10 ms |

Representative statistics from the release binary:

```text
single-pattern workload
matched                 0
directories considered  101
directories opened      81
directories pruned      20
entries inspected       13100
candidate files         13000
metadata calls          1
filesystem errors       0
elapsed                 15.775ms

100-pattern workload
matched                 0
directories considered  121
directories opened      121
directories pruned       0
entries inspected       16120
candidate files         16000
metadata calls          1
filesystem errors       0
elapsed                 32.963ms
```

Ten cold process launches were measured for each tool. Both tools emitted the complete sorted result set; both returned exactly 10,000 matches. The parent process measured wall-clock duration including process startup, module loading, filesystem traversal, sorting, and output capture.

The observed cold-process ratio for this workload was:

```text
148.53 ms / 24.99 ms = 5.94x Branchcut advantage
```

This is one workload on one Windows machine, not a universal performance claim. The Node oracle was launched as a separate process for every sample, so this is explicitly a cold comparison.

## Interpretation

The result set comparison passed for this workload and the four previously recorded fixture cases. The benchmark does **not** prove superiority over `tinyglobby` or `globset + walkdir`.

Keep the query, dataset, output requirements, runtime versions, and commit hash with any future benchmark snapshot. Retain losing results.

## zlob hot-engine comparison

The official zlob release tag `v1.6.3` was built with Zig `0.16.0` using `zig build -Doptimize=ReleaseFast`. A temporary in-process Zig harness called zlob's public `match` filesystem API with `std.heap.c_allocator`, 3 warmups, and 10 measured iterations. Branchcut used `--count --stats`; its reported `elapsed` value begins after argument parsing and planning, so process startup and output serialization are excluded.

Workload:

```text
16,000-file corpus
pattern: **/*.{rs,toml}
13,000 matches
3 warmup iterations
10 measured iterations
count-only output
```

| Engine | Median / average per query | Matches |
|---|---:|---:|
| Branchcut traversal (`--count --stats`) | 22.079 ms median | 13,000 |
| zlob v1.6.3 public `match` API | 133.408 ms average | 13,000 |

Observed startup-excluded ratio:

```text
133.408 / 22.079 = 6.04x
```

This is a narrow Windows workload. zlob's official README describes additional SIMD, platform-specific, and parallel walker paths that were not exercised by this direct single-threaded `match` API harness. Its official C-compatible benchmark path returned zero matches for the Windows drive-letter path, and the official tree-size benchmark is disabled on Windows because it requires libc glob; neither invalid result was used.

## Reproducibility note

Two clean Windows release builds were tested. Their SHA-256 hashes differed, so Branchcut does **not** claim the optional Reproducible Build bonus. The likely source is nondeterministic linker metadata in the Windows executable; this remains an explicit limitation rather than an unpublished claim.
