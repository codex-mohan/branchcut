---
title: Traversal and pruning
description: How active pattern states decide whether a directory is worth opening.
icon: Scissors
---

Traversal maintains positive and negative state sets for the current directory.

```text
directory entry
      │
      ├── advance positive states
      ├── advance exclusion states
      ├── apply hidden / ignore state
      └── inspect entry type
               │
        ┌──────┴──────┐
        │             │
      MATCH?       DESCEND?
        │             │
      emit      viable / excluded
                      │
                 open or prune
```

## Positive feasibility

If no active positive state can reach a match in a descendant, the directory is pruned. This is stronger than asking whether the directory itself matches.

## Exclusion pruning

A negative state can prove that an entire subtree is excluded. Safe trailing-globstar patterns such as `**/target/**` are particularly valuable because no descendant can escape the rule.

## Ignore pruning

Ignore rules influence traversal before opening children. Re-inclusion makes the analysis conservative: a directory stays traversable when a negated descendant rule may produce a match.

## Metadata avoidance

Branchcut uses `DirEntry::file_type()` for entry classification. Current query features do not request size or modification-time metadata, so the engine avoids per-entry metadata calls for those predicates. The traversal `metadata_calls` statistic counts root-prefix checks used to validate each path component and avoid traversing through symlinks. A pattern with an empty literal prefix generally checks only the query root; a narrowed prefix can check several ancestors. Both sequential and parallel queries use this same root preparation.

## Stack behavior

Sequential traversal is recursive and depth-first. Its depth therefore uses the Rust call stack. The regression suite exercises a 64-component tree; that test does not guarantee safe traversal of arbitrarily deep filesystem trees. An explicit-stack traversal is a future robustness improvement if real workloads warrant it.
