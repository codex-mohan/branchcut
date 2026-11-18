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

Ignore rules influence traversal before opening children. An excluded parent is pruned; a later negated child rule cannot restore it. Explicitly re-include the parent directory first, or ignore only its contents. The supported rules are checked against local Git.

## Metadata avoidance

Branchcut uses `DirEntry::file_type()` for entry classification. Size and time predicates request metadata only after cheap type, path, extension, and glob filters pass, and share one fetch. Without those predicates the engine avoids per-entry metadata calls. Root-prefix checks are also counted in `metadata_calls`; a narrowed prefix can require checking multiple ancestors to avoid traversing through symlinks or skipping ignore rules. Size predicates are restricted to regular files. Modification-time predicates use the entry's own timestamp, including symlinks; a directory's timestamp never prunes descendants. `--modified-within` captures one clock value at each execution, so a compiled query reused later has a fresh boundary.

Applications that need metadata for every selected entry can use `visit_entries` or `visit_entries_parallel`. These opt-in visitors fetch metadata only for matches, expose it in an `EntryRecord`, and include those explicit requests in `metadata_calls`; ordinary path visitors keep the metadata-avoidance path.

Depth is measured from the configured cwd, not the narrowed physical traversal root. A maximum depth prevents opening child directories beyond the bound; the minimum depth filters results without skipping shallower directories needed to reach eligible descendants. Depth-pruned directories have a dedicated `dirs_pruned_depth` counter.

## Stack behavior

Sequential traversal uses an explicit stack of directory frames and remains depth-first. Tests exercise 600-component trees and cancellation while unwinding frames. Open directory iterators still consume operating-system resources with depth, so this does not promise unlimited depth. Parallel workers also use explicit frame stacks; when the bounded task queue is full, a worker walks the child locally rather than recursing through the call stack. `--max-open` can spill pending entries from open frames into memory and reopen them later, trading memory and I/O for a handle bound.
