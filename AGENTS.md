# AGENTS.md — Branchcut

> **Branchcut — compile the query, cut the tree.**
>
> Zero-dependency, single-source-file Rust filesystem query engine for the Zero Dependency 72-Hour Hackathon.
>
> Primary Package Killer baseline: `fast-glob` 3.3.3.
> Native performance adversary: `zlob`.
> Other comparison targets: `tinyglobby`, `globset + walkdir`, and relevant `fd`/ripgrep-style workflows.

---

## 0. Mission

Branchcut is **not another glob library**.

It is an opinionated filesystem query engine that combines functionality normally split across globbers, walkers, ignore engines, file-finders, metadata filters, and command runners.

The core idea is:

> **Compile the whole filesystem query into one traversal plan, then avoid opening subtrees that cannot possibly contribute to the result.**

Example:

```bash
branchcut \
  --glob 'packages/**/src/**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**' \
  --type file \
  --modified-within 7d \
  --limit 100
```

Do not implement this as:

```text
walk everything -> glob -> ignore -> stat -> filter -> collect
```

Instead compile:

```text
positive patterns
+ exclusions
+ ignore rules
+ extension/type predicates
+ metadata predicates
+ early-stop conditions
```

into one traversal program.

The project should be useful, correct, measurably fast, memory-efficient, transparent about limitations, zero-crate, and contained in one Rust source file.

---

# 1. Hackathon Constraints

## 1.1 Zero dependencies

Rust `std` only.

`Cargo.toml` must have an empty dependency section:

```toml
[dependencies]
```

Forbidden includes:

- `glob`
- `globset`
- `walkdir`
- `ignore`
- `regex`
- `rayon`
- `crossbeam`
- `clap`
- `serde`
- `anyhow`
- `thiserror`
- `smallvec`
- `indexmap`
- `tokio`
- vendored third-party implementation code

External projects may only be used as documentation references, behavioral oracles, and benchmark baselines.

Do **not** copy their implementation.

## 1.2 Single-file target

Target the hackathon Single File bonus deliberately.

The only Rust source file should be:

```text
src/main.rs
```

Tests live inside:

```rust
#[cfg(test)]
mod tests {
    // ...
}
```

Documentation files do not violate the source-file goal.

Target repository:

```text
branchcut/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── AGENTS.md
├── STDLIB.md
├── BENCHMARKS.md
├── COMPATIBILITY.md
├── deps-proof.txt
├── LICENSE
└── src/
    └── main.rs
```

Soft source-size target: **1,500–2,500 lines**.

If the file approaches ~3,500+ lines, cut features before sacrificing code quality.

---

# 2. Positioning

Do not claim:

- first dependency-free globber;
- first single-file globber;
- full npm drop-in replacement for `fast-glob`;
- universal performance superiority.

Valid thesis:

> **Branchcut is a single-file, zero-crate Rust filesystem query engine that compiles multiple globs, exclusions, ignore rules, and file predicates into one traversal plan so it can avoid filesystem work before it happens.**

Performance claims require published evidence.

---

# 3. Projects to Study — Ideas, Not Code

## fast-glob

Study:

- mature glob semantics;
- positive/negative patterns;
- cwd behavior;
- dotfiles;
- symlink/depth options;
- practical compatibility expectations.

This is the primary Package Killer comparison.

## zlob

Study:

- pattern decomposition;
- aggressive pruning;
- specialized hot paths;
- traversal bottlenecks;
- benchmark methodology;
- why low-level directory enumeration matters.

Treat zlob as the hardest native performance opponent.

Do not port its source.

## tinyglobby

Study:

- minimal API;
- crawl optimization;
- debug/explain ideas;
- keeping common workflows small and fast.

## globset

Study:

- compiling many glob patterns together;
- reducing work as pattern count rises;
- shared automata/state concepts.

Branchcut should combine multi-pattern matching with traversal planning rather than matching after a generic walk.

## walkdir

Study:

- traversal correctness;
- symlink loops;
- error propagation;
- robust iterator semantics.

## ignore / ripgrep

Study:

- hierarchical ignore semantics;
- nested `.gitignore` precedence;
- negation/re-inclusion;
- hidden-file defaults;
- pruning ignored subtrees.

## fd

Study:

- excellent CLI ergonomics;
- smart defaults;
- `--type`;
- extension filters;
- hidden/ignore behavior;
- `--exec`;
- making simple queries convenient.

## Wax

Study:

- invariant prefixes;
- useful glob diagnostics;
- semantic traversal;
- captures;
- subtree exclusions.

## doublestar

Study:

- clean `**` semantics;
- streaming traversal;
- early-stop behavior;
- explicit I/O failure policy.

## dirwalk

Study:

- filesystem enumeration bottlenecks;
- benefits of low-level platform APIs.

