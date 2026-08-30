# Competitive Comparison

This document separates **accuracy**, **filesystem work**, **hot-engine latency**, and **cold CLI latency**. Startup is not used for the hot-engine claim.

## Tools and versions

| Tool | Version / build | Role |
|---|---|---|
| Branchcut | current release, Rust 1.96.0 | Subject under test |
| fast-glob | 3.3.3 | Node glob oracle and competitor |
| tinyglobby | 0.2.14 | Node glob competitor |
| zlob | official v1.6.3 release tag, Zig 0.16.0, `ReleaseFast` | Zig/native competitor |

External comparators were installed outside this repository. They are not Branchcut runtime dependencies.

## Dataset

The benchmark corpus contains exactly 16,000 files:

```text
20 package directories
20 × 250 Rust files under src/
20 × 250 TOML files under src/
20 × 150 generated Rust files under target/debug/
20 × 150 TypeScript files under node_modules/pkg/
```

The corpus is synthetic and generated under `C:/tmp/branchcut-speed`. It is not committed to the repository.

## Accuracy protocol

Before timing, each query was run through Branchcut, fast-glob, and tinyglobby. Results were normalized into sets, so filesystem enumeration order was ignored. The comparison checked:

```text
Branchcut − competitor = {}
competitor − Branchcut = {}
```

### Accuracy results: small fixture with hidden files

| Case | Branchcut | fast-glob | tinyglobby | Equality |
|---|---:|---:|---:|---|
| Hidden paths excluded by default | 4 | 4 | 4 | PASS |
| Hidden paths explicitly included | 5 | 5 | 5 | PASS |
| Explicit target exclusion | 4 | 4 | 4 | PASS |

### Accuracy results: 16,000-file corpus

| Case | Branchcut | fast-glob | tinyglobby | Set equality |
|---|---:|---:|---:|---|
| `packages/*/src/*.rs` | 5,000 | 5,000 | 5,000 | PASS |
| `packages/**/src/**/*.rs` | 5,000 | 5,000 | 5,000 | PASS |
| `packages/**/src/*.{rs,toml}` | 10,000 | 10,000 | 10,000 | PASS |
| Two positive patterns | 500 | 500 | 500 | PASS |
| `**/*.{rs,toml}` | 13,000 | 13,000 | 13,000 | PASS |

### zlob accuracy observation

The tested zlob Windows CLI returned 8,000 matches for `**/*.rs`, but returned **no matches** for `**/src/**/*.rs` and `packages/**/src/**/*.rs`, while Branchcut returned 5,000 and the Node oracles agreed with Branchcut. This was observed both with an absolute root argument and with the process working directory set to the corpus.

This is recorded as an observed compatibility result for the tested zlob checkout, not generalized to every zlob API or platform. The zlob README claims broad globstar support, so this discrepancy should be raised upstream before using zlob as a compatibility oracle for nested globstars.

## Hot-engine speed

Hot measurements exclude startup:

- Branchcut's `--stats elapsed` begins after argument parsing and query planning and uses `--count`, so path serialization is excluded.
- zlob v1.6.3 used a temporary Zig harness calling its public `zlob.match` filesystem API directly with `std.heap.c_allocator`, 3 warmups, and 10 measured iterations in one process.
- The harness used `nosort=true`, matching Branchcut's count-only streaming path.
- Node tools were loaded once and called synchronously 10 times after 3 warmups; their returned arrays were consumed but not serialized.

Workload for all tools:

```text
**/*.{rs,toml}
13,000 matches
hidden files excluded
no output serialization
```

| Engine | Median / average per query | P90 where captured | Matches |
|---|---:|---:|---:|
| Branchcut release traversal | **22.079 ms** | 27.043 ms | 13,000 |
| tinyglobby 0.2.14 | 35.666 ms | 40.276 ms | 13,000 |
| fast-glob 3.3.3 | 37.528 ms | 44.442 ms | 13,000 |
| zlob v1.6.3 public `match` API | 133.408 ms average | not captured | 13,000 |

Relative hot-path observations:

