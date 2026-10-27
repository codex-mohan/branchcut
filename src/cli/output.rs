use super::args::Options;
use super::exec::{parse_command_line, run_command};
use crate::error::Result;
use crate::{CompiledQuery, Stats, Visitor};
use std::io::{self, Write};
use std::ops::ControlFlow;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
pub(crate) fn write_path<W: Write>(writer: &mut W, json: bool, path: &Path) -> io::Result<()> {
    if json {
        write!(
            writer,
            "{{\"path\":\"{}\"",
            json_escape(&display_path(path))
        )?;
        if path.to_str().is_none() {
            #[cfg(unix)]
            {
                write!(writer, ",\"path_bytes\":\"")?;
                for byte in path.as_os_str().as_bytes() {
                    write!(writer, "{byte:02x}")?;
                }
                write!(writer, "\"")?;
            }
            #[cfg(windows)]
            {
                use std::os::windows::ffi::OsStrExt;
                write!(writer, ",\"path_utf16\":[")?;
                for (index, unit) in path.as_os_str().encode_wide().enumerate() {
                    if index != 0 {
                        write!(writer, ",")?;
                    }
                    write!(writer, "{unit}")?;
                }
                write!(writer, "]")?;
            }
        }
        writeln!(writer, "}}")
    } else {
        #[cfg(unix)]
        writer.write_all(path.as_os_str().as_bytes())?;
        #[cfg(not(unix))]
        writer.write_all(display_path(path).as_bytes())?;
        writer.write_all(b"\n")
    }
}

pub(crate) fn display_path(path: &Path) -> String {
    #[cfg(windows)]
    {
        path.to_string_lossy().replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        path.to_string_lossy().into_owned()
    }
}

pub(crate) fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                use std::fmt::Write as _;
                write!(escaped, "\\u{:04x}", character as u32).expect("String write cannot fail");
            }
            character => escaped.push(character),
        }
    }
    escaped
}
pub(crate) fn print_explain(plan: &CompiledQuery, options: &Options) {
    println!("QUERY PLAN\n");
    println!("ROOT\n  {}", plan.root.display());
    println!(
        "\nSHARED LITERAL PREFIX\n  {}",
        if plan.root_relative.as_os_str().is_empty() {
            "(none)".to_owned()
        } else {
            display_path(&plan.root_relative)
        }
    );
    println!("\nPOSITIVE PATTERNS");
    for pattern in &plan.positives {
        println!("  {} [{:?}]", pattern.source, pattern.class);
    }
    println!("\nEXCLUSIONS");
    if plan.negatives.is_empty() {
        println!("  (none)");
    } else {
        for pattern in &plan.negatives {
            println!("  {} [{:?}]", pattern.source, pattern.class);
        }
    }
    println!("\nLEAF FILTERS");
    println!("  type: {:?}", plan.wanted_type);
    if plan.extensions.is_empty() {
        println!("  extensions: (any)");
    } else {
        let extensions: Vec<_> = plan
            .extensions
            .iter()
            .map(|ext| String::from_utf8_lossy(ext))
            .collect();
        println!("  extensions: {}", extensions.join(", "));
    }
    println!("\nMETADATA\n  not required");
    println!(
        "\nTERMINATION\n  {}",
        options
            .limit
            .map_or("all matches".to_owned(), |limit| format!(
                "first {limit} matches"
            ))
    );
    if options.threads > 1 {
        println!("\nSTRATEGY\n  bounded parallel root-task traversal with buffered output");
    } else {
        println!(
            "\nSTRATEGY\n  sequential depth-first traversal with positive and exclusion pruning"
        );
    }
}

pub(crate) fn print_stats(stats: &Stats, elapsed: Duration) {
    eprintln!("matched                 {}", stats.matches);
    eprintln!("directories considered  {}", stats.dirs_seen);
    eprintln!("directories opened      {}", stats.dirs_opened);
    eprintln!(
        "directories pruned      {}",
        stats.dirs_pruned_positive + stats.dirs_pruned_exclude + stats.dirs_pruned_ignore
    );
    eprintln!("  positive              {}", stats.dirs_pruned_positive);
    eprintln!("  excluded              {}", stats.dirs_pruned_exclude);
    eprintln!("  ignored               {}", stats.dirs_pruned_ignore);
    eprintln!("entries inspected       {}", stats.entries_seen);
    eprintln!("candidate files         {}", stats.candidate_files);
    eprintln!("metadata calls          {}", stats.metadata_calls);
    eprintln!("filesystem errors       {}", stats.errors);
    eprintln!(
        "elapsed                 {:.3}ms",
        elapsed.as_secs_f64() * 1000.0
    );
}

pub(crate) struct Output<W: Write> {
    pub(crate) writer: W,
    pub(crate) output: Vec<PathBuf>,
    pub(crate) matched: u64,
    pub(crate) stopped: bool,
    options: Options,
    command: Option<Vec<String>>,
}
impl<W: Write> Output<W> {
    pub(crate) fn new(options: &Options, writer: W) -> Result<Self> {
        Ok(Self {
            writer,
            output: Vec::new(),
            matched: 0,
            stopped: false,
            options: options.clone(),
            command: options
                .exec
                .as_deref()
                .map(parse_command_line)
                .transpose()?,
        })
    }
    pub(crate) fn finish(&mut self) -> io::Result<()> {
        if self.options.sort && !self.options.count {
            self.output.sort_unstable();
            if let Some(limit) = self.options.limit {
                self.output.truncate(limit);
            }
            self.matched = self.output.len() as u64;
            for path in &self.output {
                if let Some(command) = &self.command {
                    run_command(command, path, &self.options.cwd)?;
                }
                write_path(&mut self.writer, self.options.json, path)?;
            }
        }
        if self.options.count {
            writeln!(self.writer, "{}", self.matched)?;
        }
        self.writer.flush()
    }
}
impl<W: Write> Visitor for Output<W> {
    fn matched(&mut self, path: &Path) -> io::Result<ControlFlow<()>> {
        self.matched += 1;
        if self.options.sort && !self.options.count {
            self.output.push(path.to_owned());
            return Ok(ControlFlow::Continue(()));
        }
        if let Some(command) = &self.command {
            run_command(command, path, &self.options.cwd)?;
        }
        if !self.options.count {
            write_path(&mut self.writer, self.options.json, path)?;
        }
        if self
            .options
            .limit
            .is_some_and(|limit| self.matched >= limit as u64)
        {
            self.stopped = true;
            Ok(ControlFlow::Break(()))
        } else {
            Ok(ControlFlow::Continue(()))
        }
    }
    fn error(&mut self, path: &Path, error: &io::Error) -> io::Result<()> {
        eprintln!("branchcut: cannot read {}: {error}", path.display());
        Ok(())
    }
}