Do not begin with handwritten syscalls. Branchcut's first advantage should come from **doing less filesystem work**, not reading every directory a few percent faster.

---

# 4. Competition Goals and Metrics

## G1 — Zero crates

Required:

```text
third-party crates = 0
```

Validate with `Cargo.toml` and:

```bash
cargo metadata --no-deps
```

## G2 — Single source file

Required:

```bash
find . -name '*.rs'
```

returns only:

```text
./src/main.rs
```

## G3 — Correctness

For every feature claimed compatible with a baseline:

```text
differential mismatches = 0
```

No benchmark result is valid if result sets differ.

## G4 — Useful real-world query engine

Must support enough functionality that a developer could actually choose Branchcut instead of wiring together a glob package plus a find/walk/filter pipeline.

## G5 — Performance

Targets, not promises:

**Strong:**

```text
>=1.25x geometric-mean hot-workload speedup vs fast-glob
>=50% lower peak RSS
```

**Excellent:**

```text
>=1.5x geometric-mean hot-workload speedup
>=60% lower peak RSS
```

## G6 — Planner advantage

Instrument:

```text
directories considered
directories opened
directories pruned
entries inspected
metadata calls
time to first result
```

For pruning-heavy workloads, Branchcut should prove that it performs less filesystem work.

---

# 5. Product Interface

Support three levels.

## Simple

```bash
branchcut config
```

Simple filename search without forcing glob syntax.

## Glob

```bash
branchcut --glob 'src/**/*.{rs,toml}'
```

## Query

```bash
branchcut \
  --glob 'packages/**/src/**' \
  -e rs -e toml \
  --type file \
  --exclude '**/target/**' \
  --modified-within 7d \
  --limit 100
```

Simple use must remain simple even as advanced capability grows.

---

# 6. P0 — Must Ship

## 6.1 Glob syntax

Support:

```text
*
?
**
[abc]
[a-z]
[!abc]
```

## 6.2 Common brace alternatives

Support:

```text
*.{js,ts}
**/*.{rs,toml}
```

Nested/advanced braces are not P0.

## 6.3 Multiple positive patterns

Example:

```text
src/**/*.rs
tests/**/*.rs
```

Compile shared structure where possible.

Do not emit duplicates.

## 6.4 Negative patterns / exclusions

Support:

```text
!**/node_modules/**
!**/target/**
!**/dist/**
```

Use exclusions to prune whole subtrees where semantics make it safe.

## 6.5 Literal-prefix extraction

Pattern:

```text
packages/core/src/**/*.rs
```

should start traversal at:

```text
packages/core/src/
```

not repository root.

## 6.6 Pattern classification

At minimum:

```text
Literal
SingleDirectory
FixedPrefixRecursive
UnboundedRecursive
```

Classification must influence execution.

## 6.7 Streaming output

Stream matches as discovered.

Do not collect every result by default.

Sorting is opt-in.

## 6.8 Early termination

Support:

```bash
--first
--limit N
```

Once the condition is satisfied, cancel further traversal immediately.

## 6.9 File types

Support:

```bash
--type file
--type dir
--type symlink
```

## 6.10 Extension filters

Support:

```bash
-e rs -e toml
```

Compile extension filters to specialized suffix matching where possible.

## 6.11 Hidden paths

Opinionated default:

```text
hidden = excluded
```

Override with:

```bash
--hidden
```

## 6.12 Root / cwd

Support:

```bash
--cwd PATH
```

## 6.13 Ignore baseline

P0 minimum:

- explicit exclusions;
- root ignore support if implementable safely.

Do not falsely claim full `.gitignore` semantics if nested precedence is incomplete.

## 6.14 Diagnostics

Invalid pattern input must produce a useful error, never an ordinary-input panic.

## 6.15 `--stats`

Required feature.

Example:

```text
matched                14822
directories opened      1304
directories pruned      8931
entries inspected     124891
metadata calls              0
elapsed                 18.4ms
```

## 6.16 `--explain`

Required differentiator.

Example:

```text
QUERY PLAN

root:
  packages/

shared prefix:
  packages/*/src/

leaf filters:
  .rs
  .ts

excluded subtrees:
  target/
  node_modules/
  dist/

metadata:
  not required

termination:
  first 100 results
```

Expose useful planning decisions, not implementation noise.

---

# 7. P1 — High-Value Features

Implement only after P0 is correct and benchmarked.

## 7.1 Hierarchical `.gitignore`

Support:

- root rules;
- nested `.gitignore` files;
- precedence;
- negation/re-inclusion where feasible.

## 7.2 Metadata predicates

Candidate syntax:

```bash
--size '>10M'
--size 0
--modified-within 7d
--modified-before ...
```

Important invariant:

