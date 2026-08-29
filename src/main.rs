use std::collections::HashSet;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

#[derive(Debug)]
struct AppError(String);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for AppError {}

type Result<T, E = AppError> = std::result::Result<T, E>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WantedType {
    Any,
    File,
    Dir,
    Symlink,
}

#[derive(Debug)]
struct Options {
    cwd: PathBuf,
    positive: Vec<String>,
    negative: Vec<String>,
    extensions: Vec<Vec<u8>>,
    wanted_type: WantedType,
    hidden: bool,
    limit: Option<usize>,
    sort: bool,
    stats: bool,
    explain: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            cwd: PathBuf::from("."),
            positive: Vec::new(),
            negative: Vec::new(),
            extensions: Vec::new(),
            wanted_type: WantedType::Any,
            hidden: false,
            limit: None,
            sort: false,
            stats: false,
            explain: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Token {
    Literal(u8),
    Star,
    Question,
    Class {
        negated: bool,
        ranges: Vec<(u8, u8)>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Segment {
    GlobStar,
    Match(Vec<Token>),
}

#[derive(Clone, Debug)]
struct Pattern {
    source: String,
    segments: Vec<Segment>,
    literal_prefix: Vec<Vec<u8>>,
}

impl Pattern {
    fn compile(source: String) -> Result<Self> {
        let normalized = source.replace('\\', "/");
        if normalized.starts_with('/') || has_drive_prefix(&normalized) {
            return Err(AppError(format!(
                "pattern must be relative to --cwd: {source}"
            )));
        }
        let mut segments = Vec::new();
        for raw in normalized
            .split('/')
            .filter(|part| !part.is_empty() && *part != ".")
        {
            if raw == ".." {
                return Err(AppError(format!(
                    "pattern cannot traverse above --cwd: {source}"
                )));
            }
            if raw == "**" {
                if !matches!(segments.last(), Some(Segment::GlobStar)) {
                    segments.push(Segment::GlobStar);
                }
            } else {
                segments.push(Segment::Match(parse_segment(raw, &source)?));
            }
        }
        if segments.is_empty() {
            return Err(AppError(format!("empty pattern: {source}")));
        }
        let literal_prefix = segments
            .iter()
            .map_while(|segment| match segment {
                Segment::Match(tokens) => literal_segment(tokens),
                Segment::GlobStar => None,
            })
            .collect();
        Ok(Self {
            source,
            segments,
            literal_prefix,
        })
    }

    fn matches(&self, components: &[&OsStr]) -> bool {
        let mut memo = vec![None; (self.segments.len() + 1) * (components.len() + 1)];
        self.matches_from(0, 0, components, &mut memo)
    }

    fn matches_from(
        &self,
        pattern_index: usize,
        path_index: usize,
        components: &[&OsStr],
        memo: &mut [Option<bool>],
    ) -> bool {
        let width = components.len() + 1;
        let slot = pattern_index * width + path_index;
        if let Some(answer) = memo[slot] {
            return answer;
        }
        let answer = if pattern_index == self.segments.len() {
            path_index == components.len()
        } else {
            match &self.segments[pattern_index] {
                Segment::GlobStar => {
                    self.matches_from(pattern_index + 1, path_index, components, memo)
                        || (path_index < components.len()
                            && self.matches_from(pattern_index, path_index + 1, components, memo))
                }
                Segment::Match(tokens) => {
                    path_index < components.len()
                        && segment_matches(tokens, components[path_index])
                        && self.matches_from(pattern_index + 1, path_index + 1, components, memo)
                }
            }
        };
        memo[slot] = Some(answer);
        answer
    }

    fn descendant_possible(&self, components: &[&OsStr]) -> bool {
        let mut states = vec![false; self.segments.len() + 1];
        states[0] = true;
        epsilon_closure(&self.segments, &mut states);
        for component in components {
            let mut next = vec![false; states.len()];
            for index in 0..self.segments.len() {
                if !states[index] {
                    continue;
                }
                match &self.segments[index] {
                    Segment::GlobStar => next[index] = true,
                    Segment::Match(tokens) if segment_matches(tokens, component) => {
                        next[index + 1] = true;
                    }
                    Segment::Match(_) => {}
                }
            }
            epsilon_closure(&self.segments, &mut next);
            states = next;
            if !states.iter().any(|active| *active) {
                return false;
            }
        }
        states
            .iter()
            .enumerate()
            .any(|(index, active)| *active && index < self.segments.len())
    }

    fn excludes_subtree(&self, components: &[&OsStr]) -> bool {
        matches!(self.segments.last(), Some(Segment::GlobStar)) && self.matches(components)
    }
}

fn epsilon_closure(segments: &[Segment], states: &mut [bool]) {
    for index in 0..segments.len() {
        if states[index] && matches!(segments[index], Segment::GlobStar) {
            states[index + 1] = true;
        }
    }
}

fn literal_segment(tokens: &[Token]) -> Option<Vec<u8>> {
    tokens
        .iter()
        .map(|token| match token {
            Token::Literal(byte) => Some(*byte),
            _ => None,
        })
        .collect()
}

fn parse_segment(raw: &str, whole_pattern: &str) -> Result<Vec<Token>> {
    let bytes = raw.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'*' => {
                if !matches!(tokens.last(), Some(Token::Star)) {
                    tokens.push(Token::Star);
                }
                index += 1;
            }
            b'?' => {
                tokens.push(Token::Question);
                index += 1;
            }
            b'[' => {
                let (class, consumed) = parse_class(&bytes[index..], whole_pattern)?;
                tokens.push(class);
                index += consumed;
            }
            b'\\' if index + 1 < bytes.len() => {
                tokens.push(Token::Literal(bytes[index + 1]));
                index += 2;
            }
            byte => {
                tokens.push(Token::Literal(byte));
                index += 1;
            }
        }
    }
    Ok(tokens)
}

fn parse_class(bytes: &[u8], whole_pattern: &str) -> Result<(Token, usize)> {
    let mut index = 1;
    let negated = matches!(bytes.get(index), Some(b'!' | b'^'));
    if negated {
        index += 1;
    }
    let mut ranges = Vec::new();
    if bytes.get(index) == Some(&b']') {
        ranges.push((b']', b']'));
        index += 1;
    }
    while let Some(&byte) = bytes.get(index) {
        if byte == b']' && !ranges.is_empty() {
            return Ok((Token::Class { negated, ranges }, index + 1));
        }
        let start = byte;
        if bytes.get(index + 1) == Some(&b'-') {
            let Some(&end) = bytes.get(index + 2) else {
                break;
            };
            if end == b']' {
                ranges.push((start, start));
                ranges.push((b'-', b'-'));
                index += 2;
                continue;
            }
            if start > end {
                return Err(AppError(format!(
                    "invalid descending character range in pattern: {whole_pattern}"
                )));
            }
            ranges.push((start, end));
            index += 3;
        } else {
            ranges.push((start, start));
            index += 1;
        }
    }
    Err(AppError(format!(
        "unterminated character class in pattern: {whole_pattern}"
    )))
}

fn segment_matches(tokens: &[Token], name: &OsStr) -> bool {
    with_os_bytes(name, |bytes| {
        let mut current = vec![false; bytes.len() + 1];
        current[0] = true;
        for token in tokens {
            let mut next = vec![false; bytes.len() + 1];
            match token {
                Token::Star => {
                    let mut reachable = false;
                    for position in 0..=bytes.len() {
                        reachable |= current[position];
                        next[position] = reachable;
                    }
                }
                Token::Question => {
                    next[1..].copy_from_slice(&current[..bytes.len()]);
                }
                Token::Literal(expected) => {
                    for position in 0..bytes.len() {
                        next[position + 1] = current[position] && bytes[position] == *expected;
                    }
                }
                Token::Class { negated, ranges } => {
                    for position in 0..bytes.len() {
                        let contained = ranges.iter().any(|(start, end)| {
                            *start <= bytes[position] && bytes[position] <= *end
                        });
                        next[position + 1] = current[position] && (contained != *negated);
                    }
                }
            }
            current = next;
        }
        current[bytes.len()]
    })
}

#[cfg(unix)]
fn with_os_bytes<T>(value: &OsStr, function: impl FnOnce(&[u8]) -> T) -> T {
    function(value.as_bytes())
}

#[cfg(not(unix))]
fn with_os_bytes<T>(value: &OsStr, function: impl FnOnce(&[u8]) -> T) -> T {
    let text = value.to_string_lossy();
    function(text.as_bytes())
}

fn has_drive_prefix(pattern: &str) -> bool {
    pattern.as_bytes().get(1) == Some(&b':')
}

fn expand_braces(pattern: &str) -> Result<Vec<String>> {
    let Some(open) = pattern.find('{') else {
        if pattern.contains('}') {
            return Err(AppError(format!(
                "unmatched closing brace in pattern: {pattern}"
            )));
        }
        return Ok(vec![pattern.to_owned()]);
    };
    let Some(relative_close) = pattern[open + 1..].find('}') else {
        return Err(AppError(format!(
            "unterminated brace alternative in pattern: {pattern}"
        )));
    };
    let close = open + 1 + relative_close;
    let body = &pattern[open + 1..close];
    if body.contains('{') || pattern[close + 1..].contains('}') {
        return Err(AppError(format!(
            "nested or unmatched braces are unsupported: {pattern}"
        )));
    }
    let alternatives: Vec<_> = body.split(',').collect();
    if alternatives.len() < 2 || alternatives.iter().any(|part| part.is_empty()) {
        return Err(AppError(format!(
            "brace alternatives require two or more non-empty values: {pattern}"
        )));
    }
    let mut expanded = Vec::new();
    for alternative in alternatives {
        let combined = format!(
            "{}{}{}",
            &pattern[..open],
            alternative,
            &pattern[close + 1..]
        );
        expanded.extend(expand_braces(&combined)?);
    }
    Ok(expanded)
}

#[derive(Debug)]
struct QueryPlan {
    root: PathBuf,
    root_relative: PathBuf,
    positives: Vec<Pattern>,
    negatives: Vec<Pattern>,
    extensions: Vec<Vec<u8>>,
    wanted_type: WantedType,
    hidden: bool,
    limit: Option<usize>,
    sort: bool,
}

impl QueryPlan {
    fn compile(options: &Options) -> Result<Self> {
        let mut positives = Vec::new();
        for source in &options.positive {
            for expanded in expand_braces(source)? {
                positives.push(Pattern::compile(expanded)?);
            }
        }
        if positives.is_empty() {
            positives.push(Pattern::compile("**/*".to_owned())?);
        }
        let mut negatives = Vec::new();
        for source in &options.negative {
            let source = source.strip_prefix('!').unwrap_or(source);
            for expanded in expand_braces(source)? {
                negatives.push(Pattern::compile(expanded)?);
            }
        }
        let common = common_literal_prefix(&positives);
        let root_relative = common.iter().fold(PathBuf::new(), |mut path, component| {
            path.push(os_string_from_pattern_bytes(component));
            path
        });
        let root = options.cwd.join(&root_relative);
        Ok(Self {
            root,
            root_relative,
            positives,
            negatives,
            extensions: options.extensions.clone(),
            wanted_type: options.wanted_type,
            hidden: options.hidden,
            limit: options.limit,
            sort: options.sort,
        })
    }

