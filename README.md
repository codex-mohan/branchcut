<div align="center">

<img src="docs/brand/branchcut-icon.svg" alt="Branchcut icon" width="128">

# Branchcut

**Compile the query. Cut the tree. Skip the packages.**

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-2024-DEA584?style=for-the-badge&amp;logo=rust&amp;logoColor=white&amp;labelColor=0A1220" alt="Rust 2024" /></a>
  <img src="https://img.shields.io/badge/core_crates-zero-00C853?style=for-the-badge&amp;labelColor=0A1220" alt="Zero third-party crates in the core engine" />
  <img src="https://img.shields.io/badge/filesystem_query-engine-3178C6?style=for-the-badge&amp;labelColor=0A1220" alt="Filesystem query engine" />
  <img src="https://img.shields.io/badge/historical_hot_result-1.70%C3%97-64748B?style=for-the-badge&amp;labelColor=0A1220" alt="Historical hackathon benchmark result; not a current general performance claim" />
  <a href="https://github.com/codex-mohan/branchcut/actions/workflows/install-matrix.yml"><img src="https://img.shields.io/github/actions/workflow/status/codex-mohan/branchcut/install-matrix.yml?style=for-the-badge&amp;label=installers&amp;labelColor=0A1220" alt="Windows, Linux, and macOS installer checks" /></a>
  <a href="https://github.com/codex-mohan/branchcut/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-BB86FC?style=for-the-badge&amp;labelColor=0A1220" alt="MIT license" /></a>
  <a href="https://github.com/codex-mohan/branchcut/stargazers"><img src="https://img.shields.io/github/stars/codex-mohan/branchcut?style=for-the-badge&amp;labelColor=0A1220&amp;color=FFD700" alt="GitHub stars" /></a>
  <a href="https://github.com/codex-mohan/branchcut/pulls"><img src="https://img.shields.io/badge/PRs-welcome-00C853?style=for-the-badge&amp;labelColor=0A1220" alt="PRs welcome" /></a>
</p>

</div>

