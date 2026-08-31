---
title: Benchmarks
description: Published workload, methodology, results, and limitations.
icon: Gauge
---

Performance claims are tied to a documented synthetic corpus and verified match sets. They are not claims of universal superiority.

## Environment

- Windows 11 Pro for Workstations
- Intel Core Ultra 5 210H, x64
- Rust 1.96.0 release build
- Node 25.2.1
- `fast-glob` 3.3.3
- tinyglobby 0.2.14
- zlob v1.6.3 built with Zig 0.16.0 `ReleaseFast`

## Corpus

```text
20 packages
├── src/                 250 .rs + 250 .toml each
├── target/debug/        150 generated .rs each
└── node_modules/pkg/    150 .ts each

total: 16,000 files
```

## Hot query

Pattern: `**/*.{rs,toml}`. Hidden files excluded. Count-only output. Three warmups and ten measurements where supported.

| Engine | Median / average | Matches |
|---|---:|---:|
| Branchcut | **22.079 ms median** | 13,000 |
| tinyglobby 0.2.14 | 35.666 ms median | 13,000 |
| fast-glob 3.3.3 | 37.528 ms median | 13,000 |
| zlob v1.6.3 public `match` API | 133.408 ms average | 13,000 |

The Branchcut measurement uses traversal elapsed time reported by `--stats` after argument parsing and planning. Node modules remain loaded for hot measurements. The zlob harness invokes its public filesystem API in one process.

## Cold query

The complete sorted 10,000-match exclusion workload measured fresh process startup and output capture:

| Tool | Median |
|---|---:|
| Branchcut release | 24.99 ms |
| fast-glob 3.3.3 | 148.53 ms |

This is explicitly a cold CLI comparison.

## Filesystem-work evidence

For the exclusion-heavy workload, Branchcut pruned 20 directories and performed one root metadata call. Cross-tool pruning counters are not invented where competitors do not expose equivalent instrumentation.

## Read the numbers correctly

- Result sets are checked before timing.
- Hot and cold measurements are labeled separately.
- The zlob measurement does not exercise its separate parallel walker.
- The corpus is synthetic and Windows-specific.
- Losing future results must remain published.
- Windows release builds were not byte-for-byte reproducible, so no reproducible-build claim is made.