    fn path_matches(&self, relative: &Path) -> bool {
        let components = normal_components(relative);
        self.positives
            .iter()
            .any(|pattern| pattern.matches(&components))
            && !self
                .negatives
                .iter()
                .any(|pattern| pattern.matches(&components))
    }

    fn directory_possible(&self, relative: &Path) -> bool {
        let components = normal_components(relative);
        self.positives
            .iter()
            .any(|pattern| pattern.descendant_possible(&components))
    }

    fn directory_excluded(&self, relative: &Path) -> bool {
        let components = normal_components(relative);
        self.negatives
            .iter()
            .any(|pattern| pattern.excludes_subtree(&components))
    }

    fn extension_matches(&self, name: &OsStr) -> bool {
        if self.extensions.is_empty() {
            return true;
        }
        with_os_bytes(name, |bytes| {
            bytes
                .iter()
                .rposition(|byte| *byte == b'.')
                .map(|dot| &bytes[dot + 1..])
                .is_some_and(|ext| self.extensions.iter().any(|wanted| wanted == ext))
        })
    }
}

fn common_literal_prefix(patterns: &[Pattern]) -> Vec<Vec<u8>> {
    let Some(first) = patterns.first() else {
        return Vec::new();
    };
    let mut length = first.literal_prefix.len();
    for pattern in &patterns[1..] {
        length = length.min(pattern.literal_prefix.len());
        for index in 0..length {
            if first.literal_prefix[index] != pattern.literal_prefix[index] {
                length = index;
                break;
            }
        }
    }
    first.literal_prefix[..length].to_vec()
}

#[cfg(unix)]
fn os_string_from_pattern_bytes(bytes: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes.to_vec())
}

