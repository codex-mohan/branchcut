//! Terminal interface. Filesystem selection lives in the independent core modules.
pub(crate) mod args;
mod completions;
mod config;
mod content_format;
mod content_inputs;
mod content_workers;
pub(crate) mod exec;
pub(crate) mod output;
use crate::error::{AppError, Result};
use crate::{CompiledQuery, ContentMatchOptions, ContentMatcher, Stats, Visitor};
use args::{Options, SearchCommand, parse_args};
use output::{Output, print_explain, print_stats, print_why};
use std::io::{self, Write};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

pub(crate) fn execute<W: Write>(
    plan: &CompiledQuery,
    options: &Options,
    sink: &mut Output<W>,
) -> io::Result<Stats> {
    sink.set_content_plan(plan);
    if options.command != SearchCommand::Legacy {
        for path in &options.search_paths {
            let input = options.cwd.join(path);
            if let Err(error) = std::fs::symlink_metadata(&input)
                && error.kind() == io::ErrorKind::NotFound
            {
                sink.error(path, &error)?;
                sink.content_errors += 1;
            }
        }
    }
    if options.content_options.stdin {
        sink.scan_stdin()?;
    }
    let mut stats = if sink.stopped {
        Stats::default()
    } else if !options.search_paths.is_empty() {
        let roots = crate::MultiRootQuery::rebase(plan, &options.search_paths)
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
    } else if options.content_options.stdin || !options.content_options.paths_from.is_empty() {
        Stats::default()
    } else if options.threads > 1 {
        plan.visit_parallel(options.threads.min(64), options.strict, sink)?
            .stats
    } else {
        plan.visit(sink)?.stats
    };
    sink.finish()?;
    stats.matches = sink.matched;
    stats.metadata_calls += sink.metadata_calls;
    stats.errors += sink.content_errors;
    stats.content_files += sink.content_files;
    stats.content_bytes += sink.content_bytes;
    stats.content_binary_skipped += sink.content_binary_skipped;
    sink.finish_content_output(&stats)?;
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
/// Factory used by separately built matching adapters. Pattern IDs are assigned
/// by the order in this slice and must be preserved in returned spans.
pub type ContentMatcherFactory =
    fn(&[Vec<u8>], ContentMatchOptions) -> io::Result<Arc<dyn ContentMatcher>>;

fn real_main(factory: Option<ContentMatcherFactory>) -> Result<ExitCode> {
    let mut options = parse_args()?;
    options.content_options.factory = factory;
    let mut consumed_stdin = options.content_options.stdin;
    for (path, nul) in &options.content_options.paths_from {
        if path == Path::new("-") {
            if consumed_stdin {
                return Err(AppError("standard input can be used only once".into()));
            }
            consumed_stdin = true;
        }
        let inputs = content_inputs::path_list(path, *nul).map_err(|error| {
            AppError(format!("cannot read path list {}: {error}", path.display()))
        })?;
        options.search_paths.extend(inputs);
    }
    let mut plan = options.query().compile()?;
    if let Some(pattern) = &options.path_regex {
        let factory = factory.ok_or_else(|| AppError("regex search requires branchcut-regex; install it from the Branchcut release archive".into()))?;
        let case_insensitive = match options.case_mode {
            crate::CaseMode::Sensitive => false,
            crate::CaseMode::Insensitive => true,
            crate::CaseMode::Smart => !pattern.chars().any(|c| {
                if options.unicode {
                    c.is_uppercase()
                } else {
                    c.is_ascii_uppercase()
                }
            }),
        };
        let matcher = factory(
            &[pattern.as_bytes().to_vec()],
            ContentMatchOptions {
                case_insensitive,
                unicode_case: options.unicode,
                ..ContentMatchOptions::default()
            },
        )
        .map_err(|error| AppError(error.to_string()))?;
        plan = plan.with_path_matcher(matcher);
    }
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
        options.content_query()?;
        if options.search_paths.is_empty() {
            print_explain(&plan, &options);
        } else {
            for root in &options.search_paths {
                print_explain(&plan.for_input(options.cwd.join(root))?, &options);
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
    if (options.strict || options.command != SearchCommand::Legacy) && stats.errors > 0 {
        return Err(AppError(format!(
            "query incomplete: {} filesystem error(s)",
            stats.errors
        )));
    }
    if options.stats {
        print_stats(&stats, started.elapsed());
    }
    Ok(
        if (options.quiet || options.command == SearchCommand::Grep) && stats.matches == 0 {
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        },
    )
}
/// Run the process-oriented command-line interface.
pub fn run() -> ExitCode {
    run_inner(None)
}

/// Run the CLI with an optional regex implementation. The ordinary binary
/// uses [run] and keeps the core dependency graph empty.
pub fn run_with_content_matcher(factory: ContentMatcherFactory) -> ExitCode {
    run_inner(Some(factory))
}

fn run_inner(factory: Option<ContentMatcherFactory>) -> ExitCode {
    match real_main(factory) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("branchcut: {error}");
            ExitCode::from(2)
        }
    }
}
