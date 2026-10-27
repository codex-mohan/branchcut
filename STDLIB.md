# Standard-Library Ledger

Branchcut's core library and ordinary CLI have no third-party crate dependencies. This ledger describes behavior implemented with Rust's standard library; it does not claim full API compatibility with the packages listed.

| Commonly used package | Branchcut implementation | Boundary |
|---|---|---|
| `clap` | `std::env::args_os` and a small explicit parser in `src/cli/args.rs` | Query syntax and option parsing are intentionally focused. |
| `glob` / `globset` | Pattern parser and shared state program in `src/pattern/` | Implements the subset documented in `COMPATIBILITY.md`. |
| `walkdir` | Sequential and bounded parallel traversal using `std::fs::read_dir` in `src/walk/` | Directory symlinks are not followed. |
| `ignore` | Hierarchical rules and filesystem loading in `src/ignore.rs` | Advanced `.gitignore` escaping and every Git edge case are not claimed. |
| `regex` | Segment tokens and specialized literal/prefix/suffix matchers in `src/pattern/` | This is glob matching, not a regular-expression engine. |
| `smallvec` | Owned `Vec` state and task buffers | No inline-storage or layout compatibility is claimed. |
| `indexmap` / result deduplication | Shared pattern states; optional collection/sorting in CLI and parallel adapters | The core visitor can stream and does not impose output ordering. |
| `anyhow` / `thiserror` | `QueryError` and `std::io::Error` | Query compilation errors and filesystem execution errors retain distinct standard error types. |
| `rayon` | Bounded `std::thread` worker pool | Parallel collection is opt-in and buffers results. |
| `serde_json` | JSON Lines output serializer in `src/cli/output.rs` | Core traversal exposes native `Path` values and leaves serialization to callers. |
| `duct` / shell helpers | `std::process::Command` in `src/cli/exec.rs` | Only the explicit CLI `--exec` option launches the executable selected by the user; no shell is used. |
| `humantime` | Not needed yet | Size and modification-time predicates are planned, not currently implemented. |

## Core library boundary

The public library API consists of `Query`, `CompiledQuery`, `EntryType`, `Stats`, `Visitor`, `Completion`, `SearchReport`, `FilesystemError`, and `CollectedMatches`. `Query::compile()` produces an immutable compiled query. `CompiledQuery::visit()` streams native paths to a visitor. `collect_parallel()` opts into bounded, buffered parallel collection. The core does not print, serialize, or launch commands.

An external consumer can add a separately built integration when a concrete workflow calls for it. Such dependencies do not belong in the core dependency graph.

No third-party implementation code is copied into this repository. Development-only oracle tools are external to Cargo and are not linked into the library or CLI.
