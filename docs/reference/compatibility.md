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
- optional sorting, counting, JSON Lines, and command execution;
- bounded parallel traversal;
- planner explanations and real traversal statistics.

## Deliberate gaps

- extglobs;
- nested braces, brace ranges, and multiple brace groups;
- metadata predicates;
- captures and watch mode;
- case-insensitive or smart-case matching;
- every advanced Git ignore escaping edge case;
- complete Windows byte preservation;
- full API and option parity with `fast-glob`, tinyglobby, or zlob.

## Comparison principle

A feature is claimed compatible only where documented tests compare normalized result sets:

```text
Branchcut − oracle = {}
oracle − Branchcut = {}
```

Ordering is not compared unless deterministic sorting is explicitly selected.
