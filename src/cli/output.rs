use super::args::{Decoration, Options};
use super::exec::{
    CommandPool, batch_arguments, command_arguments, command_batch_base_bytes, parse_command_line,
    render_template, validate_batch_template,
};
use crate::error::{AppError, Result};
use crate::{
    CaseMode, CheckOutcome, CompiledQuery, ContentMatch, ContentQuery, PathExplanation, Stats,
    Visitor,
};
use std::borrow::Cow;
use std::collections::BinaryHeap;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::ops::ControlFlow;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_EXEC_BATCH_BYTES: usize = 28 * 1024;

pub(crate) fn write_path<W: Write>(
    writer: &mut W,
    json: bool,
    print0: bool,
    path: &Path,
) -> io::Result<()> {
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
        #[cfg(windows)]
        write_windows_path(writer, path)?;
        #[cfg(not(any(unix, windows)))]
        writer.write_all(display_path(path).as_bytes())?;
        writer.write_all(if print0 { b"\0" } else { b"\n" })
    }
}

#[cfg(windows)]
fn write_windows_path<W: Write>(writer: &mut W, path: &Path) -> io::Result<()> {
    if let Some(text) = path.to_str() {
        // Ordinary names already have a borrowed UTF-8 view. Normalize separators
        // directly into the buffered writer, preserving the existing path form.
        for (index, part) in text.split('\\').enumerate() {
            if index != 0 {
                writer.write_all(b"/")?;
            }
            writer.write_all(part.as_bytes())?;
        }
        Ok(())
    } else {
        // Preserve the documented display fallback for unpaired surrogates.
        writer.write_all(display_path(path).as_bytes())
    }
}

