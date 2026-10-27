---
title: Benchmarking
description: Rules for fair, reproducible, and interpretable performance evidence.
icon: TimerReset
---

## Repeatable local checks

These development helpers use Node's built-in modules and keep `fast-glob` outside Branchcut's Rust dependency graph. From the repository root, install the pinned oracle under ignored `target/`:

```powershell
npm install --prefix target/oracle --no-save --package-lock=false --no-audit --no-fund --ignore-scripts fast-glob@3.3.3
cargo build --release --locked
node scripts/verify.mjs --binary target/release/branchcut.exe --oracle-dir target/oracle
```

On Linux or macOS, use `target/release/branchcut` as the binary path. The verification script creates and removes a deterministic temporary fixture, checks the installed oracle version, and compares complete result sets for the documented overlapping syntax. It checks for duplicates and exercises both sequential and bounded parallel runs where the query permits them. Oracle and Branchcut hidden-path and symlink options are kept aligned. Ignore-file behavior is tested in Branchcut's own Rust tests because the engines do not claim identical ignore semantics.

The comparison is limited to the semantics exercised by those cases. A passing run does not establish full `fast-glob` compatibility.

## Baseline benchmark

After the correctness checks pass, run a fresh-process, count-only comparison:

```powershell
node scripts/benchmark.mjs --binary target/release/branchcut.exe --oracle-dir target/oracle --warmups 2 --iterations 10 --output benchmark-results/windows.json
```

Use the matching native Branchcut binary path on Linux and macOS. The script builds the same 16,000-file temporary corpus for both engines and verifies the complete sorted result sets before it starts timing. Each measured sample launches a new Branchcut process or a new Node process, includes startup, query, and count output, and records the full raw samples plus the command and environment details in JSON. The query enumerates every matching path in both engines but serializes only the result count. Filesystem cache state is uncontrolled. This is **cold process timing**, not a hot-engine measurement or a cold-disk measurement.

`--output` writes exactly the retained JSON record to the requested path. Choose a durable location for records used in reports. Do not compare runs with different result counts or publish a speed claim from this one synthetic corpus alone.

The runner alternates engine order between iterations and records the temporary corpus filesystem. Use `--binary-label LABEL` to identify a preserved binary separately from the current checkout; its SHA-256 is also recorded. A dirty source tree is recorded explicitly, so the checkout commit alone is not a claim about that binary's exact source.

The temporary tree is created under the operating system's temporary directory. The scripts validate that cleanup remains under that directory before removing it. `target/oracle` contains a development-only correctness oracle; it is not a runtime dependency.

## Correctness gate

Never time a query until result sets agree for the claimed semantics.

```text
baseline − Branchcut = {}
Branchcut − baseline = {}
```

## Record the environment

Every published run must identify:

- Branchcut revision and build mode;
- competitor and version;
- Rust, Node, Zig, or other relevant runtime version;
- OS, CPU, RAM, and filesystem;
- dataset generator or public-repository revision;
- pattern and semantic options;
- warmups, sample count, output mode, and timing boundary.

## Separate hot and cold

Cold CLI measurements include startup, imports, traversal, and requested output. Hot engine measurements keep runtimes loaded and exclude startup. Never label one as the other.

## Match the work

- sort both sides or neither;
- serialize equivalent output or disable serialization on both;
- use the same hidden and symlink policy;
- consume returned arrays when a library would otherwise optimize work away;
- retain failures and losses.

## Report distributions

Prefer median, P90, P95, and dispersion over a single best sample. Use a geometric mean only across defensibly comparable workloads.