Branchcut is a native filesystem query engine that compiles globs, exclusions, ignore rules, and file filters into a traversal plan. Its goal is to find the intended paths while avoiding directories that cannot contribute to the result. The project began at the [Zero Dependency 72-Hour Hackathon](https://zerodepshack.com/); the preserved competition artifact is tagged `submission-v1`. Current development keeps the core free of third-party crates and provides a modular Rust library with a thin CLI.

The competition targeted [`fast-glob@3.3.3`](https://www.npmjs.com/package/fast-glob) for its Package Killer bonus. Its API and result semantics remain distinct from Branchcut. The comparison receipt and benchmark results below document that competition snapshot; they are historical evidence, not a claim about current performance across workloads.

```bash
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**' \
  --type file \
  --limit 100 \
  --stats
```

> **The idea:** compile the query once, use its active states and filters to guide traversal, stream useful results, and show the filesystem work the query performed.

**[Getting started](#getting-started) · [Features](#what-ships-today) · [Quick start](#quick-start) · [Query planning](#query-planning) · [Historical competition evidence](#hackathon-snapshot-package-killer-fast-glob) · [Limitations](#limitations)**

## Hackathon snapshot: Package Killer (`fast-glob`)

The hackathon defines the [Package Killer bonus](https://zerodepshack.com/#bonus-points) as cleanly reimplementing a package people actually install, documenting the replacement, and backing its popularity with real download numbers. Branchcut targets that definition directly.

| | `fast-glob@3.3.3` | Branchcut `0.1.0` submission |
|---|---|---|
| What ships | Node package and transitive dependency tree | One native executable |
| Runtime dependencies | 17 transitive npm dependencies in the tested install | **0 crates** |
| Source layout | Package implementation plus dependencies | **1 Rust file**: `src/main.rs` at the `submission-v1` tag |
| Positive globs | Yes | Yes; compiled into a shared program |
| Exclusions | Ignore matching | Matching plus safe subtree pruning |
| Result delivery | Returns a collected array | Streams by default; sorting is opt-in |
| Early stop | Caller truncates after collection | `--first` / `--limit` stop traversal |
| Ignore files | Separate configuration/tooling | Hierarchical `.gitignore` support built in |
| Diagnostics | No filesystem-work counters | `--explain` plan + `--stats` counters |
| Commands | Separate runner needed | Shell-free `--exec` built in |
| Published hot result | 37.528 ms | **22.079 ms — 1.70× in the recorded workload** |

The table describes the tagged competition artifact and one correctness-checked, 16,000-file Windows workload. It is not a current, general performance claim. See [COMPARISON.md](COMPARISON.md), [BENCHMARKS.md](BENCHMARKS.md), and the [post-competition baseline](docs/evidence/post-competition-baseline.md) for the original protocol, versions, limitations, and provenance.

### Competition-era adoption receipt

| Package | Role in this project | 12-month downloads | Average downloads/month |
|---|---|---:|---:|
| [`fast-glob`](https://www.npmjs.com/package/fast-glob) | **Primary Package Killer target and correctness oracle** | 5,536,931,736 | **461,410,978** |
| [`tinyglobby`](https://www.npmjs.com/package/tinyglobby) | Secondary modern Node benchmark opponent | 4,775,466,973 | 397,955,581 |

These download counts were retrieved on 2026-09-02 from the official npm downloads API for **2025-09-01 through 2026-08-31**. They document the competition research snapshot, not current package popularity: [`fast-glob` receipt](https://api.npmjs.org/downloads/point/2025-09-01:2026-08-31/fast-glob), [`tinyglobby` receipt](https://api.npmjs.org/downloads/point/2025-09-01:2026-08-31/tinyglobby).

The zero-crate engine uses Rust's standard library for matching, traversal, ignore rules, command execution, and JSON Lines. This does not claim full API compatibility with packages that solve those jobs. See the [standard-library ledger](STDLIB.md).

## Measured Results

Every timed comparison first checked that the result sets matched for the supported semantics.

| Engine | Hot query time | Relative to Branchcut | Matches |
|---|---:|---:|---:|
| **Branchcut** | **22.079 ms median** | **1.00×** | 13,000 |
| `tinyglobby@0.2.14` | 35.666 ms median | 1.61× slower | 13,000 |
| `fast-glob@3.3.3` | 37.528 ms median | 1.70× slower | 13,000 |
| `zlob@1.6.3` public `match` API | 133.408 ms average | 6.04× slower | 13,000 |

Workload: `**/*.{rs,toml}` over a synthetic 16,000-file corpus, hidden paths excluded, no path serialization, after warmup. The zlob result covers its direct single-threaded public `match` API on Windows, not every platform-specific walker path. See [COMPARISON.md](COMPARISON.md) and [BENCHMARKS.md](BENCHMARKS.md) before quoting these numbers.

In the exclusion-heavy cold CLI workload, Branchcut returned the same 10,000 sorted paths as `fast-glob`, opened only relevant directories, pruned 20 excluded directories, and measured **24.99 ms vs 148.53 ms**. Cold startup and hot engine results are deliberately kept separate.

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

The key distinction is architectural: Branchcut does not first walk the entire tree and then ask whether each path matched. Every directory carries the currently viable positive and exclusion states. When no positive state can match a descendant—or an exclusion safely covers the whole subtree—the walker does not open that directory.

That produces four practical advantages:

1. **Less filesystem work:** literal prefixes narrow the traversal root and impossible subtrees are pruned.
2. **One pass for many patterns:** common pattern segments are merged into one trie/NFA-style program.
3. **Results now, not later:** output streams as matches are found, and limits cancel traversal immediately.
4. **Proof instead of mystery:** `--explain` shows the plan; `--stats` shows actual directories, entries, metadata calls, matches, errors, and elapsed time.

Branchcut does not claim universal superiority or full `fast-glob` compatibility. Its claim is narrower and measurable: on planning-heavy filesystem queries, compiling the whole query can avoid work that a generic walk-then-filter pipeline performs.

## Documentation

The complete documentation lives in [`docs/`](docs/README.md). Those Markdown and MDX files are the canonical source for both repository readers and the Fumadocs site in [`website/`](website/README.md).

- [Install Branchcut](docs/getting-started/installation.md) and run the [quick start](docs/getting-started/quick-start.md)
- Learn the [mental model](docs/getting-started/mental-model.md) and [query compiler](docs/concepts/query-compiler.md)
- Use the complete [CLI reference](docs/reference/cli.md) and [glob syntax reference](docs/reference/glob-syntax.md)
- Read the published [benchmark evidence](docs/evidence/benchmarks.md), [correctness method](docs/evidence/correctness.md), and [compatibility boundaries](docs/reference/compatibility.md)
- Preview or deploy the [Fumadocs website](website/README.md)

## What Ships Today

### Query compilation

- `*`, `?`, `[abc]`, `[a-z]`, `[!abc]`, path-component `**`, and one common non-nested brace group
- Repeatable positive patterns and exclusions compiled into a shared pattern-state graph
- Literal-prefix root selection, positive feasibility pruning, and safe trailing-globstar exclusion pruning
- Specialized literal, prefix, and suffix segment matchers

### Traversal and filtering

- File, directory, and symlink selection with repeatable extension filters
- Hidden-path exclusion by default; directory symlinks are never followed
- Hierarchical, opt-in `.gitignore` rules with ordered negation and conservative re-inclusion
- Sequential streaming by default or bounded parallel traversal with `--threads`
- Native path matching without forcing hot-path UTF-8 conversion

### Output and control

- Immediate `--first` and `--limit` termination for streaming selection
- Optional deterministic sorting, count-only output, and JSON Lines
- Shell-free, native-path command execution through `std::process::Command`
- Strict or best-effort filesystem error handling
- Real query-plan and traversal diagnostics through `--explain` and `--stats`

## Getting Started

Branchcut is installed as a user-level command through Cargo. Rust and Cargo are the only prerequisites; the resulting Branchcut executable has zero third-party runtime dependencies.

### Installation

| Windows | Linux | macOS |
|:---:|:---:|:---:|
| PowerShell installer | POSIX `sh` installer | POSIX `sh` installer |
| `branchcut.exe` | `branchcut` | `branchcut` |
| Install + query + uninstall CI | Install + query + uninstall CI | Install + query + uninstall CI |

#### Recommended: install directly from GitHub

No clone is required:

```bash
cargo install --locked --git https://github.com/codex-mohan/branchcut.git
```

Cargo installs `branchcut` into `$CARGO_HOME/bin`—normally `%USERPROFILE%\.cargo\bin` on Windows or `~/.cargo/bin` on Unix. Rustup normally adds that directory to `PATH`.

Verify the installation from any directory:

```bash
branchcut --version
branchcut --help
```

#### Install from a checkout

The checked-in installers build the locked release and verify the installed executable. They also support a custom Cargo root.

Windows PowerShell:

```powershell
.\scripts\install.ps1

# Optional custom location
.\scripts\install.ps1 -InstallRoot "$HOME\.branchcut"
```

Linux and macOS:

```bash
sh scripts/install.sh

# Optional custom location
BRANCHCUT_INSTALL_ROOT="$HOME/.branchcut" sh scripts/install.sh
```

For a custom root, add its `bin` directory to `PATH`; the installer prints a warning when it is not already present.

### Uninstall

For the default installation:

```bash
cargo uninstall branchcut
```

Or use the matching checkout script:

```powershell
.\scripts\uninstall.ps1
```

```bash
sh scripts/uninstall.sh
```

Pass the same custom root used during installation:

```powershell
.\scripts\uninstall.ps1 -InstallRoot "$HOME\.branchcut"
```

```bash
BRANCHCUT_INSTALL_ROOT="$HOME/.branchcut" sh scripts/uninstall.sh
```

### Build without installing

```bash
cargo build --release
```

Release binaries are written to `target/release/branchcut.exe` on Windows and `target/release/branchcut` on Unix. Development and release gates have been exercised with Rust 1.96.0; the hackathon reference toolchain is Rust 1.98.0. See the complete [installation guide](docs/getting-started/installation.md) for PATH troubleshooting and upgrade instructions.

### Quick Start

Once installed, the same command works in PowerShell, Command Prompt, Bash, Zsh, and other shells:

```bash
branchcut --glob 'src/**/*.rs'
```

Simple filename search treats the input literally:

```bash
branchcut config
```

This finds file names containing the case-sensitive text `config`; glob metacharacters inside a simple search are not interpreted.

Positional arguments containing glob syntax are also accepted as shorthand:

```bash
branchcut '**/*.rs'
branchcut '*/*.rs'
```

Plain positional text remains a literal filename search. Use `--glob` when combining multiple explicit patterns or when you want the query to be self-documenting.

## CLI Reference

| Option | Behavior |
|---|---|
| `--glob PATTERN` | Add a positive glob; repeat to compile several patterns together |
| `--exclude PATTERN` | Add an exclusion; complete trailing-`**` subtrees can be pruned |
| `-e, --extension EXT` | Restrict file names by extension; repeatable |
| `--type file\|dir\|symlink` | Select the returned filesystem entry type |
| `--cwd PATH` | Set the query root |
| `--hidden` | Include hidden path components |
| `--first`, `--limit N` | Stop unsorted sequential traversal after enough matches; with `--sort`, select from the full sorted set |
| `--threads N` | Use bounded parallel traversal; `0` selects an available worker count |
| `--sort` | Collect and sort globally before output |
| `--count` | Print only the match count |
| `--gitignore` | Apply root and nested `.gitignore` rules |
| `--json` | Emit one JSON object per matching path |
| `--exec COMMAND` | Run a shell-free command template for every match; `{}` inserts the path |
| `--strict` | Fail the query on filesystem read errors |
| `--stats` | Write measured traversal counters to stderr |
| `--explain` | Print the compiled plan without traversing |

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

Sorting collects all matching paths before choosing the globally first results. Without `--sort`, output streams and the limit stops traversal immediately. Count mode can stop at the limit even if sorting was requested because path order does not affect the count.

Run with bounded parallel traversal:

```bash
branchcut --glob '**/*.rs' --threads 4 --count --stats
```

Parallel mode uses a bounded worker set and buffers matches before output. `--sort` is applied globally after workers finish. For exact early-stop and command-execution ordering, omit `--threads` and use the sequential engine.

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

`{}` is replaced with the native relative path under the `--cwd` root. The child command runs with that query root as its working directory. Shell pipelines, redirections, environment expansion, and shell built-ins are intentionally unsupported. With `--sort`, commands run for the final sorted result set; a sorted limit requires the full search first.

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

With `--threads N`, directory tasks use bounded per-worker queues, dynamic work stealing, outstanding-task completion, condition-variable sleeping, atomic cancellation, and reusable worker-local buffers. Results are buffered before one coordinator writes them. The sequential engine remains the default.

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

The query root and any literal-prefix ancestors are checked with `symlink_metadata` so traversal cannot skip ancestor symlinks or ignore rules. A fixed-prefix query can therefore record more than one root check. Branchcut avoids per-entry size/time metadata because those predicates are not currently implemented; `DirEntry::file_type()` supplies traversal type information.

### Runtime behavior

| Condition | Result |
|---|---|
| `--strict` I/O errors | Exit code 2 after traversal |
| `--json` | One JSON object per matching path, streamed as JSON Lines |
| `--exec COMMAND` | Runs the parsed command once per match from the query root; `{}` is replaced with the native path; no shell |
| `--gitignore` | Loads root and nested `.gitignore` files; later rules override earlier rules |
| Invalid query | Exit code 2 |
| Broken output pipe | Clean success, no panic |
| Symlink traversal | Never follows directory symlinks |

## Architecture

The implementation uses one Cargo package, a reusable Rust library, and a thin CLI. The library compiles queries and performs filesystem traversal. CLI modules parse options, present results, and launch explicitly requested commands:

```text
CLI arguments
  -> Query::compile()
  -> immutable CompiledQuery
  -> sequential visit callback or bounded parallel collection
  -> SearchReport / collected matches
  -> CLI output, sorting, or requested command execution
```

The public API exposes `Query`, `CompiledQuery`, `EntryType`, `Stats`, `Visitor`, `Completion`, `SearchReport`, and filesystem-error results. `CompiledQuery::visit` streams native `Path` values through a callback and returns a report; the caller chooses how to display or process them. Pattern and traversal internals remain private.

Common segment forms bypass the general wildcard matcher:

| Segment | Specialized operation |
|---|---|
| `literal` | byte equality |
| `prefix*` | `starts_with` |
| `*suffix` | `ends_with` |
| general wildcard | token-based matching |

## Performance and Correctness Evidence

Correctness gates:

```bash
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
cargo metadata --no-deps --format-version 1
```

The Rust test suite in `src/tests.rs` covers matcher syntax, globstar zero-component behavior, brace limits, shared compilation, planner pruning, hidden semantics, extension filtering, exclusions, literal simple search, sorted limits, broken pipes, deep traversal, parallel/sequential result equality, symlink policy on Unix, and non-UTF-8 Unix names. `tests/library.rs` exercises the public library API.

Published comparisons cover `fast-glob`, `tinyglobby`, and `zlob`, with workload definitions, correctness checks, environment details, raw measurements, and losses retained. See [COMPARISON.md](COMPARISON.md) for results and [BENCHMARKS.md](BENCHMARKS.md) for methodology.

## Historical Hackathon Judging Evidence

The hackathon rated submissions on a five-point scale across [four weighted criteria](https://zerodepshack.com/#scoring). This section records how the tagged competition submission presented its evidence; it is not a current project scorecard.

| Judging criterion | Weight | Branchcut evidence |
|---|---:|---|
| Functionality & Usefulness | 35% | Working CLI; globs, exclusions, ignore rules, type/extension filters, streaming, early stop, JSONL, exec, and parallel traversal |
| Zero-Dependency Craft | 30% | Empty `[dependencies]`; hand-written matcher/planner/walker; detailed [STDLIB.md](STDLIB.md) ledger |
| Code Quality & Idiom | 25% | The tagged submission used one Rust source; explicit errors; no unsafe code; bounded concurrency; passing tests on the recorded Windows run |
| Innovation | 10% | Compiles the whole query into traversal state so directories can be rejected before `read_dir` |

**Primary bonus claim — Package Killer (+3):** the `submission-v1` artifact targeted `fast-glob@3.3.3`; its adoption and benchmark evidence are preserved above and in the linked reports.

## Rechecking the Historical Submission

The one-file bonus applies to the preserved competition revision, not current source. Inspect it without moving the working tree:

```powershell
# 0 third-party crates
cargo metadata --no-deps --format-version 1

# Inspect the preserved competition source
git show submission-v1:src/main.rs

# Tests and optimized build
cargo test
cargo build --release

# See the compiler plan without traversing
.\target\release\branchcut.exe `
  --glob "packages/**/src/**/*.{rs,ts}" `
  --exclude "**/{target,node_modules,dist}/**" `
  --limit 100 `
  --explain

# Run a query and expose the filesystem-work counters
.\target\release\branchcut.exe `
  --glob "**/*.{rs,toml}" `
  --exclude "**/target/**" `
  --count `
  --stats
```

The current source has multiple modules by design. For the competition judging checklist, see [JUDGE.md](JUDGE.md).

## Limitations

- This is not a full `fast-glob` API or syntax replacement.
- Extglobs such as `+(foo)` and `@(foo|bar)` are unsupported.
- Nested braces and multiple brace groups are unsupported.
- `.gitignore` parsing is opt-in with `--gitignore`; comments, ordered rules, directory rules, and negation are supported, but advanced Git escaping and platform-specific ignore edge cases are not claimed.
- `--json` is JSON Lines rather than one enclosing JSON array, preserving streaming behavior.
- `--exec` uses a small quote-aware argument parser and `std::process::Command`; it never invokes a shell, and shell syntax/pipelines are unsupported.
- Backslash escaping is not a portable pattern feature; use `/` as the pattern separator.
- Negative patterns passed through `--glob` are not interpreted as exclusions; use `--exclude`.
- Matching is byte-oriented. On UTF-8 platforms, `?` consumes one encoded byte, not one Unicode scalar or grapheme.
- On Unix, native path bytes are preserved in plain output and represented by `path_bytes` in JSON when the complete relative path is not UTF-8. On Windows, matching uses `OsStr::as_encoded_bytes`; JSON adds `path_utf16` when a path is not UTF-8-representable. Human-readable Windows output can replace unpaired UTF-16 surrogates.
- Sequential traversal is recursive; tests cover a 64-component tree and do not guarantee safe traversal of arbitrarily deep trees.
- Parallel traversal is available with `--threads`; it buffers results and does not support `--limit` or `--exec`.
- Filesystem iteration order is platform- and filesystem-dependent unless `--sort` is selected.
- Permission behavior varies by platform and account privileges.
- Published performance claims are workload-specific, not universal.

See [COMPATIBILITY.md](COMPATIBILITY.md) for the exact supported surface.

## Zero-Dependency Proof

`Cargo.toml` contains an empty dependency table:

```toml
[dependencies]
```

See [deps-proof.txt](deps-proof.txt) for the dependency-graph check. [STDLIB.md](STDLIB.md) describes the standard-library components used by the core.


## Future Enhancements

- Metadata predicates such as size and modification time without unnecessary metadata calls.
- Broader syntax such as extglobs and richer brace forms after compatibility remains regression-free.

## License

MIT. See [LICENSE](LICENSE).
