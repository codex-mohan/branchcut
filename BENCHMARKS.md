# Benchmarks

These are initial Branchcut-only measurements, retained as a baseline. They are not competitor claims.

## Environment

- OS: Windows 11 Pro for Workstations
- CPU: Intel Core Ultra 5 210H, x64
- Rust compiler: `rustc 1.96.0 (ac68faa20 2026-05-25)`
- Build: `cargo build --release`
- Artifact: 236,544 bytes
- Artifact SHA-256: `971f67964b7daf9e58511ca78aa5f71ae5b4c30983115abcbdbbf6367a17273a`
- Dataset: generated temporary corpus, 16,000 files
- Repetitions: 12 process launches per workload
- Timing: parent-process wall-clock duration, including process startup

## Dataset shape

Twenty package directories were generated. Each package contained:

- `src/`: 250 `.rs` and 250 `.toml` files
- `target/debug/`: 150 generated `.rs` files
- `node_modules/dep/`: 150 `.ts` files

Generation is intentionally not committed. The exact generator used 20 packages and those counts.

## Results

| Workload | Matches | Median | P90 | Directories opened | Directories pruned |
|---|---:|---:|---:|---:|---:|
| `packages/**/src/**/*.never`, exclude target, one pattern | 0 | 27.25 ms | 30.85 ms | 81 | 20 |
| 100 absent patterns under `packages/**/src/**` | 0 | 46.22 ms | 50.98 ms | 121 | 0 |

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

The first invocation in each set included one-time OS/filesystem/cache effects; medians are reported across all 12 launches. These are cold end-to-end measurements, not hot engine measurements.

## Interpretation

The current result proves that the release binary runs and reports planner counters. It does **not** prove superiority over `fast-glob`, `tinyglobby`, `globset + walkdir`, or `zlob`.

Before publishing a speed claim, run equivalent result-set differential checks and report cold and hot measurements under identical output, query, dataset, and filesystem conditions. Keep losing results.

## Reproducibility note

Two clean Windows release builds were tested. Their SHA-256 hashes differed, so Branchcut does **not** claim the optional Reproducible Build bonus. The likely source is nondeterministic linker metadata in the Windows executable; this remains an explicit limitation rather than an unpublished claim.
