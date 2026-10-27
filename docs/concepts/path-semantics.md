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

## Native path identity

The matcher keeps filesystem names in native `OsStr`/`Path` form. On Unix it matches borrowed filename bytes. On Windows it matches the encoded bytes exposed by `OsStr::as_encoded_bytes`, rather than converting names through a lossy UTF-16-to-UTF-8 view. Wildcard matching is byte-oriented on both platforms:

- `?` consumes one byte;
- classes and ranges compare bytes;
- the engine does not claim Unicode scalar or grapheme semantics.

On Unix, plain output writes native path bytes. It remains newline-delimited, so a filename containing a newline cannot be represented unambiguously in that mode. JSON Lines includes a readable `path` field and, if the relative path is not UTF-8, a `path_bytes` field containing the complete relative native path as lowercase hexadecimal.

On Windows, human-readable output can replace an unpaired UTF-16 surrogate. JSON Lines adds a `path_utf16` array containing the complete relative path as UTF-16 code units whenever it cannot be represented as UTF-8. Consumers that need lossless path identity should use the platform-specific companion field; the `path` string is for display.

For example, a Unix JSON result for a path with invalid UTF-8 has this shape:

```json
{"path":"readable-or-lossy-display","path_bytes":"646f63732fff2e747874"}
```

The hexadecimal value stands for the full relative path bytes, not just the final filename. A Windows result with unpaired UTF-16 units similarly carries `path_utf16` as an array of integer code units.

Pattern syntax and pattern arguments must be valid UTF-8. Matching a native filename does not require converting that filename to UTF-8.

## Hidden components

A path is hidden when a component begins with `.`. Hidden paths are excluded by default, including entire hidden directories. Use `--hidden` to opt in.