```text
fast-glob / Branchcut:  1.70x
tinyglobby / Branchcut: 1.61x
zlob / Branchcut:       6.04x
```


## Cold CLI speed

For completeness, 10 fresh process launches with complete sorted output produced:

| Tool | Median | Matches |
|---|---:|---:|
| Branchcut release | 24.99 ms | 10,000 |
| fast-glob 3.3.3 | 148.53 ms | 10,000 |

This is a startup-inclusive CLI result and is not the headline engine comparison.

## Filesystem-work evidence

For Branchcut's 13,000-match corpus query, `--stats` reported:

```text
matched                 13000
directories considered    122
directories opened        122
entries inspected       16121
candidate files         16000
metadata calls              1
filesystem errors           0
```

For the exclusion-heavy 10,000-match query, Branchcut reported 20 pruned directories and one root metadata call. Node and zlob do not expose equivalent counters through the tested public CLI/API paths, so no invented cross-tool counter comparison is made.

## What each tool fetches and misses

“Fetches” here means filesystem entries opened/inspected, not network data.

| Capability | Branchcut | fast-glob 3.3.3 | tinyglobby 0.2.14 | zlob v1.6.3 |
|---|---|---|---|---|
| Recursive globstar | Supported | Supported | Supported | Supported; nested case above mismatched |
| Braces | Flat alternatives | Supported | Supported | Supported |
| Character classes | Supported | Supported | Supported | Supported |
| Multiple positives | One shared traversal program | Supported | Supported | One CLI pattern per invocation |
| Explicit exclusions | Supported; subtree pruning | Supported via ignore options | Supported via ignore options | Not exposed by tested CLI |
| Hierarchical `.gitignore` | Supported with `--gitignore` | Not enabled by default | Not enabled by default | Supported |
| Ordered negation/re-inclusion | Supported conservatively | Supported through ignore engine | Supported through ignore engine | Supported |
| Hidden default | Excluded by default; `--hidden` | `dot` option | `dot` option | `-H` |
| Hidden override | `--hidden` | `dot: true` | `dot: true` | `-H` |
| File filtering | Files default; dirs/symlinks explicit | `onlyFiles` / `onlyDirectories` | `onlyFiles` / `onlyDirectories` | `-d` and walker flags |
| Symlink following | Never follows directories | Configurable | Configurable, default true | Configurable; default does not follow |
| Streaming | Yes by default | Returns array | Returns array | CLI output iterator; API result object |
| Early termination | `--first`, `--limit` | Caller truncates after collection | Caller truncates after collection | CLI limit |
| Global sorting | `--sort` | Supported | Supported | `-s` |
| Count-only output | `--count` | Caller computes length | Caller computes length | `-c` |
| JSON output | JSON Lines via `--json` | Caller serializes | Caller serializes | Caller serializes |
| Command execution | Shell-free `std::process::Command` | Not a glob feature | Not a glob feature | Not a glob feature |
| Metadata predicates | Not implemented | Not a core glob feature | Not a core glob feature | Walker metadata API |
| Extglob | Not implemented | Supported subset | Supported through underlying engine | Supported |
| Parallel traversal | Not implemented | Internal detail | Internal detail | Separate walker API, not benchmarked here |
| Runtime dependencies | None | 17 transitive npm deps | 2 transitive npm deps | Zig/native build ecosystem |

The matrix intentionally distinguishes “not implemented” from “implemented and slower.” Branchcut's competitive thesis is a smaller, transparent planner with fewer filesystem opens, not feature parity with every mature library.

## Method validation checklist

- Same corpus for every measured query.
- Same pattern semantics where the tool supports them.
- Same hidden-file policy.
- Same match counts verified before timing.
- Node tools hot-loaded once for hot measurements.
- zlob public filesystem API called directly in one process.
- Branchcut path serialization disabled for hot measurement.
- Warmups separated from measured iterations.
- Cold CLI results labeled separately.
- zlob nested-globstar mismatch retained rather than excluded.
- No competitor was added to Cargo.toml or shipped in the artifact.
