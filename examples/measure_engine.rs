//! Development-only warm-engine probe. Not part of the ordinary CLI.
use branchcut::{CaseMode, Query, parse_duration};
use std::io;
use std::io::{BufRead, Write};
use std::ops::ControlFlow;
use std::path::Path;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut query = Query::default();
    let mut args = std::env::args().skip(1);
    let mut iterations = 11_usize;
    let mut warmups = 2_usize;
    let mut limit = None;
    let mut threads = 1;
    let mut streaming = false;
    let mut interactive = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--stream" => streaming = true,
            "--interactive" => interactive = true,
            "--gitignore" => query.gitignore = true,
            "--ignore-case" => query.case_mode = CaseMode::Insensitive,
            "--cwd" => query.cwd = args.next().ok_or("missing cwd")?.into(),
            "--glob" => query.patterns.push(args.next().ok_or("missing glob")?),
            "--exclude" => query.exclusions.push(args.next().ok_or("missing exclude")?),
            "--size" => query
                .sizes
                .push(args.next().ok_or("missing size")?.parse()?),
            "--modified-within" => {
                query.modified_within =
                    Some(parse_duration(&args.next().ok_or("missing duration")?)?);
            }
            "--limit" => limit = Some(args.next().ok_or("missing limit")?.parse::<u64>()?),
            "--threads" => threads = args.next().ok_or("missing threads")?.parse::<usize>()?,
            "--iterations" => iterations = args.next().ok_or("missing iterations")?.parse()?,
            "--warmups" => warmups = args.next().ok_or("missing warmups")?.parse()?,
            _ => return Err(format!("unknown probe option: {arg}").into()),
        }
    }
    if iterations == 0 || threads == 0 || threads > 64 || limit == Some(0) {
        return Err("invalid iteration, thread, or limit value".into());
    }
    if threads > 1 && limit.is_some() {
        return Err("parallel probe does not support limits".into());
    }
    let started = Instant::now();
    let plan = query.compile()?;
    let compile_ns = started.elapsed().as_nanos();
    println!("{{\"compile_ns\":{compile_ns}}}");
    io::stdout().flush()?;
    let stdin = io::stdin();
    let mut commands = stdin.lock();
    let mut command = String::new();
    for iteration in 0..warmups + iterations {
        if interactive {
            command.clear();
            if commands.read_line(&mut command)? == 0 {
                break;
            }
            if command.trim() != "run" {
                return Err("interactive probe expects 'run'".into());
            }
        }
        let started = Instant::now();
        let mut first = None;
        let mut count = 0;
        let stats = if threads > 1 && !streaming {
            let result = plan.collect_parallel(threads, true)?;
            count = result.paths.len() as u64;
            result.report.stats
        } else {
            let mut visitor = |_: &Path| -> io::Result<ControlFlow<()>> {
                if first.is_none() {
                    first = Some(started.elapsed().as_nanos());
                }
                count += 1;
                Ok(if limit.is_some_and(|limit| count >= limit) {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                })
            };
            if threads > 1 {
                plan.visit_parallel(threads, true, &mut visitor)?.stats
            } else {
                plan.visit(&mut visitor)?.stats
            }
        };
        let elapsed_ns = started.elapsed().as_nanos();
        if stats.errors != 0 {
            return Err("probe encountered filesystem errors".into());
        }
        if iteration >= warmups {
            let first = first.map_or_else(|| "null".into(), |value| value.to_string());
            println!(
                "{{\"elapsed_ns\":{elapsed_ns},\"first_match_ns\":{first},\"matches\":{count},\"dirs_opened\":{},\"dirs_pruned\":{},\"entries_seen\":{},\"metadata_calls\":{}}}",
                stats.dirs_opened,
                stats.dirs_pruned_positive
                    + stats.dirs_pruned_exclude
                    + stats.dirs_pruned_ignore
                    + stats.dirs_pruned_depth,
                stats.entries_seen,
                stats.metadata_calls,
            );
            io::stdout().flush()?;
        }
    }
    Ok(())
}