> If the query does not require metadata, do not call `metadata()` unnecessarily.

## 7.3 Smart case

Simple-search mode:

- lowercase input -> case-insensitive;
- uppercase present -> case-sensitive.

Explicit glob mode may remain case-sensitive by default.

## 7.4 Command execution

Use only:

```rust
std::process::Command
```

Example:

```bash
branchcut --glob '**/*.tmp' --exec rm '{}'
```

Parallel execution is optional.

## 7.5 Rich parser errors

Example:

```text
src/**/[a-
       ^^^
unterminated character class
```

## 7.6 Optional sorting

```bash
--sort
```

Do not impose sorting cost on default streaming mode.

---

# 8. P2 — Stretch Only

Possible later features:

- extglobs;
- captures;
- JSON output;
- rich formatting;
- `--why PATH` ignore explanation;
- advanced re-inclusion;
- more powerful exec templating;
- shell completion.

Do not destabilize P0/P1 for these.

---

# 9. Core Architecture

Conceptual pipeline:

```text
query
  |
  +-- positive patterns
  +-- exclusions
  +-- ignore rules
  +-- type/ext predicates
  +-- metadata predicates
  +-- limit/first
  |
  v
QUERY COMPILER
  |
  +-- traversal root
  +-- shared pattern states
  +-- prune rules
  +-- specialized leaf tests
  +-- metadata requirement
  +-- termination condition
  |
  v
TRAVERSAL ENGINE
  |
  +-- read directory
  +-- advance states
  +-- prune impossible branches
  +-- inspect only required metadata
  +-- stream matches
```

---

# 10. Shared Multi-Pattern Compilation

This is a headline differentiator.

Input:

```text
src/**/*.ts
src/**/*.tsx
src/**/test*.ts
src/components/**/*.css
```

Do not create four fully independent traversals/matchers.

Compile common structure conceptually like:

```text
src
 └── **
      ├── *.ts
      ├── *.tsx
      ├── test*.ts
      └── components ...
```

Benchmark scaling:

```text
1 pattern
10 patterns
100 patterns
1000 patterns
```

The cost per filesystem entry should grow sublinearly where patterns share structure.

---

# 11. Pattern IR

Parse each pattern once.

Example:

```text
src/**/[a-z]*.{js,ts}
```

Conceptual representation:

```text
Literal("src")
GlobStar
Segment:
  Range(a-z)
  Star
  Alternative(.js, .ts)
```

Requirements:

- immutable after compile;
- compact;
- cheap to advance;
- usable for subtree-feasibility checks;
- no regex dependency.

Prefer flat arrays + integer state IDs over pointer-heavy object graphs where practical.

---

# 12. Specialized Matching Paths

Common patterns should bypass the most general matcher.

Fast-path candidates:

```text
literal
*.rs
foo*
*foo
*.{rs,ts}
```

Potential specialized operations:

```text
exact equality
prefix match
suffix match
small extension-set match
```

Only use the general glob state machine when necessary.

---

# 13. Globstar

`**` operates at the path-component level.

Example:

```text
src/**/mod.rs
```

matches:

```text
src/mod.rs
src/a/mod.rs
src/a/b/mod.rs
```

Do not flatten entire paths into temporary UTF-8 strings merely to implement globstar.

---

# 14. Pruning

Before entering a directory ask:

> Can any active positive state still match a descendant here?

If no:

```text
PRUNE
```

For exclusions ask:

> Is this entire subtree excluded, with no re-inclusion semantics that could later make a descendant relevant?

If yes:

```text
PRUNE
```

Pruning counters are part of `--stats` and benchmark evidence.

---

# 15. Ignore Rules Must Influence Traversal

Bad architecture:

```text
walk ignored subtree
-> discover files
-> mark them ignored
```

Preferred:

```text
resolve ignore state
-> avoid opening ignored subtree
```

where semantics permit.

---

# 16. Metadata Planning

Query:

```bash
branchcut --glob '**/*.rs'
```

should generally avoid metadata calls when `DirEntry::file_type()` and names suffice.

Query:

```bash
branchcut --glob '**/*.rs' --size '>10M'
```

requires metadata.

Compiler should track:

```text
needs_metadata = true | false
```

---

# 17. Filesystem API Strategy

Start with Rust std:

```rust
std::fs::read_dir
std::fs::DirEntry
std::fs::FileType
std::path::{Path, PathBuf}
```

Avoid unnecessary:

```text
metadata()
canonicalize()
```

calls.

Do **not** begin by recreating zlob's low-level syscall layer.

First exhaust algorithmic advantages from:

- better traversal roots;
- fewer opened directories;
- fewer inspected entries;
- fewer metadata calls;
- less allocation.

