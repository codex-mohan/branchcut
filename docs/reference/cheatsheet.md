---
title: Cheat sheet
description: The Branchcut commands and semantics worth keeping within reach.
icon: FileText
---

## Everyday selections

| Goal | Command |
|---|---|
| Find a filename | `branchcut config` |
| Search a full relative path | `branchcut --full-path config` |
| Find Rust files | `branchcut --glob '**/*.rs'` |
| Select several extensions | `branchcut --glob '**/*.{rs,toml}'` |
| Match any repeated term | `branchcut README LICENSE` |
| Use an exact glob root | `branchcut --glob 'src/**/mod.rs'` |

Simple terms search the filename by default. Multiple terms match with OR. Positive globs also form a union.

## Exclude generated work

```bash
branchcut \
  --glob '**/*.{rs,ts}' \
  --exclude '**/{target,node_modules,dist}/**'
```

An exclusion removes matches. A safely covered subtree can be pruned before it is opened.

## Filter results

| Goal | Option |
|---|---|
| Files, directories, symlinks | `--type file`, `--type dir`, `--type symlink` |
| Extension union | `-e rs -e toml` |
| Large files | `--size '>100MB'` |
| Exact fractional threshold | `--size '>=1.5MiB'` |
| Recent files | `--modified-within 7d` |
| Older files | `--modified-before 30d` |
| Absolute time | `--modified-before '2025-01-01T12:00:00+05:30'` or `--modified-before '@-0.5'` |
| Depth boundary | `--min-depth 2 --max-depth 5` |
| Hidden entries | `--hidden` |
| Nested `.gitignore` | `--gitignore` |
| `.ignore` plus Git sources | `--ignore` |
| Smart case | `--smart-case` |
| Unicode scalar matching | `--unicode` |

Different filter families combine with AND. Metadata is requested only after cheap name, type, extension, and glob checks.

## Stop and order

| Goal | Option |
|---|---|
| Stop after the first match | `--first` |
| Stop after N matches | `--limit 100` |
| Globally sort paths | `--sort` |
| Count only | `--count` |

Without an explicit worker count, `--limit` uses sequential traversal and stops at N delivered matches. Explicit multi-worker traversal is allowed but results are unordered and workers may already have inspected more entries when the limit is reached. `--sort --limit N` scans the full selection because global order must be known.

## Output to another program

| Goal | Option |
|---|---|
| One path per line | default |
| JSON Lines | `--json` |
| NUL-delimited paths | `-0` / `--print0` |
| Absolute paths | `--absolute` |
| Run once per path | `--exec 'command {}'` |
| Run once per batch | `--exec-batch 'command {}'` |

`--exec` and the CLI `--limit` default to sequential traversal. Commands are shell-free and receive native path arguments. `--exec-jobs N` sets the child-process concurrency limit; values greater than one run concurrently, so side-effect order is unspecified.

## Explain and measure

```bash
branchcut --glob '**/*.rs' --exclude '**/target/**' --explain
branchcut --glob '**/*.rs' --exclude '**/target/**' --stats
branchcut --glob '**/*.rs' --why src/generated/client.rs
```

- `--explain`: intended compilation and prune strategy;
- `--stats`: actual traversal counters;
- `--why`: ordered checks for one candidate.

## Saved project query

Create `.branchcut-queries`:

```text
[source]
glob src/**/*.{rs,toml}
exclude **/target/**
exclude **/generated/**
extension rs
extension toml
gitignore
type file
```

Run it with:

```bash
branchcut --config .branchcut-queries --profile source --explain
branchcut --config .branchcut-queries --profile source
```

## Boundaries

- Matching is byte-oriented by default, with opt-in Unicode scalar matching and simple lowercase comparison for valid UTF-8 names; no normalization or multi-character case folding is performed.
- Default case options fold ASCII only. With `--unicode`, smart-case detection and lowercase matching also use Unicode scalars; invalid native byte names fall back to byte matching.
- `--gitignore` loads supported Git sources and nested `.gitignore`; `--ignore` adds `.ignore`; explicit ignore files have higher precedence. Advanced Git configuration and escaping boundaries remain documented.
- v0.2.1 searches paths and metadata. Follow [file-content search](../project/roadmap.md#file-content-search) on the roadmap.
- Prefix/suffix negated extglobs and Unicode case folding are unsupported.

See the [CLI reference](./cli.md) and [glob syntax](./glob-syntax.md) for exact constraints.
