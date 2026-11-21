---
title: Content search and regex
description: Search selected files with explicit patterns, encodings, inputs, and structured results.
icon: Search
---

`branchcut grep PATTERN [PATH...]` searches file contents. `branchcut find PATTERN [PATH...]` searches names, with `--full-path` selecting root-relative paths. Globs, ignores, exclusions, type, extension, size, and time filters narrow selected files before content scanning. Paths default to the current directory.

```sh
branchcut grep TODO src/ --glob '**/*.rs' -C 2
branchcut grep --glob '**/*.log' --content-mode all -e ERROR -e timeout
branchcut grep --glob '**/*.txt' --invert-match obsolete
```


## Command selection

```sh
branchcut find config src/
branchcut grep TODO src/
branchcut grep -e ERROR -e WARN logs/
branchcut grep -f patterns.txt logs/
branchcut grep timeout application.log -c
```

`find` treats its pattern literally by default, including glob punctuation. Add `--glob` as a separate filter. An empty `find` pattern selects every name, for example `branchcut find '' src/ --glob '**/*.rs'`.

Bare `branchcut grep` and `branchcut find` preserve the original one-word filename shortcut. To search for those words alongside other literal terms, use `branchcut -- grep find`. Command-specific help is available with `branchcut grep --help` and `branchcut find --help`.

## Patterns and regex

The ordinary `branchcut` executable searches literal patterns. Use `grep -e PATTERN` (`--regexp`) to add repeatable patterns. When `-e` or `-f FILE` (`--pattern-file`) supplies patterns, every remaining positional argument is an input path. Patterns default to `--content-mode any`; `all` requires every pattern on the same line and `none` selects lines without any pattern. `--invert-match` reverses final line selection. Pattern files are UTF-8 with one pattern per line. Empty content patterns are rejected.

`--word-regexp` (`-w`) requires word boundaries and `--line-regexp` (`-x`) requires the entire line. These flags also work with literals. `-i` enables ASCII case-insensitive matching, `--smart-case` inspects content patterns for uppercase letters, and `--unicode` enables Unicode-aware matching where supported. Simple Unicode lowercase matching does not provide normalization or full case-fold expansions.

The optional `branchcut-regex` executable adds Rust regex syntax. From a source checkout, install it with the following command.

```sh
cargo install --locked --path integrations/content-regex
```

It uses its own Cargo manifest and lockfile with the `regex` dependency. The core library and ordinary CLI use the standard library.

```sh
branchcut-regex find -E '^test_.*\.rs$'
branchcut-regex find -E '^src/.+\.rs$' --full-path
branchcut-regex grep -E 'TODO|FIXME' src/ --glob '**/*.rs'
branchcut-regex grep -E 'ERROR [0-9]+' --glob '**/*.log'
```

`branchcut-regex find -E PATTERN [PATH...]` matches the entry name. `--full-path` selects a slash-separated root-relative path. `branchcut-regex grep -E PATTERN [PATH...]` applies regex to file contents. `-F` (`--fixed-strings`) selects literals explicitly. Other filters combine with the selected name or content pattern. Use regex alternation to express several filename alternatives.

Content regexes run one line at a time. Anchors, groups, alternation, character classes, and repetitions follow Rust regex syntax. Results report full-expression spans with pattern IDs. Capture-group output, multiline content matching, replacement, PCRE lookaround, and backreferences are unsupported. Patterns that match zero bytes can select a line, while `--only-matching` emits only nonempty spans.

## Inputs

```sh
branchcut grep timeout application.log
branchcut grep ERROR - --output jsonl
branchcut grep --paths-from changed-files.txt TODO
branchcut grep --paths-from0 changed-files.bin -l --print0 TODO
```

After the pattern, positional paths accept files or directories. In `grep`, a positional `-` reads standard input, displayed as `-` in results. `--paths-from FILE` reads newline-delimited paths, accepting CRLF. `--paths-from0 FILE` reads NUL-delimited paths and requires a final NUL. Use `-` as the list filename to read paths from stdin. Paths are resolved relative to `--cwd`. Standard input can be consumed only once per invocation.

Unix NUL path lists preserve native filename bytes. Windows path lists must be valid UTF-8. List input is bounded to 100,000 paths, 1 MiB per path, and 64 MiB in total. Use NUL lists for filenames containing newlines.

`--preprocess PROGRAM` runs a selected executable once per file, passing the file as an argument, and searches its stdout. `--decompress` selects an external decompressor for `.gz`, `.bz2`, `.xz`, `.zst`, and `.zstd` files. The corresponding `gzip`, `bzip2`, `xz`, or `zstd` executable must be on `PATH`. Other extensions are read normally. A failed process is reported as a scan error. These modes are mutually exclusive and use sequential content scans. They do not enumerate ZIP or TAR archive members.

## Encodings and binary data

| `--encoding` | Behavior |
|---|---|
| `auto` | Detect UTF-8 and UTF-16 BOMs. Preserve bytes when there is no BOM. |
| `utf-8` | Remove a leading UTF-8 BOM and preserve remaining bytes. |
| `utf-16le`, `utf-16be` | Decode the specified UTF-16 byte order to UTF-8. |
| `latin1` | Decode ISO-8859-1 to UTF-8. |
| `windows-1252` | Decode Windows-1252 to UTF-8. |
| `raw`, `bytes` | Preserve all bytes, including a BOM. |

