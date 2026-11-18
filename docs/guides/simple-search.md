---
title: Simple search
description: Find filenames without writing glob syntax.
icon: Search
---

Simple search is the shortest path from a name fragment to matching files:

```bash
branchcut config
```

This searches file names for the literal text `config`. Matching is case-sensitive by default. Matching is byte-oriented unless `--unicode` is selected.

## Multiple terms

Positional terms are compiled into the simple-search plan:

```bash
branchcut config settings
```

A filename matches when it contains a supplied term. Use explicit globs when directory structure or wildcard semantics matter.

## Literal means literal

Plain positional input does not interpret wildcard syntax unless the argument contains a recognized glob metacharacter. For an unambiguous advanced query, prefer `--glob`:

```bash
branchcut --glob '**/config.*'
```

## Positional glob shorthand

Arguments containing glob syntax are accepted as shorthand:

```bash
branchcut '**/*.rs'
branchcut 'src/*/*.toml'
```

Positional glob patterns cannot be mixed with `--glob` or literal search terms. This is rejected because it would make the query mode ambiguous.

## Combine search with filters

Simple search can still use file predicates:

```bash
branchcut config -e toml --type file
branchcut cache --cwd packages --hidden
```

Use `-i` for ASCII-insensitive matching or `--smart-case` to ignore ASCII case unless the positive query contains an uppercase ASCII letter. These modes also affect extension filters; ignore rules remain case-sensitive. By default, non-ASCII bytes are compared exactly. `--unicode` opts valid UTF-8 names into scalar matching and simple lowercase comparison, without normalization or multi-character case folding; invalid native byte names keep byte matching.

Literal terms search the basename by default. Add `--full-path` to search the cwd-relative path instead; `--filename` selects the default explicitly. These switches do not change glob matching. On Windows, slash and backslash are treated as path separators for full-path literal matching, while output keeps the native path.

```bash
branchcut --smart-case README
branchcut --full-path package.json
branchcut --filename config --max-depth 3
```