#[cfg(not(unix))]
fn os_string_from_pattern_bytes(bytes: &[u8]) -> OsString {
    OsString::from(String::from_utf8_lossy(bytes).into_owned())
}

fn normal_components(path: &Path) -> Vec<&OsStr> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value),
            _ => None,
        })
        .collect()
}

#[derive(Default, Debug)]
struct Stats {
    dirs_seen: u64,
    dirs_opened: u64,
    dirs_pruned_positive: u64,
    dirs_pruned_exclude: u64,
    entries_seen: u64,
    candidate_files: u64,
    metadata_calls: u64,
    matches: u64,
    errors: u64,
}

struct Runner<'a> {
    plan: &'a QueryPlan,
    stats: Stats,
    output: Vec<PathBuf>,
    stopped: bool,
}

impl<'a> Runner<'a> {
    fn new(plan: &'a QueryPlan) -> Self {
        Self {
            plan,
            stats: Stats::default(),
            output: Vec::new(),
            stopped: false,
        }
    }

    fn run(&mut self) -> io::Result<()> {
        let root_metadata = match fs::symlink_metadata(&self.plan.root) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        self.stats.metadata_calls += 1;
        let root_type = root_metadata.file_type();
        if !root_type.is_dir() {
            let relative = self.plan.root_relative.clone();
            let type_matches = match self.plan.wanted_type {
                WantedType::Any => true,
                WantedType::File => root_type.is_file(),
                WantedType::Dir => false,
                WantedType::Symlink => root_type.is_symlink(),
            };
            if type_matches
                && self.plan.path_matches(&relative)
                && self
                    .plan
                    .extension_matches(self.plan.root.file_name().unwrap_or_default())
            {
                self.emit(relative)?;
            }
            return Ok(());
        }
        if !self.plan.root_relative.as_os_str().is_empty()
            && matches!(self.plan.wanted_type, WantedType::Any | WantedType::Dir)
            && self.plan.path_matches(&self.plan.root_relative)
        {
            self.emit(self.plan.root_relative.clone())?;
        }
        self.visit_directory(self.plan.root.clone(), self.plan.root_relative.clone())?;
        if self.plan.sort {
            self.output.sort();
            let stdout = io::stdout();
            let mut lock = stdout.lock();
            for path in &self.output {
                writeln!(lock, "{}", display_path(path))?;
            }
        }
        Ok(())
    }

