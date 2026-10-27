use crate::error::{AppError, Result};
use crate::{EntryType, Query};
use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::thread;
#[derive(Clone, Debug)]
pub(crate) struct Options {
    pub(crate) cwd: PathBuf,
    pub(crate) positive: Vec<String>,
    pub(crate) negative: Vec<String>,
    pub(crate) simple_terms: Vec<Vec<u8>>,
    pub(crate) extensions: Vec<Vec<u8>>,
    pub(crate) wanted_type: EntryType,
    pub(crate) hidden: bool,
    pub(crate) limit: Option<usize>,
    pub(crate) sort: bool,
    pub(crate) stats: bool,
    pub(crate) explain: bool,
    pub(crate) strict: bool,
    pub(crate) count: bool,
    pub(crate) gitignore: bool,
    pub(crate) json: bool,
    pub(crate) exec: Option<String>,
    pub(crate) threads: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            cwd: PathBuf::from("."),
            positive: Vec::new(),
            negative: Vec::new(),
            simple_terms: Vec::new(),
            extensions: Vec::new(),
            wanted_type: EntryType::File,
            hidden: false,
            limit: None,
            sort: false,
            stats: false,
            explain: false,
            strict: false,
            count: false,
            gitignore: false,
            json: false,
            exec: None,
            threads: 1,
        }
    }
}

impl Options {
    pub(crate) fn query(&self) -> Query {
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
            entry_type: self.wanted_type,
            hidden: self.hidden,
            gitignore: self.gitignore,
        }
    }
}
pub(crate) fn parse_args() -> Result<Options> {
    let mut options = Options::default();
    let mut args = env::args_os().skip(1).peekable();

    let mut simple = Vec::new();
    let mut positional_patterns = Vec::new();
    while let Some(argument) = args.next() {
        let text = argument
            .to_str()
            .ok_or_else(|| AppError("option names and patterns must be valid UTF-8".to_owned()))?;
        match text {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "--version" => {
                println!("branchcut {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--glob" => options.positive.push(next_utf8(&mut args, "--glob")?),
            "--exclude" => options.negative.push(next_utf8(&mut args, "--exclude")?),
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
                    other => {
                        return Err(AppError(format!(
                            "invalid --type {other}; expected file, dir, or symlink"
                        )));
                    }
                };
            }
            "--count" => options.count = true,
            "--gitignore" => options.gitignore = true,
            "--json" => options.json = true,
            "--exec" => options.exec = Some(next_utf8(&mut args, "--exec")?),
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
            }
            "--cwd" => options.cwd = PathBuf::from(next_os(&mut args, "--cwd")?),
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
            "--strict" => options.strict = true,
            "--" => {
                for value in args {
                    let value = value.to_string_lossy().into_owned();
                    if is_glob_pattern(&value) {
                        positional_patterns.push(value);
                    } else {
                        simple.push(value);
                    }
                }
                break;
            }
            _ if is_glob_pattern(text) => positional_patterns.push(text.to_owned()),
            _ => simple.push(text.to_owned()),
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
    let mut seen = HashSet::new();
    options
        .extensions
        .retain(|extension| seen.insert(extension.clone()));
    Ok(options)
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

pub(crate) fn print_help() {
    println!(
        "branchcut — compile the query, cut the tree\n\nUSAGE:\n  branchcut [PATTERN|SEARCH]\n  branchcut [OPTIONS]\n\nOPTIONS:\n  --glob PATTERN       Add a positive glob (repeatable)\n  --exclude PATTERN    Exclude a glob; subtree patterns are pruned\n  -e, --extension EXT  Match an extension (repeatable)\n  --type TYPE          Match file, dir, or symlink\n  --cwd PATH           Query root [default: .]\n  --hidden             Include hidden paths\n  --first              Stop after the first match\n  --limit N            Stop after N matches\n  --threads N          Use bounded parallel traversal workers (0 = auto)\n  --sort               Sort all matches before applying limits\n  --count              Print only the match count\n  --gitignore          Apply hierarchical .gitignore files\n  --json               Stream matching paths as JSON Lines\n  --exec COMMAND       Run a command template for each match; use {{}}\n  --strict             Fail if any filesystem entry cannot be read\n  --stats              Print traversal counters to stderr\n  --explain            Print the compiled plan without traversing\n  -h, --help           Print help\n  --version            Print version"
    );
}
pub(crate) fn is_glob_pattern(value: &str) -> bool {
    value
        .bytes()
        .any(|byte| matches!(byte, b'*' | b'?' | b'[' | b'{' | b'!'))
}
