---
title: Library and modules
description: Embed the query engine and understand its source boundaries.
icon: Blocks
---

Branchcut is one Cargo package containing a zero-crate library and a thin executable. The original single-file competition artifact remains at `submission-v1`; current development uses modules.

## Source boundaries

| Module | Responsibility |
|---|---|
| `query` | Typed selection, immutable compilation, traversal-root selection |
| `pattern` | Segment parsing, specialized matching, shared pattern programs |
| `walk` | Root preparation, sequential traversal, bounded parallel collection |
| `ignore` | Scoped ignore rules and ordered overrides |
| `path` | Native filename bytes and path components |
| `filters` | Direct name predicates |
| `stats`, `error` | Work counters and compilation errors |
| `cli` | Arguments, sorting, counts, serialization, diagnostics, commands |

The core never prints results or runs commands. It delivers native `Path` values to a visitor. The CLI is an adapter around that API and is exposed as `branchcut::cli::run()` for the executable entry point.

## Compile and stream

```rust
use branchcut::{Completion, Query};
use std::{io, ops::ControlFlow, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let compiled = Query {
        patterns: vec!["src/**/*.rs".into()],
        exclusions: vec!["**/generated/**".into()],
        gitignore: true,
        ..Query::default()
    }.compile()?;

    let report = compiled.visit(&mut |path: &Path| -> io::Result<ControlFlow<()>> {
        println!("{}", path.display());
        Ok(ControlFlow::Continue(()))
    })?;
    assert_eq!(report.completion, Completion::Complete);
    Ok(())
}
```

Compilation does not snapshot the filesystem. Reusing the plan executes a fresh traversal each time. Positive patterns and search terms each use OR semantics within their group; exclusions and the remaining filters restrict the selected paths.

Return `ControlFlow::Break(())` from the visitor to stop immediately. Visitor errors propagate to the caller. Implement the `Visitor` trait directly to handle per-path filesystem errors through its `error` callback; the default callback discards diagnostics while the report still counts them.

## Completion and errors

- `Complete`: traversal finished without reported filesystem errors.
- `Stopped`: the visitor requested early termination. Check `stats.errors` as well, since errors may have occurred before stopping.
- `Incomplete`: best-effort traversal finished with filesystem errors.

Fatal traversal failures and visitor failures return `io::Error`. A clean no-match query completes successfully. Sorting and result limits are consumer policies; they are not part of the immutable query plan.

## Parallel collection

`compiled.collect_parallel(threads, strict)` returns native paths, a report, and collected filesystem diagnostics. Worker counts must be between 1 and 64. Literal file roots and missing paths have the same selection semantics as sequential queries.

Parallel results are buffered and unordered. Sort them in the consumer when necessary. In best-effort mode, a recoverable ignore or directory-entry error is recorded while other readable entries remain eligible. Strict mode returns an error for filesystem failures, including errors in ancestor ignore files encountered before the narrowed root.

The sequential visitor is the interface for streaming and exact early stopping. Parallel streaming and count-only memory improvements are later roadmap work.

## Verification

`tests/library.rs` uses only the public API. CLI and native-filename regressions live in separate integration suites. Run `cargo doc --locked --no-deps` to build the API reference, and use the contribution verification guide for the complete local and differential gates.
