---
title: Path semantics
description: Separators, hidden components, symlinks, bytes, and platform boundaries.
icon: FolderTree
---

## Components, not flattened strings

Globstar operates on path components. In `src/**/mod.rs`, `**` may consume zero or more complete components:

```text
src/mod.rs            match
src/a/mod.rs          match
src/a/b/mod.rs        match
```

Use `/` as the portable pattern separator.

## Relative output

`--cwd PATH` changes the query root, and emitted paths are relative to that root:

```bash
branchcut --cwd /work/repo --glob 'src/**/*.rs'
```

## Symlinks

Directory symlinks are never followed. This prevents traversal cycles and makes the policy deterministic. `--type symlink` reports symlink entries themselves.

## Non-UTF-8 paths

On Unix, filesystem names are matched through borrowed OS-string bytes. Legal non-UTF-8 names do not panic and are preserved for output.

Wildcard matching is byte-oriented:

- `?` consumes one byte;
- classes and ranges compare bytes;
- the engine does not claim Unicode scalar or grapheme semantics.

On Windows, matching currently uses a lossy UTF-16-to-UTF-8 view. Characters outside that view are an explicit compatibility limitation.

## Hidden components

A path is hidden when a component begins with `.`. Hidden paths are excluded by default, including entire hidden directories. Use `--hidden` to opt in.
