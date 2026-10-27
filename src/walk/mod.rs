mod parallel;
mod sequential;
use crate::ignore::{IgnoreRule, ignored_by_rules, load_ignore_rules};
use crate::path::path_has_hidden_component;
use crate::{CompiledQuery, Stats};
use std::fs;
use std::io;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

/// Receives native paths as they are discovered. Return `Break(())` to stop.
pub trait Visitor {
    fn matched(&mut self, path: &Path) -> io::Result<ControlFlow<()>>;
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
/// Parallel execution currently buffers paths. Ordering is unspecified.
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
        let mut runner = sequential::Runner::new(self, visitor);
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
        parallel::run_parallel(self, threads, strict)
    }
}
pub(crate) struct RootStart {
    pub(crate) file_type: fs::FileType,
    pub(crate) ignore_rules: Vec<IgnoreRule>,
    pub(crate) ignored: bool,
}

// Reconstruct the context an ordinary walk would encounter before the narrowed
// root. In particular, narrowing must not bypass ignore rules or symlinks.
pub(crate) fn prepare_root<V: Visitor>(
    plan: &CompiledQuery,
    stats: &mut Stats,
    visitor: &mut V,
) -> io::Result<Option<RootStart>> {
    if !plan.hidden && path_has_hidden_component(&plan.root_relative) {
        return Ok(None);
    }
    let mut absolute = plan.cwd.clone();
    let mut relative = PathBuf::new();
    let mut ignore_rules = Vec::new();
    let mut components = plan.root_relative.components().peekable();
    loop {
        stats.metadata_calls += 1;
        let metadata = match fs::symlink_metadata(&absolute) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let file_type = metadata.file_type();
        let (ignored, may_reinclude) = ignored_by_rules(&ignore_rules, &relative);
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
                ignore_rules,
                ignored,
            }));
        };
        if !file_type.is_dir() {
            return Ok(None);
        }
        if plan.gitignore {
            let path = absolute.join(".gitignore");
            match load_ignore_rules(&path, &relative) {
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
