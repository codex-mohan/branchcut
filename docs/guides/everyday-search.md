---
title: Everyday search
description: Use size, age, case, and depth filters for practical filesystem searches.
icon: SlidersHorizontal
---

Branchcut keeps filename search concise while letting you narrow results with composable filters. Filters combine with AND; repeated literal terms and positive patterns each match with OR semantics within their own group.

## Find recently changed documentation

```bash
branchcut --glob '**/*.md' --modified-within 30d
```

`--modified-within` includes entries whose timestamp is exactly at the cutoff (`mtime >= now - age`) and accepts future timestamps. Durations are whole numbers followed by `ms`, `s`, `m`, `h`, `d`, or `w`; calendar months and years are not supported. Zero is valid. A single clock value is captured at the start of each execution, so a reused library query gets a fresh boundary each time.

To find entries older than a relative age, use the strict comparison:

```bash
branchcut --glob '**/*.log' --modified-before 90d
```

Absolute cutoff forms are UTC `YYYY-MM-DD` (midnight at the start of that date) or `YYYY-MM-DDTHH:MM:SSZ`, for years 1970 through 9999. A file exactly at a `--modified-before` cutoff does not match.

## Find large downloads

```bash
branchcut --cwd ~/Downloads --size '>100MB' --sort
```

Size expressions accept a whole number, optionally preceded by `>`, `>=`, `<`, `<=`, or `=`. Without an operator, the number means exact equality. `B`, `K`, `M`, `G`, `T`, `kB`, `KB`, `MB`, `GB`, and `TB` use decimal powers of 1000. `KiB`, `MiB`, `GiB`, and `TiB` use binary powers of 1024. Fractions are rejected. Multiple `--size` options are combined with AND, and size filters require regular-file selection (`--type file`, the default).

Metadata is fetched only for entries that pass the cheaper type, name, extension, and glob checks; requested size and time checks share that fetch. Without metadata filters, the query avoids per-entry metadata calls. Modification time belongs to the entry itself, so a symlink is checked using the link's timestamp. A directory's timestamp never causes Branchcut to prune its children.

## Limit a project search by depth

```bash
branchcut --glob '**/*.rs' --min-depth 2 --max-depth 4
```

The cwd root is depth 0 and each child component adds one. Bounds apply to result paths even if a literal prefix lets the planner begin traversal deeper in the tree. A minimum depth suppresses shallow results while still traversing through them to find deeper ones. A maximum depth stops before opening child directories beyond the bound. `--min-depth` cannot exceed `--max-depth`.

## Choose how names match

```bash
branchcut --ignore-case README
branchcut --smart-case readme
branchcut --full-path Cargo.toml
```

Case-sensitive matching remains the default. `-i` / `--ignore-case` folds ASCII letters. `--smart-case` folds ASCII unless a positive glob or literal search term contains uppercase ASCII; exclusions do not affect the decision. Extension filters use the chosen mode. `.gitignore` rules remain case-sensitive under all modes. Non-ASCII bytes are compared exactly; this is not Unicode case folding.

Simple terms search the basename by default. `--full-path` searches the path relative to `--cwd`; `--filename` selects the basename explicitly. These flags affect literal terms only, not globs. On Windows, full-path matching treats `/` and `\` as separators, while output preserves native paths. Case-insensitive glob queries search from cwd instead of narrowing to a literal prefix, because casing differences can identify different directories on case-sensitive filesystems.

Use `--explain` to see the selected case mode, search target, depth bounds, metadata requirements, and whether prefix narrowing was disabled. Use `--stats` to inspect the traversal counters, including directories pruned by the maximum depth.

## Search inside selected files

```bash
branchcut --glob '**/*.rs' --exclude '**/target/**' --content 'TODO'
branchcut --glob '**/*.log' --content ERROR --content WARN -l
branchcut --glob '**/*.toml' --content 'edition = "2024"' --count
```

`--content` (alias `--grep`) runs after the path query has selected its files. Globs, exclusions, ignore rules, type, extension, size, and time filters all narrow what is read, so content work stays inside the compiled plan. Repeating the option ORs literal patterns. Output is `path:line:column:text`, where `column` is a one-based byte offset and trailing CRLF is removed.

Default matching is case-sensitive; `-i` folds ASCII and `--smart-case` folds unless a content pattern contains uppercase ASCII. A file is treated as binary when a NUL byte appears in its first 8 KiB and is skipped unless `--text` is passed. `--count` counts matching lines, `-l` prints each matching path once, and `--limit`/`--first` stop after the requested number of matching lines. `--json` emits `path`, `line`, `column`, and `text`; `--json-metadata` adds file fields. Content search requires regular files and cannot be combined with `--exec`, `--exec-batch`, `--exec-preview`, `--print0`, or `--format`.

This is not a general-purpose grep replacement: patterns are literal and line-oriented, with no regular expressions, captures, multiline matching, or replacement. Use the path query when selection is the hard part and a literal content condition is enough; reach for a regex-capable tool when the pattern itself is complex.
