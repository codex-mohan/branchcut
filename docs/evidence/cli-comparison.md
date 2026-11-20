---
title: Direct CLI comparison
description: Same-session comparison of Branchcut, zlob, ripgrep, fd, fast-glob, and tinyglobby using official command-line or package APIs.
icon: Table2
---

## Scope

This is the historical 12 November 2026 snapshot. Later builds have separate dated evidence.

This comparison uses each project's official CLI or public package API. It does not use the development walker adapter for zlob. Every command searches the same generated 100,000-file tree and serializes the selected relative paths.

The retained [raw record](https://github.com/codex-mohan/branchcut/blob/master/benchmarks/results/2026-11-12-direct-cli-default-final.json) contains three warmups, nine measured samples per implementation, executable hashes, package versions, exact failure handling, and every raw timing.

The fixture includes ordinary source, documentation, build-output, dependency, hidden, ignored, and brace-expanded cases. Filenames are ASCII and the filesystem cache is uncontrolled.

## Method

- Every complete result is compared with a generator manifest before timing and after every measured sample.
- Every result is checked for duplicate paths.
- Unordered limits are checked for exact count and membership in the complete valid set.
- Combinations without an equivalent invocation implemented in this runner are not measured (`n/t`); this does not establish feature absence.
- Branchcut default, four-worker, and automatic modes were all measured where legal.
- ripgrep and fd default, one-worker, and four-worker modes were measured for broad, fixed-prefix, and exclusion workloads.
- Times are fresh-process wall-clock milliseconds and include startup, query planning, traversal, filtering, and stdout capture.

## Results

Each cell is the fastest tested configuration for that tool on the workload. The fast-glob and tinyglobby rows include fresh Node process startup and package loading.

| Workload | Matches | Branchcut | zlob | ripgrep | fd | fast-glob | tinyglobby |
|---|---:|---:|---:|---:|---:|---:|---:|
| Broad files | 100,002 | **64.39 ms** auto | 110.75 ms | 98.05 ms | 98.98 ms | 430.70 ms | 264.85 ms |
| Fixed-prefix Rust | 1,250 | **37.64 ms** auto | 37.99 ms | 72.67 ms | 46.59 ms | 237.99 ms | 181.28 ms |
| Rust/TOML with exclusions | 37,501 | **49.70 ms** auto | n/t | 73.35 ms | 88.09 ms | 365.77 ms | 231.10 ms |
| Hidden Rust | 37,503 | **57.50 ms** auto | 99.21 ms | 79.89 ms | 91.14 ms | 347.63 ms | 225.47 ms |
| `.gitignore` Rust | 25,000 | **64.09 ms** default | 90.47 ms | 76.18 ms | 105.95 ms | n/t | n/t |
| Directories | 202 | **51.46 ms** auto | 90.69 ms | n/t | 78.60 ms | 279.47 ms | 152.17 ms |
| Sorted Rust | 37,501 | **92.23 ms** auto | 107.54 ms | 239.39 ms | n/t | n/t | n/t |
| First 10 Rust | 10 | **36.12 ms** default | 104.92 ms | n/t | 44.49 ms | n/t | n/t |
| Broad NUL output | 100,002 | **67.21 ms** auto | n/t | 98.44 ms | 98.14 ms | n/t | n/t |
| Recursive basename Rust | 37,501 | **57.22 ms** default | 92.81 ms | 91.55 ms | 94.62 ms | 349.65 ms | 214.40 ms |

## Worker controls

| Broad-files configuration | Median | P90 |
|---|---:|---:|
| Branchcut automatic (`--threads 0`) | **64.39 ms** | **76.03 ms** |
| Branchcut adaptive default | 67.98 ms | 88.97 ms |
| Branchcut four workers | 78.77 ms | 86.80 ms |
| ripgrep default | 98.05 ms | 111.66 ms |
| fd default | 98.98 ms | 116.79 ms |
| zlob default | 110.75 ms | 133.99 ms |
| fd four workers | 120.87 ms | 130.99 ms |
| ripgrep four workers | 132.67 ms | 139.58 ms |
| fd one worker | 239.38 ms | 264.72 ms |
| ripgrep one worker | 252.19 ms | 291.78 ms |
| tinyglobby | 264.85 ms | 324.64 ms |
| fast-glob | 430.70 ms | 561.26 ms |

Branchcut leads the fixed-prefix workload by 0.9%, which is effectively a tie with zlob. Branchcut is faster on every other tested common workload.

## Interpretation

Branchcut's strongest measured advantages come from combining traversal with query planning: exclusions, hidden-path filtering, `.gitignore`, directory selection, limits, NUL output, and recursive basename matching. Its automatic worker policy also performs well on this Windows/NTFS fixture.

The result is not universal. zlob ties the fixed-prefix case and remains a strong low-level glob and walker implementation. ripgrep and fd remain excellent general-purpose defaults, and fast-glob and tinyglobby provide broader JavaScript APIs than Branchcut's current Rust interface.

Public-repository, Linux, macOS, cold-disk, and memory measurements remain separate gates.