    fn visit_directory(&mut self, absolute: PathBuf, relative: PathBuf) -> io::Result<()> {
        if self.stopped {
            return Ok(());
        }
        self.stats.dirs_seen += 1;
        if !relative.as_os_str().is_empty() {
            if self.plan.directory_excluded(&relative) {
                self.stats.dirs_pruned_exclude += 1;
                return Ok(());
            }
            if !self.plan.directory_possible(&relative) {
                self.stats.dirs_pruned_positive += 1;
                return Ok(());
            }
        }
        let entries = match fs::read_dir(&absolute) {
            Ok(entries) => entries,
            Err(error) => {
                self.stats.errors += 1;
                eprintln!("branchcut: cannot read {}: {error}", absolute.display());
                return Ok(());
            }
        };
        self.stats.dirs_opened += 1;
        for result in entries {
            if self.stopped {
                break;
            }
            let entry = match result {
                Ok(entry) => entry,
                Err(error) => {
                    self.stats.errors += 1;
                    eprintln!(
                        "branchcut: directory entry error in {}: {error}",
                        absolute.display()
                    );
                    continue;
                }
            };
            self.stats.entries_seen += 1;
            let name = entry.file_name();
            if !self.plan.hidden && is_hidden(&name) {
                continue;
            }
            let child_relative = relative.join(&name);
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    self.stats.errors += 1;
                    eprintln!(
                        "branchcut: cannot inspect {}: {error}",
                        entry.path().display()
                    );
                    continue;
                }
            };
            if file_type.is_dir() {
                if self.plan.wanted_type == WantedType::Dir
                    && self.plan.path_matches(&child_relative)
                {
                    self.emit(child_relative.clone())?;
                }
                self.visit_directory(entry.path(), child_relative)?;
            } else {
                self.stats.candidate_files += 1;
                let type_matches = match self.plan.wanted_type {
                    WantedType::Any => true,
                    WantedType::File => file_type.is_file(),
                    WantedType::Dir => false,
                    WantedType::Symlink => file_type.is_symlink(),
                };
                if type_matches
                    && self.plan.extension_matches(&name)
                    && self.plan.path_matches(&child_relative)
                {
                    self.emit(child_relative)?;
                }
            }
        }
        Ok(())
    }

    fn emit(&mut self, path: PathBuf) -> io::Result<()> {
        self.stats.matches += 1;
        if self.plan.sort {
            self.output.push(path);
        } else {
            println!("{}", display_path(&path));
        }
        if self
            .plan
            .limit
            .is_some_and(|limit| self.stats.matches as usize >= limit)
        {
            self.stopped = true;
        }
        Ok(())
    }
}