fn append_path_parts(
    buffer: &mut Vec<u8>,
    print0: bool,
    parent: &Path,
    name: &OsStr,
) -> io::Result<()> {
    buffer.clear();
    #[cfg(windows)]
    if parent.to_str().is_none() || name.to_str().is_none() {
        let path = parent.join(name);
        let mut rendered = Vec::new();
        write_path(&mut rendered, false, print0, &path)?;
        buffer.extend_from_slice(&rendered);
        return Ok(());
    }
    #[cfg(windows)]
    {
        let append = |buffer: &mut Vec<u8>, text: &str| {
            for (index, part) in text.split('\\').enumerate() {
                if index != 0 {
                    buffer.push(b'/');
                }
                buffer.extend_from_slice(part.as_bytes());
            }
        };
        append(buffer, parent.to_str().expect("checked above"));
        if !parent.as_os_str().is_empty() {
            buffer.push(b'/');
        }
        append(buffer, name.to_str().expect("checked above"));
    }
    #[cfg(unix)]
    {
        if !parent.as_os_str().is_empty() {
            buffer.extend_from_slice(parent.as_os_str().as_bytes());
            buffer.push(b'/');
        }
        buffer.extend_from_slice(name.as_bytes());
    }
    #[cfg(not(any(unix, windows)))]
    {
        if !parent.as_os_str().is_empty() {
            buffer.extend_from_slice(display_path(parent).as_bytes());
            buffer.push(b'/');
        }
        buffer.extend_from_slice(name.to_string_lossy().as_bytes());
    }
    buffer.push(if print0 { b'\0' } else { b'\n' });
    Ok(())
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
        if plan.case_insensitive {
            "disabled: case-insensitive matching requires searching from the cwd".to_owned()
        } else if plan.follow_symlinks || plan.same_file_system {
            "disabled: traversal policy requires validating each ancestor".to_owned()
        } else if plan.root_relative.as_os_str().is_empty() {
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
    println!("\nIGNORE POLICY");
    println!("  Git source discovery: {}", plan.gitignore);
    println!("  hierarchical .ignore: {}", plan.dot_ignore);
    for path in &plan.ignore_files {
        println!("  explicit override: {}", path.display());
    }
    println!("\nLEAF FILTERS");
    if plan.entry_types.len() == 1 {
        println!("  type: {:?}", plan.wanted_type);
    } else {
        println!("  types: {:?}", plan.entry_types);
    }
    println!("  Unicode matching: {}", plan.unicode);
    println!("  backslash escapes: {}", plan.escape_patterns);
    if !plan.and_terms.is_empty() {
        println!(
            "  additional required literal terms: {}",
            plan.and_terms.len()
        );
    }
    println!(
        "  empty: {}; executable: {}",
        plan.require_empty, plan.require_executable
    );
    if plan.owner.is_some() || plan.group.is_some() {
        println!("  owner: {:?}; group: {:?}", plan.owner, plan.group);
    }
    println!(
        "  case: {}",
        if plan.case_insensitive {
            "insensitive"
        } else {
            "sensitive"
        }
    );
    println!(
        "  simple search target: {}",
        match plan.search_target {
            crate::SearchTarget::Name => "entry name",
            crate::SearchTarget::Path => "cwd-relative path",
        }
    );
    println!(
        "  depth from cwd: {}..{}",
        plan.min_depth,
        plan.max_depth
            .map_or_else(|| "unbounded".to_owned(), |depth| depth.to_string())
    );
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
    println!("\nMETADATA");
    if !plan.needs_metadata() && !options.json_metadata {
        println!("  not required");
    } else {
        if plan.metadata.sizes.is_empty() {
            println!("  size: (any)");
        } else {
            let sizes: Vec<_> = plan
                .metadata
                .sizes
                .iter()
                .map(ToString::to_string)
                .collect();
            println!("  size: {} (all predicates)", sizes.join(" and "));
        }
        if let Some(within) = plan.metadata.modified_within {
            println!("  modified within: {within:?}");
        }
        if let Some(before) = plan.metadata.modified_before {
            println!("  modified before: {before:?}");
        }
        println!("  evaluation: after type, name, extension, and glob checks");
        println!("  relative time boundaries: resolved when each execution starts");
    }
    if !options.content.is_empty() {
        let insensitive = match options.case_mode {
            CaseMode::Insensitive => true,
            CaseMode::Sensitive => false,
            CaseMode::Smart => !options
                .content
                .iter()
                .any(|pattern| pattern.chars().any(char::is_uppercase)),
        };
        println!("\nCONTENT SEARCH");
        println!("  patterns: {}", options.content.len());
        for pattern in &options.content {
            println!("  pattern: {pattern}");
        }
        println!(
            "  case: {}",
            if insensitive {
                "insensitive"
            } else {
                "sensitive"
            }
        );
        println!(
            "  binary files: {}",
            if options.content_text {
                "read as text"
            } else {
                "skipped when a NUL byte appears in the first 8192 bytes"
            }
        );
        println!(
            "  output: path:line:column:text{}",
            if options.files_with_matches {
                " (paths only)"
            } else {
                ""
            }
        );
    }
    println!(
        "\nTERMINATION\n  {}",
        match options.limit {
            None => "all matches".to_owned(),
            Some(limit) if options.sort && !options.count => {
                format!("globally first {limit} sorted matches (requires a full scan)")
            }
            Some(limit) if options.count => format!("count at most {limit} matches"),
            Some(limit) => format!("first {limit} matches in traversal order"),
        }
    );
    println!("\nDEPTH\n  depth is measured from --cwd; literal-prefix roots do not reset it");
    println!(
        "\nTRAVERSAL POLICY\n  follow symlinks: {}\n  one filesystem: {}\n  open directory budget: {}\n  contents first: {}",
        plan.follow_symlinks, plan.same_file_system, plan.max_open, plan.contents_first
    );
    for path in &options.search_paths {
        println!("  search root: {}", path.display());
    }
    if options.exec.is_some() || options.exec_batch.is_some() {
        println!(
            "\nCOMMANDS\n  concurrent children: {}\n  preview only: {}",
            options.exec_jobs, options.exec_preview
        );
    }
    if options.threads > 1 && !plan.requires_sequential_policy() {
        println!("\nSTRATEGY\n  bounded parallel traversal with streaming output and backpressure");
    } else {
        println!(
            "\nSTRATEGY\n  sequential depth-first traversal with positive and exclusion pruning"
        );
    }
}

pub(crate) fn print_why(explanation: &PathExplanation) {
    println!("WHY\n");
    println!("PATH\n  {}", display_path(&explanation.path));
    println!(
        "\nRESULT\n  {}",
        if explanation.selected {
            "MATCHED"
        } else {
            "EXCLUDED"
        }
    );
    println!("\nCHECKS");
    for check in &explanation.checks {
        let status = match check.outcome {
            CheckOutcome::Passed => "PASS",
            CheckOutcome::Failed => "FAIL",
            CheckOutcome::NotApplicable => "SKIP",
        };
        println!("  [{status}] {}", check.label);
        if let Some(detail) = &check.detail {
            println!("         {detail}");
        }
    }
}

pub(crate) fn print_stats(stats: &Stats, elapsed: Duration) {
    eprintln!("matched                 {}", stats.matches);
    eprintln!("directories considered  {}", stats.dirs_seen);
    eprintln!("directories opened      {}", stats.dirs_opened);
    eprintln!(
        "directories pruned      {}",
        stats.dirs_pruned_positive
            + stats.dirs_pruned_exclude
            + stats.dirs_pruned_ignore
            + stats.dirs_pruned_depth
            + stats.dirs_pruned_policy
    );
    eprintln!("  positive              {}", stats.dirs_pruned_positive);
    eprintln!("  excluded              {}", stats.dirs_pruned_exclude);
    eprintln!("  ignored               {}", stats.dirs_pruned_ignore);
    eprintln!("  depth                 {}", stats.dirs_pruned_depth);
    eprintln!("  traversal policy      {}", stats.dirs_pruned_policy);
    eprintln!("entries inspected       {}", stats.entries_seen);
    eprintln!("candidate files         {}", stats.candidate_files);
    eprintln!("metadata calls          {}", stats.metadata_calls);
    if stats.content_files > 0 || stats.content_binary_skipped > 0 {
        eprintln!("content files           {}", stats.content_files);
        eprintln!("content bytes           {}", stats.content_bytes);
        eprintln!("binary files skipped    {}", stats.content_binary_skipped);
    }
    eprintln!("filesystem errors       {}", stats.errors);
    eprintln!(
        "elapsed                 {:.3}ms",
        elapsed.as_secs_f64() * 1000.0
    );
}

pub(crate) struct Output<W: Write> {
    pub(crate) writer: W,
    pub(crate) output: Vec<PathBuf>,
    selection: Option<BinaryHeap<PathBuf>>,
    pub(crate) matched: u64,
    pub(crate) stopped: bool,
    pub(crate) metadata_calls: u64,
    pub(crate) content_files: u64,
    pub(crate) content_bytes: u64,
    pub(crate) content_binary_skipped: u64,
    pub(crate) content_errors: u64,
    content: Option<ContentQuery>,
    options: Options,
    command: Option<Vec<String>>,
    batch_command: Option<Vec<String>>,
    absolute_root: Option<PathBuf>,
    batch: Vec<PathBuf>,
    batch_bytes: usize,
    batch_base_bytes: usize,
    line_buffer: Vec<u8>,
    command_pool: CommandPool,
    color: bool,
    hyperlink: bool,
}
impl<W: Write> Output<W> {
    pub(crate) fn new(options: &Options, writer: W) -> Result<Self> {
        let command = options
            .exec
            .as_deref()
            .map(parse_command_line)
            .transpose()?;
        let batch_command = options
            .exec_batch
            .as_deref()
            .map(parse_command_line)
            .transpose()?;
        if let Some(command) = &batch_command {
            validate_batch_template(command)?;
        }
        let batch_base_bytes = batch_command.as_deref().map_or(0, command_batch_base_bytes);
        if batch_base_bytes > MAX_EXEC_BATCH_BYTES {
            return Err(AppError(format!(
                "--exec-batch template exceeds the {MAX_EXEC_BATCH_BYTES}-byte command budget"
            )));
        }
        let absolute_root = if options.absolute {
            Some(fs::canonicalize(&options.cwd).map_err(|error| {
                AppError(format!(
                    "cannot resolve --cwd {} for absolute output: {error}",
                    options.cwd.display()
                ))
            })?)
        } else {
            None
        };
        let terminal = io::stdout().is_terminal()
            && std::env::var_os("TERM").is_none_or(|term| term != "dumb");
        let plain = !options.json
            && !options.print0
            && !options.count
            && !options.quiet
            && !options.exec_preview;
        let decorate =
            |mode| plain && (mode == Decoration::Always || mode == Decoration::Auto && terminal);
        let content = options.content_query()?;
        let content_mode = content.is_some();
        Ok(Self {
            writer,
            output: Vec::new(),
            selection: (options.sort && !options.count && options.limit.is_some() && !content_mode)
                .then(BinaryHeap::new),
            matched: 0,
            stopped: false,
            metadata_calls: 0,
            content_files: 0,
            content_bytes: 0,
            content_binary_skipped: 0,
            content_errors: 0,
            content,
            options: options.clone(),
            command,
            batch_command,
            absolute_root,
            batch: Vec::new(),
            batch_bytes: 0,
            batch_base_bytes,
            line_buffer: Vec::new(),
            command_pool: CommandPool::new(options.exec_jobs),
            color: !content_mode
                && decorate(options.color)
                && (options.color == Decoration::Always || std::env::var_os("NO_COLOR").is_none()),
            hyperlink: !content_mode && decorate(options.hyperlink),
        })
    }

    fn output_path<'a>(&self, path: &'a Path) -> Cow<'a, Path> {
        self.absolute_root
            .as_ref()
            .map_or_else(|| Cow::Borrowed(path), |root| Cow::Owned(root.join(path)))
    }

    fn queue_batch(&mut self, path: PathBuf) -> io::Result<()> {
        let path_bytes = native_argument_cost(&path);
        if !self.batch.is_empty()
            && (self.batch.len() >= self.options.batch_size
                || self
                    .batch_base_bytes
                    .saturating_add(self.batch_bytes)
                    .saturating_add(path_bytes)
                    > MAX_EXEC_BATCH_BYTES)
        {
            self.flush_batch()?;
        }
        if self.batch_base_bytes.saturating_add(path_bytes) > MAX_EXEC_BATCH_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "path exceeds the {MAX_EXEC_BATCH_BYTES}-byte --exec-batch command budget: {}",
                    path.display()
                ),
            ));
        }
        self.batch_bytes = self.batch_bytes.saturating_add(path_bytes);
        self.batch.push(path);
        if self.batch.len() >= self.options.batch_size {
            self.flush_batch()?;
        }
        Ok(())
    }

    fn flush_batch(&mut self) -> io::Result<()> {
        let Some(command) = self.batch_command.as_deref() else {
            return Ok(());
        };
        if self.batch.is_empty() {
            return Ok(());
        }
        let arguments = batch_arguments(command, &self.batch);
        self.submit_command(&arguments)?;
        self.batch.clear();
        self.batch_bytes = 0;
        Ok(())
    }

    fn submit_command(&mut self, arguments: &[std::ffi::OsString]) -> io::Result<()> {
        if self.options.exec_preview {
            write!(
                self.writer,
                "{{\"cwd\":\"{}\",\"argv\":[",
                json_escape(&display_path(&self.options.cwd))
            )?;
            for (index, argument) in arguments.iter().enumerate() {
                if index > 0 {
                    write!(self.writer, ",")?;
                }
                write!(
                    self.writer,
                    "\"{}\"",
                    json_escape(&argument.to_string_lossy())
                )?;
            }
            write!(self.writer, "]")?;
            if arguments.iter().any(|argument| argument.to_str().is_none()) {
                write!(self.writer, ",\"argv_native\":[")?;
                for (index, argument) in arguments.iter().enumerate() {
                    if index > 0 {
                        write!(self.writer, ",")?;
                    }
                    write_native_argument(&mut self.writer, argument)?;
                }
                write!(self.writer, "]")?;
            }
            if self.options.cwd.to_str().is_none() {
                write!(self.writer, ",\"cwd_native\":")?;
                write_native_argument(&mut self.writer, self.options.cwd.as_os_str())?;
            }
            writeln!(self.writer, "}}")
        } else {
            self.command_pool.submit(arguments, &self.options.cwd)
        }
    }

    fn selected_path(&mut self, path: &Path) -> io::Result<()> {
        if let Some(command) = &self.command {
            let arguments = command_arguments(command, path);
            self.submit_command(&arguments)?;
        }
        if self.batch_command.is_some() {
            self.queue_batch(path.to_owned())?;
        }
        if self.options.count || self.options.quiet || self.options.exec_preview {
            return Ok(());
        }
        if self.options.json_metadata {
            let absolute = if path.is_absolute() {
                path.to_owned()
            } else {
                self.options.cwd.join(path)
            };
            self.metadata_calls += 1;
            let metadata = fs::symlink_metadata(&absolute)?;
            let mut record = Vec::new();
            write_path(&mut record, true, false, path)?;
            record.truncate(record.len() - 2);
            self.writer.write_all(&record)?;
            let kind = if metadata.is_file() {
                "file"
            } else if metadata.is_dir() {
                "dir"
            } else if metadata.file_type().is_symlink() {
                "symlink"
            } else {
                "other"
            };
            write!(
                self.writer,
                ",\"type\":\"{kind}\",\"size\":{},\"readonly\":{}",
                metadata.len(),
                metadata.permissions().readonly()
            )?;
            if let Ok(modified) = metadata.modified() {
                let milliseconds = match modified.duration_since(std::time::UNIX_EPOCH) {
                    Ok(age) => age.as_millis() as i128,
                    Err(error) => -(error.duration().as_millis() as i128),
                };
                write!(self.writer, ",\"modified_unix_ms\":{milliseconds}")?;
            }
            return writeln!(self.writer, "}}");
        }
        if self.hyperlink {
            let absolute = std::path::absolute(if path.is_absolute() {
                path.to_owned()
            } else {
                self.options.cwd.join(path)
            })?;
            write!(self.writer, "\x1b]8;;{}\x1b\\", file_url(&absolute))?;
        }
        if self.color {
            self.writer.write_all(b"\x1b[32m")?;
        }
        if let Some(template) = &self.options.format {
            let rendered = render_template(template, path);
            #[cfg(unix)]
            self.writer.write_all(rendered.as_bytes())?;
            #[cfg(not(unix))]
            self.writer
                .write_all(rendered.to_string_lossy().as_bytes())?;
        } else if self.color || self.hyperlink {
            let mut line = Vec::new();
            write_path(&mut line, false, false, path)?;
            line.pop();
            self.writer.write_all(&line)?;
        } else {
            return write_path(
                &mut self.writer,
                self.options.json,
                self.options.print0,
                path,
            );
        }
        if self.color {
            self.writer.write_all(b"\x1b[0m")?;
        }
        if self.hyperlink {
            self.writer.write_all(b"\x1b]8;;\x1b\\")?;
        }
        self.writer.write_all(b"\n")
    }

    pub(crate) fn finish(&mut self) -> io::Result<()> {
        if self.options.sort && !self.options.count {
            if self.content.is_some() {
                self.output.sort_unstable();
                let paths = std::mem::take(&mut self.output);
                for path in &paths {
                    if self.scan_content(path)?.is_break() {
                        break;
                    }
                }
                self.output = paths;
            } else {
                if let Some(selection) = self.selection.take() {
                    self.output = selection.into_sorted_vec();
                } else {
                    self.output.sort_unstable();
                }
                self.matched = self.output.len() as u64;
                let paths = std::mem::take(&mut self.output);
                for path in &paths {
                    let path = self.output_path(path);
                    self.selected_path(&path)?;
                }
                self.output = paths;
            }
        }
        self.flush_batch()?;
        self.command_pool.finish()?;
        if self.options.count {
            writeln!(self.writer, "{}", self.matched)?;
        }
        self.writer.flush()
    }

    /// Read one selected file and emit its matching lines. Non-file entries are
    /// skipped: content search applies to regular files only.
    fn scan_content(&mut self, path: &Path) -> io::Result<ControlFlow<()>> {
        let Some(query) = self.content.clone() else {
            return Ok(ControlFlow::Continue(()));
        };
        let absolute = if path.is_absolute() {
            path.to_owned()
        } else {
            self.options.cwd.join(path)
        };
        let metadata = match fs::metadata(&absolute) {
            Ok(metadata) => metadata,
            Err(error) => {
                self.content_errors += 1;
                self.error(path, &error)?;
                return Ok(ControlFlow::Continue(()));
            }
        };
        self.metadata_calls += 1;
        if !metadata.is_file() {
            return Ok(ControlFlow::Continue(()));
        }
        let limit = self.options.limit.map(|limit| limit as u64);
        let remaining = limit.map(|limit| limit.saturating_sub(self.matched));
        if remaining == Some(0) {
            self.stopped = true;
            return Ok(ControlFlow::Break(()));
        }
        let files_only = self.options.files_with_matches;
        let quiet = self.options.quiet;
        let count = self.options.count;
        let scan_limit = if files_only || quiet {
            Some(1)
        } else {
            remaining
        };
        let display = self.output_path(path).into_owned();
        let mut emitted = 0u64;
        let mut write_error = None;
        let scanned = query.scan_file(&absolute, scan_limit, |found| {
            if files_only || quiet {
                emitted = 1;
                return Ok(ControlFlow::Break(()));
            }
            if !count && let Err(error) = self.write_content_match(&display, &metadata, found) {
                write_error = Some(error);
                return Ok(ControlFlow::Break(()));
            }
            self.matched += 1;
            emitted += 1;
            if limit.is_some_and(|limit| self.matched >= limit) {
                return Ok(ControlFlow::Break(()));
            }
            Ok(ControlFlow::Continue(()))
        });
        if let Some(error) = write_error {
            return Err(error);
        }
        let stats = match scanned {
            Ok(stats) => stats,
            Err(error) => {
                self.content_errors += 1;
                self.error(path, &error)?;
                return Ok(ControlFlow::Continue(()));
            }
        };
        self.content_files += 1;
        self.content_bytes = self.content_bytes.saturating_add(stats.bytes_read);
        if stats.binary_skipped {
            self.content_binary_skipped += 1;
        }
        if files_only {
            if emitted == 0 {
                return Ok(ControlFlow::Continue(()));
            }
            self.matched += 1;
            self.selected_path(&display)?;
            return Ok(self.finish_match());
        }
        if quiet {
            if emitted == 0 {
                return Ok(ControlFlow::Continue(()));
            }
            self.matched += 1;
        }
        Ok(self.finish_match())
    }

    fn write_content_match(
        &mut self,
        display: &Path,
        metadata: &fs::Metadata,
        found: ContentMatch<'_>,
    ) -> io::Result<()> {
        let mut record = Vec::with_capacity(found.line.len() + 64);
        if self.options.json {
            write_path(&mut record, true, false, display)?;
            // Drop the closing brace and newline written by write_path.
            record.truncate(record.len() - 2);
            write!(
                &mut record,
                ",\"line\":{},\"column\":{}",
                found.line_number, found.column
            )?;
            record.extend_from_slice(b",\"text\":\"");
            record.extend_from_slice(json_escape(&String::from_utf8_lossy(found.line)).as_bytes());
            record.push(b'"');
            if self.options.json_metadata {
                write!(
                    &mut record,
                    ",\"type\":\"file\",\"size\":{},\"readonly\":{}",
                    metadata.len(),
                    metadata.permissions().readonly()
                )?;
                if let Ok(modified) = metadata.modified() {
                    let milliseconds = match modified.duration_since(std::time::UNIX_EPOCH) {
                        Ok(age) => age.as_millis() as i128,
                        Err(error) => -(error.duration().as_millis() as i128),
                    };
                    write!(&mut record, ",\"modified_unix_ms\":{milliseconds}")?;
                }
            }
            record.extend_from_slice(b"}\n");
        } else {
            write_path(&mut record, false, false, display)?;
            // Replace the trailing newline with the result separator.
            record.pop();
            write!(&mut record, ":{}:{}:", found.line_number, found.column)?;
            record.extend_from_slice(found.line);
            record.push(b'\n');
        }
        self.writer.write_all(&record)
    }
}
impl<W: Write> Visitor for Output<W> {
    fn requires_path(&self) -> bool {
        self.content.is_some()
            || (!self.options.quiet
                && (!self.options.count || self.command.is_some() || self.batch_command.is_some()))
    }

