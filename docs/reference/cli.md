---
title: CLI reference
description: Complete command-line reference for Branchcut 0.3.0.
icon: SquareTerminal
---

## Usage

```text
branchcut find [OPTIONS] PATTERN [PATH...]
branchcut grep [OPTIONS] PATTERN [PATH...]
branchcut grep [OPTIONS] -e PATTERN... [PATH...]
branchcut [PATTERN|SEARCH]
branchcut [OPTIONS]
```

Files are the default result type. Hidden paths are excluded, traversal uses adaptive bounded parallelism, and output streams in discovery order unless an option changes that behavior. Limits and per-match execution default to one worker; explicit worker counts can enable parallel limits or execution with an explicit child-job budget.

## Command selection

`find` searches filenames or root-relative paths, and `grep` searches contents. Both use literal patterns by default. The optional executable supports `branchcut-regex find -E PATTERN` and `branchcut-regex grep -E PATTERN`. `-F` explicitly selects literals. Paths default to the current directory.

```sh
branchcut find config src/
branchcut grep TODO src/
branchcut grep -e ERROR -e WARN logs/
branchcut grep -f patterns.txt logs/
branchcut grep ERROR -
```

`grep -e` (`--regexp`) and `-f` (`--pattern-file`) supply patterns explicitly. When either is present, all remaining positionals are paths. In `grep`, positional `-` reads standard input. `find` accepts an empty pattern to select all names, for example `branchcut find '' src/ --glob '**/*.rs'`. Use `branchcut grep --help` or `branchcut find --help` for focused help.

Bare `branchcut grep` and `branchcut find` retain the original one-word filename shortcut. Use `branchcut -- grep find` to treat both words as ordinary literal search terms. The legacy flag-based interface remains available, including `--content` and `--regex`.

## Query inputs

| Option | Meaning |
|---|---|
| `--glob PATTERN` | Add a positive glob. Repeatable. |
| `--exclude PATTERN` | Add an exclusion. Repeatable. |
| `--and TEXT` | Require an additional literal search term; repeatable. |
| `--content PATTERN`, `--grep PATTERN` | Search selected regular files. See [Content search](#content-search). |
| `branchcut-regex find -E PATTERN [PATH...]` | Match a filename regex. `--full-path` selects slash-separated root-relative paths. |
| positional glob | Shorthand when the argument contains glob syntax. |
| positional text | Literal filename containment search; case behavior is controlled by the case-mode options. |
| `--cwd PATH` | Set the filesystem root. Output remains relative to it. |
| `--search-path PATH` | Add another search root, resolved relative to `--cwd`; repeatable. |
| `--config FILE` | Load a saved query profile file; its directory is the default query root. |
| `--profile NAME` | Select a named profile from `--config`; the default profile is used when omitted. |

In the legacy interface, positional globs cannot be combined with `--glob` or simple search terms, and positional literal terms cannot be combined with `--glob`. The `find` and `grep` commands accept `--glob` as a path filter alongside their own patterns.

An unrecognized option is an error. Put `--` before a literal term that begins with `-`; remaining arguments are treated as literal search terms. Those terms still cannot be combined with explicit globs.

## Filters

| Option | Meaning |
|---|---|
| `--extension EXT` | Match an extension; repeatable and follows the selected case mode. `-e` is an extension alias outside `grep`; within `grep`, `-e` adds a content pattern. |
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

## Content search

Content reads apply only to selected regular files. Globs, exclusions, ignore rules, name/extension/type filters, sizes, and time filters decide which files reach the scanner. The [content search and regex guide](../guides/content-search.md) defines encodings, inputs, schemas, byte offsets, limits, and examples.

| Option | Meaning |
|---|---|
| `PATTERN` | Positional literal pattern for `grep`. |
| `-e PATTERN`, `--regexp PATTERN` | Add a pattern. Repeatable. Remaining positionals are paths. |
| `-f FILE`, `--pattern-file FILE` | Add UTF-8 patterns, one per line. Remaining positionals are paths. |
| `-E`, `--regex` | Interpret patterns as Rust regexes in `branchcut-regex`. |
| `-F`, `--fixed-strings` | Interpret patterns literally. Default. |
| `--content-mode any\|all\|none` | Select lines matching any pattern, every pattern, or no pattern. |
| `-w`, `--word-regexp` | Require word boundaries. Also works with literal patterns. |
| `-x`, `--line-regexp` | Require whole-line matches. |
| `-v`, `--invert-match` | Invert final line selection. |
| `-A N`, `--after-context N` | Add following context lines. |
| `-B N`, `--before-context N` | Add preceding context lines. |
| `-C N`, `--context N` | Add context on both sides. |
| `-o`, `--only-matching` | Emit each nonempty matched span. |
| `-l`, `--files-with-matches` | Emit files containing a selected line. |
| `-L`, `--files-without-match` | Emit scanned text files with no selected lines. |
| `-c`, `--count-per-file` | Emit `path:count`, including zero counts. |
| `--encoding NAME` | `auto`, `utf-8`, `utf-16le`, `utf-16be`, `latin1`, `windows-1252`, or `raw`. |
| `-a`, `--text` | Disable the bounded binary skip. |
| `--max-line-bytes N` | Maximum decoded line length. Default 8 MiB, maximum 64 MiB. |
| Positional `-` | Search standard input with `grep`. |
| Positional `PATH` | Add a file or directory input with `find` or `grep`. |
| `--paths-from FILE` | Read newline-delimited paths. `-` reads the list from stdin. |
| `--paths-from0 FILE` | Read NUL-delimited paths. `-` reads the list from stdin. |
| `--preprocess PROGRAM` | Search stdout from an executable invoked for each selected file. |
| `--decompress` | Use external gzip, bzip2, xz, or zstd tools for supported compressed extensions. |
| `--output FORMAT` | Content `text`, `jsonl`, `json`, `csv`, or `tsv`. |
| `--explain-matches` | Add path-selection reasons to result records. |
| `--summary` | Add totals to structured output or to stderr in text mode. |

The ordinary CLI uses literal matching. Build the optional regex executable from a checkout with `cargo install --locked --path integrations/content-regex`. `branchcut-regex find -E PATTERN` selects names or paths, while `branchcut-regex grep -E PATTERN` selects content lines. The integration has its own declared dependency graph.

Line numbers and columns are one-based. Columns count decoded UTF-8 bytes. Structured `byte_offset` and spans are zero-based decoded byte coordinates after BOM removal, and span ends are exclusive. `--json` aliases JSONL in content mode. `--json-metadata` adds file facts to JSON/JSONL records. `--format` accepts content fields including `{path}`, `{line}`, `{column}`, `{text}`, and `{spans}`.

`--count` counts selected lines. `--first`/`--limit` count lines, or files with `-l`/`-L`. Context and repeated spans do not increase the count. Sorted content mode collects candidate paths, sorts them, and then scans contents. Limited content scans use sequential scanning for exact termination. File-list modes support `--print0`. Content search rejects command execution and preview. Context and span-only output cannot be combined with file/count/quiet modes, and context cannot be combined with span-only output. Per-file count requires unrestricted text output.

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
| `-a`, `--text` | Read binary-looking files as text during `grep` search instead of skipping them. |
| `-l`, `--files-with-matches` | With `grep`, print each matching path once instead of matching lines. |
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
