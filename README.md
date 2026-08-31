<div align="center">

<img src="docs/brand/branchcut-icon.svg" alt="Branchcut icon" width="128">

# Branchcut

**Compile the query. Cut the tree. Skip the packages.**

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-2024-DEA584?style=for-the-badge&amp;logo=rust&amp;logoColor=white&amp;labelColor=0A1220" alt="Rust 2024" /></a>
  <img src="https://img.shields.io/badge/dependencies-zero-00C853?style=for-the-badge&amp;labelColor=0A1220" alt="Zero dependencies" />
  <img src="https://img.shields.io/badge/Rust_sources-one-3178C6?style=for-the-badge&amp;labelColor=0A1220" alt="Single Rust source file" />
  <img src="https://img.shields.io/badge/Package_Killer-fast--glob-FF6B35?style=for-the-badge&amp;labelColor=0A1220" alt="Package Killer target: fast-glob" />
  <img src="https://img.shields.io/badge/hot_benchmark-1.70%C3%97_faster-00C853?style=for-the-badge&amp;labelColor=0A1220" alt="1.70 times faster than fast-glob in the published hot benchmark" />
  <a href="https://github.com/codex-mohan/branchcut/actions/workflows/install-matrix.yml"><img src="https://img.shields.io/github/actions/workflow/status/codex-mohan/branchcut/install-matrix.yml?style=for-the-badge&amp;label=installers&amp;labelColor=0A1220" alt="Windows, Linux, and macOS installer checks" /></a>
  <a href="https://github.com/codex-mohan/branchcut/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-BB86FC?style=for-the-badge&amp;labelColor=0A1220" alt="MIT license" /></a>
  <a href="https://github.com/codex-mohan/branchcut/stargazers"><img src="https://img.shields.io/github/stars/codex-mohan/branchcut?style=for-the-badge&amp;labelColor=0A1220&amp;color=FFD700" alt="GitHub stars" /></a>
  <a href="https://github.com/codex-mohan/branchcut/pulls"><img src="https://img.shields.io/badge/PRs-welcome-00C853?style=for-the-badge&amp;labelColor=0A1220" alt="PRs welcome" /></a>
</p>

</div>