    fn matched(&mut self, path: &Path) -> io::Result<ControlFlow<()>> {
        if self.content.is_some() {
            if self.options.sort && !self.options.count {
                self.output.push(path.to_owned());
                return Ok(ControlFlow::Continue(()));
            }
            return self.scan_content(path);
        }
        self.matched += 1;
        if self.options.sort && !self.options.count {
            if let Some(selection) = &mut self.selection {
                // Keep the greatest retained path at the root. Later smaller
                // paths replace it, so discovery order cannot change the top N.
                if selection.len() < self.options.limit.expect("bounded selection has a limit") {
                    selection.push(path.to_owned());
                } else if let Some(mut greatest) = selection.peek_mut()
                    && path < greatest.as_path()
                {
                    *greatest = path.to_owned();
                }
            } else {
                self.output.push(path.to_owned());
            }
            return Ok(ControlFlow::Continue(()));
        }
        if !self.options.quiet
            && (!self.options.count || self.command.is_some() || self.batch_command.is_some())
        {
            let path = self.output_path(path);
            self.selected_path(&path)?;
        }
        Ok(self.finish_match())
    }

    fn matched_child(&mut self, parent: &Path, name: &OsStr) -> io::Result<ControlFlow<()>> {
        if self.content.is_some() {
            return self.matched(&parent.join(name));
        }
        if self.options.sort
            || self.options.json
            || self.absolute_root.is_some()
            || self.command.is_some()
            || self.batch_command.is_some()
            || self.options.format.is_some()
            || self.color
            || self.hyperlink
            || self.options.quiet
        {
            return self.matched(&parent.join(name));
        }
        if self.options.count {
            return self.matched_without_path();
        }
        self.matched += 1;
        append_path_parts(&mut self.line_buffer, self.options.print0, parent, name)?;
        self.writer.write_all(&self.line_buffer)?;
        Ok(self.finish_match())
    }

