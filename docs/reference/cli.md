---
title: CLI reference
description: Complete command-line reference for Branchcut 0.1.0.
icon: SquareTerminal
---

## Usage

```text
branchcut [PATTERN|SEARCH]
branchcut [OPTIONS]
```

Files are the default result type. Hidden paths are excluded, traversal is sequential, and output streams in filesystem order unless an option changes that behavior.

## Query inputs

| Option | Meaning |
|---|---|
| `--glob PATTERN` | Add a positive glob. Repeatable. |
| `--exclude PATTERN` | Add an exclusion. Repeatable. |
| positional glob | Shorthand when the argument contains glob syntax. |
| positional text | Literal, case-sensitive filename containment search. |
| `--cwd PATH` | Set the filesystem root. Output remains relative to it. |

Positional globs cannot be combined with `--glob` or simple search terms. Positional search terms cannot be combined with `--glob`.

## Filters

| Option | Meaning |
|---|---|
| `-e EXT`, `--extension EXT` | Match an extension; repeatable and case-sensitive. |
| `--type file` | Emit files; this is the default. |
| `--type dir` | Emit directories. |
| `--type symlink` | Emit symlinks without following directory links. |
| `--hidden` | Include hidden path components. |
| `--gitignore` | Apply root and nested `.gitignore` rules. |

Type aliases `f`, `d`, `directory`, `l`, and `link` are accepted.

## Termination and execution

| Option | Meaning |
|---|---|
| `--first` | Stop after the first sequential match. |
| `--limit N` | Stop after `N` sequential matches. `N` must be greater than zero. |
| `--threads N` | Use bounded parallel traversal. `0` selects up to 16 available workers. |
| `--exec COMMAND` | Execute an argv template once per match; `{}` inserts the path. |

`--threads` cannot be combined with `--limit`, `--first`, or `--exec`.

## Output

| Option | Meaning |
|---|---|
| `--sort` | Collect, globally sort, then apply limits. |
| `--count` | Print only the match count. |
| `--json` | Emit one JSON object per line. |
| `--stats` | Write traversal counters to standard error. |
| `--strict` | Return an error if any filesystem entry cannot be read. |

## Diagnostics

| Option | Meaning |
|---|---|
| `--explain` | Print the compiled plan without traversing. |
| `-h`, `--help` | Print help. |
| `--version` | Print the package version. |

## Option interaction summary

```text
streaming sequential ── supports first / limit / exec
          │
          ├── + sort ── collect all, sort, then truncate
          │
          └── + threads ── buffer results; reject limit and exec
```
