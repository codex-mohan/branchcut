---
title: Correctness
description: Test categories, differential protocol, and compatibility evidence.
icon: ShieldCheck
---

Correctness precedes timing. Every benchmarked query must return an equivalent result set before its duration is considered valid.

## Local quality gates

```bash
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
cargo metadata --no-deps --format-version 1
```

## Inline regression suite

Unit and command-line tests cover:

- literal, star, question, class, range, and negated-class matching;
- zero-or-many globstar components;
- flat brace expansion and invalid pattern diagnostics;
- shared program construction and common-prefix boundaries;
- positive and exclusion pruning;
- hidden paths, extension filters, and literal simple search;
- early limits, globally sorted limits, count mode, JSON path escaping, and native command-path substitution;
- hierarchical ignore precedence and re-inclusion;
- deep traversal and broken output pipes;
- parallel/sequential set equality;
- Unix symlink and non-UTF-8 behavior where supported by the platform. Windows-native path handling is implemented separately and needs Windows CI coverage to establish it.

Run the local gates with `cargo fmt -- --check`, `cargo test --locked`, and `cargo clippy --locked --all-targets -- -D warnings`. Cross-platform behavior is not established by a Windows-only run; Linux and macOS checks belong in CI.

## Differential protocol

Development-only oracles live outside the Rust package:

```text
fixture tree
  ├── Branchcut
  ├── fast-glob 3.3.3
  └── tinyglobby 0.2.14
          │
          ▼
 normalize separators and compare sets
```

The competition-era recorded fixture and 16,000-file corpus cases passed normalized set equality for the claimed overlapping syntax. Those results predate the current modular source and are not a fresh differential run of this revision.

An observed zlob nested-globstar mismatch on Windows remains documented instead of being removed from the comparison.