    fn matched_without_path(&mut self) -> io::Result<ControlFlow<()>> {
        self.matched += 1;
        Ok(self.finish_match())
    }

    fn error(&mut self, path: &Path, error: &io::Error) -> io::Result<()> {
        eprintln!("branchcut: cannot read {}: {error}", path.display());
        Ok(())
    }
}

impl<W: Write> Output<W> {
    fn finish_match(&mut self) -> ControlFlow<()> {
        if self
            .options
            .limit
            .is_some_and(|limit| self.matched >= limit as u64)
        {
            self.stopped = true;
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

fn native_argument_cost(path: &Path) -> usize {
    #[cfg(unix)]
    {
        path.as_os_str().as_bytes().len() + 1
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        path.as_os_str().encode_wide().count() * 2 + 2
    }
    #[cfg(not(any(unix, windows)))]
    {
        path.to_string_lossy().len() + 1
    }
}

fn file_url(path: &Path) -> String {
    #[cfg(unix)]
    let bytes = path.as_os_str().as_bytes();
    #[cfg(not(unix))]
    let display = display_path(path);
    #[cfg(not(unix))]
    let bytes = display.as_bytes();
    #[cfg(windows)]
    let (mut url, bytes) = if let Some(unc) = bytes.strip_prefix(b"//") {
        ("file://".to_owned(), unc)
    } else {
        ("file:///".to_owned(), bytes)
    };
    #[cfg(not(windows))]
    let mut url = "file://".to_owned();
    use std::fmt::Write as _;
    for &byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b':' | b'-' | b'_' | b'.' | b'~') {
            url.push(byte as char);
        } else {
            write!(url, "%{byte:02X}").expect("String writes cannot fail");
        }
    }
    url
}

fn write_native_argument(writer: &mut impl Write, argument: &OsStr) -> io::Result<()> {
    #[cfg(unix)]
    {
        write!(writer, "{{\"bytes\":\"")?;
        for byte in argument.as_bytes() {
            write!(writer, "{byte:02x}")?;
        }
        write!(writer, "\"}}")
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        write!(writer, "{{\"utf16\":[")?;
        for (index, unit) in argument.encode_wide().enumerate() {
            if index > 0 {
                write!(writer, ",")?;
            }
            write!(writer, "{unit}")?;
        }
        write!(writer, "]}}")
    }
    #[cfg(not(any(unix, windows)))]
    write!(writer, "\"{}\"", json_escape(&argument.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn file_urls_preserve_unc_hosts_and_percent_encode_controls() {
        assert_eq!(
            file_url(Path::new(r"\\server\share\space name.rs")),
            "file://server/share/space%20name.rs"
        );
        assert_eq!(
            file_url(Path::new(r"C:\space name.rs")),
            "file:///C:/space%20name.rs"
        );
    }

    #[test]
    fn borrowed_path_parts_render_the_same_line_and_nul_forms() {
        let mut buffer = Vec::new();
        append_path_parts(
            &mut buffer,
            false,
            Path::new("packages/core"),
            OsStr::new("main.rs"),
        )
        .unwrap();
        assert_eq!(buffer, b"packages/core/main.rs\n");

        append_path_parts(&mut buffer, true, Path::new(""), OsStr::new("item")).unwrap();
        assert_eq!(buffer, b"item\0");
    }

    #[test]
    fn sorted_limits_equal_full_native_sort_and_bound_retained_paths() {
        let paths: Vec<PathBuf> = (0..503)
            .map(|index| PathBuf::from(format!("dir{}/file{:04}.rs", index % 7, index)))
            .collect();
        let mut expected = paths.clone();
        expected.sort_unstable();
        for limit in [1, 2, 10, 251, 503, 504, usize::MAX] {
            for stride in [1, 13, 502] {
                let options = Options {
                    sort: true,
                    limit: Some(limit),
                    ..Options::default()
                };
                let mut sink = Output::new(&options, Vec::new()).unwrap();
                for index in 0..paths.len() {
                    assert!(
                        sink.matched(&paths[index * stride % paths.len()])
                            .unwrap()
                            .is_continue()
                    );
                    assert!(sink.output.is_empty());
                    assert!(sink.selection.as_ref().unwrap().len() <= limit);
                }
                sink.finish().unwrap();
                assert_eq!(sink.output, expected[..limit.min(paths.len())]);
                assert_eq!(sink.matched as usize, limit.min(paths.len()));
            }
        }
    }
}
