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

Branchcut uses `DirEntry::file_type()` for entry classification. Current query features do not request size or modification-time metadata, so the engine avoids per-entry `metadata()` calls. One root metadata check establishes the starting path.

## Stack behavior

Sequential traversal is iterative and depth-first. Deep trees therefore do not consume the Rust call stack. The test suite includes a deep-tree regression.
