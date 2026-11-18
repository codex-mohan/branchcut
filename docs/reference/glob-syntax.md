---
title: Glob syntax
description: Supported wildcard grammar and deliberate syntax boundaries.
icon: Asterisk
---

## Supported forms

| Syntax | Semantics | Example |
|---|---|---|
| `*` | Zero or more matching units within one component; bytes by default, Unicode scalar values with `--unicode` for valid UTF-8 names | `*.rs` |
| `?` | Exactly one matching unit within one component; byte by default, Unicode scalar with `--unicode` | `file?.rs` |
| `**` | Zero or more complete path components | `src/**/mod.rs` |
| `[abc]` | One listed byte | `[abc].rs` |
| `[a-z]` | One byte in an inclusive range | `[a-z]*` |
| `[!abc]` | One byte not in the class | `[!0-9]*` |
| `[^abc]` | Alternate negated class | `[^.]*` |
| `{rs,ts}` | Alternatives; groups may nest and compose | `{src,tests}/**/*.{rs,ts}` |
| `{01..05..2}` | Inclusive integer range with optional positive step and zero padding | `file-{01..05..2}.rs` |
| `{a..c}` | Same-case ASCII letter range, ascending or descending | `{a..c}.txt` |

## Globstar

Globstar is special only as a complete path segment:

```text
src/**/mod.rs
```

It matches `src/mod.rs`, `src/a/mod.rs`, and `src/a/b/mod.rs`.

## Character classes

An unterminated class, invalid range, or other malformed pattern produces a user-facing error. Ordinary invalid input does not panic.

## Braces

Multiple groups, nested alternatives, and integer/ASCII-letter ranges expand before pattern compilation:

```text
{src,tests}/**/*.{rs,toml}
{src/{lib,main},tests/unit}.rs
file-{01..05..2}.txt
```

Ranges include endpoints when reached by the positive step and may descend. Expansion rejects empty alternatives, malformed groups, zero steps, oversized integers, more than 256 results, more than eight expansion levels, or over 1 MiB of expanded patterns.

## Extended globs

Within one component, `@(a|b)` matches one alternative, `?(a)` zero or one, `+(a)` one or more, and `*(a)` zero or more. Groups can nest and positive groups can have prefixes or suffixes. Whole-component `!(foo|bar)` rejects complete alternative matches.

Negation with a prefix or suffix, such as `!(foo).rs`, is rejected because its semantics differ between mature glob implementations. Use explicit exclusions when they express the desired selection. These groups do not cross separators. Limits are 4,096 bytes per extended-glob component, 512 compiled nodes, and 16 nested groups; evaluation memoizes reachable offsets instead of exponential backtracking. Ignore files use Git syntax, so extglob operators there are literal punctuation.

## Not supported

- prefix/suffix negated extglobs;
- captures;
- regex syntax;
- portable backslash escaping.

Matching is byte-oriented by default. With `--unicode`, valid UTF-8 names use Unicode scalar wildcard matching and simple lowercase conversion for case-insensitive comparison; no normalization or full case-fold expansion is performed. Invalid native byte names use byte matching. Neither mode counts grapheme clusters. `--ignore-case` and `--smart-case` select the case mode; case-sensitive matching is the default. See the [CLI reference](cli.md) for mode details.
