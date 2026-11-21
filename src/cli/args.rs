use super::ContentMatcherFactory;
use super::config;
use super::content_format::{ContentFormat, validate_template};
use crate::error::{AppError, Result};
use crate::{
    CaseMode, ContentBoolean, ContentMatchOptions, ContentQuery, ContentScanOptions, EntryType,
    ModifiedBefore, Query, SearchTarget, SizeFilter, parse_duration,
};
use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::thread;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Decoration {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ContentOptions {
    pub(crate) scan: ContentScanOptions,
    pub(crate) matching: ContentMatchOptions,
    pub(crate) format: ContentFormat,
    pub(crate) regex: bool,
    pub(crate) factory: Option<ContentMatcherFactory>,
    pub(crate) only_matching: bool,
    pub(crate) files_without_matches: bool,
    pub(crate) per_file_count: bool,
    pub(crate) explain_matches: bool,
    pub(crate) summary: bool,
    pub(crate) stdin: bool,
    pub(crate) paths_from: Vec<(PathBuf, bool)>,
    pub(crate) preprocess: Option<OsString>,
    pub(crate) decompress: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Options {
    pub(crate) command: SearchCommand,
    pub(crate) path_regex: Option<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) search_paths: Vec<PathBuf>,
    pub(crate) positive: Vec<String>,
    pub(crate) negative: Vec<String>,
    pub(crate) simple_terms: Vec<Vec<u8>>,
    pub(crate) extensions: Vec<Vec<u8>>,
    pub(crate) wanted_type: EntryType,
    pub(crate) entry_types: Vec<EntryType>,
    pub(crate) require_empty: bool,
    pub(crate) require_executable: bool,
    pub(crate) owner: Option<u32>,
    pub(crate) group: Option<u32>,
    pub(crate) and_terms: Vec<String>,
    pub(crate) content: Vec<String>,
    pub(crate) content_options: ContentOptions,
    pub(crate) content_text: bool,
    pub(crate) files_with_matches: bool,
    pub(crate) unicode: bool,
    pub(crate) escape_patterns: bool,
    pub(crate) follow_symlinks: bool,
    pub(crate) same_file_system: bool,
    pub(crate) max_open: usize,
    pub(crate) contents_first: bool,
    pub(crate) hidden: bool,
    pub(crate) limit: Option<usize>,
    pub(crate) sort: bool,
    pub(crate) stats: bool,
    pub(crate) explain: bool,
    pub(crate) why: Option<PathBuf>,
    pub(crate) strict: bool,
    pub(crate) count: bool,
    pub(crate) quiet: bool,
    pub(crate) format: Option<String>,
    pub(crate) color: Decoration,
    pub(crate) hyperlink: Decoration,
    pub(crate) gitignore: bool,
    pub(crate) ignore: bool,
    pub(crate) ignore_files: Vec<PathBuf>,
    pub(crate) json: bool,
    pub(crate) json_metadata: bool,
    pub(crate) print0: bool,
    pub(crate) absolute: bool,
    pub(crate) exec: Option<String>,
    pub(crate) exec_batch: Option<String>,
    pub(crate) exec_jobs: usize,
    pub(crate) exec_preview: bool,
    pub(crate) batch_size: usize,
    pub(crate) threads: usize,
    pub(crate) threads_explicit: bool,
    pub(crate) case_mode: CaseMode,
    pub(crate) search_target: SearchTarget,
    pub(crate) min_depth: usize,
    pub(crate) max_depth: Option<usize>,
    pub(crate) sizes: Vec<SizeFilter>,
    pub(crate) modified_within: Option<std::time::Duration>,
    pub(crate) modified_before: Option<ModifiedBefore>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            command: SearchCommand::Legacy,
            path_regex: None,
            cwd: PathBuf::from("."),
            search_paths: Vec::new(),
            positive: Vec::new(),
            negative: Vec::new(),
            simple_terms: Vec::new(),
            extensions: Vec::new(),
            wanted_type: EntryType::File,
            entry_types: Vec::new(),
            require_empty: false,
            require_executable: false,
            owner: None,
            group: None,
            and_terms: Vec::new(),
            content: Vec::new(),
            content_options: ContentOptions::default(),
            content_text: false,
            files_with_matches: false,
            unicode: false,
            escape_patterns: false,
            follow_symlinks: false,
            same_file_system: false,
            max_open: 64,
            contents_first: false,
            hidden: false,
            limit: None,
            sort: false,
            stats: false,
            explain: false,
            why: None,
            strict: false,
            count: false,
            quiet: false,
            format: None,
            color: Decoration::Auto,
            hyperlink: Decoration::Never,
            gitignore: false,
            ignore: false,
            ignore_files: Vec::new(),
            json: false,
            json_metadata: false,
            print0: false,
            absolute: false,
            exec: None,
            exec_batch: None,
            exec_jobs: 1,
            exec_preview: false,
            batch_size: 128,
            threads: default_worker_count(),
            threads_explicit: false,
            case_mode: CaseMode::Sensitive,
            search_target: SearchTarget::Name,
            min_depth: 0,
            max_depth: None,
            sizes: Vec::new(),
            modified_within: None,
            modified_before: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchCommand {
    Legacy,
    Find,
    Grep,
}

impl Options {
    pub(crate) fn query(&self) -> Query {
        let content = !self.content.is_empty();
        Query {
            cwd: self.cwd.clone(),
            patterns: self.positive.clone(),
            exclusions: self.negative.clone(),
            search_terms: self
                .simple_terms
                .iter()
                .map(|value| String::from_utf8(value.clone()).expect("CLI terms are UTF-8"))
                .collect(),
            extensions: self
                .extensions
                .iter()
                .map(|value| String::from_utf8(value.clone()).expect("CLI extensions are UTF-8"))
                .collect(),
            entry_type: if content {
                EntryType::File
            } else {
                self.wanted_type
            },
            entry_types: if content {
                Vec::new()
            } else {
                self.entry_types.clone()
            },
            require_empty: self.require_empty,
            require_executable: self.require_executable,
            owner: self.owner,
            group: self.group,
            and_terms: self.and_terms.clone(),
            unicode: self.unicode,
            escape_patterns: self.escape_patterns,
            follow_symlinks: self.follow_symlinks,
            same_file_system: self.same_file_system,
            max_open: self.max_open,
            contents_first: self.contents_first,
            cancellation: None,
            hidden: self.hidden,
            gitignore: self.gitignore,
            ignore: self.ignore,
            ignore_files: self.ignore_files.clone(),
            case_mode: self.case_mode,
            search_target: self.search_target,
            min_depth: self.min_depth,
            max_depth: self.max_depth,
            sizes: self.sizes.clone(),
            modified_within: self.modified_within,
            modified_before: self.modified_before,
        }
    }

    /// Smart case inspects the content patterns themselves.
    pub(crate) fn content_query(&self) -> Result<Option<ContentQuery>> {
        if self.content.is_empty() {
            return Ok(None);
        }
        let mut matching = self.content_options.matching;
        matching.unicode_case = self.unicode;
        matching.case_insensitive = match self.case_mode {
            CaseMode::Insensitive => true,
            CaseMode::Sensitive => false,
            CaseMode::Smart => !self.content.iter().any(|pattern| {
                if self.unicode {
                    pattern.chars().any(char::is_uppercase)
                } else {
                    pattern.bytes().any(|byte| byte.is_ascii_uppercase())
                }
            }),
        };
        let patterns = self
            .content
            .iter()
            .map(|pattern| pattern.as_bytes().to_vec())
            .collect::<Vec<_>>();
        let query = if self.content_options.regex {
            let factory = self.content_options.factory.ok_or_else(|| {
                AppError("regex search requires branchcut-regex; install it from the Branchcut release archive".into())
            })?;
            let matcher =
                factory(&patterns, matching).map_err(|error| AppError(error.to_string()))?;
            ContentQuery::with_matcher(patterns, matcher, matching, self.content_text)
        } else {
            ContentQuery::with_options(patterns, matching, self.content_text)
        };
        query.map(Some).map_err(|error| AppError(error.to_string()))
    }

    pub(crate) fn content_format(&self) -> ContentFormat {
        if self.json && self.content_options.format == ContentFormat::Text {
            ContentFormat::Jsonl
        } else {
            self.content_options.format
        }
    }

    pub(crate) fn content_paths_only(&self) -> bool {
        self.files_with_matches || self.content_options.files_without_matches
    }
}

fn validate_content(options: &Options) -> Result<()> {
    let content = &options.content_options;
    if options.content.is_empty() {
        if options.content_text {
            return Err(AppError("--text requires --content".to_owned()));
        }
        if options.files_with_matches {
            return Err(AppError(
                "--files-with-matches requires --content".to_owned(),
            ));
        }
        if content.regex
            || content.only_matching
            || content.files_without_matches
            || content.per_file_count
            || content.stdin
            || !content.paths_from.is_empty()
            || content.preprocess.is_some()
            || content.decompress
            || content.explain_matches
            || content.summary
            || content.format != ContentFormat::Text
            || content.matching.whole_word
            || content.matching.whole_line
            || content.matching.invert
            || content.matching.mode != ContentBoolean::Any
            || content.scan.before_context > 0
            || content.scan.after_context > 0
            || content.scan.encoding != crate::ContentEncoding::Auto
            || content.scan.max_line_bytes != ContentScanOptions::default().max_line_bytes
        {
            return Err(AppError(
                "content-search options require --content or --pattern-file".into(),
            ));
        }
        return Ok(());
    }
    if options.content.iter().any(String::is_empty) {
        return Err(AppError("content patterns cannot be empty".to_owned()));
    }
    if options.exec.is_some() || options.exec_batch.is_some() || options.exec_preview {
        return Err(AppError(
            "--content cannot be combined with command execution or preview".to_owned(),
        ));
    }
    if options.print0 && !options.content_paths_only() {
        return Err(AppError(
            "--content cannot be combined with --print0 without a file-list mode".to_owned(),
        ));
    }
    if content.summary && (options.quiet || options.print0) {
        return Err(AppError(
            "--summary requires non-quiet, non-NUL output".into(),
        ));
    }
    if options.format.is_some() && options.content_format() != ContentFormat::Text {
        return Err(AppError("content templates require text output".to_owned()));
    }
    if options.files_with_matches && content.files_without_matches {
        return Err(AppError(
            "--files-with-matches and --files-without-match are mutually exclusive".into(),
        ));
    }
    if options.content_paths_only() && options.count {
        return Err(AppError(
            "--files-with-matches cannot be combined with --count".to_owned(),
        ));
    }
    let context = content.scan.before_context > 0 || content.scan.after_context > 0;
    if (context || content.only_matching)
        && (options.content_paths_only()
            || options.count
            || options.quiet
            || content.per_file_count)
        || content.only_matching
            && (context || content.matching.invert || content.matching.mode == ContentBoolean::None)
    {
        return Err(AppError(
            "context or match-only output conflicts with the selected output mode".into(),
        ));
    }
    if content.per_file_count
        && (options.count
            || options.quiet
            || options.content_paths_only()
            || options.format.is_some()
            || options.content_format() != ContentFormat::Text
            || options.limit.is_some())
    {
        return Err(AppError(
            "--count-per-file requires unrestricted text output".into(),
        ));
    }
    if options.content_format() != ContentFormat::Text
        && (options.quiet || options.count || options.print0)
    {
        return Err(AppError(
            "structured content output cannot be combined with quiet, count, or NUL output".into(),
        ));
    }
    if options.json_metadata
        && !matches!(
            options.content_format(),
            ContentFormat::Json | ContentFormat::Jsonl
        )
    {
        return Err(AppError(
            "--json-metadata requires JSON or JSONL output".into(),
        ));
    }
    if content.explain_matches
        && (options.quiet || options.count || options.print0 || content.per_file_count)
    {
        return Err(AppError(
            "--explain-matches requires match or file records".into(),
        ));
    }
    if content.preprocess.is_some() && content.decompress {
        return Err(AppError(
            "--preprocess and --decompress are mutually exclusive".into(),
        ));
    }
    if !options.entry_types.is_empty() && !options.entry_types.contains(&EntryType::File) {
        return Err(AppError(
            "--content searches regular files; include --type file or remove the type filter"
                .to_owned(),
        ));
    }
    Ok(())
}
pub(crate) fn parse_args() -> Result<Options> {
    let mut arguments: Vec<OsString> = env::args_os().skip(1).collect();
    // Preserve the single-word filename shortcut, including `grep` and `find`.
    let command = if arguments.len() > 1 {
        match arguments[0].to_str() {
            Some("grep") => SearchCommand::Grep,
            Some("find") => SearchCommand::Find,
            _ => SearchCommand::Legacy,
        }
    } else {
        SearchCommand::Legacy
    };
    if command != SearchCommand::Legacy {
        arguments.remove(0);
    }
    let mut options = if has_immediate_control(&arguments) {
        Options::default()
    } else if let Some(selection) = config::find_selection(&arguments)? {
        config::load(&selection)?
    } else {
        Options::default()
    };
    options.command = command;
    let mut args = arguments.into_iter().peekable();

    let mut simple = Vec::new();
    let mut positional_patterns = Vec::new();
    let mut selected_case_mode = None;
    let mut selected_search_target = None;
    let mut selected_batch_size = false;
    let mut selected_type = false;
    let mut selected_exec_jobs = false;
    let mut pattern_files = Vec::new();
    let mut positionals = Vec::new();
    let mut regex = false;
    while let Some(argument) = args.next() {
        if command != SearchCommand::Legacy && !argument.as_encoded_bytes().starts_with(b"-") {
            positionals.push(argument);
            continue;
        }
        let text = argument
            .to_str()
            .ok_or_else(|| AppError("option names and patterns must be valid UTF-8".to_owned()))?;
        match text {
            "-h" | "--help" => {
                print_help(command);
                std::process::exit(0);
            }
            "--version" => {
                println!("branchcut {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--completions" => {
                super::completions::print(&next_utf8(&mut args, "--completions")?)?;
                std::process::exit(0);
            }
            "--glob" => options.positive.push(next_utf8(&mut args, "--glob")?),
            "--exclude" => options.negative.push(next_utf8(&mut args, "--exclude")?),
            "--ignore-file" => options
                .ignore_files
                .push(PathBuf::from(next_os(&mut args, "--ignore-file")?)),
            "-e" | "--regexp" if command == SearchCommand::Grep => {
                options.content.push(next_utf8(&mut args, text)?);
            }
            "-e" | "--extension" => {
                let extension = next_utf8(&mut args, text)?;
                options
                    .extensions
                    .push(extension.trim_start_matches('.').as_bytes().to_vec());
            }
            "--type" => {
                options.wanted_type = match next_utf8(&mut args, "--type")?.as_str() {
                    "file" | "f" => EntryType::File,
                    "dir" | "directory" | "d" => EntryType::Dir,
                    "symlink" | "link" | "l" => EntryType::Symlink,
                    "socket" | "s" => EntryType::Socket,
                    "fifo" | "pipe" | "p" => EntryType::Fifo,
                    "block-device" | "b" => EntryType::BlockDevice,
                    "char-device" | "c" => EntryType::CharDevice,
                    other => {
                        return Err(AppError(format!(
                            "invalid --type {other}; expected file, dir, symlink, socket, fifo, block-device, or char-device"
                        )));
                    }
                };
                if !selected_type {
                    options.entry_types.clear();
                    selected_type = true;
                }
                if !options.entry_types.contains(&options.wanted_type) {
                    options.entry_types.push(options.wanted_type);
                }
            }
            "--empty" => options.require_empty = true,
            "--executable" => options.require_executable = true,
            "--owner" => {
                let value = next_utf8(&mut args, "--owner")?;
                let (owner, group) = parse_owner(&value)?;
                options.owner = owner;
                options.group = group;
            }
            "--and" => options.and_terms.push(next_utf8(&mut args, "--and")?),
            "--content" | "--grep" => options.content.push(next_utf8(&mut args, "--content")?),
            "-f" if command == SearchCommand::Grep => {
                pattern_files.push(PathBuf::from(next_os(&mut args, text)?));
            }
            "--pattern-file" => pattern_files.push(PathBuf::from(next_os(&mut args, text)?)),
            "-E" | "--regex" if command != SearchCommand::Legacy => regex = true,
            "-F" | "--fixed-strings" if command != SearchCommand::Legacy => regex = false,
            "--regex" => options.content_options.regex = true,
            "-w" | "--word-regexp" => options.content_options.matching.whole_word = true,
            "-x" | "--line-regexp" => options.content_options.matching.whole_line = true,
            "-v" | "--invert-match" => options.content_options.matching.invert = true,
            "--content-mode" => {
                options.content_options.matching.mode = match next_utf8(&mut args, text)?.as_str() {
                    "any" => ContentBoolean::Any,
                    "all" => ContentBoolean::All,
                    "none" => ContentBoolean::None,
                    _ => return Err(AppError("--content-mode expects any, all, or none".into())),
                };
            }
            "-A" | "--after-context" => {
                options.content_options.scan.after_context = context_lines(&mut args, text)?
            }
            "-B" | "--before-context" => {
                options.content_options.scan.before_context = context_lines(&mut args, text)?
            }
            "-C" | "--context" => {
                let lines = context_lines(&mut args, text)?;
                options.content_options.scan.before_context = lines;
                options.content_options.scan.after_context = lines;
            }
            "--encoding" => {
                options.content_options.scan.encoding = next_utf8(&mut args, text)?
                    .parse()
                    .map_err(|error: std::io::Error| AppError(error.to_string()))?
            }
            "--max-line-bytes" => {
                options.content_options.scan.max_line_bytes =
                    positive_integer(&next_utf8(&mut args, text)?, text)?
            }
            "--output" => {
                options.content_options.format = next_utf8(&mut args, text)?
                    .parse()
                    .map_err(|error: std::io::Error| AppError(error.to_string()))?
            }
            "-o" | "--only-matching" => options.content_options.only_matching = true,
            "-L" | "--files-without-match" => options.content_options.files_without_matches = true,
            "--count-per-file" => options.content_options.per_file_count = true,
            "--explain-matches" => options.content_options.explain_matches = true,
            "--summary" => options.content_options.summary = true,
            "--paths-from" | "--paths-from0" => options.content_options.paths_from.push((
                PathBuf::from(next_os(&mut args, text)?),
                text.ends_with('0'),
            )),
            "--preprocess" => options.content_options.preprocess = Some(next_os(&mut args, text)?),
            "--decompress" => options.content_options.decompress = true,
            "-a" | "--text" => options.content_text = true,
            "-l" | "--files-with-matches" => options.files_with_matches = true,
            "--unicode" => options.unicode = true,
            "--escaped-globs" => options.escape_patterns = true,
            "--follow" => options.follow_symlinks = true,
            "--one-file-system" => options.same_file_system = true,
            "--max-open" => {
                options.max_open = positive_integer(&next_utf8(&mut args, text)?, text)?
            }
            "--contents-first" => options.contents_first = true,
            "-i" | "--ignore-case" => {
                select_mode(
                    &mut options.case_mode,
                    &mut selected_case_mode,
                    CaseMode::Insensitive,
                    text,
                    "case modes",
                )?;
            }
            "-s" | "--case-sensitive" => {
                select_mode(
                    &mut options.case_mode,
                    &mut selected_case_mode,
                    CaseMode::Sensitive,
                    text,
                    "case modes",
                )?;
            }
            "--smart-case" => {
                select_mode(
                    &mut options.case_mode,
                    &mut selected_case_mode,
                    CaseMode::Smart,
                    text,
                    "case modes",
                )?;
            }
            "--full-path" => {
                select_mode(
                    &mut options.search_target,
                    &mut selected_search_target,
                    SearchTarget::Path,
                    text,
                    "search-target modes",
                )?;
            }
            "--filename" => {
                select_mode(
                    &mut options.search_target,
                    &mut selected_search_target,
                    SearchTarget::Name,
                    text,
                    "search-target modes",
                )?;
            }
            "--size" => {
                let value = next_utf8(&mut args, "--size")?;
                let filter = value
                    .parse::<SizeFilter>()
                    .map_err(|error| AppError(format!("invalid --size {value:?}: {error}")))?;
                options.sizes.push(filter);
            }
            "--modified-within" => {
                let value = next_utf8(&mut args, "--modified-within")?;
                options.modified_within = Some(parse_duration(&value).map_err(|error| {
                    AppError(format!("invalid --modified-within {value:?}: {error}"))
                })?);
            }
            "--modified-before" => {
                let value = next_utf8(&mut args, "--modified-before")?;
                options.modified_before =
                    Some(value.parse::<ModifiedBefore>().map_err(|error| {
                        AppError(format!("invalid --modified-before {value:?}: {error}"))
                    })?);
            }
            "--min-depth" => {
                let value = next_utf8(&mut args, "--min-depth")?;
                options.min_depth = parse_depth(&value, "--min-depth")?;
            }
            "--max-depth" => {
                let value = next_utf8(&mut args, "--max-depth")?;
                options.max_depth = if value == "none" {
                    None
                } else {
                    Some(parse_depth(&value, "--max-depth")?)
                };
            }
            "-c" if command == SearchCommand::Grep => options.content_options.per_file_count = true,
            "--count" => options.count = true,
            "-q" | "--quiet" | "--has-results" => options.quiet = true,
            "--format" => options.format = Some(next_utf8(&mut args, "--format")?),
            "--color" => {
                options.color = parse_decoration(&next_utf8(&mut args, "--color")?, "--color")?
            }
            "--hyperlink" => {
                options.hyperlink =
                    parse_decoration(&next_utf8(&mut args, "--hyperlink")?, "--hyperlink")?
            }
            "--gitignore" => options.gitignore = true,
            "--ignore" => options.ignore = true,
            "--json" => options.json = true,
            "--json-metadata" => {
                options.json = true;
                options.json_metadata = true;
            }
            "-0" | "--print0" => options.print0 = true,
            "--absolute" => options.absolute = true,
            "--exec" => options.exec = Some(next_utf8(&mut args, "--exec")?),
            "--exec-batch" => options.exec_batch = Some(next_utf8(&mut args, "--exec-batch")?),
            "--exec-jobs" => {
                selected_exec_jobs = true;
                options.exec_jobs =
                    positive_integer(&next_utf8(&mut args, "--exec-jobs")?, "--exec-jobs")?;
                if options.exec_jobs > 64 {
                    return Err(AppError("--exec-jobs must be between 1 and 64".to_owned()));
                }
            }
            "--exec-preview" => options.exec_preview = true,
            "--batch-size" => {
                selected_batch_size = true;
                let value = next_utf8(&mut args, "--batch-size")?;
                options.batch_size = value.parse::<usize>().map_err(|_| {
                    AppError(format!(
                        "invalid --batch-size value {value:?}; expected a positive integer"
                    ))
                })?;
                if options.batch_size == 0 {
                    return Err(AppError(
                        "--batch-size must be greater than zero".to_owned(),
                    ));
                }
            }
            "--threads" => {
                let value = next_utf8(&mut args, "--threads")?;
                options.threads = value
                    .parse::<usize>()
                    .map_err(|_| AppError(format!("invalid --threads value: {value}")))?;
                if options.threads == 0 {
                    options.threads = thread::available_parallelism()
                        .map(|count| count.get().min(16))
                        .unwrap_or(1);
                }
                if options.threads == 0 {
                    return Err(AppError("--threads must be greater than zero".to_owned()));
                }
                options.threads_explicit = true;
            }
            "--cwd" => options.cwd = PathBuf::from(next_os(&mut args, "--cwd")?),
            "--search-path" => options
                .search_paths
                .push(PathBuf::from(next_os(&mut args, "--search-path")?)),
            "--hidden" => options.hidden = true,
            "--first" => options.limit = Some(1),
            "--limit" => {
                let value = next_utf8(&mut args, "--limit")?;
                let limit = value
                    .parse::<usize>()
                    .map_err(|_| AppError(format!("invalid --limit value: {value}")))?;
                if limit == 0 {
                    return Err(AppError("--limit must be greater than zero".to_owned()));
                }
                options.limit = Some(limit);
            }
            "--sort" => options.sort = true,
            "--stats" => options.stats = true,
            "--explain" => options.explain = true,
            "--why" => options.why = Some(PathBuf::from(next_os(&mut args, "--why")?)),
            "--config" => {
                next_os(&mut args, "--config")?;
            }
            "--profile" => {
                next_utf8(&mut args, "--profile")?;
            }
            "--strict" => options.strict = true,
            "-" if command == SearchCommand::Grep => positionals.push(argument),
            "--" if command != SearchCommand::Legacy => {
                positionals.extend(args);
                break;
            }
            "--" => {
                for value in args {
                    let value = value.into_string().map_err(|_| {
                        AppError("search terms after -- must be valid UTF-8".to_owned())
                    })?;
                    simple.push(value);
                }
                break;
            }
            _ if text.starts_with('-') => {
                return Err(AppError(format!(
                    "unknown option {text:?}; use --help for supported options (put a literal search term after --)"
                )));
            }
            _ if is_glob_pattern(text) => positional_patterns.push(text.to_owned()),
            _ => simple.push(text.to_owned()),
        }
    }
    if command != SearchCommand::Legacy {
        let mut values = positionals.into_iter();
        let explicit_patterns = !options.content.is_empty() || !pattern_files.is_empty();
        if command != SearchCommand::Grep || !explicit_patterns {
            if let Some(value) = values.next() {
                let pattern = value
                    .into_string()
                    .map_err(|_| AppError("the pattern must be valid UTF-8".into()))?;
                match command {
                    SearchCommand::Grep => options.content.push(pattern),
                    SearchCommand::Find if regex => options.path_regex = Some(pattern),
                    SearchCommand::Find if !pattern.is_empty() => {
                        options.simple_terms.push(pattern.into_bytes())
                    }
                    _ => {}
                }
            } else if command == SearchCommand::Grep || regex {
                return Err(AppError(
                    "missing pattern; see --help on this command".into(),
                ));
            }
        }
        if command == SearchCommand::Grep {
            options.content_options.regex = regex;
        }
        for value in values {
            if command == SearchCommand::Grep && value == "-" {
                if options.content_options.stdin {
                    return Err(AppError("standard input can be used only once".into()));
                }
                options.content_options.stdin = true;
            } else {
                options.search_paths.push(value.into());
            }
        }
    }
    if !positional_patterns.is_empty() {
        if !options.positive.is_empty() || !simple.is_empty() {
            return Err(AppError(
                "positional glob patterns cannot be combined with --glob or simple searches"
                    .to_owned(),
            ));
        }
        options.positive.extend(positional_patterns);
    }
    if !simple.is_empty() {
        if !options.positive.is_empty() {
            return Err(AppError(
                "positional search terms cannot be combined with --glob".to_owned(),
            ));
        }
        if simple.iter().any(String::is_empty) {
            return Err(AppError("search terms cannot be empty".to_owned()));
        }
        options.simple_terms = simple.into_iter().map(|term| term.into_bytes()).collect();
    }
    if options.explain && options.why.is_some() {
        return Err(AppError(
            "--explain and --why cannot be combined".to_owned(),
        ));
    }
    if options.json && options.print0 {
        return Err(AppError(
            "--json and --print0 cannot be combined".to_owned(),
        ));
    }
    if options.count && options.print0 {
        return Err(AppError(
            "--count and --print0 cannot be combined".to_owned(),
        ));
    }
    if options.exec.is_some() && options.exec_batch.is_some() {
        return Err(AppError(
            "--exec and --exec-batch cannot be combined".to_owned(),
        ));
    }
    if options.exec_batch.is_none() && selected_batch_size {
        return Err(AppError("--batch-size requires --exec-batch".to_owned()));
    }
    if options.exec_preview && (options.count || options.print0 || options.json_metadata) {
        return Err(AppError(
            "--exec-preview cannot be combined with count, NUL, or metadata output".to_owned(),
        ));
    }
    if options.json_metadata && options.count {
        return Err(AppError(
            "--json-metadata cannot be combined with --count".to_owned(),
        ));
    }
    if options.exec.is_none()
        && options.exec_batch.is_none()
        && (selected_exec_jobs || options.exec_preview)
    {
        return Err(AppError(
            "--exec-jobs and --exec-preview require --exec or --exec-batch".to_owned(),
        ));
    }
    if options.quiet
        && (options.count
            || options.json
            || options.print0
            || options.format.is_some()
            || options.exec.is_some()
            || options.exec_batch.is_some()
            || options.explain
            || options.why.is_some())
    {
        return Err(AppError(
            "--quiet cannot be combined with output or command modes".to_owned(),
        ));
    }
    if options.format.is_some()
        && (options.json || options.print0 || options.count || options.exec_preview)
    {
        return Err(AppError(
            "--format cannot be combined with JSON, NUL, count, or command preview".to_owned(),
        ));
    }
    if options.quiet {
        options.limit = Some(1);
        options.sort = false;
    }
    for path in pattern_files {
        let text = std::fs::read_to_string(&path).map_err(|error| {
            AppError(format!(
                "cannot read pattern file {}: {error}",
                path.display()
            ))
        })?;
        if text.is_empty() {
            return Err(AppError(format!(
                "pattern file {} is empty",
                path.display()
            )));
        }
        options.content.extend(text.lines().map(str::to_owned));
    }
    if let Some(format) = &options.format {
        if options.content.is_empty() {
            super::exec::validate_format(format)?;
        } else {
            validate_template(format).map_err(|error| AppError(error.to_string()))?;
        }
    }
    validate_content(&options)?;
    resolve_workers(&mut options);
    if options.threads_explicit
        && options.threads > 1
        && options.exec.is_some()
        && !selected_exec_jobs
    {
        return Err(AppError(
            "--threads cannot be combined with --exec unless --exec-jobs is specified; use sequential mode for exact ordering"
                .to_owned(),
        ));
    }
    let mut seen = HashSet::new();
    options
        .extensions
        .retain(|extension| seen.insert(extension.clone()));
    Ok(options)
}

fn context_lines(args: &mut impl Iterator<Item = OsString>, flag: &str) -> Result<usize> {
    let value = next_utf8(args, flag)?;
    value
        .parse::<usize>()
        .ok()
        .filter(|lines| *lines <= 1024)
        .ok_or_else(|| AppError(format!("{flag} expects a line count between 0 and 1024")))
}

fn has_immediate_control(arguments: &[OsString]) -> bool {
    arguments
        .iter()
        .take_while(|argument| *argument != "--")
        .any(|argument| {
            argument == "-h"
                || argument == "--help"
                || argument == "--version"
                || argument == "--completions"
        })
}

fn positive_integer(value: &str, option: &str) -> Result<usize> {
    let result = value
        .parse::<usize>()
        .map_err(|_| AppError(format!("{option} expects a positive integer")))?;
    if result == 0 {
        return Err(AppError(format!("{option} must be greater than zero")));
    }
    Ok(result)
}

fn parse_decoration(value: &str, option: &str) -> Result<Decoration> {
    match value {
        "auto" => Ok(Decoration::Auto),
        "always" => Ok(Decoration::Always),
        "never" => Ok(Decoration::Never),
        _ => Err(AppError(format!("{option} expects auto, always, or never"))),
    }
}

pub(super) fn parse_owner(value: &str) -> Result<(Option<u32>, Option<u32>)> {
    let (owner, group) = value.split_once(':').unwrap_or((value, ""));
    let parse = |part: &str| -> Result<Option<u32>> {
        if part.is_empty() {
            Ok(None)
        } else {
            part.parse::<u32>()
                .map(Some)
                .map_err(|_| AppError("--owner expects numeric UID[:GID]".to_owned()))
        }
    };
    let result = (parse(owner)?, parse(group)?);
    if result == (None, None) {
        return Err(AppError("--owner expects numeric UID[:GID]".to_owned()));
    }
    Ok(result)
}

fn select_mode<T: Copy + Eq + std::fmt::Debug>(
    target: &mut T,
    selected: &mut Option<T>,
    value: T,
    flag: &str,
    mode_group: &str,
) -> Result<()> {
    if let Some(previous) = *selected {
        if previous != value {
            return Err(AppError(format!(
                "conflicting option {flag}; {mode_group} are mutually exclusive"
            )));
        }
    } else {
        *selected = Some(value);
    }
    *target = value;
    Ok(())
}

fn parse_depth(value: &str, option: &str) -> Result<usize> {
    value.parse::<usize>().map_err(|_| {
        AppError(format!(
            "invalid {option} value {value:?}; expected a non-negative integer"
        ))
    })
}

fn default_worker_count() -> usize {
    thread::available_parallelism()
        .map(|count| count.get().min(16))
        .unwrap_or(1)
}

fn resolve_workers(options: &mut Options) {
    if options.threads_explicit {
        return;
    }
    options.threads = if options.limit.is_some() || options.exec.is_some() {
        1
    } else {
        default_worker_count()
    };
}

pub(crate) fn next_utf8(args: &mut impl Iterator<Item = OsString>, option: &str) -> Result<String> {
    next_os(args, option)?
        .into_string()
        .map_err(|_| AppError(format!("{option} value must be valid UTF-8")))
}

pub(crate) fn next_os(args: &mut impl Iterator<Item = OsString>, option: &str) -> Result<OsString> {
    args.next()
        .ok_or_else(|| AppError(format!("missing value for {option}")))
}

fn print_help(command: SearchCommand) {
    match command {
        SearchCommand::Grep => {
            println!(
                r#"Search file contents

Usage: branchcut grep [OPTIONS] PATTERN [PATH...]
       branchcut grep [OPTIONS] -e PATTERN... [PATH...]

  branchcut grep TODO src/
  branchcut grep -i -C 2 'connection refused' logs/
  branchcut-regex grep -E 'ERROR [0-9]+' logs/
  cat app.log | branchcut grep ERROR -

Paths default to the current directory. Use - for stdin and -- before a
pattern or path that starts with a dash. Exit codes: 0 match, 1 no match,
2 error.

Matching:
  -E, --regex             Regular expressions (branchcut-regex)
  -F, --fixed-strings     Literal pattern (default)
  -e, --regexp PATTERN    Add a pattern (repeatable)
  -f, --pattern-file FILE Read patterns, one per line
  -i, --ignore-case       Ignore case
  -w, --word-regexp       Match whole words
  -x, --line-regexp       Match whole lines
  -v, --invert-match      Select nonmatching lines
  --glob GLOB             Filter paths before searching contents
  --ignore                Apply Git and .ignore rules

Output:
  -C, --context N         Show N surrounding lines (-A after, -B before)
  -l, --files-with-matches   Print matching filenames
  -L, --files-without-match  Print filenames without a match
  -c, --count-per-file    Print the matching line count per input
  -o, --only-matching     Print matched text on separate lines
  -q, --quiet             Signal a match with the exit code
  --json                  Stream JSON Lines

See branchcut --help for encoding, other output formats, and traversal options."#
            );
            return;
        }
        SearchCommand::Find => {
            println!(
                r#"Find files by name or path

Usage: branchcut find [OPTIONS] PATTERN [PATH...]

  branchcut find config src/
  branchcut find --glob '**/*.rs' '' src/
  branchcut-regex find -E '^test_.*[.]py$' tests/

Paths default to the current directory. An empty pattern selects all names.

  -E, --regex           Regular expressions (branchcut-regex)
  -F, --fixed-strings   Literal pattern (default)
  -i, --ignore-case     Ignore case
  --full-path           Match against the root-relative path
  --glob GLOB           Filter paths by glob
  --type TYPE           Filter by entry type
  --ignore              Apply Git and .ignore rules

See branchcut --help for additional path filters and output options."#
            );
            return;
        }
        SearchCommand::Legacy => {}
    }
    println!(
        r#"branchcut — compile the query, cut the tree

USAGE:
  branchcut [PATTERN|SEARCH]
  branchcut find [OPTIONS] PATTERN [PATH...]
  branchcut grep [OPTIONS] PATTERN [PATH...]
  branchcut [OPTIONS]

EXAMPLES:
  branchcut config                    Find filenames containing config
  branchcut find config src/          Find them under src/
  branchcut grep TODO src/            Search file contents
  branchcut grep --help               Show content-search examples and options

SEARCH OPTIONS:
  --glob PATTERN          Add a positive glob (repeatable)
  --exclude PATTERN       Exclude a glob; safe subtrees are pruned
  -e, --extension EXT     Match an extension (repeatable)
  --type TYPE             Match file, dir, symlink, socket, fifo, block-device, or char-device (repeatable)
  --empty                 Select empty files or directories
  --executable            Select executable regular files
  --owner UID[:GID]       Select numeric ownership (Unix)
  --and TEXT              Require an additional literal search term (repeatable)
  -i, --ignore-case       Match without case distinctions
  -s, --case-sensitive    Match with case distinctions (default)
  --smart-case            Ignore case unless the query has uppercase letters
  --unicode               Use Unicode scalar wildcards and case matching
  --escaped-globs         Treat backslashes as pattern escapes, not Windows separators
  --full-path             Search simple terms in the cwd-relative path
  --filename              Search simple terms in the entry name (default)
  --size EXPR             Filter by size, e.g. '>10M' (repeatable, AND)
  --modified-within AGE   Match entries modified within an age, e.g. 7d
  --modified-before WHEN  Match older entries by age or UTC timestamp
  --min-depth N           Minimum depth from cwd (default: 0)
  --max-depth N|none      Maximum depth from cwd
  --cwd PATH              Query root [default: .]
  --search-path PATH      Search an additional root (repeatable; relative to cwd)
  --hidden                Include hidden paths
  --follow                Follow directory symlinks with cycle detection
  --one-file-system       Stay on the starting filesystem
  --max-open N            Maximum open directory handles (default: 64)
  --contents-first        Return directories after their descendants

CONTENT SEARCH:
  --content PATTERN       Search selected file contents (repeatable; alias --grep)
  --pattern-file FILE     Add patterns from a UTF-8 file, one per line
  --regex                 Use regular expressions (in branchcut-regex)
  --content-mode MODE     Select lines matching any, all, or none of the patterns
  -w, --word-regexp       Require whole-word matches
  -x, --line-regexp       Require whole-line matches
  -v, --invert-match      Invert the line selection
  -a, --text              Search inputs that the binary probe would skip
  --encoding ENCODING     auto, utf-8, utf-16le, utf-16be, latin1, windows-1252, or raw
  --max-line-bytes N      Maximum decoded bytes per line (default: 8388608)
  --paths-from FILE|-     Read newline-delimited paths
  --paths-from0 FILE|-    Read NUL-delimited paths
  --preprocess PROGRAM    Search an executable's stdout for each selected file
  --decompress            Open gzip, bzip2, xz, and Zstandard streams via external tools
  -A, --after-context N   Emit N lines after each match
  -B, --before-context N  Emit N lines before each match
  -C, --context N         Emit N lines on both sides of each match
  -o, --only-matching     Emit each nonempty matched span on its own
  -l, --files-with-matches  Emit files containing a content match
  -L, --files-without-match Emit text files with no content matches
  --count-per-file        Emit path and matching line count per file
  --output FORMAT         text, jsonl, json, csv, or tsv
  --explain-matches       Include the path selection reasons alongside matches
  --summary               Include totals in structured output or on stderr for text

OUTPUT OPTIONS:
  --first                 Stop after the first match
  --limit N               Stop after N matches
  --sort                  Sort all matches before applying limits
  --count                 Print only the match count
  -q, --quiet             Print nothing; exit 0 for a match, 1 for no match
  --json                  Stream matching paths or content records as JSON Lines
  --json-metadata         Add type, size, readonly, and modification time to JSON Lines
  -0, --print0            Terminate native path output with NUL bytes
  --absolute              Emit and execute absolute paths
  --format TEMPLATE       Format paths with {{}}, {{/}}, {{//}}, {{.}}, or {{/.}}
                          Content fields include {{path}}, {{line}}, {{column}}, {{text}}, and {{spans}}
  --color WHEN            auto, always, or never (plain output only)
  --hyperlink WHEN        auto, always, or never (plain output only)
  --exec COMMAND          Run a command template for each match; use {{}}
  --exec-batch COMMAND    Run one command for each bounded batch; use {{}}
  --batch-size N          Maximum matches per --exec-batch invocation (default: 128)
  --exec-jobs N           Maximum concurrent child processes (1-64; default: 1)
  --exec-preview          Print command argv as JSON Lines without executing
  --stats                 Print traversal counters to stderr
  --explain               Print the compiled plan without traversing
  --why PATH              Explain why one cwd-relative path matches or is excluded

OTHER OPTIONS:
  --config FILE           Load a saved query profile file
  --profile NAME          Select a named profile from --config [default: default]
  --gitignore             Apply hierarchical .gitignore files
  --ignore                Apply Git ignore sources and hierarchical .ignore files
  --ignore-file FILE      Add a Git-style ignore file (repeatable; overrides discovered rules)
  --threads N             Worker count; 1 is sequential, 0 uses up to 16 (default: adaptive)
  --strict                Fail if any filesystem entry cannot be read
  --completions SHELL     Print bash, zsh, fish, or powershell completions
  --                      Treat remaining arguments as literal search terms
  -h, --help              Print help
  --version               Print version"#
    );
}
pub(crate) fn is_glob_pattern(value: &str) -> bool {
    value
        .bytes()
        .any(|byte| matches!(byte, b'*' | b'?' | b'[' | b'{' | b'!'))
        || value
            .as_bytes()
            .windows(2)
            .any(|pair| matches!(pair[0], b'@' | b'+') && pair[1] == b'(')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_workers_keep_ordering_sensitive_queries_sequential() {
        let automatic = Options::default().threads;
        assert_eq!(automatic, default_worker_count());

        let limited = Options {
            limit: Some(1),
            ..Options::default()
        };
        let mut limited = limited;
        resolve_workers(&mut limited);
        assert_eq!(limited.threads, 1);

        let executed = Options {
            exec: Some("echo {}".to_owned()),
            ..Options::default()
        };
        let mut executed = executed;
        resolve_workers(&mut executed);
        assert_eq!(executed.threads, 1);

        let explicit = Options {
            limit: Some(1),
            threads: 4,
            threads_explicit: true,
            ..Options::default()
        };
        let mut explicit = explicit;
        resolve_workers(&mut explicit);
        assert_eq!(explicit.threads, 4);
    }
}
