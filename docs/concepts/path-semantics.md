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

## Symlinks and filesystem boundaries

By default, directory symlinks are not followed; `--type symlink` can still select the link entry itself. `--follow` opts into following directory symlinks with cycle detection. This policy forces sequential traversal. The target and link path remain distinct names in results.

On Unix, `--one-file-system` prevents descending to a different device from the starting root. This option is not currently available on Windows.

`--max-open N` bounds open directory handles. When the limit is reached, pending entries from an older open frame are retained in memory and the directory is reopened later. This trades memory and additional I/O for fewer simultaneous handles.

## Native path identity

The matcher keeps filesystem names in native `OsStr`/`Path` form. On Unix it matches borrowed filename bytes. On Windows it matches the encoded bytes exposed by `OsStr::as_encoded_bytes`, rather than converting names through a lossy UTF-16-to-UTF-8 view. Matching is byte-oriented by default:

- `?` consumes one byte;
- classes and ranges compare bytes;
- non-ASCII bytes are not case-folded in this default mode.

With `--unicode`, wildcard matching uses Unicode scalar values for valid UTF-8 names. Case-insensitive matching uses simple lowercase conversion; it does not normalize text or perform multi-character case-folding. Invalid native byte names fall back to byte matching. Neither mode counts grapheme clusters.

On Unix, plain output writes native path bytes. It remains newline-delimited, so a filename containing a newline cannot be represented unambiguously in that mode. JSON Lines includes a readable `path` field and, if the relative path is not UTF-8, a `path_bytes` field containing the complete relative native path as lowercase hexadecimal.

On Windows, human-readable output can replace an unpaired UTF-16 surrogate. JSON Lines adds a `path_utf16` array containing the complete relative path as UTF-16 code units whenever it cannot be represented as UTF-8. Consumers that need lossless path identity should use the platform-specific companion field; the `path` string is for display.

For example, a Unix JSON result for a path with invalid UTF-8 has this shape:

```json
{"path":"readable-or-lossy-display","path_bytes":"646f63732fff2e747874"}
```

The hexadecimal value stands for the full relative path bytes, not just the final filename. A Windows result with unpaired UTF-16 units similarly carries `path_utf16` as an array of integer code units.

Pattern syntax and pattern arguments must be valid UTF-8. Matching a native filename does not require converting that filename to UTF-8.

## Literal search and case

Literal positional search checks the basename by default; `--full-path` selects the cwd-relative path and `--filename` explicitly selects the basename. Glob patterns always match path components and are unaffected by this choice. Case modes fold ASCII letters by default; `--unicode` opts valid UTF-8 names and simple search terms into scalar matching and simple lowercase comparisons. Ignore-file rules are always case-sensitive. On Windows, full-path literal matching recognizes both slash forms as separators, while returned paths retain their native form.

## Hidden components

A path is hidden when a component begins with `.`. Hidden paths are excluded by default, including entire hidden directories. Use `--hidden` to opt in.
