<div align="center">

<img src="docs/brand/branchcut-icon.svg" alt="Branchcut icon" width="112">

# Branchcut

**Compile the query. Cut the tree.**

<p>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-2024-DEA584?style=for-the-badge&amp;logo=rust&amp;logoColor=white&amp;labelColor=0A1220" alt="Rust 2024" /></a>
  <img src="https://img.shields.io/badge/core_crates-zero-00C853?style=for-the-badge&amp;labelColor=0A1220" alt="Zero third-party crates in the core" />
  <a href="https://github.com/codex-mohan/branchcut/actions/workflows/install-matrix.yml"><img src="https://img.shields.io/github/actions/workflow/status/codex-mohan/branchcut/install-matrix.yml?style=for-the-badge&amp;label=installers&amp;labelColor=0A1220" alt="Installer checks" /></a>
  <a href="https://github.com/codex-mohan/branchcut/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-MIT-BB86FC?style=for-the-badge&amp;labelColor=0A1220" alt="MIT license" /></a>
  <a href="https://github.com/codex-mohan/branchcut/stargazers"><img src="https://img.shields.io/github/stars/codex-mohan/branchcut?style=for-the-badge&amp;labelColor=0A1220&amp;color=FFD700" alt="GitHub stars" /></a>
  <a href="https://github.com/codex-mohan/branchcut/pulls"><img src="https://img.shields.io/badge/PRs-welcome-00C853?style=for-the-badge&amp;labelColor=0A1220" alt="PRs welcome" /></a>
</p>

<p align="center">
  <strong>English</strong> ·
  <a href="README.es.md">Español</a> ·
  <a href="README.fr.md">Français</a> ·
  <a href="README.zh-CN.md">简体中文</a> ·
  <a href="README.ja.md">日本語</a> ·
  <a href="README.hi.md">हिन्दी</a>
</p>

</div>

Branchcut is a native filesystem query engine for everyday search, development workflows, and scripts. It compiles patterns, exclusions, ignore rules, entry filters, and stopping conditions into one traversal plan. It can prune directories that cannot contribute to a result, stream matching paths, and explain the work it performed.

The library and CLI are split into focused Rust modules. The core engine and ordinary CLI use only the Rust standard library; no third-party crates are in the runtime dependency graph.

