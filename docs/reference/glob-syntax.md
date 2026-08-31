---
title: Glob syntax
description: Supported wildcard grammar and deliberate syntax boundaries.
icon: Asterisk
---

## Supported forms

| Syntax | Semantics | Example |
|---|---|---|
| `*` | Zero or more bytes within one component | `*.rs` |
| `?` | Exactly one byte within one component | `file?.rs` |
| `**` | Zero or more complete path components | `src/**/mod.rs` |
| `[abc]` | One listed byte | `[abc].rs` |
| `[a-z]` | One byte in an inclusive range | `[a-z]*` |
| `[!abc]` | One byte not in the class | `[!0-9]*` |
| `[^abc]` | Alternate negated class | `[^.]*` |
| `{rs,ts}` | One flat alternative group | `*.{rs,ts}` |

## Globstar

Globstar is special only as a complete path segment:

```text
src/**/mod.rs
```

It matches `src/mod.rs`, `src/a/mod.rs`, and `src/a/b/mod.rs`.

## Character classes

An unterminated class, invalid range, or other malformed pattern produces a user-facing error. Ordinary invalid input does not panic.

## Braces

Exactly one common non-nested brace group is expanded:

```text
**/*.{rs,toml}
```

Not supported:

```text
{src,tests}/**/*.{rs,toml}   # multiple groups
*.{js,{m,c}js}               # nested groups
file{1..10}.txt              # range expansion
```

## Not supported

- extglobs such as `+(foo)`, `@(foo|bar)`, and `!(tmp)`;
- captures;
- regex syntax;
- portable backslash escaping;
- case-insensitive or smart-case matching.

Matching is byte-oriented, not Unicode-grapheme-oriented.
