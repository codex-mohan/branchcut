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

Files are the default result type. Hidden paths are excluded, traversal uses adaptive bounded parallelism, and output streams in discovery order unless an option changes that behavior. Limits and per-match execution default to one worker; explicit worker counts can enable parallel limits or execution with an explicit child-job budget.

## Query inputs

| Option | Meaning |
|---|---|
| `--glob PATTERN` | Add a positive glob. Repeatable. |
| `--exclude PATTERN` | Add an exclusion. Repeatable. |
| `--and TEXT` | Require an additional literal search term; repeatable. |
| positional glob | Shorthand when the argument contains glob syntax. |
| positional text | Literal filename containment search; case behavior is controlled by the case-mode options. |
| `--cwd PATH` | Set the filesystem root. Output remains relative to it. |
| `--search-path PATH` | Add another search root, resolved relative to `--cwd`; repeatable. |
| `--config FILE` | Load a saved query profile file; its directory is the default query root. |
| `--profile NAME` | Select a named profile from `--config`; the default profile is used when omitted. |

Positional globs cannot be combined with `--glob` or simple search terms. Positional search terms cannot be combined with `--glob`.

An unrecognized option is an error. Put `--` before a literal term that begins with `-`; remaining arguments are treated as literal search terms. Those terms still cannot be combined with explicit globs.

## Filters

