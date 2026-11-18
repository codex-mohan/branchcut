mod parallel;
mod sequential;
use crate::ignore::{IgnoreRules, directory_rule_sources, ignored_by_rules, initial_ignore_rules};
use crate::path::path_has_hidden_component;
use crate::{CompiledQuery, Stats};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

// Snapshot only spilled entries. Holding a Unix DirEntry can retain the
// directory handle; these owned records do not retain the iterator.
struct BufferedEntry {
    name: std::ffi::OsString,
    absolute: PathBuf,
    file_type: fs::FileType,
}

/// Receives native paths as they are discovered. Return `Break(())` to stop.
pub trait Visitor {
    fn matched(&mut self, path: &Path) -> io::Result<ControlFlow<()>>;
    /// Whether matched callbacks need a materialized cwd-relative path.
    ///
    /// Visitors that only count matches can return `false` and override
    /// [`Visitor::matched_without_path`] to avoid per-entry path allocation.
    fn requires_path(&self) -> bool {
        true
    }
    /// Receives a match when [`Visitor::requires_path`] returns `false`.
    fn matched_without_path(&mut self) -> io::Result<ControlFlow<()>> {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "visitor does not require paths but does not handle pathless matches",
        ))
    }
    /// Receives a cwd-relative child without requiring the engine to join it.
    /// The default preserves the ordinary owned-path callback semantics.
    fn matched_child(&mut self, parent: &Path, name: &OsStr) -> io::Result<ControlFlow<()>> {
        let path = parent.join(name);
        self.matched(&path)
    }
    /// Best-effort errors are reported here and counted in the final summary.
    fn error(&mut self, _path: &Path, _error: &io::Error) -> io::Result<()> {
        Ok(())
    }
}
impl<F> Visitor for F
where
    F: FnMut(&Path) -> io::Result<ControlFlow<()>>,
{
    fn matched(&mut self, path: &Path) -> io::Result<ControlFlow<()>> {
        self(path)
    }
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Completion {
    Complete,
    Stopped,
    Incomplete,
}
#[derive(Debug)]
pub struct SearchReport {
    pub stats: Stats,
    pub completion: Completion,
}
#[derive(Debug)]
pub struct FilesystemError {
    pub path: PathBuf,
    pub error: io::Error,
}
/// Collected parallel results. Ordering is unspecified.
#[derive(Debug)]
pub struct CollectedMatches {
    pub paths: Vec<PathBuf>,
    pub report: SearchReport,
    pub errors: Vec<FilesystemError>,
}
pub(crate) fn report(stats: Stats, stopped: bool) -> SearchReport {
    let completion = if stopped {
        Completion::Stopped
    } else if stats.errors > 0 {
        Completion::Incomplete
    } else {
        Completion::Complete
    };
    SearchReport { stats, completion }
}
#[derive(Default)]
pub(crate) struct ErrorCollector(pub(crate) Vec<FilesystemError>);
impl Visitor for ErrorCollector {
    fn matched(&mut self, _: &Path) -> io::Result<ControlFlow<()>> {
        Ok(ControlFlow::Continue(()))
    }
    fn requires_path(&self) -> bool {
        false
    }
    fn matched_without_path(&mut self) -> io::Result<ControlFlow<()>> {
        Ok(ControlFlow::Continue(()))
    }
    fn error(&mut self, path: &Path, error: &io::Error) -> io::Result<()> {
        self.0.push(FilesystemError {
            path: path.to_owned(),
            error: io::Error::new(error.kind(), error.to_string()),
        });
        Ok(())
    }
}
impl CompiledQuery {
    /// Stream matches without collecting them or performing output/commands.
    pub fn visit<V: Visitor>(&self, visitor: &mut V) -> io::Result<SearchReport> {
        let now = SystemTime::now();
        self.metadata.validate_time(now)?;
        let mut runner = sequential::Runner::new(self, visitor, now);
        runner.run()?;
        Ok(report(runner.stats, runner.stopped))
    }
    /// Collect with a bounded worker pool. `strict` aborts on filesystem errors.
    pub fn collect_parallel(&self, threads: usize, strict: bool) -> io::Result<CollectedMatches> {
        if threads == 0 || threads > 64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "threads must be between 1 and 64",
            ));
        }
        if self.requires_sequential_policy() {
            let now = SystemTime::now();
            self.metadata.validate_time(now)?;
            let mut visitor = SequentialCollector {
                strict,
                ..Default::default()
            };
            let mut runner = sequential::Runner::new(self, &mut visitor, now);
            runner.run()?;
            let stats = std::mem::take(&mut runner.stats);
            let stopped = runner.stopped;
            drop(runner);
            return Ok(CollectedMatches {
                paths: visitor.paths,
                report: report(stats, stopped),
                errors: visitor.errors,
            });
        }
        parallel::run_parallel(self, threads, strict)
    }

    /// Stream unordered parallel matches through one coordinator. Worker
    /// events use a bounded buffer; visitor callbacks need not be `Send`.
    /// Returning `Break(())` cancels pending work and reports `Stopped`.
    pub fn visit_parallel<V: Visitor>(
        &self,
        threads: usize,
        strict: bool,
        visitor: &mut V,
    ) -> io::Result<SearchReport> {
        if threads == 0 || threads > 64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "threads must be between 1 and 64",
            ));
        }
        if self.requires_sequential_policy() {
            let now = SystemTime::now();
            self.metadata.validate_time(now)?;
            let mut adapter = SequentialAdapter {
                inner: visitor,
                strict,
            };
            let mut runner = sequential::Runner::new(self, &mut adapter, now);
            runner.run()?;
            return Ok(report(runner.stats, runner.stopped));
        }
        parallel::visit_parallel(self, threads, strict, visitor)
    }

    pub(crate) fn requires_sequential_policy(&self) -> bool {
        self.follow_symlinks || self.same_file_system || self.contents_first || self.max_open < 64
    }
}

