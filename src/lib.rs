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
mod error;
mod filters;
mod ignore;
mod path;
mod pattern;
mod query;
mod stats;
mod walk;
pub use error::QueryError;
pub use query::{CompiledQuery, EntryType, Query};
pub use stats::Stats;
pub use walk::{CollectedMatches, Completion, FilesystemError, SearchReport, Visitor};
#[cfg(test)]
mod tests;
