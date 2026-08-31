---
title: Simple search
description: Find filenames without writing glob syntax.
icon: Search
---

Simple search is the shortest path from a name fragment to matching files:

```bash
branchcut config
```

This searches file names for the literal, case-sensitive byte sequence `config`.

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

Extension matching is case-sensitive. A leading dot is optional in the extension argument.