Only consider deeper low-level work if measurements prove std enumeration is the dominant remaining bottleneck and hackathon rules permit the chosen approach.

---

# 18. Non-UTF-8 Paths

On Unix, avoid forcing path names through UTF-8.

Prefer std path/OS-string interfaces.

Do not casually use:

```rust
to_string_lossy()
```

in hot matching paths.

Required:

- legal non-UTF-8 names do not panic;
- behavior is documented;
- regression tests exist.

---

# 19. Allocation Policy

Avoid unnecessary repeated creation of:

```text
String
PathBuf
Vec
HashSet
```

Prefer:

```text
borrowed entry name
+ current directory state
+ compact pattern state IDs
```

Construct full paths only when necessary for traversal, output, execution, metadata, or diagnostics.

Do not optimize blindly; measure allocations indirectly through RSS and timing if richer profiling is unavailable.

---

# 20. Sequential Before Parallel

First build a correct sequential engine.

Do not start with a worker pool.

Reasons:

- easier correctness;
- easier differential tests;
- easier pruning analysis;
- easier performance attribution.

Threads cannot rescue an engine that opens unnecessary directories.

---

# 21. Parallel Traversal

No Rayon.

Possible std primitives:

```rust
std::thread
std::sync::{Arc, Mutex, Condvar}
std::sync::mpsc
std::sync::atomic
```

Use a bounded worker pool.

Never spawn one thread per directory.

Benchmark thread counts:

```text
1
2
4
8
logical core count
```

Allow sequential/adaptive mode if it wins small or narrow workloads.

---

# 22. CLI Parsing

No Clap.

Use:

```rust
std::env::args_os
```

Prefer `args_os` because paths need not be valid UTF-8.

Keep grammar small and explicit.

---

# 23. Error Handling

No `anyhow` or `thiserror`.

Use:

```rust
std::error::Error
std::fmt::Display
```

Errors should identify:

- pattern/query;
- source position when relevant;
- filesystem path where relevant;
- operation and reason.

Invalid user input should not panic.

---

# 24. Correctness Strategy

Use pinned `fast-glob` as the behavioral oracle where semantics overlap.

Conceptual test flow:

```text
fixture tree
  |-- fast-glob
  `-- Branchcut

normalize paths
-> compare sets
```

Required for a claimed compatible case:

```text
baseline - branchcut = {}
branchcut - baseline = {}
```

Do not compare ordering unless ordering compatibility is claimed.

---

# 25. Deterministic Generated Tests

No QuickCheck/proptest crates.

Generate deterministic combinations of:

- literal path components;
- `*`;
- `?`;
- `**`;
- character classes;
- brace alternatives;
- positive patterns;
- exclusions;
- synthetic directory trees.

Every mismatch must print enough data to reproduce it.

Every discovered bug becomes a fixed regression test in `main.rs`.

---

# 26. Required Test Categories

Must cover:

- literals;
- shallow wildcard;
- globstar;
- question wildcard;
- character classes;
- negated classes;
- brace alternatives;
- multiple positive patterns;
- overlapping positives;
- exclusions;
- hidden files;
- no-match cases;
- duplicate prevention;
- deep trees;
- wide trees;
- symlink policy;
- permission errors;
- non-UTF-8 Unix names;
- early stop;
- extension filters;
- metadata predicates;
- ignore precedence if implemented.

---

# 27. Performance Competitors

## Tier 1 — Primary Package Killer

```text
fast-glob 3.3.3
```

## Tier 2 — Modern Node competitor

```text
tinyglobby
```

## Tier 3 — Conventional Rust stack

Where semantics align:

```text
globset + walkdir
```

## Tier 4 — Native ceiling

```text
zlob
```

Do not assume Branchcut will beat zlob on raw full-tree enumeration.

Our strategic battlefield is **query-planning-heavy work**.

---

# 28. Strategic Benchmark Workloads

## 28.1 Raw full tree

```text
**/*
```

Useful baseline only.

Not the primary differentiation target.

## 28.2 Fixed-prefix

```text
packages/core/src/**/*.rs
```

Branchcut should avoid unrelated trees entirely.

## 28.3 Exclusion-heavy

```text
**/*.{rs,ts}
!**/node_modules/**
!**/target/**
!**/dist/**
```

## 28.4 Multi-pattern

Use:

```text
1
10
100
1000
```

patterns with shared prefixes.

This is one of Branchcut's most important benchmarks.

## 28.5 Monorepo

Representative shape:

```text
packages/
node_modules/
target/
dist/
src/
tests/
generated/
.git/
```

## 28.6 Early termination

```bash
--first
--limit 10
--limit 100
```

Measure time to completion and time to first result.

## 28.7 Metadata constrained

```bash
--glob '**/*.log' --size '>10M'
```

Measure metadata-call overhead and planner behavior.

---

# 29. Benchmark Fairness

Because `fast-glob` is a Node library and Branchcut is native, maintain two benchmark classes.

## Cold end-to-end

Measure:

```text
Node startup + module import + query
```

versus:

```text
Branchcut startup + query
```

Label it clearly as cold invocation.

## Hot engine

Keep the Node process alive and load `fast-glob` once.

Keep equivalent Branchcut benchmark process/context alive.

Repeat the query.

This is the headline engine comparison.

---

# 30. Benchmark Datasets

Required:

- ~10k-file synthetic tree;
- ~100k-file synthetic tree;
- ~1M-file tree if practical;
- deep tree;
- wide tree;
- JS/TS monorepo shape;
- Rust workspace shape;
- at least one substantial public repository.

Record exact commit hashes for public repositories.

---

# 31. Benchmark Metrics

Record:

```text
wall-clock time
CPU time where practical
peak RSS
matches returned
time to first result
directories opened
directories pruned
entries inspected
metadata calls
```

For repeated measurements report:

```text
median
p90
p95
dispersion
```

Use geometric mean or another defensible aggregate for cross-workload speed comparisons.

Never publish only the best case.

---

# 32. Benchmark Integrity Rules

Every public claim must record:

- competitor/version;
- Node/Rust version where relevant;
- OS/kernel;
- CPU;
- RAM;
- filesystem;
- dataset/repo version;
- query/pattern;
- semantic options;
- iteration count;
- Branchcut commit hash.

Rules:

1. Same tree.
2. Same query semantics.
3. Same output requirements.
4. Verify correctness before timing.
5. Do not sort one side and not the other.
6. Do not hide losses.
7. Raw benchmark data must be retained.
8. Do not call a startup benchmark an engine benchmark.

---

# 33. `--stats` Instrumentation

Track at minimum:

```text
dirs_seen
dirs_opened
dirs_pruned_positive
dirs_pruned_ignore
entries_seen
candidate_files
metadata_calls
matches
elapsed
```

Optional:

```text
pattern_states_evaluated
shared_states_reused
```

This is part of the algorithmic proof, not decorative telemetry.

---

# 34. `--explain` Requirements

Explain compiler decisions a developer can act on.

Example:

```text
ROOT
  packages/

