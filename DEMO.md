# Five-Minute Demo Script

Target duration: **4:30–4:55**. Leave at least five seconds below the five-minute maximum.

## Before recording

Use a clean terminal at the repository root. Build once before starting:

```powershell
cargo build --release
```

Create a small fixture so the output is deterministic:

```powershell
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

Set these variables before recording:

```powershell
$bin = (Resolve-Path .\target\release\branchcut.exe).Path
```

Use a large terminal font and record only the terminal plus a browser tab showing the public repository. Do not record passwords, tokens, or private paths.

## Timeline and narration

### 0:00–0:25 — The problem

Say:

> Real filesystem queries usually combine a globber, directory walker, ignore engine, CLI parser, filters, and result collection. Branchcut compiles those decisions into one traversal plan and avoids directories that cannot contribute.

Show the repository title and public URL:

```text
https://github.com/codex-mohan/branchcut
```

### 0:25–0:50 — Zero-dependency proof

Run:

```powershell
Get-Content Cargo.toml
cargo metadata --no-deps --format-version 1
Get-ChildItem -Recurse -Filter *.rs | Select-Object -ExpandProperty FullName
```

Say:

> The shipped Rust manifest has an empty dependency table. Cargo reports no dependencies, and the implementation is one Rust source file.

Point out:

```text
[dependencies]

src/main.rs
```

### 0:50–1:30 — Basic useful queries

Run:

```powershell
& $bin --cwd $root --glob "src/**/*.rs"
& $bin --cwd $root --glob "**/*.{rs,toml}" --exclude "**/target/**" --sort
& $bin --cwd $root --glob "**/*.rs" --first
```

Say:

> Globs are streamed by default. Braces, globstar, exclusions, and early termination are handled without a post-processing walk.

### 1:30–2:05 — Planner explanation

Run:

```powershell
& $bin --cwd $root `
  --glob "packages/**/src/**/*.{rs,ts}" `
  --exclude "**/target/**" `
  --limit 100 `
  --explain
```

Say:

> Explain shows the selected root, shared pattern classification, exclusions, filters, and termination policy. This is the differentiator: the query is planned before traversal.

### 2:05–2:35 — Real traversal statistics

Run:

```powershell
& $bin --cwd $root `
  --glob "**/*.{rs,toml}" `
  --exclude "**/target/**" `
  --count `
  --stats
```

Point to:

```text
matched
 directories considered
 directories opened
 directories pruned
 entries inspected
 metadata calls
 elapsed
```

Say:

> Stats are real counters, not decorative telemetry. A query that does not request metadata avoids per-entry metadata calls.

### 2:35–3:05 — Hierarchical ignore, JSON, and execution

Run:

```powershell
& $bin --cwd $root --glob "**/*" --gitignore --sort --json
& $bin --cwd $root --glob "src/*.rs" --exec "cmd.exe /c echo {}"
```

Say:

> Nested `.gitignore` files are opt-in and ordered. JSON Lines preserve streaming. Exec uses `std::process::Command` without implicitly invoking a shell; it is shell-free, not a sandbox.

### 3:05–3:35 — Correctness and diagnostics

Run:

```powershell
cargo test
& $bin --glob "src/[abc"
```

Say:

> The regression suite covers matcher syntax, globstar, braces, pruning, hidden paths, limits, broken pipes, deep paths, symlinks, non-UTF-8 Unix names, nested ignore rules, JSON escaping, and command parsing. Invalid patterns return an error instead of panicking.

When the invalid pattern command exits with code 2, mention that it is expected.

### 3:35–4:10 — Performance evidence

Open `COMPARISON.md` and show the methodology before the numbers. Say:

> Accuracy was checked before timing. Branchcut, fast-glob, and tinyglobby produced equal result sets for the documented corpus. The hot comparison excludes startup and output serialization.

Show the table:

```text
Branchcut:   18.393 ms/query
Tinyglobby:  27.227 ms/query
fast-glob:   28.721 ms/query
zlob:       120.090 ms/query
```

Immediately qualify it:

> These are workload-specific Windows measurements. The zlob parallel walker was not exercised, so this is not a universal ranking.

### 4:10–4:35 — Honest limits and close

Open `COMPATIBILITY.md` and say:

> Branchcut deliberately does not claim full fast-glob compatibility. Extglobs, advanced Git escaping, metadata predicates, and parallel traversal remain outside the current surface. The claims here are narrow, tested, and reproducible.

Finish by showing:

```text
README.md
STDLIB.md
COMPATIBILITY.md
COMPARISON.md
JUDGE.md
deps-proof.txt
```

## Recording checklist

- [ ] Duration is below five minutes.
- [ ] Repository URL is visible.
- [ ] Empty `[dependencies]` is visible.
- [ ] `cargo metadata --no-deps` output is visible.
- [ ] Single `src/main.rs` source proof is visible.
- [ ] Basic glob query works.
- [ ] `--explain` shows planning decisions.
- [ ] `--stats` shows real counters.
- [ ] `.gitignore`, JSON Lines, and `--exec` are demonstrated.
- [ ] `cargo test` passes.
- [ ] Invalid input returns a controlled error.
- [ ] Performance numbers are shown only after accuracy methodology.
- [ ] zlob limitation is stated honestly.
- [ ] No secrets or private machine information appear.
- [ ] Video link is accessible to judges.
