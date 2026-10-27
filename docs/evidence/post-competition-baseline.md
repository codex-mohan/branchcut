---
title: Post-competition baseline
description: Starting revision, initial local verification, and inherited benchmark evidence.
icon: GitBranch
---

This record separates the initial post-competition checkout from work performed afterward. It is a development baseline, not a fresh reproduction of the published benchmark claims.

## Revisions

- Starting checkout: `070a0cd5f7a42a7ebb0ad33d8e37b8aba9ae0725` (`master` before creating the development branch).
- Preserved `submission-v1` tag: `a00adccdf4e7001088328a5c2321e6d2f5021750`.
- The starting checkout differs from the preserved submission tag. Keep both revisions and the tag unchanged; use the checkout revision for subsequent comparisons.

## Initial local verification

Recorded on 2026-10-27 on Windows. These commands ran against the starting checkout before post-competition source changes:

| Check | Result |
|---|---|
| `cargo fmt -- --check` | Passed |
| `cargo test --locked` | Passed: 19 tests |
| `cargo build --locked --release` | Passed |
| `cargo clippy --locked --all-targets -- -D warnings` | Not run in the initial snapshot |
| Linux/macOS test matrix | Not run locally; rely on CI once its checks are expanded |

Tool versions recorded for the snapshot: `rustc 1.96.0 (ac68faa20 2026-05-25)`, Node.js `v25.2.1`, and Python `3.13.9`.

Passing the initial test suite does not establish that the implementation is correct. The follow-up work tracks concrete regression candidates and will add focused tests before fixing them.

## Inherited competition benchmark evidence

The repository already publishes comparisons on a synthetic 16,000-file Windows workload using `fast-glob@3.3.3`, `tinyglobby@0.2.14`, and a zlob public API path. See [BENCHMARKS.md](../../BENCHMARKS.md) and [comparison notes](../../COMPARISON.md) for the original methodology and limitations.

Those documents describe competition-era measurements, not a reproducible post-competition benchmark run. The original synthetic-corpus generator was not checked in. Add a versioned generator, correctness-first runners, environment capture, and retained raw samples before publishing new performance claims. Do not replace historical results without keeping the methodology and losses visible.

## Initial correctness work queue

These items were identified during code and behavior review. Each needs a regression test and confirmation against a fresh build before it is reported as fixed:

- Shared positive-pattern globstar transitions may admit an unrelated sibling-pattern match.
- `--sort --limit` may stop on discovery order before it can select the global first results.
- Literal-prefix traversal may omit ancestor `.gitignore` context.
- Lossy display-path conversion may change legal filesystem names in output and command arguments.
- Child commands may interpret relative match paths from a working directory different from the query root.
- Documentation describes sequential traversal as iterative; confirm the actual stack behavior and align the claim or implementation.

The pre-existing release executable used during the earlier review was not tied to a known source commit. Treat its observed results as diagnostic leads, not proof about a given revision.
