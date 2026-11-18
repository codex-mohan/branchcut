---
title: Saved project queries
description: Store named filesystem selections and inspect the resolved query.
icon: Bookmark
---

Project profiles keep a repeated selection in one small text file. They use the same query concepts as the CLI and do not add a second matcher or configuration format.

## Define profiles

Create a file such as `.branchcut-queries`:

```text
[source]
glob src/**/*.{rs,toml}
exclude **/target/**
exclude **/generated/**
extension rs
extension toml
gitignore
ignore true
ignore-file .branchcut-ignore
case sensitive
type file
unicode false

[recent]
glob **/*.rs
modified-within 7d
max-depth 5

[large]
glob **/*
size >100MB
```

Each non-empty line is one option. The option name and value are separated by whitespace; the value is the complete remainder of the line. Empty lines and lines beginning with `#` are ignored.

Supported selection options are:

```text
glob PATTERN
exclude PATTERN
extension EXT
cwd PATH
type file|dir|symlink
hidden [true|false]
gitignore [true|false]
case sensitive|insensitive|smart
search-target filename|path
search TEXT
size EXPR
modified-within AGE
modified-before WHEN
ignore [true|false]
ignore-file PATH
and TEXT
search-path PATH
unicode [true|false]
follow [true|false]
one-file-system [true|false]
contents-first [true|false]
empty [true|false]
executable [true|false]
owner UID[:GID]
max-open N
min-depth N
max-depth N|none
```

`glob`, `exclude`, `extension`, `search`, `size`, `ignore-file`, `and`, and `search-path` may appear multiple times. Other options may appear once in a profile. `ignore true` enables Git ignore sources plus nested `.ignore` files; `gitignore true` enables Git sources only. Explicit ignore-file entries have the highest rule priority.

## Resolve a query

Select the `default` profile with `--config FILE`, or choose another profile with `--profile NAME`:

```bash
branchcut --config .branchcut-queries --profile recent
branchcut --config .branchcut-queries --profile recent --limit 20
```

Profiles are independent. A named profile does not inherit the `default` profile, so each saved query is understandable on its own.

The query root defaults to the config file's directory, so `branchcut --config /work/project/queries.conf --profile source` works from any launch directory. A relative `cwd` option is resolved from that config directory. Supplying `--cwd` on the command line overrides either form.

Command-line arguments are applied after the selected profile. Scalar flags such as `--type`, `--smart-case`, `--min-depth`, and `--max-depth` override the profile. Repeatable query options append, while repeated extension filters are de-duplicated. Output and execution options belong to the command, so place them on the command line.

Use `--explain` to print the fully resolved plan before traversing:

```bash
branchcut \
  --config .branchcut-queries \
  --profile recent \
  --max-depth 3 \
  --explain
```

Use `--why PATH` with the same config and profile to inspect one candidate under the resolved selection.
