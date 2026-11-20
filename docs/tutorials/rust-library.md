---
title: Use Branchcut from Rust
description: Compile one reusable query and stream matching native paths from a Rust application.
icon: Braces
---

The command-line adapter is built on the `branchcut` library. The same typed query and traversal API is available to Rust applications without shelling out.

The package is not yet published to crates.io. From a checkout, add a local path dependency to the consuming package:

```toml
[dependencies]
branchcut = { path = "../branchcut" }
```

## Compile and stream

```rust
use branchcut::{Completion, Query};
use std::{io, ops::ControlFlow, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plan = Query {
        patterns: vec!["packages/**/src/**/*.rs".into()],
        exclusions: vec!["**/target/**".into()],
        gitignore: true,
        ..Query::default()
    }
    .compile()?;

    let report = plan.visit(&mut |path: &Path| -> io::Result<ControlFlow<()>> {
        println!("{}", path.display());
        Ok(ControlFlow::Continue(()))
    })?;

    assert_eq!(report.completion, Completion::Complete);
    Ok(())
}
```

Compilation captures query semantics. It does not snapshot the filesystem. Calling `visit` later starts a fresh traversal against the tree that exists at that time.

## Stop early

Return `ControlFlow::Break(())` from the visitor:

```rust
use std::{io, ops::ControlFlow, path::Path};

let mut emitted = 0;
let report = plan.visit(&mut |path: &Path| -> io::Result<ControlFlow<()>> {
    println!("{}", path.display());
    emitted += 1;
    Ok(if emitted == 20 {
        ControlFlow::Break(())
    } else {
        ControlFlow::Continue(())
    })
})?;
```

The report records whether traversal completed, stopped at the visitor's request, or finished with recoverable filesystem errors.

## Parallel streaming

```rust
let report = plan.visit_parallel(4, false, &mut |path: &Path| -> io::Result<ControlFlow<()>> {
    println!("{}", path.display());
    Ok(ControlFlow::Continue(()))
})?;
```

Parallel results are delivered through a bounded coordinator channel and are unordered. Use `collect_parallel` only when the application explicitly wants to retain the complete result set.

## Match paths without walking

`PathMatcher` compiles globs for callers that already have paths and only need to check them. It does not enumerate directories or read metadata. `matching_patterns` returns the zero-based IDs of the input patterns that matched, even when a pattern expands through braces:

```rust
use branchcut::{CaseMode, MatchOptions, PathMatcher};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matcher = PathMatcher::with_options(
        &["src/**/*.rs", "tests/**/*.{rs,toml}"],
        MatchOptions {
            case_mode: CaseMode::Smart,
            unicode: true,
            escaped_patterns: false,
        },
    )?;

    assert_eq!(matcher.matching_patterns(Path::new("src/lib.rs")), vec![0]);
    Ok(())
}
```

Unicode matching is opt-in. Valid UTF-8 paths use scalar wildcards and simple lowercase case matching; invalid native names use byte matching. This does not normalize Unicode or perform multi-character case folding. Use `escape_path_pattern` to quote UTF-8 literal paths for escaped-pattern matching; a non-UTF-8 name cannot be represented as a text pattern.

## Search multiple roots

`Query::compile_roots` builds one plan per root. Results are absolute native paths and duplicate lexical paths are delivered only once:

```rust
use branchcut::Query;
use std::{io, ops::ControlFlow, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let query = Query {
        patterns: vec!["**/*.rs".into()],
        ..Query::default()
    };
    let roots = query.compile_roots(["services/api", "packages/cli"])?;
    for root in roots.roots() {
        eprintln!("search root: {}", root.display());
    }
    let _report = roots.visit(&mut |path: &Path| -> io::Result<ControlFlow<()>> {
        println!("{}", path.display());
        Ok(ControlFlow::Continue(()))
    })?;
    Ok(())
}
```

`visit_parallel(threads, strict, visitor)` is also available. Root plans run independently and are combined in input order. Deduplication retains a set of emitted absolute paths in memory, so memory use grows with unique results; symlink aliases remain distinct lexical paths.

## Include selected-entry metadata

Ordinary path visitors avoid metadata calls unless the query itself requests a metadata filter. Use `visit_entries` when an application needs metadata for each selected entry:

```rust
use branchcut::Query;
use std::{io, ops::ControlFlow, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plan = Query {
        patterns: vec!["**/*.rs".into()],
        ..Query::default()
    }
    .compile()?;
    let report = plan.visit_entries(&mut |entry| -> io::Result<ControlFlow<()>> {
        println!("{}\t{} bytes", entry.path.display(), entry.metadata.len());
        Ok(ControlFlow::Continue(()))
    })?;
    println!("metadata requests: {}", report.stats.metadata_calls);
    Ok(())
}
```

`visit_entries_parallel(threads, strict, callback)` has the same record shape. The callback receives one entry at a time; these APIs do not collect all records. Metadata requests for selected entries are included in the report's `metadata_calls` counter. As with ordinary searches, symlink metadata follows the query's `follow_symlinks` setting.

## Cancel from another thread

`CancellationToken` lets a host application request a cooperative stop after a query has started. Clone the token into the thread that decides when to cancel, then put a clone on `Query` before compiling:

```rust
use branchcut::{CancellationToken, Query};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cancellation = CancellationToken::new();
    let query = Query {
        cancellation: Some(cancellation.clone()),
        ..Query::default()
    };
    let _plan = query.compile()?;
    // Give `cancellation` to the controlling thread; call `cancel()` when needed.
    // A stopped execution reports Completion::Stopped.
    Ok(())
}
```

Cancellation is cooperative and may allow already-running filesystem work to finish. A cancelled token stays cancelled; create a fresh token for a later execution.

## API boundaries

- The core returns native `Path` values and does not print.
- Sorting, limits, serialization, and command execution remain caller concerns.
- Filesystem errors are reported through the visitor and completion report.
- Multi-root search deduplicates lexical absolute paths in memory; it does not resolve symlink aliases to a canonical target.
- Search content, indexing, and a GUI are outside the current library scope.

## Next

- Read [Library and modules](../concepts/library-and-modules.md).
- Inspect [Parallel traversal](../concepts/parallelism.md).
- Verify changes with [Verification](../contributing/verification.md).
