# Standard-Library Ledger

Branchcut is Rust `std` only. `Cargo.toml` has no runtime crates. The following substitutions are implemented in `src/main.rs`.

| Normally installed | Branchcut replacement | Rationale |
|---|---|---|
| `clap` | `std::env::args_os` plus explicit matching | Preserves non-UTF-8 paths and keeps the CLI grammar small. |
| `glob` | Hand-written `Token`, `Segment`, and parser types | Implements the claimed glob subset without a parser dependency. |
| `globset` | `PatternProgram` shared trie/NFA | Shares common segments across positive and negative patterns. |
| `walkdir` | `std::fs::read_dir` and `DirEntry` | Traverses only directories that can contribute. |
| `ignore` | `std::fs::read_to_string` plus hierarchical `IgnoreRule` state | Loads nested rules, applies ordered overrides, and preserves possible negated descendants. |
| `regex` | Direct wildcard state matching | Avoids compiling path patterns into a general regex engine. |
| `smallvec` | Bounded `Vec` state buffers | The state representation is simple and owned by the single-file engine. |
| `indexmap` / duplicate collection | Shared state graph and sorted output | No duplicate result set is required for multi-pattern matches. |
| `anyhow` | `AppError` implementing `Display` and `Error` | Keeps errors dependency-free and contextual. |
| `thiserror` | Manual error formatting | The binary has one focused error type. |
| `itoa` | `format!` and standard formatting | Statistics use Rust's standard formatting machinery. |
| `rayon` | Sequential traversal plus `std::fs` | Correct sequential planning comes before optional concurrency. |
| `crossbeam-channel` | Not needed | Streaming writes directly through a buffered `std::io::Write`. |
| `path-clean` | `std::path::Component` | Relative path components are normalized without a path crate. |
| `serde_json` | Manual JSON Lines escaping | Emits one valid `{"path":"..."}` object per match without a serializer crate. |
| `duct` / shell command helpers | `std::process::Command` | Executes argv safely without shell interpretation or hidden runtime tools. |
| `humantime` | Not needed | Current P0 does not expose duration predicates. |

## Rule boundary

The event's Rust guidance states that Rust 1.98 has no standard glob, regex, JSON, HTTP, or async runtime. This project writes the glob parser and traversal planner itself and uses the available standard APIs: `std::env`, `std::fs`, `std::ffi`, `std::path`, `std::io`, `std::time`, and `#[test]`.

No third-party implementation was copied into this repository. No executable invokes an external tool at runtime. The only Rust implementation source is `src/main.rs`.

The project does not claim substitutions for features it does not implement: metadata predicates, async traversal, or JSON arrays. Hierarchical `.gitignore`, JSON Lines, and shell-free command execution are implemented directly with `std`.