Malformed UTF-16 produces replacement characters. Without a UTF-16 encoding, a NUL byte in the first 8 KiB identifies binary input and skips it. `--text` disables that binary skip. This probe is bounded and does not establish that the rest of a file is textual.

Line endings are removed from emitted text, including CRLF. Line numbers start at 1. Columns count decoded bytes starting at 1. `byte_offset` counts decoded stream bytes from 0, after BOM removal. These coordinates differ from original file byte offsets when decoding changes the input.

## Output and limits

```sh
branchcut grep TODO --output jsonl --summary --explain-matches
branchcut grep TODO --output json --json-metadata
branchcut grep TODO --output csv
branchcut grep TODO --output tsv
branchcut grep TODO -o --format '{path}:{line}:{column} {text}'
```

| Mode | Meaning |
|---|---|
| Default text | `path:line:column:text`, one record per selected line. |
| `--only-matching`, `-o` | One record per nonempty matched span. |
| `--count` | Total selected line count. |
| `-c`, `--count-per-file` | `path:count` for each scanned text file, including zero matches. |
| `--files-with-matches`, `-l` | Each file containing a selected line once. |
| `--files-without-match`, `-L` | Each scanned text file with no selected lines once. Binary-skipped inputs are omitted. |
| `--output jsonl` | One schema-versioned record per line. `--json` is an alias for JSONL in content mode. |
| `--output json` | One object with `schema_version`, a `records` array, and optional `summary`. |
| `--output csv`, `--output tsv` | A header and escaped rows sharing the structured record fields. |

`--before-context`, `--after-context`, and `--context` (`-B`, `-A`, `-C`) add surrounding lines. Overlapping context is emitted once. Text context uses `path-line-text`. Machine output identifies these records with `kind: "context"`.

`--first` and `--limit N` count selected lines, or selected files in either file-list mode. Context records and multiple spans on a line do not increase this count. The last match can still be followed by requested trailing context. `--sort` sorts candidate paths before scanning their contents and must collect that path selection first. Limited content scans use sequential scanning for exact termination. Unrestricted scans can use bounded workers with `--threads`.

`--print0` is available in file-list modes and emits native paths separated by NUL. Structured output, quiet/count output, context, and matched-span output have explicit compatibility checks. Context and `-o` cannot be combined with each other or file/count/quiet modes. `-c` (`--count-per-file`) requires unrestricted text output. `--json-metadata` requires JSON or JSONL and adds file type, size, permissions, and modification time. Content search cannot be combined with `--exec`, `--exec-batch`, or command preview.

`--summary` adds totals to structured output. In text mode it writes totals to stderr. Summaries include selected matches, completed inputs, decoded bytes consumed, binary skips, errors, pruned directories, and whether a limit stopped the search. A limited scan can report bytes consumed from only the beginning of an input.

## Structured record contract

Schema version 1 records have a `kind` of `match`, `context`, `file`, `error`, or `summary`. Match and context rows use the following coordinates.

| Field | Meaning |
|---|---|
| `path` | Display path, relative to the query root unless absolute output is requested. |
| `line` | One-based decoded line number. |
| `column` | One-based byte column of the first match, or 1 when there is no match span. |
| `byte_offset` | Zero-based decoded stream offset at the beginning of the full line. |
| `text` | Line text, or the matched substring with `-o`. |
| `text_start` | Zero-based full-line byte position where the `text` payload starts. |
| `spans` | Full-line byte ranges with `start`, exclusive `end`, and zero-based `pattern` ID. |
| `eligibility` | Path-selection reasons when `--explain-matches` is requested. |

Pattern IDs follow the compiled input-pattern order. Span coordinates remain relative to the full line under `-o`, and `text_start` locates its shortened payload. JSON adds hexadecimal `text_bytes` when invalid UTF-8 would make readable text lossy. Native path companions use hexadecimal `path_bytes` on Unix or `path_utf16` on Windows when required. Consumers should use these companions to reconstruct exact data. CSV and TSV include the corresponding columns.

Scan errors become structured error records as well as stderr diagnostics. `grep` exits 0 when a result is selected, 1 when no result is selected, and 2 for invalid input or errors. `--strict` fails on any unreadable input. Ordinary filename searches, including empty results, return 0 unless quiet mode requests an existence status. Legacy flag-based content queries retain their previous exit-code behavior.

Text templates support `{path}`, `{line}`, `{column}`, `{byte_offset}`, `{text_start}`, `{text}`, `{spans}`, `{pattern}`, `{kind}`, and `{eligibility}`, together with the existing path placeholders. Use `{{` and `}}` for literal braces. Templates require text output. Color and hyperlinks apply to terminal text output and are suppressed for machine formats.

## Resource bounds

The default maximum decoded line length is 8 MiB, configurable with `--max-line-bytes` from 1 byte through 64 MiB. Each context side permits at most 1,024 lines, with a 32 MiB before-context buffer. A line can produce at most 65,536 match spans. Exceeding a bound reports a scan error.

The regex integration accepts at most 128 expressions, with bounded compiled state, cache size, and nesting. Worker counts are bounded and reduced when configured line/context buffers would make the requested concurrency excessive. Filesystem-only queries retain their existing traversal behavior.
