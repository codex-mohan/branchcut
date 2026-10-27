//! Terminal interface. Filesystem selection lives in the independent core modules.
pub(crate) mod args;
pub(crate) mod exec;
pub(crate) mod output;
use crate::error::{AppError, Result};
use crate::{CompiledQuery, Stats, Visitor};
use args::{Options, parse_args};
use output::{Output, print_explain, print_stats};
use std::io::{self, Write};
use std::process::ExitCode;
use std::time::Instant;

pub(crate) fn execute<W: Write>(
    plan: &CompiledQuery,
    options: &Options,
    sink: &mut Output<W>,
) -> io::Result<Stats> {
    let mut stats = if options.threads > 1
        && std::fs::symlink_metadata(&plan.root).is_ok_and(|metadata| metadata.is_dir())
    {
        let collected = plan.collect_parallel(options.threads.min(64), options.strict)?;
        for error in &collected.errors {
            sink.error(&error.path, &error.error)?;
        }
        for path in &collected.paths {
            if sink.matched(path)?.is_break() {
                break;
            }
        }
        collected.report.stats
    } else {
        plan.visit(sink)?.stats
    };
    sink.finish()?;
    stats.matches = sink.matched;
    Ok(stats)
}
fn real_main() -> Result<()> {
    let options = parse_args()?;
    let plan = options.query().compile()?;
    let stdout = io::stdout();
    let mut sink = Output::new(
        &options,
        io::BufWriter::with_capacity(64 * 1024, stdout.lock()),
    )?;
    if options.threads > 1 && (options.limit.is_some() || options.exec.is_some()) {
        return Err(AppError("--threads cannot be combined with --limit or --exec; use sequential mode for exact ordering".to_owned()));
    }
    if options.explain {
        print_explain(&plan, &options);
        return Ok(());
    }
    let started = Instant::now();
    let stats = match execute(&plan, &options, &mut sink) {
        Ok(stats) => stats,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
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
    Ok(())
}
/// Run the process-oriented command-line interface.
pub fn run() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("branchcut: {error}");
            ExitCode::from(2)
        }
    }
}
