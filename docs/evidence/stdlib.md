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
| size / duration parsing crates | Exact decimal/binary byte thresholds and fixed-duration/timestamp parsers in `src/metadata.rs` |

## Boundary

The documentation site has JavaScript dependencies because it is a separate Fumadocs application. Those packages are not linked into, invoked by, or required to distribute the Branchcut executable.

Content serialization implements schema-versioned JSONL, JSON documents, CSV, and TSV in `src/cli/content_format.rs`. The streaming decoder provides UTF BOM detection, UTF-16, Latin-1, Windows-1252, and raw input.

`integrations/content-regex` is a separate package whose manifest declares `regex` and whose own lockfile pins the dependency graph. It adds filename/path and line-oriented content regex to the `branchcut-regex` executable. The core dependency graph is empty. See `STDLIB.md` in the repository and the [content guide](../guides/content-search.md).