SHARED PREFIX
  packages/*/src/

ACTIVE LEAF FILTERS
  .rs
  .ts

EXCLUSIONS
  target/
  node_modules/
  dist/

METADATA
  none

TERMINATION
  first 100 matches

STRATEGY
  fixed-prefix parallel traversal
```

Do not dump opaque internal automaton states by default.

---

# 35. Optional `--why`

P2/P1 if time permits.

Example:

```bash
branchcut --why packages/foo/dist/index.js
```

Possible answer:

```text
IGNORED

rule:
  dist/

source:
  packages/foo/.gitignore:17
```

or explain re-inclusion/matching.

This is useful but must not jeopardize the core engine.

---

# 36. STDLIB.md Candidate Ledger

Only claim substitutions actually implemented.

| Normally installed | Branchcut replacement |
|---|---|
| `fast-glob` | custom query/traversal engine |
| Rust `glob` | custom parser/matcher |
| `globset` | shared compiled pattern states |
| `walkdir` | `std::fs::read_dir` traversal |
| `ignore` | custom ignore/pruning engine |
| `regex` | direct wildcard matcher |
| `rayon` | std worker threads |
| `crossbeam` | std sync/channels |
| `clap` | `std::env::args_os` |
| `anyhow` | custom std errors |
| `thiserror` | manual `Display`/`Error` |
| `smallvec` | std collections / compact layouts |
| `indexmap` | std maps/sets |
| filesize parser crate | custom unit parser |
| duration parser crate | custom duration parser |

Do not pad the list with fake substitutions.

---

# 37. Implementation Sequence

## Phase 1 — Matcher core

Implement/test:

```text
literal
*
?
character classes
```

Then `**`.

## Phase 2 — Sequential traversal

Implement:

- `read_dir`;
- prefix root selection;
- shallow/recursive traversal;
- file/dir filtering;
- hidden handling;
- streaming output.

## Phase 3 — Differential correctness

Pin `fast-glob 3.3.3` and compare supported semantics.

Fix mismatches before optimization.

## Phase 4 — Query compiler

Add:

- multiple positives;
- shared prefixes;
- exclusions;
- braces;
- extension specialization;
- early stop.

## Phase 5 — Stats and explain

Implement `--stats` and `--explain` before major optimization so planner changes can be measured.

## Phase 6 — Structural optimization

Priority order:

1. traversal-root narrowing;
2. shared multi-pattern states;
3. subtree pruning;
4. ignore pruning;
5. metadata avoidance;
6. syscall reduction;
7. allocation reduction;
8. specialized matchers.

## Phase 7 — Sequential benchmark baseline

Measure before adding concurrency.

Identify actual bottlenecks.

## Phase 8 — Parallel traversal

Add bounded std-only worker pool only if representative workloads improve.

## Phase 9 — P1 product features

Pick the highest-value unfinished feature(s):

- hierarchical ignore;
- metadata filters;
- smart case;
- command execution;
- rich diagnostics.

## Phase 10 — Final competitor matrix

Benchmark Branchcut fairly against:

```text
fast-glob
tinyglobby
globset + walkdir
zlob
```

where semantics are comparable.

## Phase 11 — Submission polish

Finalize:

- README;
- STDLIB;
- BENCHMARKS;
- COMPATIBILITY;
- limitations;
- dependency proof;
- one-source-file proof;
- demo.

---

# 38. Agent Rules

Every coding agent must obey:

1. Rust `std` only.
2. Never add a crate without explicit owner approval; expected answer is no.
3. Never copy source from zlob, fast-glob, globset, fd, ripgrep, Wax, or other projects.
4. Independent reimplementation of understood algorithms is allowed.
5. Correctness before optimization.
6. Verify equivalence before benchmarking.
7. Re-run regression tests after optimization.
8. Do not remove benchmarks Branchcut loses.
9. Never claim unsupported syntax or semantics.
10. Never claim universal speed superiority.
11. Prefer pruning filesystem work over speeding up post-hoc filtering.
12. Prefer shared multi-pattern compilation over independent passes.
13. Avoid unnecessary metadata calls.
14. Avoid unnecessary `canonicalize()`.
15. Avoid hot-path UTF-8 conversion of filesystem names.
16. Do not introduce parallel traversal before sequential correctness.
17. Never create unbounded worker/thread counts.
18. Avoid unsafe code unless a measured bottleneck justifies it and the owner approves.
19. Do not chase zlob's low-level syscall tricks before planner-level optimization is exhausted.
20. Preserve the one-source-file goal unless explicitly abandoned.
21. Cut features rather than make `main.rs` incomprehensible.
22. Keep CLI simple.
23. `--stats` must report real counters.
24. `--explain` must report real planning decisions.
25. Every generated-test failure becomes a deterministic regression test.
26. Keep `COMPATIBILITY.md` honest.
27. Keep `STDLIB.md` honest.
28. Keep benchmark raw data.
29. One polished differentiator beats five half-built features.
30. The project wins by opening fewer irrelevant directories, not by feature count.

---

# 39. Non-Goals

Do not build:

- npm/N-API bindings;
- a GUI/TUI;
- a generic regex engine;
- a file watcher;
- a full shell;
- a search-content engine like ripgrep;
- a database/index;
- full `fast-glob` API compatibility;
- every extglob variant;
- cross-platform syscall layers before the main planner is proven.

---

# 40. Five-Minute Demo

## 0:00–0:30 — Problem

Show that typical workflows combine globbing, ignores, file filters, and traversal.

Introduce Branchcut as one compiled query engine.

## 0:30–1:00 — Hackathon proof

Show:

```bash
find . -name '*.rs'
```

Expected:

```text
./src/main.rs
```

Then show empty `[dependencies]`.

## 1:00–1:45 — Useful CLI

Run:

```bash
branchcut --glob 'src/**/*.{rs,toml}'
branchcut config
branchcut -e rs --type file
```

## 1:45–2:45 — Planner

Run a complex query with `--explain` and `--stats`.

Show:

- shared prefixes;
- excluded subtrees;
- metadata avoidance;
- directory pruning.

## 2:45–3:30 — Correctness

Show differential result equivalence against the pinned baseline for representative patterns.

## 3:30–4:30 — Performance

Show hot-engine results first, then cold-start separately.

Include:

- multi-pattern workload;
- exclusion-heavy monorepo workload;
- RSS;
- directories opened/pruned;
- time to first result.

Show zlob honestly even if it wins raw traversal.

## 4:30–4:50 — Zero-dependency craft

Show meaningful stdlib substitutions.

## 4:50–5:00 — Limitations

Explicitly state unsupported syntax/platform behavior and where another tool remains better.

---

# 41. Definition of Done

## Compliance

- [ ] Zero third-party crates.
- [ ] One `.rs` source file.
- [ ] No copied third-party implementation.
- [ ] License included.
- [ ] Dependency proof included.

## P0

- [ ] `*`
- [ ] `?`
- [ ] `**`
- [ ] character classes
- [ ] braces
- [ ] multiple positives
- [ ] exclusions
- [ ] literal-prefix planning
- [ ] streaming
- [ ] `--first`
- [ ] `--limit`
- [ ] `--type`
- [ ] `-e`
- [ ] hidden handling
- [ ] cwd/root
- [ ] `--stats`
- [ ] `--explain`

## Correctness

- [ ] Differential suite has zero mismatches for claimed semantics.
- [ ] No duplicate results.
- [ ] Invalid patterns do not panic.
- [ ] Deep/wide-tree tests pass.
- [ ] Symlink policy tested.
- [ ] Non-UTF-8 Unix regression passes.
- [ ] Early termination tested.

## Performance

- [ ] Cold benchmark recorded.
- [ ] Hot benchmark recorded.
- [ ] Multi-pattern scaling recorded.
- [ ] Exclusion-heavy workload recorded.
- [ ] Peak RSS recorded.
- [ ] Time-to-first-result recorded.
- [ ] Pruning counters recorded.
- [ ] Raw data retained.
- [ ] Losing results retained.

## Documentation

- [ ] README.md
- [ ] STDLIB.md
- [ ] BENCHMARKS.md
- [ ] COMPATIBILITY.md
- [ ] limitations
- [ ] demo commands

---

# 42. Strategic North Star

Every significant design decision should strengthen this statement:

> **Branchcut is a single-file, zero-crate Rust filesystem query engine that compiles multiple globs, exclusions, ignore rules, and file predicates into one traversal plan. Rather than merely walking the filesystem faster, it aims to avoid opening directories that cannot contribute to the query, then proves its correctness and performance against established Node and native implementations.**

The project wins by being:

```text
useful
+ correct
+ aggressively planned
+ measurably fast
+ memory-efficient
+ transparent
+ zero-dependency
+ one source file
```

---

# 45. Git and Commit Discipline

The Git history is part of the hackathon evidence. Treat it as an engineering log, not merely a backup mechanism.

## 45.1 Timing Integrity

- Never commit implementation code written before the official coding window.
- Never backdate Git author or committer timestamps.
- Never import an older implementation and recommit it as new work.
- Planning/docs created before kickoff may remain in history if clearly identifiable as planning artifacts.
- After pushing hackathon implementation history, do not rewrite it merely to make the graph look cleaner.
- Do not use force-push to conceal or replace development history.

If a history rewrite is genuinely required to remove a secret or corrupted artifact, document why it happened.

## 45.2 Commit Ownership

Agents are expected to commit their completed work when Git access is available.

Do not leave a large pile of unrelated completed changes for another agent to commit later.

A commit should represent one coherent engineering change that can be reviewed independently.

Good boundaries:

```text
pattern parser
character-class matcher
literal-prefix planner
negative subtree pruning
stats instrumentation
benchmark harness
non-UTF-8 regression fix
```

Bad boundaries:

```text
misc changes
updates
WIP
everything so far
final changes
```

## 45.3 Commit Frequency

Commit after a coherent milestone when:

1. the implementation is internally consistent;
2. relevant tests pass;
3. the working tree does not contain unrelated edits.

Do not commit every few lines.

Do not wait until several major features have accumulated into one giant commit.

As a practical guideline, most active implementation periods should naturally produce a commit roughly every 30–90 minutes, but **logical atomicity matters more than clock time**.

## 45.4 Commit Message Format

Use:

```text
<type>(<scope>): <imperative summary>
```

Allowed primary types:

```text
feat     new product behavior
fix      correctness bug
perf     measured performance improvement
refactor structural change without intentional behavior change
test     tests / fixtures / differential cases
bench    benchmark infrastructure or benchmark corpus
docs     documentation
chore    repository/tooling maintenance
```

Recommended scopes:

```text
parser
matcher
planner
walk
ignore
query
meta
cli
stats
exec
bench
tests
```

Examples:

```text
feat(matcher): support globstar path components
feat(planner): prune fixed-prefix subtrees
perf(walk): avoid metadata calls for type-free queries
fix(ignore): preserve negated descendant re-inclusion
test(parser): add unterminated class regressions
bench(multipat): add 100-pattern monorepo workload
docs(compat): document brace expansion limits
```

Use imperative summaries:

```text
add
avoid
preserve
reject
compile
prune
stream
```

Avoid:

```text
added stuff
fixing bug
updates
works now
final
```

## 45.5 Pre-Commit Gates

Before every implementation commit, run the smallest relevant validation set.

At minimum:

```bash
cargo fmt -- --check
cargo test
```

When available and appropriate:

```bash
cargo clippy -- -D warnings
```

Before commits touching query semantics, matcher behavior, traversal, ignores, or path handling:

```text
run the relevant differential tests against the pinned oracle
```

A knowingly failing commit is permitted only when there is a strong reason to preserve an intermediate state, and its subject must begin with:

```text
wip:
```

WIP commits must not remain in the final submission history unless they provide meaningful evidence of development and are clearly understandable.

Prefer working commits.

## 45.6 Performance Commit Rules

A `perf(...)` commit must not mean merely:

> this looks faster.

Before labeling a commit `perf`:

1. correctness tests must pass;
2. the relevant benchmark must be run before and after;
3. benchmark conditions must be equivalent;
4. the result must show a repeatable improvement or a clearly justified structural optimization.

The commit body should include concise evidence when meaningful, for example:

```text
perf(planner): share fixed-prefix states across positive globs

