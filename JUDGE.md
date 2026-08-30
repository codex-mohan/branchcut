# Judge Verification Runbook

## Build

```powershell
cargo build --release
.\target\release\branchcut.exe --version
```

## Verify zero dependencies and single source

```powershell
cargo metadata --no-deps --format-version 1
Get-Content Cargo.toml
Get-ChildItem -Recurse -Filter *.rs | Select-Object -ExpandProperty FullName
```

Expected: `dependencies: []` and only `src/main.rs` as the Rust implementation source.

## Run quality gates

```powershell
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
```

## Run real queries

```powershell
.\target\release\branchcut.exe --glob "src/**/*.rs" --type file
.\target\release\branchcut.exe --glob "src/**/*.{rs,toml}" --exclude "**/target/**" --sort
.\target\release\branchcut.exe --glob "**/*.rs" --first
.\target\release\branchcut.exe --glob "**/*.rs" --count --stats
.\target\release\branchcut.exe --glob "packages/**/src/**/*.rs" --exclude "**/node_modules/**" --explain
.\target\release\branchcut.exe --glob "**/*" --gitignore --sort
.\target\release\branchcut.exe --glob "**/*.rs" --json
.\target\release\branchcut.exe --glob "src/*.rs" --exec "cmd.exe /c echo {}"
```

`--stats` reports directories considered/opened/pruned, entries inspected, candidate files, metadata calls, errors, and elapsed traversal time. `--explain` reports the selected root, pattern classifications, filters, and strategy.

## Reproduce accuracy checks

The maintainers used external development-only oracles outside the repository:

```text
fast-glob 3.3.3
 tinyglobby 0.2.14
 zlob v1.6.3 / Zig 0.16.0
```

The exact set-normalization protocol and every tested case are in `COMPARISON.md`. Results are compared before timing:

```text
Branchcut - competitor = {}
competitor - Branchcut = {}
```

The documented Branchcut/fast-glob/tinyglobby fixture cases all passed set equality. A nested-globstar mismatch was retained for the tested zlob CLI rather than hidden.

## Reproduce the benchmark claims

Use the same generated 16,000-file corpus described in `COMPARISON.md`, or generate an equivalent corpus with:

```text
20 packages
250 .rs + 250 .toml files per package under src/
150 .rs files per package under target/debug/
150 .ts files per package under node_modules/pkg/
```

For hot measurements:

- Branchcut: `--count --stats`; use the `elapsed` field, not parent-process wall time.
- Node: load each module once, warm up three times, run ten synchronous calls, consume returned arrays without printing.
- zlob: use its public filesystem API in one Zig process, warm up three times, run ten calls, use `std.heap.c_allocator`, `nosort=true`, and count results.

All measured cases must return identical counts and sets before speed numbers are published.

## Important limitations

- The zlob official C-compatible benchmark path returned zero matches for a Windows drive-letter path; zlob's own tree-size benchmark is disabled on Windows because it requires libc glob. The published zlob number therefore uses the public API harness, not a fabricated zero result.
- Hot zlob comparison does not exercise zlob's separate parallel walker.
- Performance results are workload-specific and should not be generalized.
- Branchcut does not claim full `fast-glob`, tinyglobby, or zlob compatibility.