Branchcut is a filesystem query engine built for the [Zero Dependency 72-Hour Hackathon](https://zerodepshack.com/). It replaces the work normally delegated to a glob package, directory walker, ignore engine, argument parser, JSON serializer, and worker-pool library with **one Rust source file and zero crates**.

Its primary **Package Killer** target is [`fast-glob@3.3.3`](https://www.npmjs.com/package/fast-glob): a package averaging **461.4 million downloads per month** over the latest completed 12-month window. Branchcut does not imitate its JavaScript API. It replaces the real filesystem-query workflow with a native CLI that compiles globs, exclusions, ignore rules, filters, and termination into one traversal plan—and can avoid opening irrelevant directories in the first place.

```bash
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**' \
  --type file \
  --limit 100 \
  --stats
```

> **The pitch:** `fast-glob` finds paths. Branchcut compiles the whole filesystem query so it can prune the search tree, stream results, stop early, and explain exactly what work it performed.

**[Getting started](#getting-started) · [Package Killer case](#package-killer-fast-glob) · [Features](#what-ships-today) · [Quick start](#quick-start) · [Query planning](#query-planning) · [Benchmarks](#measured-results) · [Judge checklist](#60-second-judge-check) · [Limitations](#limitations)**

## Package Killer: `fast-glob`

The hackathon defines the [Package Killer bonus](https://zerodepshack.com/#bonus-points) as cleanly reimplementing a package people actually install, documenting the replacement, and backing its popularity with real download numbers. Branchcut targets that definition directly.

| | `fast-glob@3.3.3` | Branchcut `0.1.0` |
|---|---|---|
| What ships | Node package and transitive dependency tree | One native executable |
| Runtime dependencies | 17 transitive npm dependencies in the tested install | **0 crates** |
| Source layout | Package implementation plus dependencies | **1 Rust file**: `src/main.rs` |
| Positive globs | Yes | Yes; compiled into a shared program |
| Exclusions | Ignore matching | Matching plus safe subtree pruning |
| Result delivery | Returns a collected array | Streams by default; sorting is opt-in |
| Early stop | Caller truncates after collection | `--first` / `--limit` stop traversal |
| Ignore files | Separate configuration/tooling | Hierarchical `.gitignore` support built in |
| Diagnostics | No filesystem-work counters | `--explain` plan + `--stats` counters |
| Commands | Separate runner needed | Shell-free `--exec` built in |
| Published hot result | 37.528 ms | **22.079 ms — 1.70× faster** |

The benchmark row is one correctness-checked, 16,000-file Windows workload—not a universal claim. The complete protocol, versions, losses, and caveats are in [COMPARISON.md](COMPARISON.md).

### Adoption receipt

| Package | Role in this project | 12-month downloads | Average downloads/month |
|---|---|---:|---:|
| [`fast-glob`](https://www.npmjs.com/package/fast-glob) | **Primary Package Killer target and correctness oracle** | 5,536,931,736 | **461,410,978** |
| [`tinyglobby`](https://www.npmjs.com/package/tinyglobby) | Secondary modern Node benchmark opponent | 4,775,466,973 | 397,955,581 |

Download figures use the official npm downloads API for **2025-09-01 through 2026-08-31**, divided by 12 and rounded to the nearest whole download: [`fast-glob` receipt](https://api.npmjs.org/downloads/point/2025-09-01:2026-08-31/fast-glob), [`tinyglobby` receipt](https://api.npmjs.org/downloads/point/2025-09-01:2026-08-31/tinyglobby). Retrieved 2026-09-02. npm does not publish a first-party package “rating,” so this README uses auditable adoption and benchmark data instead of inventing one.

Branchcut also replaces work commonly split among Rust crates such as `globset`, `walkdir`, `ignore`, `regex`, `clap`, `rayon`, and `serde_json`. Those are **stdlib substitutions**, not claims of full API compatibility. The implementation-by-implementation ledger is in [STDLIB.md](STDLIB.md).

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
- Unix non-UTF-8 path support without forcing hot-path UTF-8 conversion

### Output and control

- Immediate `--first` and `--limit` termination in sequential streaming mode
- Optional deterministic sorting, count-only output, and JSON Lines
- Shell-free command execution through `std::process::Command`
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
| `--first`, `--limit N` | Stop sequential traversal after enough matches |
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

Sorting necessarily collects all matching paths before truncation. Without `--sort`, output streams and the limit stops traversal immediately.

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

The root is inspected once with `symlink_metadata`. Branchcut does not call per-entry `metadata()` for currently supported filters; `DirEntry::file_type()` supplies traversal type information.

### Runtime behavior

| Condition | Result |
|---|---|
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
  -> compact Pattern IR
  -> shared PatternProgram trie/NFA
  -> QueryPlan
  -> sequential state-carrying traversal
       or bounded parallel task queues
  -> streamed, counted, or globally sorted output
```

Each traversal frame carries the active positive and negative program states. Child names advance those states once. Branchcut does not rebuild and rematch the full relative path against every pattern for every entry.

Common segment forms bypass the general wildcard matcher:

| Segment | Specialized operation |
|---|---|
| `literal` | byte equality |
| `prefix*` | `starts_with` |
| `*suffix` | `ends_with` |
| general wildcard | allocation-free star backtracking |

## Performance and Correctness Evidence

Correctness gates:

```bash
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
cargo metadata --no-deps --format-version 1
```

The inline Rust suite covers matcher syntax, globstar zero-component behavior, brace limits, shared compilation, planner pruning, hidden semantics, extension filtering, exclusions, literal simple search, sorted limits, broken pipes, deep traversal, parallel/sequential result equality, symlink policy on Unix, and non-UTF-8 Unix names.

Published comparisons cover `fast-glob`, `tinyglobby`, and `zlob`, with workload definitions, correctness checks, environment details, raw measurements, and losses retained. See [COMPARISON.md](COMPARISON.md) for results and [BENCHMARKS.md](BENCHMARKS.md) for methodology.

## How This Maps to Hackathon Judging

The hackathon rates every submission on a five-point scale across [four weighted criteria](https://zerodepshack.com/#scoring). This table points to evidence; it is not a self-awarded score.

| Judging criterion | Weight | Branchcut evidence |
|---|---:|---|
| Functionality & Usefulness | 35% | Working CLI; globs, exclusions, ignore rules, type/extension filters, streaming, early stop, JSONL, exec, and parallel traversal |
| Zero-Dependency Craft | 30% | Empty `[dependencies]`; hand-written matcher/planner/walker; detailed [STDLIB.md](STDLIB.md) ledger |
| Code Quality & Idiom | 25% | One readable Rust source; explicit errors; no unsafe code; bounded concurrency; 19 passing tests on the recorded Windows run |
| Innovation | 10% | Compiles the whole query into traversal state so directories can be rejected before `read_dir` |

**Primary bonus claim — Package Killer (+3):** `fast-glob@3.3.3` is the named target, its npm adoption is documented above, overlapping result sets are checked before timing, and limitations are explicit. Branchcut also meets the mechanical **Single File** condition: the only `.rs` file is `src/main.rs`.

## 60-Second Judge Check

```powershell
# 0 third-party crates
cargo metadata --no-deps --format-version 1

# 1 Rust source file
Get-ChildItem -Recurse -Filter *.rs | Select-Object -ExpandProperty FullName

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

Expected dependency result: `"dependencies":[]`. Expected source result: only `src/main.rs`. For the complete verification path, use [JUDGE.md](JUDGE.md).

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
- Unix non-UTF-8 names are preserved and do not panic. Windows matching currently uses a lossy UTF-16-to-UTF-8 view.
- Parallel traversal is available with `--threads`; it buffers results and does not support `--limit` or `--exec` because those require exact global ordering.
- Filesystem iteration order is platform- and filesystem-dependent unless `--sort` is selected.
- Permission behavior varies by platform and account privileges.
- Published performance claims are workload-specific, not universal.

See [COMPATIBILITY.md](COMPATIBILITY.md) for the exact supported surface.

## Zero-Dependency Proof

`Cargo.toml` contains an empty dependency table:

```toml
[dependencies]
```

See [deps-proof.txt](deps-proof.txt) for recorded Cargo metadata and source-file proof. [STDLIB.md](STDLIB.md) maps each implemented standard-library component to the crate it replaces.


## Future Enhancements

- Metadata predicates such as size and modification time without unnecessary metadata calls.
- Broader syntax such as extglobs and richer brace forms after compatibility remains regression-free.

## License

MIT. See [LICENSE](LICENSE).
