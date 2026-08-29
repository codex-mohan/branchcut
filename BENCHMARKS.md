# Benchmarks

These measurements compare Branchcut against the pinned `fast-glob@3.3.3` oracle. They are cold end-to-end measurements, not hot engine measurements.

## Environment

- OS: Windows 11 Pro for Workstations
- CPU: Intel Core Ultra 5 210H, x64
- Rust compiler: `rustc 1.96.0 (ac68faa20 2026-05-25)`
- Build: `cargo build --release`
- Artifact: 236,544 bytes
- Dataset: generated temporary corpus, 16,000 files
- Artifact SHA-256: `4feccbc68617b55d8a23082db7c4bc21c6b234e2b32795c4a4ac2b33961c5b14`
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

This is one workload on one Windows machine, not a universal performance claim. The Node oracle was launched as a separate process for every sample, so this is explicitly a cold comparison. A fair hot-engine comparison requires a persistent Node process and a persistent Branchcut benchmark harness.

## Interpretation

The result set comparison passed for this workload and the four previously recorded fixture cases. The benchmark does **not** prove superiority over `tinyglobby`, `globset + walkdir`, or `zlob`.

Keep the query, dataset, output requirements, runtime versions, and commit hash with any future benchmark snapshot. Retain losing results.

## zlob comparison

The official zlob repository was cloned at its current shallow-checkout revision and built with Zig `0.16.0` using `zig build -Doptimize=ReleaseFast`. Its CLI was benchmarked with the same 16,000-file corpus and the equivalent query `**/*.{rs,toml}` with sorted complete output. This raw workload intentionally does not use exclusions because the zlob CLI invocation tested here has no exclusion option.

| Tool | Median | Matches |
|---|---:|---:|
| Branchcut release | 37.51 ms | 13,000 |
| zlob ReleaseFast CLI | 130.11 ms | 13,000 |

Ten cold Windows process launches per tool produced an observed `3.47x` zlob/Branchcut ratio in this workload. This is not a claim that Branchcut is universally faster: zlob supports a broader feature set, has a separate walker API, and its published benchmarks use different workloads and hardware. The zlob CLI was the artifact actually exercised here.

## Reproducibility note

Two clean Windows release builds were tested. Their SHA-256 hashes differed, so Branchcut does **not** claim the optional Reproducible Build bonus. The likely source is nondeterministic linker metadata in the Windows executable; this remains an explicit limitation rather than an unpublished claim.
