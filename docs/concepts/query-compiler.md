---
title: Query compiler
description: How Branchcut turns many patterns into one immutable traversal program.
icon: Cpu
---

## Segment IR

Each path component is parsed once into tokens:

```text
src/**/[a-z]*.rs

Literal("src")
GlobStar
Segment(
  Class(a-z),
  Star,
  Literal(".rs")
)
```

Specialized segment matchers replace the general token machine when possible:

| Shape | Fast path |
|---|---|
| `literal` | byte equality |
| `prefix*` | `starts_with` |
| `*suffix` | `ends_with` |
| general wildcard | direct star backtracking |

## Shared program

All positive patterns are inserted into one flat node graph. Common segments share nodes rather than creating independent matchers or independent directory walks.

```text
src
└── **
    ├── *.rs
    ├── *.toml
    ├── test*.rs
    └── components
        └── **/*.css
```

Active state IDs advance for each discovered component. Globstar nodes contribute both an epsilon transition—matching zero components—and a consuming transition—matching another component.

## Classification

The compiler classifies each pattern:

- `Literal`: no wildcard work is required;
- `SingleDirectory`: matching is confined to one directory level;
- `FixedPrefixRecursive`: a literal prefix narrows the traversal root;
- `UnboundedRecursive`: traversal may begin at the configured root.

## Plan invariants

The compiled plan owns immutable positives, exclusions, programs, root selection, filters, ignore policy, and termination settings. Traversal consumes that plan; it does not reparse query strings in the hot path.
