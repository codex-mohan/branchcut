//! Terminal interface. Filesystem selection lives in the independent core modules.
pub(crate) mod args;
mod completions;
mod config;
pub(crate) mod exec;
pub(crate) mod output;
use crate::error::{AppError, Result};
use crate::{CompiledQuery, Stats, Visitor};
use args::{Options, parse_args};
use output::{Output, print_explain, print_stats, print_why};
use std::io::{self, Write};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

pub(crate) fn execute<W: Write>(
    plan: &CompiledQuery,
    options: &Options,
    sink: &mut Output<W>,
) -> io::Result<Stats> {
    let mut stats = if !options.search_paths.is_empty() {
        let roots = options
            .query()
            .compile_roots(&options.search_paths)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
        let mut adapter = RootOutput {
            cwd: std::path::absolute(&options.cwd)?,
            sink,
        };
        if options.threads > 1 {
            roots.visit_parallel(options.threads.min(64), options.strict, &mut adapter)?
        } else {
            roots.visit(&mut adapter)?
        }
        .stats
    } else if options.threads > 1 {
        plan.visit_parallel(options.threads.min(64), options.strict, sink)?
            .stats
    } else {
        plan.visit(sink)?.stats
    };
    sink.finish()?;
    stats.matches = sink.matched;
    stats.metadata_calls += sink.metadata_calls;
    Ok(stats)
}
struct RootOutput<'a, W: Write> {
    cwd: PathBuf,
    sink: &'a mut Output<W>,
}
impl<W: Write> Visitor for RootOutput<'_, W> {
    fn matched(&mut self, absolute: &Path) -> io::Result<ControlFlow<()>> {
        self.sink
            .matched(absolute.strip_prefix(&self.cwd).unwrap_or(absolute))
    }
    fn error(&mut self, path: &Path, error: &io::Error) -> io::Result<()> {
        self.sink.error(path, error)
    }
}
fn real_main() -> Result<ExitCode> {
    let options = parse_args()?;
    let plan = options.query().compile()?;
    if !options.search_paths.is_empty() && options.why.is_some() {
        return Err(AppError(
            "--why requires a single --cwd; omit --search-path".into(),
        ));
    }
    if let Some(path) = &options.why {
        let explanation = plan
            .explain_path(path)
            .map_err(|error| AppError(format!("cannot explain {}: {error}", path.display())))?;
        print_why(&explanation);
        return Ok(ExitCode::SUCCESS);
    }
    if options.explain {
        if options.search_paths.is_empty() {
            print_explain(&plan, &options);
        } else {
            for root in &options.search_paths {
                print_explain(&plan.for_root(options.cwd.join(root))?, &options);
            }
        }
        return Ok(ExitCode::SUCCESS);
    }
    let stdout = io::stdout();
    let mut sink = Output::new(
        &options,
        io::BufWriter::with_capacity(64 * 1024, stdout.lock()),
    )?;
    let started = Instant::now();
    let stats = match execute(&plan, &options, &mut sink) {
        Ok(stats) => stats,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(ExitCode::SUCCESS),
        Err(error) => return Err(AppError(format!("query failed: {error}"))),
    };
    if options.strict && stats.errors > 0 {
        return Err(AppError(format!(
            "query incomplete: {} filesystem error(s)",
            stats.errors
        )));
    }
    if options.stats {
        print_stats(&stats, started.elapsed());
    }
    Ok(if options.quiet && stats.matches == 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}
/// Run the process-oriented command-line interface.
pub fn run() -> ExitCode {
    match real_main() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("branchcut: {error}");
            ExitCode::from(2)
        }
    }
}
