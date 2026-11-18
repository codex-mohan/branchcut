use crate::{CompiledQuery, Completion, Query, QueryError, SearchReport, Stats, Visitor};
use std::collections::HashSet;
use std::io;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

/// Union of independently planned roots. Results are absolute native paths,
/// deduplicated lexically. Symlink aliases remain distinct paths.
#[derive(Clone, Debug)]
pub struct MultiRootQuery {
    plans: Vec<CompiledQuery>,
}

impl Query {
    pub fn compile_roots<I, P>(&self, roots: I) -> Result<MultiRootQuery, QueryError>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let plans = roots
            .into_iter()
            .map(|root| {
                let mut query = self.clone();
                query.cwd = std::path::absolute(self.cwd.join(root))
                    .map_err(|error| QueryError(error.to_string()))?;
                query.compile()
            })
            .collect::<Result<Vec<_>, _>>()?;
        if plans.is_empty() {
            return Err(QueryError("at least one search root is required".into()));
        }
        Ok(MultiRootQuery { plans })
    }
}
impl MultiRootQuery {
    pub fn roots(&self) -> impl Iterator<Item = &Path> {
        self.plans.iter().map(|plan| plan.query().cwd.as_path())
    }
    pub fn visit<V: Visitor>(&self, visitor: &mut V) -> io::Result<SearchReport> {
        self.run(visitor, None)
    }
    pub fn visit_parallel<V: Visitor>(
        &self,
        threads: usize,
        strict: bool,
        visitor: &mut V,
    ) -> io::Result<SearchReport> {
        if !(1..=64).contains(&threads) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "threads must be between 1 and 64",
            ));
        }
        self.run(visitor, Some((threads, strict)))
    }
    fn run<V: Visitor>(
        &self,
        visitor: &mut V,
        parallel: Option<(usize, bool)>,
    ) -> io::Result<SearchReport> {
        let mut seen = HashSet::new();
        let mut stats = Stats::default();
        let mut completion = Completion::Complete;
        for plan in &self.plans {
            let mut adapter = RootVisitor {
                root: &plan.query().cwd,
                visitor,
                seen: &mut seen,
            };
            let report = match parallel {
                Some((threads, strict)) => plan.visit_parallel(threads, strict, &mut adapter)?,
                None => plan.visit(&mut adapter)?,
            };
            merge_stats(&mut stats, &report.stats);
            if report.completion == Completion::Stopped {
                completion = Completion::Stopped;
                break;
            }
            if report.completion == Completion::Incomplete {
                completion = Completion::Incomplete;
            }
        }
        stats.matches = seen.len() as u64;
        Ok(SearchReport { stats, completion })
    }
}
struct RootVisitor<'a, V> {
    root: &'a Path,
    visitor: &'a mut V,
    seen: &'a mut HashSet<PathBuf>,
}
impl<V: Visitor> Visitor for RootVisitor<'_, V> {
    fn matched(&mut self, relative: &Path) -> io::Result<ControlFlow<()>> {
        let absolute = self.root.join(relative);
        if !self.seen.insert(absolute.clone()) {
            return Ok(ControlFlow::Continue(()));
        }
        self.visitor.matched(&absolute)
    }
    fn error(&mut self, path: &Path, error: &io::Error) -> io::Result<()> {
        self.visitor.error(path, error)
    }
}
fn merge_stats(total: &mut Stats, next: &Stats) {
    total.matches += next.matches;
    total.dirs_seen += next.dirs_seen;
    total.dirs_opened += next.dirs_opened;
    total.dirs_pruned_positive += next.dirs_pruned_positive;
    total.dirs_pruned_exclude += next.dirs_pruned_exclude;
    total.dirs_pruned_ignore += next.dirs_pruned_ignore;
    total.dirs_pruned_depth += next.dirs_pruned_depth;
    total.dirs_pruned_policy += next.dirs_pruned_policy;
    total.entries_seen += next.entries_seen;
    total.candidate_files += next.candidate_files;
    total.metadata_calls += next.metadata_calls;
    total.errors += next.errors;
}
