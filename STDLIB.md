# Standard-Library Ledger

Branchcut's core library and ordinary CLI have no third-party crate dependencies. This ledger describes behavior implemented with Rust's standard library; it does not claim full API compatibility with the packages listed.

| Commonly used package | Branchcut implementation | Boundary |
|---|---|---|
| `clap` | `std::env::args_os` and a small explicit parser in `src/cli/args.rs` | Query syntax and option parsing are intentionally focused. |
| `glob` / `globset` | Pattern parser and shared state program in `src/pattern/` | Implements the subset documented in `COMPATIBILITY.md`. |
| `walkdir` | Sequential and bounded parallel traversal using `std::fs::read_dir` in `src/walk/` | Directory symlinks are followed only with explicit `--follow`, with cycle detection. |
| `ignore` | Hierarchical rules and filesystem loading in `src/ignore.rs` | Advanced `.gitignore` escaping and every Git edge case are not claimed. |
| `regex` | Segment tokens and specialized literal/prefix/suffix matchers in `src/pattern/` | This is glob matching, not a regular-expression engine. |
| `smallvec` | Owned `Vec` state and task buffers | No inline-storage or layout compatibility is claimed. |
| `indexmap` / result deduplication | Shared pattern states; optional collection/sorting in CLI and parallel adapters | The core visitor can stream and does not impose output ordering. |
| `anyhow` / `thiserror` | `QueryError` and `std::io::Error` | Query compilation errors and filesystem execution errors retain distinct standard error types. |
| `rayon` | Bounded `std::thread` worker pool | Parallel collection is opt-in and buffers results. |
| `serde_json` / CSV serializers | Path JSON and content JSONL/JSON/CSV/TSV serializers in `src/cli/output.rs` and `src/cli/content_format.rs` | Structured content records use schema version 1 and preserve native path/invalid-text companions. |
| `duct` / shell helpers | `std::process::Command` in `src/cli/exec.rs` and `src/cli/content_inputs.rs` | Explicit execution, preprocessing, or decompression options launch argument-based subprocesses. |
| size / duration parsing crates | Small checked parsers in `src/metadata.rs` | Exact decimal or binary sizes that resolve to whole bytes, fixed durations, and timestamp parsing. |
| Encoding libraries | Streaming decoder in `src/content/encoding.rs` | UTF BOM detection, UTF-16, ISO-8859-1, Windows-1252, and raw bytes. |
| Literal content matchers | Compiled literal matching in `src/content/` | Repeatable literals, line selection, spans, and context with explicit resource bounds. |

## Core library boundary

The public library API consists of `Query`, `CompiledQuery`, `EntryType`, `Stats`, `Visitor`, `Completion`, `SearchReport`, `FilesystemError`, and `CollectedMatches`. `Query::compile()` produces an immutable compiled query. `CompiledQuery::visit()` streams native paths to a visitor. `collect_parallel()` opts into bounded, buffered parallel collection. The core does not print, serialize, or launch commands.

## Optional integration dependencies

`integrations/content-regex` is a separate Cargo package and executable. Its manifest declares the `regex` crate and a path dependency on Branchcut, and its own lockfile pins the resolved graph. `regex` provides regular-expression compilation and matching for filenames, paths, and file contents. The ordinary package does not depend on this integration.

Verify both graphs independently.

```sh
cargo tree -e normal
cargo tree --locked -e normal --manifest-path integrations/content-regex/Cargo.toml
```

External decompression and preprocessing use explicitly requested subprocesses. `--decompress` requires the relevant gzip, bzip2, xz, or zstd executable on `PATH`. `--preprocess PROGRAM` invokes that executable with the selected file as an argument.

No third-party implementation code is copied into this repository. Development-only oracle tools are external to Cargo and are not linked into the library or CLI.
