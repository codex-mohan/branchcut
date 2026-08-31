---
title: Standard-library ledger
description: What Branchcut implements with Rust std instead of crates.
icon: Boxes
---

`Cargo.toml` contains an empty `[dependencies]` table. The executable uses Rust `std` only.

| Common dependency | Branchcut implementation |
|---|---|
| `clap` | `std::env::args_os` and explicit parsing |
| `glob` | hand-written tokens, segments, and parser |
| `globset` | shared flat pattern program |
| `walkdir` | `std::fs::read_dir` traversal |
| `ignore` | hierarchical ignore-rule state |
| `regex` | direct wildcard matching |
| `anyhow` / `thiserror` | one manual `AppError` |
| `rayon` | bounded `std::thread` workers |
| `serde_json` | streaming JSON Lines escaping |
| shell helpers | `std::process::Command` |

## Boundary

The documentation site has JavaScript dependencies because it is a separate Fumadocs application. Those packages are not linked into, invoked by, or required to distribute the Branchcut executable.

The project does not pad this ledger with unimplemented substitutions. Metadata predicates, an async runtime, and JSON arrays are not claimed.