[Website and docs](https://codex-mohan.github.io/branchcut/) · [Releases](https://github.com/codex-mohan/branchcut/releases) · [Feature inventory](docs/reference/feature-status.md) · [Report a bug](https://github.com/codex-mohan/branchcut/issues)

## Installation

Download the archive for your system from [GitHub Releases](https://github.com/codex-mohan/branchcut/releases/latest), verify it against `SHA256SUMS`, extract it, and put the executable on your `PATH`. Prebuilt binaries do not require Rust or Cargo. See the [archive instructions](docs/contributing/releases.md#verify-and-install-an-archive).

The script installers can bootstrap Rust when Cargo is missing. Direct Cargo installation requires an existing Rust toolchain. Each method builds Branchcut from source:

**Windows PowerShell:**

```powershell
irm https://raw.githubusercontent.com/codex-mohan/branchcut/master/scripts/install.ps1 | iex
```

**Linux and macOS:**

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/codex-mohan/branchcut/master/scripts/install.sh | sh
```

**Cargo:**

```sh
cargo install --locked --git https://github.com/codex-mohan/branchcut.git
```

Verify the command is available:

```sh
branchcut --version
branchcut --help
```

The scripts install a release build. See [installation](docs/getting-started/installation.md) for custom locations, uninstall, and PATH troubleshooting.

## Usage

Find Rust source files below the current directory:

```sh
branchcut --glob 'src/**/*.rs'
```

Plain text searches file names literally:

```sh
branchcut config
```

Combine patterns, exclusions, file filters, and a result limit:

```sh
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**' \
  --type file \
  --limit 100
```

See the plan and actual traversal counters:

```sh
branchcut --glob '**/*.rs' --explain
branchcut --glob '**/*.rs' --stats
```

### Everyday recipes and saved queries

Search common source and config files while respecting ignore sources and including hidden paths:

```sh
branchcut --glob '**/*.{rs,toml}' --ignore --hidden
```

Search by size and modification time:

```sh
branchcut --glob '**/*' --size '>=1.5MiB' --modified-within 30d
branchcut --glob '**/*.log' --modified-before '2025-01-01T12:00:00+05:30'
```

Save repeatable project selections in `.branchcut-queries`:

```text
[workspace]
glob packages/**/src/**/*.{rs,ts}
exclude **/target/**
ignore true
max-depth 8
```

Then run the profile:

```sh
branchcut --config .branchcut-queries --profile workspace --explain
```

More recipes are in [Everyday search](docs/guides/everyday-search.md), [ignore and hidden paths](docs/guides/ignore-and-hidden.md), and [saved project queries](docs/guides/project-queries.md).

### Pipelines and commands

Use NUL-delimited paths when passing arbitrary Unix filenames to another program:

```sh
branchcut --glob '**/*.rs' --print0 | xargs -0 rustfmt
```

Use JSON Lines when downstream tools need structured paths, or add filesystem facts with `--json-metadata`:

```sh
branchcut --glob '**/*.log' --json-metadata
```

Run a command with each result as an argument without a shell interpreting the path:

```sh
branchcut --glob '**/*.tmp' --exec 'rm {}'
```

For large selections, `--exec-batch` groups paths into bounded invocations. Use `--exec-preview` to inspect the argv without running commands. See [output and execution](docs/guides/output-and-exec.md) for placeholders, platform details, and failure behavior.

## Features

- **Glob search:** `*`, `?`, `**`, character classes, nested braces and bounded ranges, plus component-local extglobs.
- **Composed queries:** multiple positive patterns, exclusions, extension and type selection, depth, literal terms, and metadata filters in one query.
- **Content search:** opt-in literal search inside selected files with `--content` (alias `--grep`), repeatable OR patterns, `path:line:column:text` output, `-l`, `--count`, case modes, binary auto-skip with `--text`, and early stop. No regex or multiline modes.
- **Ignore controls:** hidden paths are excluded by default; opt into Git ignore sources, `.ignore`, or explicit ignore files.
- **Traversal controls:** streaming output, `--first`, `--limit`, sorting, worker selection, symlink following, one-filesystem traversal on Unix, and bounded open-directory handling.
- **Metadata filters:** exact decimal or binary file-size thresholds and modified-time windows. Metadata is avoided when the query does not request it.
- **Reusable queries:** named project-query profiles in a small text config file.
- **Diagnostics:** `--explain`, `--why PATH`, and `--stats` reveal the plan, a candidate's decision, and measured traversal work.
- **Output and workflows:** plain paths, JSON Lines with optional metadata, NUL-delimited paths, formatting, color, hyperlinks, shell-free command execution, bounded batches, and shell completions.
- **Library API:** compile a `Query` once and visit results through the Rust library without invoking the CLI.

See [CLI reference](docs/reference/cli.md), [glob syntax](docs/reference/glob-syntax.md), and [feature status](docs/reference/feature-status.md) for syntax, limits, and implementation status.

## How it works

A typical multi-filter search can be implemented as a full walk followed by glob checks, exclusions, metadata reads, and collection. Branchcut instead compiles these parts into a query plan and carries viable pattern states through the tree.

```text
patterns + exclusions + ignores + filters + termination
                          │
                          ▼
             compiled traversal plan
       narrowed root · active pattern states
       pruning rules · metadata requirements
                          │
                          ▼
             stream matching filesystem paths
```

This design aims to avoid unnecessary filesystem work. It does not guarantee that every query is faster than every other tool. See the measured comparison below and the recorded methodology.

## Performance

The table reports **default CLI invocations** from the 18 November 2026 Windows/NTFS run against the committed search implementation (`8035ad6`). Each tool searches the same 100,000-file fixture; result sets are checked before and after timing. Values are medians of nine fresh-process runs after three warmups and include startup and output capture.

`n/t` means the runner did not measure an equivalent invocation. It does not establish feature absence. ripgrep is measured in `--files` mode, not content-search mode.

| Workload | Branchcut | zlob | ripgrep (`rg --files`) | fd | fast-glob | tinyglobby |
|---|---:|---:|---:|---:|---:|---:|
| Broad files | **68.79 ms** | 123.94 ms | 109.17 ms | 117.14 ms | 370.45 ms | 242.30 ms |
| Fixed-prefix Rust | 40.34 ms | **35.46 ms** | 75.23 ms | 40.91 ms | 210.08 ms | 153.56 ms |
| Rust/TOML with exclusions | **59.27 ms** | n/t | 79.27 ms | 92.57 ms | 333.23 ms | 208.11 ms |
| Hidden Rust | **64.51 ms** | 98.29 ms | 84.53 ms | 94.45 ms | 314.34 ms | 195.53 ms |
| `.gitignore` Rust | **68.29 ms** | 80.07 ms | 77.18 ms | 113.49 ms | n/t | n/t |
| Directories | **53.76 ms** | 80.35 ms | n/t | 84.66 ms | 251.20 ms | 152.86 ms |
| Sorted Rust | **101.96 ms** | 103.99 ms | 260.04 ms | n/t | n/t | n/t |
| First 10 Rust | **37.50 ms** | 98.46 ms | n/t | 46.00 ms | n/t | n/t |
| Broad NUL output | **72.61 ms** | n/t | 105.54 ms | 109.36 ms | n/t | n/t |
| Recursive basename Rust | **64.26 ms** | 96.60 ms | 76.10 ms | 94.15 ms | 318.07 ms | 208.26 ms |

Zlob wins the default fixed-prefix query; four-worker Branchcut records 33.39 ms for that query. Branchcut leads the other default rows on this fixture, with sorted results close to zlob. These are workload-specific measurements, not a universal ranking. Default Branchcut's observed peak working set was 6.38 MiB on the broad-files query; memory samples for every tool and tested worker configuration are retained.

- [Complete defaults, tuned results, memory and method](docs/evidence/current-cli-comparison.md)
- [Raw 18 November result record](benchmarks/results/2026-11-18-gap-closure-cli.json)
- [Historical 12 November comparison](docs/evidence/cli-comparison.md)
- [Benchmark method and limits](BENCHMARKS.md)

## Rust library

The Rust library lets a host application compile a query once and stream paths to a callback:

```rust
use branchcut::{Completion, Query};
use std::{io, ops::ControlFlow, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let query = Query {
        patterns: vec!["src/**/*.rs".into()],
        ..Query::default()
    }
    .compile()?;

    let report = query.visit(&mut |path: &Path| -> io::Result<ControlFlow<()>> {
        println!("{}", path.display());
        Ok(ControlFlow::Continue(()))
    })?;

    assert_eq!(report.completion, Completion::Complete);
    Ok(())
}
```

The callback can stop the traversal with `ControlFlow::Break`. See the [Rust library tutorial](docs/tutorials/rust-library.md) for error handling, sorting, and visitor examples.

## Compatibility and limits

Branchcut is a focused filesystem query tool, not a drop-in replacement for the complete APIs of fast-glob, tinyglobby, zlob, fd, or ripgrep. Compatibility applies only to documented and checked semantics.

- Ignore handling is opt-in. `--gitignore` enables Git ignore sources and nested `.gitignore`; `--ignore` adds nested `.ignore`; explicit `--ignore-file` rules have the highest precedence. Some Git config conditions and ignore syntax remain unsupported.
- Default matching is byte-oriented. `--unicode` opts valid UTF-8 paths into Unicode scalar wildcards and simple lowercase case matching. It does not provide full case folding, normalization, or grapheme matching.
- Decimal size thresholds are exact and must resolve to whole bytes. Modification dates support UTC dates, RFC3339 offsets/fractions, and Unix `@seconds` instants.
- Directory symlinks are not followed unless requested. Following links uses sequential traversal with cycle detection. `--one-file-system` is Unix-only.
- A finite `--max-open` bounds directory handles by spilling pending entries to memory and resuming buffered frames; this trades memory for fewer open handles.
- Branchcut selects paths and reads filesystem metadata. Opt-in literal text search inside selected files is available through `--content` (alias `--grep`): repeated patterns combine with OR, output is `path:line:column:text`, binary files are skipped unless `--text` is passed, and queries that do not request it read no file contents. It does not implement regular expressions, captures, or multiline matching. Branchcut also does not maintain an index, watch for changes, or provide a GUI.

- Without `--strict`, filesystem errors are reported while discovery continues. Sorting scans before output; ordinary unsorted queries stream.

## Documentation

| Start here | Reference and advanced workflows |
|---|---|
| [Quick start](docs/getting-started/quick-start.md) | [CLI reference](docs/reference/cli.md) |
| [Everyday recipes](docs/guides/everyday-search.md) | [Glob syntax](docs/reference/glob-syntax.md) |
| [Saved queries](docs/guides/project-queries.md) | [Ignores and hidden paths](docs/guides/ignore-and-hidden.md) |
| [Rust library tutorial](docs/tutorials/rust-library.md) | [Output and execution](docs/guides/output-and-exec.md) |
| [Feature status](docs/reference/feature-status.md) | [Compatibility reference](docs/reference/compatibility.md) |

The language links above translate this README. Detailed documentation is currently in English. The canonical docs also power the site in [website/](website/README.md).

## Contributing

Bug reports, focused pull requests and translation corrections are welcome. Every confirmed behavior regression needs a test; performance claims need reproducible measurements with losing results retained.

```sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo tree -e normal
```

Read the [contributor verification guide](docs/contributing/verification.md), [architecture rules](AGENTS.md), [roadmap](ROADMAP.md), [documentation guide](docs/contributing/documentation.md) and [benchmark methodology](BENCHMARKS.md). The website is separate from the zero-crate Rust dependency graph.

## Origins and license

The versioned [competition submission](https://github.com/codex-mohan/branchcut/tree/submission-v1) remains as historical evidence. Branchcut was first built for the Zero Dependency 72-Hour Hackathon; ongoing development follows the post-competition product and architecture rules in [AGENTS.md](AGENTS.md), without rewriting the submission tag.

Branchcut is [MIT licensed](LICENSE). See [STDLIB.md](STDLIB.md) for the dependency ledger.
