use crate::{CompiledQuery, SearchReport, Visitor};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::{fs, io};

/// Reusable cooperative cancellation shared by an execution and its caller.
/// A cancelled token stays cancelled; use a fresh token for a new search.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);
impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// Opt-in metadata record. The path is cwd-relative, like ordinary visitors.
#[derive(Clone, Debug)]
pub struct EntryRecord {
    pub path: PathBuf,
    pub metadata: fs::Metadata,
}

impl CompiledQuery {
    /// Read metadata for each selected entry, rather than every candidate.
    /// These explicit requests are counted in the returned report. Ordinary
    /// path visitors retain the metadata-avoidance fast path.
    pub fn visit_entries<F>(&self, callback: &mut F) -> io::Result<SearchReport>
    where
        F: FnMut(&EntryRecord) -> io::Result<ControlFlow<()>>,
    {
        let mut adapter = RecordVisitor {
            plan: self,
            callback,
            requests: 0,
        };
        let mut report = self.visit(&mut adapter)?;
        report.stats.metadata_calls += adapter.requests;
        Ok(report)
    }
    pub fn visit_entries_parallel<F>(
        &self,
        threads: usize,
        strict: bool,
        callback: &mut F,
    ) -> io::Result<SearchReport>
    where
        F: FnMut(&EntryRecord) -> io::Result<ControlFlow<()>>,
    {
        let mut adapter = RecordVisitor {
            plan: self,
            callback,
            requests: 0,
        };
        let mut report = self.visit_parallel(threads, strict, &mut adapter)?;
        report.stats.metadata_calls += adapter.requests;
        Ok(report)
    }
}
struct RecordVisitor<'a, F> {
    plan: &'a CompiledQuery,
    callback: &'a mut F,
    requests: u64,
}
impl<F> Visitor for RecordVisitor<'_, F>
where
    F: FnMut(&EntryRecord) -> io::Result<ControlFlow<()>>,
{
    fn matched(&mut self, path: &Path) -> io::Result<ControlFlow<()>> {
        let absolute = self.plan.query().cwd.join(path);
        self.requests += 1;
        let metadata = if self.plan.follow_symlinks {
            fs::metadata(absolute)?
        } else {
            fs::symlink_metadata(absolute)?
        };
        (self.callback)(&EntryRecord {
            path: path.to_owned(),
            metadata,
        })
    }
}