fn is_hidden(name: &OsStr) -> bool {
    with_os_bytes(name, |bytes| {
        bytes.first() == Some(&b'.') && bytes != b"." && bytes != b".."
    })
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn parse_args() -> Result<Options> {
    let mut options = Options::default();
    let mut args = env::args_os().skip(1).peekable();
    let mut simple = Vec::new();
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
                    "file" | "f" => WantedType::File,
                    "dir" | "directory" | "d" => WantedType::Dir,
                    "symlink" | "link" | "l" => WantedType::Symlink,
                    other => {
                        return Err(AppError(format!(
                            "invalid --type {other}; expected file, dir, or symlink"
                        )));
                    }
                };
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
            "--" => {
                simple.extend(args.map(|value| value.to_string_lossy().into_owned()));
                break;
            }
            _ if text.starts_with('-') => return Err(AppError(format!("unknown option: {text}"))),
            _ => simple.push(text.to_owned()),
        }
    }
    if !simple.is_empty() {
        if !options.positive.is_empty() {
            return Err(AppError(
                "positional search terms cannot be combined with --glob".to_owned(),
            ));
        }
        options
            .positive
            .extend(simple.into_iter().map(|term| format!("**/*{term}*")));
    }
    let mut seen = HashSet::new();
    options
        .extensions
        .retain(|extension| seen.insert(extension.clone()));
    Ok(options)
}

fn next_utf8(args: &mut impl Iterator<Item = OsString>, option: &str) -> Result<String> {
    next_os(args, option)?
        .into_string()
        .map_err(|_| AppError(format!("{option} value must be valid UTF-8")))
}

fn next_os(args: &mut impl Iterator<Item = OsString>, option: &str) -> Result<OsString> {
    args.next()
        .ok_or_else(|| AppError(format!("missing value for {option}")))
}

fn print_help() {
    println!(
        "branchcut — compile the query, cut the tree\n\nUSAGE:\n  branchcut [SEARCH]\n  branchcut [OPTIONS]\n\nOPTIONS:\n  --glob PATTERN       Add a positive glob (repeatable)\n  --exclude PATTERN    Exclude a glob; subtree patterns are pruned\n  -e, --extension EXT  Match an extension (repeatable)\n  --type TYPE          Match file, dir, or symlink\n  --cwd PATH           Query root [default: .]\n  --hidden             Include hidden paths\n  --first              Stop after the first match\n  --limit N            Stop after N matches\n  --sort               Sort output instead of streaming\n  --stats              Print traversal counters to stderr\n  --explain            Print the compiled plan without traversing\n  -h, --help           Print help\n  --version            Print version"
    );
}

fn print_explain(plan: &QueryPlan) {
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
        println!("  {}", pattern.source);
    }
    println!("\nEXCLUSIONS");
    if plan.negatives.is_empty() {
        println!("  (none)");
    } else {
        for pattern in &plan.negatives {
            println!("  {}", pattern.source);
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
        plan.limit.map_or("all matches".to_owned(), |limit| format!(
            "first {limit} matches"
        ))
    );
    println!("\nSTRATEGY\n  sequential depth-first traversal with positive and exclusion pruning");
}

