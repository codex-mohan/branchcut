<div align="center">

# Branchcut

**Compile the query, cut the tree.**

[![Rust](https://img.shields.io/badge/Rust-std_only-000000?style=flat-square&logo=rust)](https://www.rust-lang.org/)
![Dependencies](https://img.shields.io/badge/runtime_dependencies-0-2ea44f?style=flat-square)
![Source](https://img.shields.io/badge/Rust_source_files-1-4c1?style=flat-square)
![Track](https://img.shields.io/badge/Zero_Dependency-Track_A-6f42c1?style=flat-square)

</div>

Branchcut is a single-file, zero-crate Rust filesystem query engine. It compiles positive globs, exclusions, file predicates, and termination rules into one traversal plan. The planner narrows literal roots, shares common pattern segments, and prunes directories that cannot contribute to the result.

It targets workflows normally implemented with `fast-glob`, `globset`, `walkdir`, `ignore`, `regex`, and a CLI framework—without shipping any of them.

## Why Branchcut

A conventional filesystem query often performs these stages independently:

```text
walk every entry -> glob -> exclude -> inspect -> filter -> collect
```

Branchcut instead compiles one query:

```text
positive patterns
+ exclusions
+ type and extension filters
+ hidden-path policy
+ first/limit termination
        |
        v
shared pattern program
+ narrowed traversal root
+ subtree feasibility checks
        |
        v
open only directories that may contribute
```

The goal is not to claim universal superiority over mature globbers. The goal is to perform less irrelevant filesystem work on planning-heavy queries, then publish correctness and performance evidence honestly.

## Current Features

- Segment syntax: `*`, `?`, `[abc]`, `[a-z]`, `[!abc]`
- Path-component globstar: `**`
- One common, non-nested brace group such as `*.{js,ts}`
- Repeatable positive patterns
- Repeatable exclusions with safe trailing-globstar subtree pruning
- Shared pattern-state graph for common pattern segments
- Literal-prefix traversal root selection
- Specialized literal, prefix, and suffix segment matchers
- File, directory, and symlink filtering
- Repeatable extension filters
- Hidden-path exclusion by default
- Streaming output by default
- Deterministic global sorting on request
- Immediate `--first` and `--limit` termination when streaming
- Planner and traversal diagnostics through `--explain` and `--stats`
- Strict and best-effort filesystem error policies
- Hierarchical nested `.gitignore` support with ordered negation/re-inclusion
- Streaming JSON Lines output with `--json`
- Shell-free per-match command execution with `--exec "command {}"`
- No directory-symlink traversal
- Raw non-UTF-8 path support on Unix

## Build

### Prerequisite

Rust with Cargo. Development and release gates have been exercised with Rust 1.96.0. The hackathon reference toolchain is Rust 1.98.0.

### One-command release build

```bash
cargo build --release
```

Result:

```text
Windows: target/release/branchcut.exe
Unix:    target/release/branchcut
```

The release profile enables optimization level 3, fat LTO, one codegen unit, symbol stripping, and abort-on-panic.

## Quick Start

PowerShell:

```powershell
.\target\release\branchcut.exe --glob "src/**/*.rs"
```

Unix shell:

```bash
./target/release/branchcut --glob 'src/**/*.rs'
```

Simple filename search treats the input literally:

```bash
branchcut config
```

This finds file names containing the case-sensitive text `config`; glob metacharacters inside a simple search are not interpreted.

## Query Examples

Find Rust and TOML files beneath `src`:

```bash
branchcut --glob 'src/**/*.{rs,toml}'
```

Combine shared-prefix patterns:

```bash
branchcut \
  --glob 'src/**/*.rs' \
  --glob 'src/**/*.toml' \
  --glob 'src/**/test*.rs'
```

Exclude complete subtrees:

```bash
branchcut \
  --glob '**/*.{rs,ts}' \
  --exclude '**/target/**' \
  --exclude '**/node_modules/**' \
  --exclude '**/dist/**'
```

Filter by extension and type:

```bash
branchcut -e rs -e toml --type file
```

Search another root:

```bash
branchcut --cwd /path/to/repository --glob 'packages/**/src/**/*.rs'
```

Stop immediately after a match:

```bash
branchcut --glob '**/*.rs' --first
branchcut --glob '**/*.rs' --limit 100
```

Sort globally, then apply the limit:

```bash
branchcut --glob '**/*.rs' --sort --limit 100
```

Sorting necessarily collects all matching paths before truncation. Without `--sort`, output streams and the limit stops traversal immediately.

Count matches without serializing paths:

```bash
branchcut --glob '**/*.rs' --count
```

Include hidden paths:

```bash
branchcut --glob '**/*' --hidden
```

Fail if any filesystem entry cannot be read:

```bash
branchcut --glob '**/*.rs' --strict
```
Without `--strict`, Branchcut reports unreadable entries to stderr, continues, and exits successfully with every result it could inspect.

Apply hierarchical `.gitignore` rules:

```bash
branchcut --glob '**/*' --gitignore
```

Rules are read when each directory is entered. Parent rules remain active, child rules are appended, and the last matching rule wins. Negated rules such as `!keep.txt` are honored conservatively; a directory is kept open when a descendant might be re-included.

Stream JSON Lines:

```bash
branchcut --glob '**/*.rs' --json
```

Each match is emitted as one valid object:

```json
{"path":"src/main.rs"}
```

Run a command once per match without invoking a shell:

```bash
branchcut --glob '**/*.tmp' --exec 'rm {}'
```

`{}` is replaced with the displayed relative path. Shell pipelines, redirections, environment expansion, and shell built-ins are intentionally unsupported.


## Query Planning

Inspect compiler decisions without walking the filesystem:

```bash
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/target/**' \
  --limit 100 \
  --explain
```

`--explain` reports:

- selected traversal root;
- shared literal prefix;
- expanded positive and negative patterns;
- pattern classification;
- leaf filters;
- metadata requirements;
- termination policy;
- traversal strategy.

## Traversal Statistics

```bash
branchcut --glob '**/*.rs' --exclude '**/target/**' --stats
```

Statistics are written to stderr and include:

```text
matched
directories considered
directories opened
directories pruned
entries inspected
candidate files
metadata calls
filesystem errors
elapsed
```

The root is inspected once with `symlink_metadata`. Branchcut does not call per-entry `metadata()` for currently supported filters; `DirEntry::file_type()` supplies traversal type information.

| `--strict` I/O errors | Exit code 2 after traversal |
| `--json` | One JSON object per matching path, streamed as JSON Lines |
| `--exec COMMAND` | Runs the parsed command once per match; `{}` is replaced by the displayed path; no shell |
| `--gitignore` | Loads root and nested `.gitignore` files; later rules override earlier rules |
| Invalid query | Exit code 2 |
| Broken output pipe | Clean success, no panic |
| Symlink traversal | Never follows directory symlinks |

## Architecture

The implementation lives entirely in `src/main.rs`:

```text
CLI parser
  -> brace expansion and pattern parser
  -> Pattern IR
  -> shared PatternProgram trie/NFA
  -> QueryPlan
  -> state-carrying depth-first traversal
  -> buffered streaming output or sorted collection
```

Each traversal frame carries the active positive and negative program states. Child names advance those states once. Branchcut does not rebuild and rematch the full relative path against every pattern for every entry.

Common segment forms bypass the general wildcard matcher:

| Segment | Specialized operation |
|---|---|
| `literal` | byte equality |
| `prefix*` | `starts_with` |
| `*suffix` | `ends_with` |
| general wildcard | allocation-free star backtracking |

## Tests and Quality Gates

```bash
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
cargo metadata --no-deps --format-version 1
```

The inline Rust suite covers matcher syntax, globstar zero-component behavior, brace limits, shared compilation, planner pruning, hidden semantics, extension filtering, exclusions, literal simple search, sorted limits, broken pipes, deep traversal, symlink policy on Unix, and non-UTF-8 Unix names.

- This is not a full `fast-glob` API or syntax replacement.
- Extglobs such as `+(foo)` and `@(foo|bar)` are unsupported.
- Nested braces and multiple brace groups are unsupported.
- `.gitignore` parsing is opt-in with `--gitignore`; comments, ordered rules, directory rules, and negation are supported, but advanced Git escaping and platform-specific ignore edge cases are not claimed.
- `--json` is JSON Lines rather than one enclosing JSON array, preserving streaming behavior.
- `--exec` uses a small quote-aware argument parser and `std::process::Command`; it never invokes a shell, and shell syntax/pipelines are unsupported.
- Backslash escaping is not a portable pattern feature; use `/` as the pattern separator.
- Negative patterns passed through `--glob` are not interpreted as exclusions; use `--exclude`.
- Matching is byte-oriented. On UTF-8 platforms, `?` consumes one encoded byte, not one Unicode scalar or grapheme.
- Unix non-UTF-8 names are preserved and do not panic. Windows matching currently uses a lossy UTF-16-to-UTF-8 view.
- Traversal is sequential. No worker pool or async runtime is included.
- Filesystem iteration order is platform- and filesystem-dependent unless `--sort` is selected.
- Permission behavior varies by platform and account privileges.
- Published comparisons include fast-glob, tinyglobby, and zlob; accuracy and methodology are documented in `COMPARISON.md`. Claims are workload-specific, not universal.

See [COMPATIBILITY.md](COMPATIBILITY.md) for the exact supported surface and [BENCHMARKS.md](BENCHMARKS.md) for methodology and raw measurements.

## Zero-Dependency Proof

`Cargo.toml` contains an empty dependency table:


```toml
[dependencies]
```

See [deps-proof.txt](deps-proof.txt) for recorded Cargo metadata and source-file proof. See [STDLIB.md](STDLIB.md) for every implemented standard-library substitution.

## Five-Minute Demo Runbook

### 0:00–0:30 — Problem

Explain why real filesystem queries usually combine a glob package, walker, ignore engine, CLI parser, and post-filter pipeline.

### 0:30–1:00 — Constraint proof

```bash
cargo metadata --no-deps --format-version 1
```

Show the empty `[dependencies]` section and the single `src/main.rs` implementation.

### 1:00–1:45 — Useful CLI

```bash
branchcut config
branchcut --glob 'src/**/*.{rs,toml}'
branchcut -e rs --type file
```

### 1:45–2:45 — Planner advantage

```bash
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/target/**' \
  --exclude '**/node_modules/**' \
  --limit 100 \
  --explain
```

Then run the same query with `--stats` and point to directories opened, directories pruned, metadata calls, and immediate termination.

### 2:45–3:30 — Correctness

Run the Rust suite and show representative differential-corpus cases:

```bash
cargo test
```

Show the 16,000-file corpus, result-set equality checks, and raw hot-engine medians in `COMPARISON.md`. State explicitly that the hot comparison excludes startup and serialization, while the separate cold section includes process startup.

### 4:30–4:50 — Standard-library craft

Open `STDLIB.md` and connect the custom parser, shared state program, `read_dir` traversal, manual CLI, buffered output, and error types to the packages they replace.

### 4:50–5:00 — Limits

State the unsupported extglobs, lack of `.gitignore` parsing, byte-oriented wildcard semantics, sequential traversal, and workload-specific rather than universal performance claims.

## Future Enhancements

- **Bounded parallel traversal:** add a fixed-size `std::thread` worker pool after differential testing against the sequential engine. Required invariants: no thread per directory, atomic cancellation for `--first`/`--limit`, synchronized streaming output, deterministic `--sort`, and benchmarked worker counts of 1/2/4/8/logical cores.
- **Hierarchical `.gitignore`:** compile nested ignore state and prune ignored subtrees while preserving re-inclusion semantics.
- **Metadata predicates:** add size and modification-time filters without introducing metadata calls for queries that do not require them.
- **Broader syntax:** consider extglobs and richer brace forms only after the current compatibility surface remains regression-free.

## License

MIT. See [LICENSE](LICENSE).
