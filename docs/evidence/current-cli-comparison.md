---

title: Current CLI comparison

description: Default and tuned CLI timings plus memory for the committed filesystem gap-closure implementation.

icon: Table2

---

This is the **18 November 2026** Windows/NTFS snapshot for source commit `8035ad6e4551b7aade7d0cf77871521fdf02f1c3`. The [raw record](https://github.com/codex-mohan/branchcut/blob/master/benchmarks/results/2026-11-18-gap-closure-cli.json) retains executable hashes, source manifest, versions, invocations, samples and coverage boundaries. The [12 November comparison](./cli-comparison.md) remains historical; its timings are not mixed into this table.

## Method

- The same generated 100,000-file corpus is searched through each official CLI or public Node package API. No zlob walker adapter is used.

- Complete normalized path sets and duplicate prevention are checked before timing and after every sample. Limits are checked for count, uniqueness and membership, since discovery order differs.

- Each timing is a fresh process including startup, planning, traversal, filtering and output capture: three warmups, nine measured samples, median reported. Cache state is uncontrolled. This is not a warm-engine or cold-disk benchmark.

- Windows 11 build 26200, NTFS, Intel Core 5 210H, 12 logical CPUs, 16 GiB RAM; Rust 1.96.0 and Node 25.2.1. Competitors: zlob 1.7.0, ripgrep 15.1.0, fd 10.4.2, fast-glob 3.3.3, tinyglobby 0.2.14.

- `n/t` means no equivalent invocation was implemented in this runner. It is not a claim that a tool lacks the feature. ripgrep is measured only with `--files`; content search is not compared.

- The runtime source was committed before this run. The record's dirty flag includes pending documentation/site work; the runtime manifest identifies the exact measured source.

## Default invocations

No worker override is applied. Branchcut normally chooses up to 16 available workers (12 on this machine), and uses one for an unordered limit unless explicitly overridden. Other tools retain their own defaults.

| Workload | Matches | Branchcut | zlob | ripgrep (`rg --files`) | fd | fast-glob | tinyglobby |
|---|---:|---:|---:|---:|---:|---:|---:|
| Broad files | 100,002 | **68.79 ms** | 123.94 ms | 109.17 ms | 117.14 ms | 370.45 ms | 242.30 ms |
| Fixed-prefix Rust | 1,250 | 40.34 ms | **35.46 ms** | 75.23 ms | 40.91 ms | 210.08 ms | 153.56 ms |
| Rust/TOML with exclusions | 37,501 | **59.27 ms** | n/t | 79.27 ms | 92.57 ms | 333.23 ms | 208.11 ms |
| Hidden Rust | 37,503 | **64.51 ms** | 98.29 ms | 84.53 ms | 94.45 ms | 314.34 ms | 195.53 ms |
| `.gitignore` Rust | 25,000 | **68.29 ms** | 80.07 ms | 77.18 ms | 113.49 ms | n/t | n/t |
| Directories | 202 | **53.76 ms** | 80.35 ms | n/t | 84.66 ms | 251.20 ms | 152.86 ms |
| Sorted Rust | 37,501 | **101.96 ms** | 103.99 ms | 260.04 ms | n/t | n/t | n/t |
| First 10 Rust | 10 | **37.50 ms** | 98.46 ms | n/t | 46.00 ms | n/t | n/t |
| Broad NUL output | 100,002 | **72.61 ms** | n/t | 105.54 ms | 109.36 ms | n/t | n/t |
| Recursive basename Rust | 37,501 | **64.26 ms** | 96.60 ms | 76.10 ms | 94.15 ms | 318.07 ms | 208.26 ms |

## Fastest tested worker configuration

This table selects the lowest median among the recorded worker configurations. It is tuning evidence, not the default-user result. `(4)`, `(1)`, and `(auto)` identify explicit worker controls; no suffix means the default invocation. Explicit alternatives for ripgrep and fd were tested on broad, prefix and exclusion workloads; Branchcut alternatives were tested throughout. This is not an exhaustive tuning search.

| Workload | Matches | Branchcut | zlob | ripgrep (`rg --files`) | fd | fast-glob | tinyglobby |
|---|---:|---:|---:|---:|---:|---:|---:|
| Broad files | 100,002 | **68.02 ms** (auto) | 123.94 ms | 109.17 ms | 117.14 ms | 370.45 ms | 242.30 ms |
| Fixed-prefix Rust | 1,250 | **33.39 ms** (4) | 35.46 ms | 75.23 ms | 39.63 ms (1) | 210.08 ms | 153.56 ms |
| Rust/TOML with exclusions | 37,501 | **58.88 ms** (auto) | n/t | 79.27 ms | 92.57 ms | 333.23 ms | 208.11 ms |
| Hidden Rust | 37,503 | **63.23 ms** (auto) | 98.29 ms | 84.53 ms | 94.45 ms | 314.34 ms | 195.53 ms |
| `.gitignore` Rust | 25,000 | **67.23 ms** (auto) | 80.07 ms | 77.18 ms | 113.49 ms | n/t | n/t |
| Directories | 202 | **53.76 ms** | 80.35 ms | n/t | 84.66 ms | 251.20 ms | 152.86 ms |
| Sorted Rust | 37,501 | **98.62 ms** (auto) | 103.99 ms | 260.04 ms | n/t | n/t | n/t |
| First 10 Rust | 10 | **35.41 ms** (4) | 98.46 ms | n/t | 46.00 ms | n/t | n/t |
| Broad NUL output | 100,002 | **71.57 ms** (auto) | n/t | 105.54 ms | 109.36 ms | n/t | n/t |
| Recursive basename Rust | 37,501 | **60.36 ms** (auto) | 96.60 ms | 76.10 ms | 94.15 ms | 318.07 ms | 208.26 ms |

Branchcut leads nine default rows; zlob leads the default fixed-prefix query. Four-worker Branchcut reverses that fixed-prefix result in this run. Sorted and prefix comparisons are close enough that variation matters. Retaining both tables avoids presenting a tuned result as a default or claiming universal superiority.

## Memory on the same broad-files query

Memory uses separate fresh runs of the **same broad-files invocations and 100,002-path output contract**, with three samples per configuration. The measuring parent captures and validates stdout; its own memory is excluded. The sampler observes the child's Windows OS peak working set at start and approximately every 1 ms, so peaks after the final observation can be missed. This is observed process working set, not allocator usage or Unix RSS. Memory-run timings are not used in the timing tables.

| Tool, default invocation | Median peak | Observed range |
|---|---:|---:|
| Branchcut | 6.38 MiB | 6.36–6.38 MiB |
| zlob | 20.17 MiB | 20.17–20.18 MiB |
| ripgrep (`rg --files`) | 14.29 MiB | 13.96–21.12 MiB |
| fd | 18.25 MiB | 17.86–19.09 MiB |
| fast-glob | 111.43 MiB | 109.96–116.94 MiB |
| tinyglobby | 84.71 MiB | 83.40–91.76 MiB |

All tested worker configurations are retained below. Smaller worker budgets can reduce competitor memory substantially; a default-only comparison does not establish that Branchcut always uses less memory than every configuration.

| Broad-files configuration | Median | P90 | P95 | Peak working set, median |
|---|---:|---:|---:|---:|
| `branchcut` | 68.79 ms | 77.46 ms | 77.46 ms | 6.38 MiB |
| `rg` | 109.17 ms | 118.05 ms | 118.05 ms | 14.29 MiB |
| `fd` | 117.14 ms | 124.11 ms | 124.11 ms | 18.25 MiB |
| `zlob` | 123.94 ms | 137.34 ms | 137.34 ms | 20.17 MiB |
| `fast_glob` | 370.45 ms | 426.88 ms | 426.88 ms | 111.43 MiB |
| `tinyglobby` | 242.30 ms | 283.79 ms | 283.79 ms | 84.71 MiB |
| `branchcut_4` | 95.55 ms | 109.32 ms | 109.32 ms | 5.87 MiB |
| `branchcut_auto` | 68.02 ms | 77.13 ms | 77.13 ms | 6.62 MiB |
| `rg_1` | 239.32 ms | 281.34 ms | 281.34 ms | 6.43 MiB |
| `rg_4` | 118.11 ms | 140.54 ms | 140.54 ms | 12.97 MiB |
| `fd_1` | 246.29 ms | 297.38 ms | 297.38 ms | 10.84 MiB |
| `fd_4` | 138.23 ms | 180.43 ms | 180.43 ms | 13.08 MiB |

## Limits and reproduction

These are ASCII synthetic-tree results on one Windows machine. Public repository, native Linux/macOS timing, cold storage, Unicode-heavy trees and broader metadata workloads need separate evidence. This record does not prove full feature compatibility with any competitor.

The supported-semantics suite separately checks 28 equivalent cases in both traversal modes. One composed-negation case retains an explicit Bash/micromatch dialect difference; it is not counted as fast-glob compatibility.

```sh
cargo build --release --locked
node scripts/compare-cli.mjs --binary target/release/branchcut.exe --zlob PATH_TO_ZLOB --oracle-dir target/oracle --output benchmarks/results/NEW_RECORD.json --sizes 100000 --warmups 3 --iterations 9 --memory-samples 3
```

Install pinned development-only Node oracles and make `rg` and `fd` available as described in the [benchmarking guide](../contributing/benchmarking.md). Use platform-appropriate binary paths. The memory flag currently requires Windows PowerShell. The [preflight run](https://github.com/codex-mohan/branchcut/blob/master/benchmarks/results/2026-11-18-gap-closure-preflight-cli.json) is retained separately; it preceded the last correctness corrections and is not the final result.
