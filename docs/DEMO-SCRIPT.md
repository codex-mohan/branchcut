# Branchcut — Five-Minute Demo Script

**Target runtime:** 4:30–4:50. Never exceed five minutes.

**Submission:** upload the final video link through the official hackathon form and provide:

```text
https://github.com/codex-mohan/branchcut
final tag: submission-v1
```

## Recording setup

Use one terminal and, optionally, one browser tab showing the public repository. Use a large terminal font. Do not show credentials, tokens, or private paths.

Prepare the release binary and fixture before recording. Do not spend video time waiting for builds.

```powershell
cargo build --release
$bin = (Resolve-Path .\target\release\branchcut.exe).Path
$root = Join-Path $env:TEMP "branchcut-demo"
Remove-Item -Recurse -Force $root -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force `
  "$root\src\nested", `
  "$root\packages\core\src", `
  "$root\target\debug", `
  "$root\.hidden" | Out-Null
Set-Content "$root\src\main.rs" ""
Set-Content "$root\src\nested\config.toml" ""
Set-Content "$root\packages\core\src\lib.rs" ""
Set-Content "$root\packages\core\src\view.ts" ""
Set-Content "$root\target\debug\generated.rs" ""
Set-Content "$root\.hidden\secret.rs" ""
Set-Content "$root\.gitignore" "target/`n*.tmp`n!src/nested/config.toml"
```

## Timeline

### 0:00–0:25 — Problem

**Show:** repository title and public GitHub URL.

**Say:**

> Real filesystem queries usually combine a globber, directory walker, ignore engine, CLI parser, filters, and result collection. Branchcut compiles those decisions into one traversal plan and avoids directories that cannot contribute.

### 0:25–0:50 — Zero-dependency proof

**Run:**

```powershell
Get-Content Cargo.toml
cargo metadata --no-deps --format-version 1
Get-ChildItem -Recurse -Filter *.rs | Select-Object -ExpandProperty FullName
```

**Say:**

> The shipped Rust manifest has an empty dependency table. Cargo reports no dependencies, and the complete implementation is one Rust source file.

**Point to:**

```text
[dependencies]
src/main.rs
dependencies: []
```

### 0:50–1:25 — Useful glob queries

**Run:**

```powershell
& $bin --cwd $root --glob "src/**/*.rs"
& $bin --cwd $root --glob "**/*.{rs,toml}" --exclude "**/target/**" --sort
& $bin --cwd $root --glob "**/*.rs" --first
```

**Say:**

> Branchcut supports literals, wildcards, globstar, character classes, braces, exclusions, sorting, and immediate early termination. Output streams by default.

### 1:25–1:55 — Query planner

**Run:**

```powershell
& $bin --cwd $root `
  --glob "packages/**/src/**/*.{rs,ts}" `
  --exclude "**/target/**" `
  --limit 100 `
  --explain
```

**Say:**

> Explain shows the selected traversal root, shared pattern classification, exclusions, filters, and termination policy. The differentiator is planning before filesystem traversal.

### 1:55–2:25 — Real traversal statistics

**Run:**

```powershell
& $bin --cwd $root `
  --glob "**/*.{rs,toml}" `
  --exclude "**/target/**" `
  --count `
  --stats
```

**Say:**

> These are real counters: directories considered, opened, and pruned; entries inspected; metadata calls; errors; matches; and elapsed traversal time. Queries without metadata predicates avoid per-entry metadata calls.

### 2:25–3:00 — New workflows

#### Hierarchical `.gitignore`

```powershell
& $bin --cwd $root --glob "**/*" --gitignore --sort
```

**Say:**

> Root and nested `.gitignore` files are loaded as directories are entered. Parent rules remain active, child rules are evaluated later, the last matching rule wins, and negated descendants are preserved conservatively.

#### JSON Lines

```powershell
& $bin --cwd $root --glob "**/*.rs" --json
```

**Say:**

> JSON output is dependency-free JSON Lines, so every match is emitted as a valid object without collecting one giant array.

#### Shell-free execution

```powershell
& $bin --cwd $root --glob "src/*.rs" --exec "cmd.exe /c echo {}"
```

**Say:**

> Execution uses `std::process::Command` and does not implicitly invoke a shell. It reduces shell-injection risk from filenames, but it is not a sandbox; a user can explicitly choose a shell.

### 3:00–3:30 — Correctness and diagnostics

**Run:**

```powershell
cargo test
& $bin --glob "src/[abc"
```

**Say:**

> The regression suite covers matcher syntax, globstar, braces, shared states, pruning, hidden paths, limits, broken pipes, deep trees, symlinks, non-UTF-8 Unix paths, nested ignore rules, JSON escaping, and command parsing. Invalid patterns return a controlled error with exit code two instead of panicking.

### 3:30–4:10 — Accuracy before performance

**Show:** `COMPARISON.md`.

**Say:**

> We compare result sets before timing. On the documented corpus, Branchcut, fast-glob 3.3.3, and tinyglobby 0.2.14 produced identical sets for the tested syntax and options. A tested zlob nested-globstar mismatch is retained and documented rather than hidden.

**Show:**

```text
16,000-file corpus
13,000 matching files
same result counts and sets
```

### 4:10–4:35 — Hot-engine performance

**Show:** the hot benchmark table in `COMPARISON.md`.

```text
Branchcut:   18.393 ms/query
Tinyglobby:  27.227 ms/query
fast-glob:   28.721 ms/query
zlob:       120.090 ms/query
```

**Say:**

> These measurements exclude startup and path serialization. Branchcut is sequential; zlob's separate parallel walker was not used. These are workload-specific results, not a universal performance claim.

### 4:35–4:50 — Honest close

**Show:**

```text
README.md
STDLIB.md
COMPATIBILITY.md
COMPARISON.md
JUDGE.md
deps-proof.txt
```

**Say:**

> Branchcut is a single-file, zero-crate Rust filesystem query engine. It is useful now, transparent about its limits, and proves its claims with result-set checks, planner counters, and reproducible commands.

## Final checklist

- [ ] Video is under five minutes.
- [ ] Public GitHub URL is visible.
- [ ] Empty `[dependencies]` is visible.
- [ ] `cargo metadata --no-deps` is visible.
- [ ] Only `src/main.rs` is shown as implementation source.
- [ ] Glob query works.
- [ ] `--explain` works.
- [ ] `--stats` works.
- [ ] `--gitignore` works.
- [ ] `--json` works.
- [ ] `--exec` works.
- [ ] `cargo test` passes.
- [ ] Invalid pattern returns controlled exit code 2.
- [ ] Accuracy methodology is shown before speed numbers.
- [ ] zlob comparison limitations are stated.
- [ ] No secret or private machine data appears.
- [ ] Video link permissions allow judges to view it.