struct SequentialAdapter<'a, V> {
    inner: &'a mut V,
    strict: bool,
}
impl<V: Visitor> Visitor for SequentialAdapter<'_, V> {
    fn matched(&mut self, path: &Path) -> io::Result<ControlFlow<()>> {
        self.inner.matched(path)
    }
    fn requires_path(&self) -> bool {
        self.inner.requires_path()
    }
    fn matched_without_path(&mut self) -> io::Result<ControlFlow<()>> {
        self.inner.matched_without_path()
    }
    fn matched_child(&mut self, parent: &Path, name: &OsStr) -> io::Result<ControlFlow<()>> {
        self.inner.matched_child(parent, name)
    }
    fn error(&mut self, path: &Path, error: &io::Error) -> io::Result<()> {
        if self.strict {
            Err(io::Error::new(
                error.kind(),
                format!("{}: {error}", path.display()),
            ))
        } else {
            self.inner.error(path, error)
        }
    }
}

#[derive(Default)]
struct SequentialCollector {
    paths: Vec<PathBuf>,
    errors: Vec<FilesystemError>,
    strict: bool,
}
impl Visitor for SequentialCollector {
    fn matched(&mut self, path: &Path) -> io::Result<ControlFlow<()>> {
        self.paths.push(path.to_owned());
        Ok(ControlFlow::Continue(()))
    }
    fn error(&mut self, path: &Path, error: &io::Error) -> io::Result<()> {
        if self.strict {
            return Err(io::Error::new(
                error.kind(),
                format!("{}: {error}", path.display()),
            ));
        }
        self.errors.push(FilesystemError {
            path: path.to_owned(),
            error: io::Error::new(error.kind(), error.to_string()),
        });
        Ok(())
    }
}
pub(crate) struct RootStart {
    pub(crate) file_type: fs::FileType,
    pub(crate) metadata: fs::Metadata,
    pub(crate) ignore_rules: IgnoreRules,
    pub(crate) ignored: bool,
}

// Reconstruct the context an ordinary walk would encounter before the narrowed
// root. In particular, narrowing must not bypass ignore rules or symlinks.
pub(crate) fn prepare_root<V: Visitor>(
    plan: &CompiledQuery,
    stats: &mut Stats,
    visitor: &mut V,
) -> io::Result<Option<RootStart>> {
    if plan
        .max_depth
        .is_some_and(|max| plan.root_relative.components().count() > max)
    {
        return Ok(None);
    }
    if !plan.hidden && path_has_hidden_component(&plan.root_relative) {
        return Ok(None);
    }
    let mut absolute = plan.cwd.clone();
    let mut relative = PathBuf::new();
    let mut ignore_rules = if plan.uses_ignores() {
        let initial = initial_ignore_rules(plan)?;
        for (path, error) in initial.errors {
            stats.errors += 1;
            visitor.error(&path, &error)?;
        }
        if initial.blocked.is_some() {
            stats.dirs_seen += 1;
            stats.dirs_pruned_ignore += 1;
            return Ok(None);
        }
        initial.rules
    } else {
        IgnoreRules::default()
    };
    let mut components = plan.root_relative.components().peekable();
    loop {
        stats.metadata_calls += 1;
        let link_metadata = match fs::symlink_metadata(&absolute) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let metadata = if plan.follow_symlinks && link_metadata.file_type().is_symlink() {
            stats.metadata_calls += 1;
            match fs::metadata(&absolute) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(error),
            }
        } else {
            link_metadata
        };
        let file_type = metadata.file_type();
        let (ignored, may_reinclude) =
            ignored_by_rules(ignore_rules.as_slice(), &relative, file_type.is_dir());
        if ignored && !(file_type.is_dir() && may_reinclude) {
            if file_type.is_dir() {
                stats.dirs_seen += 1;
                stats.dirs_pruned_ignore += 1;
            }
            return Ok(None);
        }
        let Some(component) = components.next() else {
            return Ok(Some(RootStart {
                file_type,
                metadata,
                ignore_rules,
                ignored,
            }));
        };
        if !file_type.is_dir() {
            return Ok(None);
        }
        for (path, loaded) in directory_rule_sources(plan, &absolute, &relative) {
            match loaded {
                Ok(rules) => ignore_rules.extend(rules),
                Err(error) => {
                    stats.errors += 1;
                    visitor.error(&path, &error)?;
                }
            }
        }
        absolute.push(component.as_os_str());
        relative.push(component.as_os_str());
    }
}

pub(crate) fn metadata_matches(
    plan: &CompiledQuery,
    absolute: &Path,
    stats: &mut Stats,
    now: SystemTime,
    cached: Option<&fs::Metadata>,
) -> io::Result<bool> {
    if !plan.needs_metadata() {
        return Ok(true);
    }
    if let Some(metadata) = cached {
        return plan.matches_metadata(metadata, absolute, now);
    }
    stats.metadata_calls += 1;
    plan.matches_metadata(&fs::symlink_metadata(absolute)?, absolute, now)
}
