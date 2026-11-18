//! Compile filesystem selections once, then stream native paths without a
//! terminal, serializer, command runner, or third-party dependency.
//!
//! ```
//! use branchcut::{Query, Completion};
//! use std::{io, ops::ControlFlow, path::Path};
//! let query = Query { patterns: vec!["src/**/*.rs".into()], ..Query::default() }.compile()?;
//! let report = query.visit(&mut |path: &Path| -> io::Result<ControlFlow<()>> {
//!     println!("{}", path.display());
//!     Ok(ControlFlow::Continue(()))
//! })?;
//! assert_eq!(report.completion, Completion::Complete);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
pub mod cli;
mod diagnostic;
mod entries;
mod error;
mod filters;
mod ignore;
mod matcher;
mod metadata;
mod multi_root;
mod path;
mod pattern;
mod query;
mod stats;
mod walk;
pub use diagnostic::{CheckOutcome, PathCheck, PathExplanation};
pub use entries::{CancellationToken, EntryRecord};
pub use error::QueryError;
pub use matcher::{MatchOptions, PathMatcher, escape_path_pattern};
pub use metadata::{MetadataFilters, ModifiedBefore, SizeFilter, parse_duration};
pub use multi_root::MultiRootQuery;
pub use query::{CaseMode, CompiledQuery, EntryType, Query, SearchTarget};
pub use stats::Stats;
pub use walk::{CollectedMatches, Completion, FilesystemError, SearchReport, Visitor};
#[cfg(test)]
mod tests;
