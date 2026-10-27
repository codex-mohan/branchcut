---
title: Compatibility
description: Supported behavior and honest gaps relative to mature glob tools.
icon: BadgeCheck
---

Branchcut intentionally implements a focused filesystem-query surface. It is not a drop-in replacement for the full `fast-glob` API.

## Supported

- `*`, `?`, component-level `**`;
- byte classes, ranges, and negated classes;
- one flat brace group;
- shared multiple positive patterns;
- explicit subtree exclusions;
- file, directory, and symlink result types;
- repeatable extension filters;
- hidden-path control and hierarchical `.gitignore`;
- sequential streaming and early stop;
- optional global sorting, counting, JSON Lines, and command execution;
- bounded parallel traversal;
- planner explanations and real traversal statistics.

## Deliberate gaps

- extglobs;
- nested braces, brace ranges, and multiple brace groups;
- metadata predicates;
- captures and watch mode;
- case-insensitive or smart-case matching;
- every advanced Git ignore escaping edge case;
- Unicode scalar or grapheme matching (wildcard tokens operate on bytes in the platform's encoded representation);
- full API and option parity with `fast-glob`, tinyglobby, or zlob.

## Paths, sorting, and commands

- Unix JSON Lines adds `path_bytes` (lowercase hexadecimal) when the complete relative path is not UTF-8. Windows JSON Lines adds `path_utf16` (UTF-16 code units) when a path cannot be represented as UTF-8. The ordinary `path` field is for display in those cases.
- Unix plain output writes native bytes followed by a newline; filenames containing newlines are ambiguous in this format.
- `--sort --limit N` returns the first N results from the global sorted match set, so it must finish traversal before output. Count-only queries can stop after reaching the count limit.
- `--exec` substitutes native path values and runs with the query root as its working directory. Sorted execution processes the final sorted, limited result set.
- The sequential walker is recursive. Tests cover a 64-component tree, not unlimited depth.

## Comparison principle

A feature is claimed compatible only where documented tests compare normalized result sets:

```text
Branchcut − oracle = {}
oracle − Branchcut = {}
```

Ordering is not compared unless deterministic sorting is explicitly selected.
