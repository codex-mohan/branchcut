---
title: Glossary
description: The terms Branchcut uses for queries, traversal, output, and evidence.
icon: BookA
---

## Query and compilation

**Positive pattern**  
A glob or search term that can select a result. Multiple positives form a union within their supported group.

**Exclusion**  
A rule that removes matching paths and may safely prune an entire subtree.

**Query compilation**  
Turning the resolved query into immutable pattern states, filters, roots, and stop conditions before traversal.

**Compiled plan**  
The reusable output of query compilation. It can be executed multiple times against the current filesystem.

**Literal prefix**  
Leading path components known exactly enough to narrow where traversal starts.

## Traversal

**Entry**  
One directory item returned during traversal. An entry is not necessarily a final match or a regular file.

**Traversal root**  
The physical directory where the walk begins. It may be deeper than `--cwd` when a literal prefix narrows the search.

**Pruning**  
Skipping a directory because no descendant can contribute to the result under the active positive, exclusion, ignore, or depth rules.

**Metadata call**  
An explicit request for additional filesystem information such as size or modification time. Name and type checks may avoid this work.

**Streaming**  
Delivering matches as traversal discovers them instead of collecting the entire selection first.

**Adaptive parallelism**  
Using a bounded worker policy chosen from the query. CLI limits and per-match execution default to one worker, but an explicit worker count can request unordered early termination or concurrent commands when `--exec-jobs` is set.

## Paths and output

**Native path**  
The platform's original path representation. It may not be valid UTF-8 on Unix or losslessly displayable as a single Windows string.

**JSON Lines**  
A stream where each line is an independent JSON object. Branchcut does not wrap the stream in one JSON array.

**NUL-delimited output**  
One native path followed by a zero byte. It is the preferred general-purpose Unix path separator.

**Absolute output**  
Each result is joined to the resolved query root rather than emitted relative to it.

## Evidence

**Oracle**  
An independent implementation or pinned behavior set used by differential tests.

**Result-set equality**  
Both set differences between Branchcut and the oracle are empty after documented normalization.

**Retained loss**  
A measured result where another tool performs better. It remains published with the winning results.

**Work counter**  
A `--stats` measurement such as opened directories, pruned directories, entries inspected, metadata calls, or matches.