fn print_stats(stats: &Stats, elapsed: Duration) {
    eprintln!("matched                 {}", stats.matches);
    eprintln!("directories considered  {}", stats.dirs_seen);
    eprintln!("directories opened      {}", stats.dirs_opened);
    eprintln!(
        "directories pruned      {}",
        stats.dirs_pruned_positive + stats.dirs_pruned_exclude
    );
    eprintln!("  positive              {}", stats.dirs_pruned_positive);
    eprintln!("  excluded              {}", stats.dirs_pruned_exclude);
    eprintln!("entries inspected       {}", stats.entries_seen);
    eprintln!("candidate files         {}", stats.candidate_files);
    eprintln!("metadata calls          {}", stats.metadata_calls);
    eprintln!("filesystem errors       {}", stats.errors);
    eprintln!(
        "elapsed                 {:.3}ms",
        elapsed.as_secs_f64() * 1000.0
    );
}

fn real_main() -> Result<()> {
    let options = parse_args()?;
    let plan = QueryPlan::compile(&options)?;
    if options.explain {
        print_explain(&plan);
        return Ok(());
    }
    let started = Instant::now();
    let mut runner = Runner::new(&plan);
    runner
        .run()
        .map_err(|error| AppError(format!("output failed: {error}")))?;
    if options.stats {
        print_stats(&runner.stats, started.elapsed());
    }
    Ok(())
}

fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("branchcut: {error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(path: &Path) -> Vec<&OsStr> {
        normal_components(path)
    }

    fn matches(pattern: &str, path: &str) -> bool {
        Pattern::compile(pattern.to_owned())
            .unwrap()
            .matches(&parts(Path::new(path)))
    }

    #[test]
    fn matches_basic_segment_syntax() {
        assert!(matches("src/*.rs", "src/main.rs"));
        assert!(matches("src/ma?n.rs", "src/main.rs"));
        assert!(matches("src/[a-z]ain.rs", "src/main.rs"));
        assert!(matches("src/[!0-9]*.rs", "src/main.rs"));
        assert!(!matches("src/*.rs", "src/nested/main.rs"));
        assert!(!matches("src/[!a-z]*.rs", "src/main.rs"));
    }

    #[test]
    fn globstar_matches_zero_or_many_components() {
        assert!(matches("src/**/mod.rs", "src/mod.rs"));
        assert!(matches("src/**/mod.rs", "src/a/b/mod.rs"));
        assert!(!matches("src/**/mod.rs", "other/mod.rs"));
    }

    #[test]
    fn brace_expansion_is_flat_and_deterministic() {
        assert_eq!(
            expand_braces("**/*.{rs,toml}").unwrap(),
            ["**/*.rs", "**/*.toml"]
        );
        assert!(expand_braces("*.{rs}").is_err());
        assert!(expand_braces("*.{rs,{ts,tsx}}").is_err());
    }

    #[test]
    fn prefix_viability_prunes_impossible_directories() {
        let pattern = Pattern::compile("packages/*/src/**/*.rs".to_owned()).unwrap();
        assert!(pattern.descendant_possible(&parts(Path::new("packages/core"))));
        assert!(pattern.descendant_possible(&parts(Path::new("packages/core/src/deep"))));
        assert!(!pattern.descendant_possible(&parts(Path::new("unrelated"))));
    }

    #[test]
    fn common_prefix_stops_at_first_difference() {
        let patterns = [
            Pattern::compile("packages/core/src/**/*.rs".to_owned()).unwrap(),
            Pattern::compile("packages/core/tests/**/*.rs".to_owned()).unwrap(),
        ];
        assert_eq!(
            common_literal_prefix(&patterns),
            [b"packages".to_vec(), b"core".to_vec()]
        );
    }

    #[test]
    fn invalid_patterns_return_errors() {
        assert!(Pattern::compile("src/[abc".to_owned()).is_err());
        assert!(Pattern::compile("../*.rs".to_owned()).is_err());
        assert!(Pattern::compile("/tmp/*.rs".to_owned()).is_err());
    }
}