100-pattern monorepo, warm median:
  before: 41.8 ms
  after:  29.7 ms
  delta: -28.9%

Differential corpus: 0 mismatches.
```

Do not label unmeasured changes as performance wins.

If an optimization improves one workload but regresses another materially, document both.

## 45.7 Correctness Fix Rules

Every non-trivial correctness bug discovered through:

- differential testing;
- generated tests;
- fuzz-style deterministic generation;
- real repository testing;

must ideally produce this sequence:

```text
1. add reproducible regression test
2. demonstrate failure
3. fix implementation
4. demonstrate pass
5. commit test + fix together when they form one atomic correction
```

The commit message should describe the behavioral failure, not the implementation accident.

Good:

```text
fix(matcher): allow globstar to match zero path components
```

Weak:

```text
fix(matcher): change loop condition
```

## 45.8 Benchmark History

Benchmark infrastructure and optimization code should be separable where practical.

Prefer:

```text
bench(multipat): add shared-prefix scaling workload
perf(planner): merge equivalent traversal states
```

rather than one commit that simultaneously changes:

- benchmark workload;
- implementation;
- expected result;
- README claim.

This prevents accidental benchmark manipulation and makes performance evolution auditable.

For headline benchmark snapshots, record the Branchcut commit hash in `BENCHMARKS.md`.

## 45.9 Documentation Synchronization

When a commit changes user-visible behavior, update the relevant documentation in the same commit when practical:

```text
README
COMPATIBILITY
--help text
STDLIB
BENCHMARKS
```

Do not leave known documentation lies for later cleanup.

Performance claims must not be updated until the corresponding benchmark has actually been run.

## 45.10 No Mixed Cleanup Commits During Hot Work

Do not combine a functional change with unrelated formatting or renaming across the whole source file.

Because Branchcut intentionally uses one Rust source file, indiscriminate formatting/reordering can make diffs enormous.

Keep changes reviewable.

Large structural movement should be its own `refactor(...)` commit.

## 45.11 Generated and Benchmark Artifacts

Do not commit:

- release binaries;
- `target/`;
- temporary benchmark trees;
- cloned benchmark repositories;
- OS/editor junk;
- multi-gigabyte raw datasets.

Commit small raw benchmark result files only when they are useful for auditability and repository size remains reasonable.

Record reproducible dataset-generation commands instead of committing giant synthetic trees.

## 45.12 Secrets and Machine-Specific Data

Before committing, verify no file contains:

- tokens;
- credentials;
- private paths that reveal sensitive information;
- machine-specific temporary configuration;
- private repository URLs.

Do not commit `.env` files.

## 45.13 Branching

For the 72-hour event, prefer a simple history.

Recommended:

```text
main
```

with short-lived branches only when multiple agents are working concurrently on genuinely independent changes.

If branches are used:

- keep them short-lived;
- rebase/merge carefully before benchmark snapshots;
- do not duplicate competing implementations indefinitely;
- ensure final `main` contains the authoritative implementation.

## 45.14 Milestone Tags

Create lightweight or annotated tags at meaningful stable points when useful:

```text
baseline-correct
p0-complete
benchmark-freeze
submission-v1
```

At minimum, create a final submission tag after all checks pass.

Recommended:

```bash
git tag -a submission-v1 -m "Zero Dependency Hackathon submission"
```

## 45.15 Final Submission History Gate

Before final submission:

```bash
git status --short
git log --oneline --decorate --graph --all
```

Requirements:

- working tree clean;
- no implementation files accidentally untracked;
- no secrets;
- no dependency additions;
- no unexplained pre-kickoff implementation commits;
- benchmark commit hash matches published results;
- final tag points to the exact submitted commit.

Then run the full project gates:

```bash
cargo fmt -- --check
cargo test
cargo metadata --no-deps
find . -name '*.rs'
```

Expected source-file result:

```text
./src/main.rs
```

## 45.16 Commit North Star

The history should allow a judge or senior engineer to understand how Branchcut evolved:

```text
correct matcher
→ correct traversal
→ query planner
→ pruning
→ instrumentation
→ measured optimization
→ product features
→ benchmark freeze
→ submission
```

A clean history is not one with the fewest commits.

It is one where every meaningful change has a defensible purpose, validation, and timestamp.