| Option | Meaning |
|---|---|
| `-e EXT`, `--extension EXT` | Match an extension; repeatable and follows the selected case mode. |
| `--type TYPE` | Select `file` (default), `dir`, `symlink`, `socket`, `fifo`, `block-device`, or `char-device`; repeatable. Aliases include `f`, `d`, `l`, `s`, `p`, `b`, and `c`. |
| `--hidden` | Include hidden path components. |
| `--empty` | Select empty files or directories. |
| `--executable` | Select executable regular files. |
| `--owner UID[:GID]` | Filter by numeric user and optional group ID; Unix only. |
| `--gitignore` | Apply Git ignore sources, including hierarchical `.gitignore` and repository/global excludes. |
| `--ignore` | Apply the Git ignore sources plus hierarchical `.ignore` files. |
| `--ignore-file FILE` | Add an explicit Git-style ignore file; repeatable and highest priority. |
| `-i`, `--ignore-case` | ASCII-insensitive matching for patterns, terms, and extensions. |
| `-s`, `--case-sensitive` | Case-sensitive matching; this is the default. |
| `--smart-case` | ASCII-insensitive unless a positive pattern or literal search term contains uppercase ASCII. Exclusions do not change the decision. |
| `--full-path` | Match literal search terms against the cwd-relative path. Glob behavior is unchanged. |
| `--filename` | Match literal search terms against the entry name (default). |
| `--size EXPR` | Filter regular files by size; repeatable and ANDed. See [everyday search](../guides/everyday-search.md#find-large-downloads). |
| `--modified-within AGE` | Include entries with mtime greater than or equal to `now - AGE`. |
| `--modified-before WHEN` | Include entries with mtime strictly earlier than an age cutoff, UTC/RFC3339 timestamp, or Unix `@seconds` instant. |
| `--min-depth N` | Require result depth at least `N`; cwd is depth 0. |
| `--max-depth N\|none` | Require result depth at most `N`; `none` clears a profile bound. |
| `--follow` | Follow directory symlinks with cycle detection; selects sequential traversal. |
| `--one-file-system` | Do not descend across device boundaries; currently Unix only. |
| `--max-open N` | Limit open directory handles; spilled entries are held in memory and buffered frames resume later. |
| `--contents-first` | Emit matching directories after descendants; selects sequential traversal. |
| `--escaped-globs` | Treat backslashes as pattern escapes instead of Windows path separators. |

Different case-mode options cannot be combined. By default, wildcard matching is byte-oriented and case folding is ASCII-only; non-ASCII bytes are exact. `--unicode` opts valid UTF-8 names into Unicode scalar wildcard matching and simple lowercase case matching. It does not perform full Unicode case folding, normalization, or grapheme matching; invalid native byte names fall back to byte matching. Ignore-file rules are case-sensitive regardless of these options. Case-insensitive mode searches from cwd because literal-prefix root narrowing could miss differently cased directory names on case-sensitive filesystems. On Windows, full-path literal matching treats either slash form as a separator; emitted paths retain native spelling.

Saved profiles use an explicit `--config FILE` path and independent named sections; see [Saved project queries](../guides/project-queries.md). Profile query options are loaded first. Command-line scalar options override the profile, and repeatable query options append. Output and execution behavior stays on the command line.

Size expressions use a number, optionally prefixed by `>`, `>=`, `<`, `<=`, or `=`; a bare number means equality. Decimal fractions are calculated exactly, without floating-point rounding, and are accepted only when unit conversion gives a whole-byte threshold (`1.5MiB` is valid; `0.1B` is rejected). `B`, `K`, `M`, `G`, `T`, `kB`, `KB`, `MB`, `GB`, and `TB` use decimal powers of 1000. `KiB`, `MiB`, `GiB`, and `TiB` use powers of 1024. Repeating `--size` combines conditions with AND.

Durations are a whole number followed by `ms`, `s`, `m`, `h`, `d`, or `w`; zero is allowed, and months/years are not. Each execution captures one clock value for all relative time checks. `--modified-within` includes the exact boundary and permits future timestamps; `--modified-before` excludes the exact boundary. Absolute forms are UTC `YYYY-MM-DD`, `YYYY-MM-DDTHH:MM:SSZ`, RFC3339 timestamps with numeric `±HH:MM` offsets and up to nine fractional-second digits, or Unix seconds prefixed by `@` (for example `@1735689600.25` or `@-0.5`). Calendar date forms are limited to years 1970–9999; Unix seconds may be before the epoch if representable by the platform.

Metadata predicates run after type, name, extension, and glob checks and share one metadata fetch. If no metadata filter is requested, entries avoid per-entry metadata calls. Size selection requires `--type file`. Modification-time checks use the entry itself (including a symlink's own timestamp); directory timestamps never prune descendants.

Type aliases `f`, `d`, `directory`, `l`, and `link` are accepted.

## Termination and execution

| Option | Meaning |
|---|---|
| `--first` | Stop after the first discovered match. |
| `--limit N` | Stop after `N` discovered matches; with sorting, select the first N in global order. `N` must be greater than zero. |
| `--threads N` | Select workers explicitly. `1` is sequential; `0` requests up to 16 available workers. The default adapts to the query. |
| `--exec COMMAND` | Execute an argv template once per match; `{}` inserts the path. |
| `--exec-jobs N` | Bound child-process concurrency; from 1 to 64. |

Explicit worker counts support `--limit` and `--first` with exact coordinator cancellation; the selected paths are unordered unless sorted. Per-match execution with explicit parallel workers also requires an explicit `--exec-jobs` budget. Without an explicit worker count, limits and per-match execution use one worker.

## Output

| Option | Meaning |
|---|---|
| `--sort` | Global native-path order; sorted limits retain at most N paths while scanning all candidates. |
| `--count` | Print only the match count. |
| `--json` | Emit one JSON object per line. |
| `--json-metadata` | Add entry type, size, readonly state, and modification time to JSON Lines. |
| `-0`, `--print0` | Terminate native path output with NUL bytes; incompatible with `--json` and `--count`. |
| `--absolute` | Emit and execute paths rooted at the resolved `--cwd`. |
| `--format TEMPLATE` | Format each path using the documented path placeholders. |
| `--color WHEN`, `--hyperlink WHEN` | Control plain-output decoration: `auto`, `always`, or `never`. |
| `--stats` | Write traversal counters to standard error. |
| `--strict` | Return an error if any filesystem entry cannot be read. |
| `--exec COMMAND` | Execute an argv template once per match; `{}` inserts one path. |
| `--exec-batch COMMAND` | Execute an argv template for bounded path batches; exactly one whole `{}` argument is required. |
| `--batch-size N` | Set the maximum paths per batch; defaults to 128 and requires `--exec-batch`. |
| `--exec-jobs N` | Limit concurrent child commands to 1–64. |
| `--exec-preview` | Print command argv as JSON Lines without launching child processes. |

## Diagnostics

| Option | Meaning |
|---|---|
| `--explain` | Print the compiled plan without traversing. |
| `--why PATH` | Explain one cwd-relative candidate against the query without traversing unrelated directories. |
| `-q`, `--quiet` | Print no paths and return success if any match is found. |
| `--format TEMPLATE` | Format paths using `{}`, `{/}`, `{//}`, `{.}`, or `{/.}`. |
| `--color WHEN`, `--hyperlink WHEN` | Set plain-output decoration to `auto`, `always`, or `never`. |
| `--completions SHELL` | Print completion definitions for bash, zsh, fish, or PowerShell. |
| `-h`, `--help` | Print help. |
| `--version` | Print the package version. |

`--why` reports `MATCHED` or `EXCLUDED` followed by `PASS`, `FAIL`, and `SKIP` checks. When ignore processing is enabled, the decisive ignore rule includes its source path and line. `--explain` and `--why` cannot be combined.

## Option interaction summary

```text
streaming sequential ── supports first / limit / exec
          │
          ├── + sort ── scan all; retain N if limited, otherwise collect all
          │
          └── adaptive workers ── bounded stream; explicit parallel limits and budgeted execution
```

Batch execution is also compatible with sorting. The command runner uses native path arguments and applies a 28 KiB command-size budget in addition to `--batch-size`. `--exec-preview` displays the planned argv but does not run it.
