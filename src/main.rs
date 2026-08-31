use std::collections::{HashSet, VecDeque};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
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
    File,
    Dir,
    Symlink,
}

#[derive(Debug)]
struct Options {
    cwd: PathBuf,
    positive: Vec<String>,
    negative: Vec<String>,
    simple_terms: Vec<Vec<u8>>,
    extensions: Vec<Vec<u8>>,
    wanted_type: WantedType,
    hidden: bool,
    limit: Option<usize>,
    sort: bool,
    stats: bool,
    explain: bool,
    strict: bool,
    count: bool,
    gitignore: bool,
    json: bool,
    exec: Option<String>,
    threads: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            cwd: PathBuf::from("."),
            positive: Vec::new(),
            negative: Vec::new(),
            simple_terms: Vec::new(),
            extensions: Vec::new(),
            wanted_type: WantedType::File,
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
enum SegmentMatcher {
    Literal(Vec<u8>),
    Prefix(Vec<u8>),
    Suffix(Vec<u8>),
    General(Vec<Token>),
}

impl SegmentMatcher {
    fn compile(tokens: Vec<Token>) -> Self {
        if let Some(literal) = tokens_as_literal(&tokens) {
            return Self::Literal(literal);
        }
        if matches!(tokens.last(), Some(Token::Star))
            && let Some(prefix) = tokens_as_literal(&tokens[..tokens.len() - 1])
        {
            return Self::Prefix(prefix);
        }
        if matches!(tokens.first(), Some(Token::Star))
            && let Some(suffix) = tokens_as_literal(&tokens[1..])
        {
            return Self::Suffix(suffix);
        }
        Self::General(tokens)
    }

    fn literal(&self) -> Option<&[u8]> {
        match self {
            Self::Literal(literal) => Some(literal),
            _ => None,
        }
    }

    fn matches(&self, name: &OsStr) -> bool {
        with_os_bytes(name, |bytes| match self {
            Self::Literal(literal) => bytes == literal,
            Self::Prefix(prefix) => bytes.starts_with(prefix),
            Self::Suffix(suffix) => bytes.ends_with(suffix),
            Self::General(tokens) => segment_matches(tokens, bytes),
        })
    }
}

fn tokens_as_literal(tokens: &[Token]) -> Option<Vec<u8>> {
    tokens
        .iter()
        .map(|token| match token {
            Token::Literal(byte) => Some(*byte),
            _ => None,
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PatternClass {
    Literal,
    SingleDirectory,
    FixedPrefixRecursive,
    UnboundedRecursive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Segment {
    GlobStar,
    Match(SegmentMatcher),
}

#[derive(Clone, Debug)]
struct Pattern {
    source: String,
    segments: Vec<Segment>,
    literal_prefix: Vec<Vec<u8>>,
    class: PatternClass,
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
                segments.push(Segment::Match(SegmentMatcher::compile(parse_segment(
                    raw, &source,
                )?)));
            }
        }
        if segments.is_empty() {
            return Err(AppError(format!("empty pattern: {source}")));
        }
        let literal_prefix: Vec<Vec<u8>> = segments
            .iter()
            .map_while(|segment| match segment {
                Segment::Match(matcher) => matcher.literal().map(<[u8]>::to_vec),
                Segment::GlobStar => None,
            })
            .collect();
        let class = if literal_prefix.len() == segments.len() {
            PatternClass::Literal
        } else if !segments
            .iter()
            .any(|segment| matches!(segment, Segment::GlobStar))
        {
            PatternClass::SingleDirectory
        } else if literal_prefix.is_empty() {
            PatternClass::UnboundedRecursive
        } else {
            PatternClass::FixedPrefixRecursive
        };
        Ok(Self {
            source,
            segments,
            literal_prefix,
            class,
        })
    }
}

#[derive(Clone, Debug)]
struct ProgramNode {
    edges: Vec<(Segment, usize)>,
    terminal: bool,
    subtree_terminal: bool,
}

impl ProgramNode {
    fn new() -> Self {
        Self {
            edges: Vec::new(),
            terminal: false,
            subtree_terminal: false,
        }
    }
}

#[derive(Clone, Debug)]
struct PatternProgram {
    nodes: Vec<ProgramNode>,
}

impl PatternProgram {
    fn compile(patterns: &[Pattern]) -> Self {
        let mut nodes = vec![ProgramNode::new()];
        for pattern in patterns {
            let mut node = 0;
            for segment in &pattern.segments {
                let existing = nodes[node]
                    .edges
                    .iter()
                    .find_map(|(edge, child)| (edge == segment).then_some(*child));
                node = if let Some(child) = existing {
                    child
                } else {
                    let child = nodes.len();
                    nodes.push(ProgramNode::new());
                    nodes[node].edges.push((segment.clone(), child));
                    child
                };
            }
            nodes[node].terminal = true;
            if matches!(pattern.segments.last(), Some(Segment::GlobStar)) {
                nodes[node].subtree_terminal = true;
            }
        }
        Self { nodes }
    }

    fn initial_states(&self) -> Vec<usize> {
        let mut states = vec![0];
        self.add_epsilon_closure(&mut states);
        states
    }

    fn advance(&self, states: &[usize], component: &OsStr) -> Vec<usize> {
        let mut next = Vec::with_capacity(states.len());
        for &node in states {
            for (segment, child) in &self.nodes[node].edges {
                match segment {
                    Segment::GlobStar => push_unique(&mut next, node),
                    Segment::Match(matcher) if matcher.matches(component) => {
                        push_unique(&mut next, *child);
                    }
                    Segment::Match(_) => {}
                }
            }
        }
        self.add_epsilon_closure(&mut next);
        next
    }

    fn matches_path(&self, components: &[&OsStr]) -> bool {
        let states = components
            .iter()
            .fold(self.initial_states(), |states, component| {
                self.advance(&states, component)
            });
        self.states_match(&states)
    }

    fn states_match(&self, states: &[usize]) -> bool {
        states.iter().any(|node| self.nodes[*node].terminal)
    }

    fn states_descendant_possible(&self, states: &[usize]) -> bool {
        states
            .iter()
            .any(|node| !self.nodes[*node].edges.is_empty())
    }

    fn states_exclude_subtree(&self, states: &[usize]) -> bool {
        states.iter().any(|node| self.nodes[*node].subtree_terminal)
    }

    fn add_epsilon_closure(&self, states: &mut Vec<usize>) {
        let mut cursor = 0;
        while cursor < states.len() {
            let node = states[cursor];
            for (segment, child) in &self.nodes[node].edges {
                if matches!(segment, Segment::GlobStar) {
                    push_unique(states, *child);
                }
            }
            cursor += 1;
        }
    }
}

fn push_unique(states: &mut Vec<usize>, node: usize) {
    if !states.contains(&node) {
        states.push(node);
    }
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

fn segment_matches(tokens: &[Token], bytes: &[u8]) -> bool {
    let mut token_index = 0;
    let mut byte_index = 0;
    let mut star_token = None;
    let mut star_byte = 0;

    while byte_index < bytes.len() {
        match tokens.get(token_index) {
            Some(Token::Star) => {
                star_token = Some(token_index);
                star_byte = byte_index;
                token_index += 1;
            }
            Some(token) if token_matches_byte(token, bytes[byte_index]) => {
                token_index += 1;
                byte_index += 1;
            }
            _ => {
                let Some(star) = star_token else {
                    return false;
                };
                star_byte += 1;
                byte_index = star_byte;
                token_index = star + 1;
            }
        }
    }
    tokens[token_index..]
        .iter()
        .all(|token| matches!(token, Token::Star))
}

fn token_matches_byte(token: &Token, byte: u8) -> bool {
    match token {
        Token::Literal(expected) => byte == *expected,
        Token::Question => true,
        Token::Class { negated, ranges } => {
            let contained = ranges
                .iter()
                .any(|(start, end)| *start <= byte && byte <= *end);
            contained != *negated
        }
        Token::Star => false,
    }
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

#[derive(Clone, Debug)]
struct QueryPlan {
    root: PathBuf,
    root_relative: PathBuf,
    positives: Vec<Pattern>,
    positive_program: PatternProgram,
    negative_program: PatternProgram,
    negatives: Vec<Pattern>,
    simple_terms: Vec<Vec<u8>>,
    extensions: Vec<Vec<u8>>,
    wanted_type: WantedType,
    hidden: bool,
    limit: Option<usize>,
    sort: bool,
    count: bool,
    gitignore: bool,
    json: bool,
    exec: Option<Vec<String>>,
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
        let positive_program = PatternProgram::compile(&positives);
        let negative_program = PatternProgram::compile(&negatives);
        Ok(Self {
            root,
            root_relative,
            positives,
            negatives,
            simple_terms: options.simple_terms.clone(),
            extensions: options.extensions.clone(),
            wanted_type: options.wanted_type,
            hidden: options.hidden,
            limit: options.limit,
            positive_program,
            negative_program,
            sort: options.sort,
            count: options.count,
            gitignore: options.gitignore,
            json: options.json,
            exec: options
                .exec
                .as_deref()
                .map(parse_command_line)
                .transpose()?,
        })
    }

    fn root_states(&self) -> (Vec<usize>, Vec<usize>) {
        let mut positive = self.positive_program.initial_states();
        let mut negative = self.negative_program.initial_states();
        for component in self.root_relative.components() {
            let Component::Normal(name) = component else {
                continue;
            };
            positive = self.positive_program.advance(&positive, name);
            negative = self.negative_program.advance(&negative, name);
        }
        (positive, negative)
    }

    fn states_match(&self, positive: &[usize], negative: &[usize], name: &OsStr) -> bool {
        self.positive_program.states_match(positive)
            && !self.negative_program.states_match(negative)
            && self.simple_matches(name)
    }

    fn simple_matches(&self, name: &OsStr) -> bool {
        self.simple_terms.is_empty()
            || with_os_bytes(name, |bytes| {
                self.simple_terms
                    .iter()
                    .any(|term| contains_bytes(bytes, term))
            })
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

#[derive(Clone, Debug)]
struct IgnoreRule {
    program: PatternProgram,
    base: PathBuf,
    negated: bool,
}

impl IgnoreRule {
    fn parse(line: &str, base: &Path) -> Option<Result<Self>> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let negated = line.starts_with('!');
        let mut source = line.strip_prefix('!').unwrap_or(line).to_owned();
        if source.ends_with('/') {
            source.pop();
            source.push_str("/**");
        }
        let anchored = source.starts_with('/');
        if anchored {
            source.remove(0);
        }
        let has_separator = source.contains('/');
        let compiled_source = if anchored || has_separator {
            source
        } else {
            format!("**/{source}")
        };
        Some(Pattern::compile(compiled_source).map(|pattern| Self {
            program: PatternProgram::compile(std::slice::from_ref(&pattern)),
            base: base.to_owned(),
            negated,
        }))
    }

    fn matches(&self, relative: &Path) -> bool {
        let Ok(local) = relative.strip_prefix(&self.base) else {
            return false;
        };
        self.program.matches_path(&normal_components(local))
    }
}

fn load_ignore_rules(path: &Path, base: &Path) -> io::Result<Vec<IgnoreRule>> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut rules = Vec::new();
    for line in contents.lines() {
        if let Some(rule) = IgnoreRule::parse(line, base) {
            rules.push(rule.map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.0))?);
        }
    }
    Ok(rules)
}

fn ignored_by_rules(rules: &[IgnoreRule], relative: &Path) -> (bool, bool) {
    let ignored = rules
        .iter()
        .filter(|rule| rule.matches(relative))
        .fold(false, |_, rule| !rule.negated);
    let may_reinclude = ignored && rules.iter().any(|rule| rule.negated);
    (ignored, may_reinclude)
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
    dirs_pruned_ignore: u64,
    entries_seen: u64,
    candidate_files: u64,
    metadata_calls: u64,
    matches: u64,
    errors: u64,
}

impl Stats {
    fn merge(&mut self, other: Stats) {
        self.dirs_seen += other.dirs_seen;
        self.dirs_opened += other.dirs_opened;
        self.dirs_pruned_positive += other.dirs_pruned_positive;
        self.dirs_pruned_exclude += other.dirs_pruned_exclude;
        self.dirs_pruned_ignore += other.dirs_pruned_ignore;
        self.entries_seen += other.entries_seen;
        self.candidate_files += other.candidate_files;
        self.metadata_calls += other.metadata_calls;
        self.matches += other.matches;
        self.errors += other.errors;
    }
}

struct Runner<'a, W: Write> {
    plan: &'a QueryPlan,
    writer: W,
    stats: Stats,
    output: Vec<PathBuf>,
    stopped: bool,
}

impl<'a, W: Write> Runner<'a, W> {
    fn new(plan: &'a QueryPlan, writer: W) -> Self {
        Self {
            plan,
            writer,
            stats: Stats::default(),
            output: Vec::new(),
            stopped: false,
        }
    }

    fn run(&mut self) -> io::Result<()> {
        if !self.plan.hidden && path_has_hidden_component(&self.plan.root_relative) {
            return Ok(());
        }
        let root_metadata = match fs::symlink_metadata(&self.plan.root) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        self.stats.metadata_calls += 1;
        let root_type = root_metadata.file_type();
        let (positive_states, negative_states) = self.plan.root_states();
        let root_name = self.plan.root.file_name().unwrap_or_default();
        if !root_type.is_dir() {
            let type_matches = match self.plan.wanted_type {
                WantedType::File => root_type.is_file(),
                WantedType::Dir => false,
                WantedType::Symlink => root_type.is_symlink(),
            };
            if type_matches
                && self
                    .plan
                    .states_match(&positive_states, &negative_states, root_name)
                && self.plan.extension_matches(root_name)
            {
                self.emit(self.plan.root_relative.clone())?;
            }
            return self.finish();
        }
        if !self.plan.root_relative.as_os_str().is_empty()
            && self.plan.wanted_type == WantedType::Dir
            && self
                .plan
                .states_match(&positive_states, &negative_states, root_name)
        {
            self.emit(self.plan.root_relative.clone())?;
        }
        self.visit_directory(
            self.plan.root.clone(),
            self.plan.root_relative.clone(),
            positive_states,
            negative_states,
            Vec::new(),
        )?;
        self.finish()
    }

    fn finish(&mut self) -> io::Result<()> {
        if self.plan.sort && !self.plan.count {
            self.output.sort_unstable();
            if let Some(limit) = self.plan.limit
                && self.output.len() > limit
            {
                self.output.truncate(limit);
                self.stopped = true;
            }
            for path in self.output.clone() {
                self.write_match(&path)?;
            }
        }
        self.writer.flush()
    }

    fn visit_directory(
        &mut self,
        absolute: PathBuf,
        relative: PathBuf,
        positive_states: Vec<usize>,
        negative_states: Vec<usize>,
        mut ignore_rules: Vec<IgnoreRule>,
    ) -> io::Result<()> {
        if self.stopped {
            return Ok(());
        }
        self.stats.dirs_seen += 1;
        if self.plan.gitignore {
            match load_ignore_rules(&absolute.join(".gitignore"), &relative) {
                Ok(rules) => ignore_rules.extend(rules),
                Err(error) => {
                    self.stats.errors += 1;
                    eprintln!(
                        "branchcut: cannot read {}: {error}",
                        absolute.join(".gitignore").display()
                    );
                }
            }
        }
        if self
            .plan
            .negative_program
            .states_exclude_subtree(&negative_states)
        {
            self.stats.dirs_pruned_exclude += 1;
            return Ok(());
        }
        if !self
            .plan
            .positive_program
            .states_descendant_possible(&positive_states)
        {
            self.stats.dirs_pruned_positive += 1;
            return Ok(());
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
            let child_relative = relative.join(&name);
            if self.plan.gitignore {
                let (ignored, may_reinclude) = ignored_by_rules(&ignore_rules, &child_relative);
                if ignored && !(file_type.is_dir() && may_reinclude) {
                    if file_type.is_dir() {
                        self.stats.dirs_pruned_ignore += 1;
                    }
                    continue;
                }
            }
            let child_positive = self.plan.positive_program.advance(&positive_states, &name);
            let child_negative = self.plan.negative_program.advance(&negative_states, &name);
            if file_type.is_dir() {
                if self.plan.wanted_type == WantedType::Dir
                    && self
                        .plan
                        .states_match(&child_positive, &child_negative, &name)
                {
                    self.emit(child_relative.clone())?;
                }
                self.visit_directory(
                    entry.path(),
                    child_relative,
                    child_positive,
                    child_negative,
                    ignore_rules.clone(),
                )?;
            } else {
                self.stats.candidate_files += 1;
                let type_matches = match self.plan.wanted_type {
                    WantedType::File => file_type.is_file(),
                    WantedType::Dir => false,
                    WantedType::Symlink => file_type.is_symlink(),
                };
                if type_matches
                    && self.plan.extension_matches(&name)
                    && self
                        .plan
                        .states_match(&child_positive, &child_negative, &name)
                {
                    self.emit(child_relative)?;
                }
            }
        }
        Ok(())
    }

    fn emit(&mut self, path: PathBuf) -> io::Result<()> {
        self.stats.matches += 1;
        if let Some(command) = &self.plan.exec {
            run_command(command, &path)?;
        }
        if self.plan.sort && !self.plan.count {
            self.output.push(path);
        } else if !self.plan.sort && !self.plan.count {
            self.write_match(&path)?;
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

    fn write_match(&mut self, path: &Path) -> io::Result<()> {
        if self.plan.json {
            writeln!(
                self.writer,
                "{{\"path\":\"{}\"}}",
                json_escape(&display_path(path))
            )
        } else {
            writeln!(self.writer, "{}", display_path(path))
        }
    }
}
#[derive(Clone)]
struct ParallelTask {
    absolute: PathBuf,
    relative: PathBuf,
    positive: Vec<usize>,
    negative: Vec<usize>,
    ignore_rules: Vec<IgnoreRule>,
}

#[derive(Default)]
struct ParallelQueueState {
    queued: usize,
    outstanding: usize,
    closed: bool,
    failure: Option<io::Error>,
}

enum ParallelPush {
    Queued,
    Full(ParallelTask),
}

struct ParallelQueue {
    locals: Vec<Mutex<VecDeque<ParallelTask>>>,
    state: Mutex<ParallelQueueState>,
    ready: Condvar,
    cancelled: AtomicBool,
    max_queued: usize,
}

impl ParallelQueue {
    fn new(workers: usize) -> Self {
        Self {
            locals: (0..workers).map(|_| Mutex::new(VecDeque::new())).collect(),
            state: Mutex::new(ParallelQueueState::default()),
            ready: Condvar::new(),
            cancelled: AtomicBool::new(false),
            max_queued: workers.saturating_mul(256).max(256),
        }
    }

    fn push(&self, worker: usize, task: ParallelTask) -> ParallelPush {
        let mut state = self.state.lock().expect("parallel queue state poisoned");
        if state.closed || self.cancelled.load(Ordering::Acquire) || state.queued >= self.max_queued
        {
            return ParallelPush::Full(task);
        }
        state.queued += 1;
        state.outstanding += 1;
        drop(state);
        self.locals[worker % self.locals.len()]
            .lock()
            .expect("parallel local queue poisoned")
            .push_back(task);
        self.ready.notify_one();
        ParallelPush::Queued
    }

    fn try_pop(&self, worker: usize) -> Option<ParallelTask> {
        if self.cancelled.load(Ordering::Acquire) {
            return None;
        }
        if let Some(task) = self.locals[worker]
            .lock()
            .expect("parallel local queue poisoned")
            .pop_back()
        {
            self.decrement_queued();
            return Some(task);
        }
        for offset in 1..self.locals.len() {
            let index = (worker + offset) % self.locals.len();
            if let Some(task) = self.locals[index]
                .lock()
                .expect("parallel local queue poisoned")
                .pop_front()
            {
                self.decrement_queued();
                return Some(task);
            }
        }
        None
    }

    fn pop(&self, worker: usize) -> Option<ParallelTask> {
        loop {
            if let Some(task) = self.try_pop(worker) {
                return Some(task);
            }
            let mut state = self.state.lock().expect("parallel queue state poisoned");
            while state.queued == 0 && !state.closed && !self.cancelled.load(Ordering::Acquire) {
                state = self
                    .ready
                    .wait(state)
                    .expect("parallel queue state poisoned while waiting");
            }
            if state.closed || self.cancelled.load(Ordering::Acquire) {
                return None;
            }
        }
    }

    fn decrement_queued(&self) {
        let mut state = self.state.lock().expect("parallel queue state poisoned");
        state.queued -= 1;
    }

    fn task_done(&self) {
        let mut state = self.state.lock().expect("parallel queue state poisoned");
        state.outstanding -= 1;
        if state.outstanding == 0 {
            state.closed = true;
            self.ready.notify_all();
        }
    }

    fn cancel(&self, error: io::Error) {
        self.cancelled.store(true, Ordering::Release);
        let mut state = self.state.lock().expect("parallel queue state poisoned");
        if state.failure.is_none() {
            state.failure = Some(error);
        }
        state.closed = true;
        self.ready.notify_all();
    }

    fn take_failure(&self) -> Option<io::Error> {
        self.state
            .lock()
            .expect("parallel queue state poisoned")
            .failure
            .take()
    }

    fn queued_count(&self) -> usize {
        self.state
            .lock()
            .expect("parallel queue state poisoned")
            .queued
    }

    fn close_if_idle(&self) {
        let mut state = self.state.lock().expect("parallel queue state poisoned");
        if state.outstanding == 0 {
            state.closed = true;
            self.ready.notify_all();
        }
    }
}

struct ParallelWorker<'a> {
    id: usize,
    plan: &'a QueryPlan,
    queue: Arc<ParallelQueue>,
    strict: bool,
    entries: Vec<fs::DirEntry>,
    output: Vec<PathBuf>,
    stats: Stats,
}

impl<'a> ParallelWorker<'a> {
    fn new(id: usize, plan: &'a QueryPlan, queue: Arc<ParallelQueue>, strict: bool) -> Self {
        Self {
            id,
            plan,
            queue,
            strict,
            entries: Vec::with_capacity(128),
            output: Vec::new(),
            stats: Stats::default(),
        }
    }

    fn run(mut self) -> ParallelWorkerResult {
        while let Some(task) = self.queue.pop(self.id) {
            if let Err(error) = self.process(task) {
                self.stats.errors += 1;
                if self.strict {
                    self.queue.cancel(error);
                }
            }
            self.queue.task_done();
        }
        ParallelWorkerResult {
            stats: self.stats,
            output: self.output,
        }
    }

    fn process(&mut self, task: ParallelTask) -> io::Result<()> {
        if self.queue.cancelled.load(Ordering::Acquire) {
            return Ok(());
        }
        self.stats.dirs_seen += 1;
        if self
            .plan
            .negative_program
            .states_exclude_subtree(&task.negative)
        {
            self.stats.dirs_pruned_exclude += 1;
            return Ok(());
        }
        if !self
            .plan
            .positive_program
            .states_descendant_possible(&task.positive)
        {
            self.stats.dirs_pruned_positive += 1;
            return Ok(());
        }
        let mut ignore_rules = task.ignore_rules.clone();
        if self.plan.gitignore {
            ignore_rules.extend(load_ignore_rules(
                &task.absolute.join(".gitignore"),
                &task.relative,
            )?);
        }
        self.entries.clear();
        for entry in fs::read_dir(&task.absolute)? {
            self.entries.push(entry?);
        }
        self.stats.dirs_opened += 1;
        self.stats.entries_seen += self.entries.len() as u64;
        let entries = std::mem::take(&mut self.entries);
        for entry in entries {
            if self.queue.cancelled.load(Ordering::Acquire) {
                break;
            }
            self.process_entry(&task, &ignore_rules, entry)?;
        }
        self.entries.clear();
        Ok(())
    }

    fn process_entry(
        &mut self,
        parent: &ParallelTask,
        ignore_rules: &[IgnoreRule],
        entry: fs::DirEntry,
    ) -> io::Result<()> {
        let name = entry.file_name();
        if !self.plan.hidden && is_hidden(&name) {
            return Ok(());
        }
        let file_type = entry.file_type()?;
        let relative = parent.relative.join(&name);
        if self.plan.gitignore {
            let (ignored, may_reinclude) = ignored_by_rules(ignore_rules, &relative);
            if ignored && !(file_type.is_dir() && may_reinclude) {
                if file_type.is_dir() {
                    self.stats.dirs_pruned_ignore += 1;
                }
                return Ok(());
            }
        }
        let positive = self.plan.positive_program.advance(&parent.positive, &name);
        let negative = self.plan.negative_program.advance(&parent.negative, &name);
        if file_type.is_dir() {
            if self.plan.wanted_type == WantedType::Dir
                && self.plan.states_match(&positive, &negative, &name)
            {
                self.output.push(relative.clone());
                self.stats.matches += 1;
            }
            let child = ParallelTask {
                absolute: entry.path(),
                relative,
                positive,
                negative,
                ignore_rules: ignore_rules.to_vec(),
            };
            if let ParallelPush::Full(child) = self.queue.push(self.id, child) {
                self.process(child)?;
            }
        } else {
            self.stats.candidate_files += 1;
            let type_matches = match self.plan.wanted_type {
                WantedType::File => file_type.is_file(),
                WantedType::Dir => false,
                WantedType::Symlink => file_type.is_symlink(),
            };
            if type_matches
                && self.plan.extension_matches(&name)
                && self.plan.states_match(&positive, &negative, &name)
            {
                self.output.push(relative);
                self.stats.matches += 1;
            }
        }
        Ok(())
    }
}

struct ParallelWorkerResult {
    stats: Stats,
    output: Vec<PathBuf>,
}

fn run_parallel(
    plan: &QueryPlan,
    requested_threads: usize,
    strict: bool,
) -> io::Result<(Stats, Vec<PathBuf>)> {
    let metadata = fs::symlink_metadata(&plan.root)?;
    if !metadata.file_type().is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "parallel traversal requires a directory root",
        ));
    }
    if !plan.hidden && path_has_hidden_component(&plan.root_relative) {
        return Ok((Stats::default(), Vec::new()));
    }
    let worker_count = requested_threads.clamp(1, 64);
    let queue = Arc::new(ParallelQueue::new(worker_count));
    let (positive, negative) = plan.root_states();
    let root = ParallelTask {
        absolute: plan.root.clone(),
        relative: plan.root_relative.clone(),
        positive,
        negative,
        ignore_rules: Vec::new(),
    };
    let mut main_worker = ParallelWorker::new(0, plan, Arc::clone(&queue), strict);
    if let Err(error) = main_worker.process(root) {
        main_worker.stats.errors += 1;
        if strict {
            queue.cancel(error);
        }
    }
    while worker_count > 1 && queue.queued_count() <= 2 && !queue.cancelled.load(Ordering::Acquire)
    {
        let Some(task) = queue.try_pop(0) else {
            break;
        };
        if let Err(error) = main_worker.process(task) {
            main_worker.stats.errors += 1;
            if strict {
                queue.cancel(error);
            }
        }
        queue.task_done();
    }
    queue.close_if_idle();
    let helper_count = if queue.queued_count() > 2 {
        worker_count.saturating_sub(1)
    } else {
        0
    };
    let results = thread::scope(|scope| {
        let handles: Vec<_> = (1..=helper_count)
            .map(|id| {
                let queue = Arc::clone(&queue);
                scope.spawn(move || ParallelWorker::new(id, plan, queue, strict).run())
            })
            .collect();
        let mut results = vec![main_worker.run()];
        for handle in handles {
            results.push(handle.join().expect("parallel worker panicked"));
        }
        results
    });
    if let Some(error) = queue.take_failure() {
        return Err(error);
    }
    let mut stats = Stats {
        metadata_calls: 1,
        ..Stats::default()
    };
    let mut output = Vec::new();
    for result in results {
        stats.merge(result.stats);
        output.extend(result.output);
    }
    Ok((stats, output))
}
fn is_hidden(name: &OsStr) -> bool {
    with_os_bytes(name, |bytes| {
        bytes.first() == Some(&b'.') && bytes != b"." && bytes != b".."
    })
}

fn path_has_hidden_component(path: &Path) -> bool {
    normal_components(path).into_iter().any(is_hidden)
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

fn write_path<W: Write>(writer: &mut W, json: bool, path: &Path) -> io::Result<()> {
    if json {
        writeln!(
            writer,
            "{{\"path\":\"{}\"}}",
            json_escape(&display_path(path))
        )
    } else {
        writeln!(writer, "{}", display_path(path))
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn json_escape(value: &str) -> String {
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

fn is_glob_pattern(value: &str) -> bool {
    value
        .bytes()
        .any(|byte| matches!(byte, b'*' | b'?' | b'[' | b'{' | b'!'))
}

fn run_command(command: &[String], path: &Path) -> io::Result<()> {
    let rendered = display_path(path);
    let mut process = Command::new(&command[0]);
    for argument in &command[1..] {
        process.arg(argument.replace("{}", &rendered));
    }
    let status = process.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "command exited unsuccessfully for {}: {status}",
            rendered
        )))
    }
}

fn parse_args() -> Result<Options> {
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
        "branchcut — compile the query, cut the tree\n\nUSAGE:\n  branchcut [PATTERN|SEARCH]\n  branchcut [OPTIONS]\n\nOPTIONS:\n  --glob PATTERN       Add a positive glob (repeatable)\n  --exclude PATTERN    Exclude a glob; subtree patterns are pruned\n  -e, --extension EXT  Match an extension (repeatable)\n  --type TYPE          Match file, dir, or symlink\n  --cwd PATH           Query root [default: .]\n  --hidden             Include hidden paths\n  --first              Stop after the first match\n  --limit N            Stop after N matches\n  --threads N          Use bounded parallel traversal workers (0 = auto)\n  --sort               Sort all matches before applying limits\n  --count              Print only the match count\n  --gitignore          Apply hierarchical .gitignore files\n  --json               Stream matching paths as JSON Lines\n  --exec COMMAND       Run a command template for each match; use {{}}\n  --strict             Fail if any filesystem entry cannot be read\n  --stats              Print traversal counters to stderr\n  --explain            Print the compiled plan without traversing\n  -h, --help           Print help\n  --version            Print version"
    );
}

fn parse_command_line(command: &str) -> Result<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in command.chars() {
        if escaped {
            word.push(character);
            escaped = false;
        } else if character == '\\' && quote != Some('\'') {
            escaped = true;
        } else if let Some(active) = quote {
            if character == active {
                quote = None;
            } else {
                word.push(character);
            }
        } else if character == '\'' || character == '"' {
            quote = Some(character);
        } else if character.is_whitespace() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(character);
        }
    }
    if escaped || quote.is_some() {
        return Err(AppError(
            "unterminated quote or escape in --exec command".to_owned(),
        ));
    }
    if !word.is_empty() {
        words.push(word);
    }
    if words.is_empty() {
        return Err(AppError("--exec command cannot be empty".to_owned()));
    }
    Ok(words)
}

fn print_explain(plan: &QueryPlan, threads: usize) {
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
        plan.limit.map_or("all matches".to_owned(), |limit| format!(
            "first {limit} matches"
        ))
    );
    if threads > 1 {
        println!("\nSTRATEGY\n  bounded parallel root-task traversal with buffered output");
    } else {
        println!(
            "\nSTRATEGY\n  sequential depth-first traversal with positive and exclusion pruning"
        );
    }
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
        print_explain(&plan, options.threads);
        return Ok(());
    }
    if options.threads > 1 && plan.root.is_dir() {
        if options.limit.is_some() || options.exec.is_some() {
            return Err(AppError(
                "--threads cannot be combined with --limit or --exec; use sequential mode for exact ordering"
                    .to_owned(),
            ));
        }
        let started = Instant::now();
        let (stats, mut output) = run_parallel(&plan, options.threads, options.strict)
            .map_err(|error| AppError(format!("parallel query failed: {error}")))?;
        if options.strict && stats.errors > 0 {
            return Err(AppError(format!(
                "query incomplete: {} filesystem error(s)",
                stats.errors
            )));
        }
        if options.sort {
            output.sort_unstable();
        }
        let stdout = io::stdout();
        let mut writer = io::BufWriter::with_capacity(64 * 1024, stdout.lock());
        if options.count {
            writeln!(writer, "{}", stats.matches)
                .map_err(|error| AppError(format!("output failed: {error}")))?;
        } else {
            for path in &output {
                write_path(&mut writer, options.json, path)
                    .map_err(|error| AppError(format!("output failed: {error}")))?;
            }
        }
        writer
            .flush()
            .map_err(|error| AppError(format!("output failed: {error}")))?;
        if options.stats {
            print_stats(&stats, started.elapsed());
        }
        return Ok(());
    }
    let started = Instant::now();
    let stdout = io::stdout();
    let mut runner = Runner::new(
        &plan,
        io::BufWriter::with_capacity(64 * 1024, stdout.lock()),
    );
    if let Err(error) = runner.run() {
        if error.kind() == io::ErrorKind::BrokenPipe {
            return Ok(());
        }
        return Err(AppError(format!("query failed: {error}")));
    }
    if options.strict && runner.stats.errors > 0 {
        return Err(AppError(format!(
            "query incomplete: {} filesystem error(s)",
            runner.stats.errors
        )));
    }
    if options.count {
        println!("{}", runner.stats.matches);
    }
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

    fn program_states(program: &PatternProgram, path: &Path) -> Vec<usize> {
        parts(path)
            .into_iter()
            .fold(program.initial_states(), |states, component| {
                program.advance(&states, component)
            })
    }

    fn matches(pattern: &str, path: &str) -> bool {
        let patterns = [Pattern::compile(pattern.to_owned()).unwrap()];
        let program = PatternProgram::compile(&patterns);
        program.states_match(&program_states(&program, Path::new(path)))
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
    fn classifies_patterns_and_specializes_common_segments() {
        let literal = Pattern::compile("src/main.rs".to_owned()).unwrap();
        let shallow = Pattern::compile("src/*.rs".to_owned()).unwrap();
        let fixed = Pattern::compile("src/**/*.rs".to_owned()).unwrap();
        let unbounded = Pattern::compile("**/*.rs".to_owned()).unwrap();
        let prefix = Pattern::compile("src/test*".to_owned()).unwrap();

        assert_eq!(literal.class, PatternClass::Literal);
        assert_eq!(shallow.class, PatternClass::SingleDirectory);
        assert_eq!(fixed.class, PatternClass::FixedPrefixRecursive);
        assert_eq!(unbounded.class, PatternClass::UnboundedRecursive);
        assert!(matches!(
            shallow.segments.last(),
            Some(Segment::Match(SegmentMatcher::Suffix(suffix))) if suffix == b".rs"
        ));
        assert!(matches!(
            prefix.segments.last(),
            Some(Segment::Match(SegmentMatcher::Prefix(value))) if value == b"test"
        ));
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
        let patterns = [Pattern::compile("packages/*/src/**/*.rs".to_owned()).unwrap()];
        let program = PatternProgram::compile(&patterns);
        assert!(
            program
                .states_descendant_possible(&program_states(&program, Path::new("packages/core")))
        );
        assert!(program.states_descendant_possible(&program_states(
            &program,
            Path::new("packages/core/src/deep")
        )));
        assert!(
            !program.states_descendant_possible(&program_states(&program, Path::new("unrelated")))
        );
    }

    #[test]
    fn shared_program_merges_common_pattern_segments() {
        let patterns = [
            Pattern::compile("src/**/*.rs".to_owned()).unwrap(),
            Pattern::compile("src/**/*.toml".to_owned()).unwrap(),
            Pattern::compile("src/**/test*.rs".to_owned()).unwrap(),
        ];
        let independent_nodes = 1 + patterns
            .iter()
            .map(|pattern| pattern.segments.len())
            .sum::<usize>();
        let program = PatternProgram::compile(&patterns);

        assert!(program.nodes.len() < independent_nodes);
        assert!(program.states_match(&program_states(&program, Path::new("src/deep/lib.rs"))));
        assert!(program.states_match(&program_states(&program, Path::new("src/Cargo.toml"))));
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

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            use std::time::{SystemTime, UNIX_EPOCH};
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = env::temp_dir().join(format!("branchcut-{}-{nonce}", std::process::id()));
            fs::create_dir_all(root.join("src/nested")).unwrap();
            fs::create_dir_all(root.join("target/debug")).unwrap();
            fs::create_dir_all(root.join(".hidden")).unwrap();
            fs::write(root.join("src/lib.rs"), b"").unwrap();
            fs::write(root.join("src/nested/config.toml"), b"").unwrap();
            fs::write(root.join("target/debug/generated.rs"), b"").unwrap();
            fs::write(root.join(".hidden/secret.rs"), b"").unwrap();
            Self(root)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn traversal_filters_extensions_hidden_paths_and_excluded_subtrees() {
        let fixture = Fixture::new();
        let options = Options {
            cwd: fixture.0.clone(),
            positive: vec!["**/*.{rs,toml}".to_owned()],
            negative: vec!["**/target/**".to_owned()],
            extensions: vec![b"rs".to_vec()],
            sort: true,
            ..Options::default()
        };
        let plan = QueryPlan::compile(&options).unwrap();
        let mut runner = Runner::new(&plan, Vec::new());
        runner.run().unwrap();

        assert_eq!(runner.output, [PathBuf::from("src/lib.rs")]);
        assert_eq!(runner.stats.matches, 1);
        assert_eq!(runner.stats.dirs_pruned_exclude, 1);
        assert_eq!(runner.stats.metadata_calls, 1);
    }

    #[test]
    fn traversal_honors_early_limit() {
        let fixture = Fixture::new();
        let options = Options {
            cwd: fixture.0.clone(),
            positive: vec!["**/*.rs".to_owned()],
            hidden: true,
            limit: Some(1),
            sort: true,
            ..Options::default()
        };
        let plan = QueryPlan::compile(&options).unwrap();
        let mut runner = Runner::new(&plan, Vec::new());
        runner.run().unwrap();

        assert_eq!(runner.output, [PathBuf::from(".hidden/secret.rs")]);
        assert_eq!(runner.stats.matches, 1);
        assert!(runner.stopped);
    }

    #[test]
    fn literal_root_preserves_type_and_hidden_semantics() {
        let fixture = Fixture::new();

        let default_dir = Options {
            cwd: fixture.0.clone(),
            positive: vec!["src".to_owned()],
            sort: true,
            ..Options::default()
        };
        let default_plan = QueryPlan::compile(&default_dir).unwrap();
        let mut default_runner = Runner::new(&default_plan, Vec::new());
        default_runner.run().unwrap();
        assert!(default_runner.output.is_empty());

        let explicit_dir = Options {
            wanted_type: WantedType::Dir,
            ..default_dir
        };
        let explicit_plan = QueryPlan::compile(&explicit_dir).unwrap();
        let mut explicit_runner = Runner::new(&explicit_plan, Vec::new());
        explicit_runner.run().unwrap();
        assert_eq!(explicit_runner.output, [PathBuf::from("src")]);

        let hidden = Options {
            cwd: fixture.0.clone(),
            positive: vec![".hidden/**/*.rs".to_owned()],
            sort: true,
            ..Options::default()
        };
        let hidden_plan = QueryPlan::compile(&hidden).unwrap();
        let mut hidden_runner = Runner::new(&hidden_plan, Vec::new());
        hidden_runner.run().unwrap();
        assert!(hidden_runner.output.is_empty());
        assert_eq!(hidden_runner.stats.dirs_opened, 0);
    }

    #[test]
    fn simple_search_treats_glob_metacharacters_literally() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("src/config[1].rs"), b"").unwrap();
        let options = Options {
            cwd: fixture.0.clone(),
            simple_terms: vec![b"config[1]".to_vec()],
            sort: true,
            ..Options::default()
        };
        let plan = QueryPlan::compile(&options).unwrap();
        let mut runner = Runner::new(&plan, Vec::new());
        runner.run().unwrap();

        assert_eq!(runner.output, [PathBuf::from("src/config[1].rs")]);
    }

    #[test]
    fn hierarchical_gitignore_uses_child_precedence_and_reinclusion() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("src/config[1].rs"), b"").unwrap();
        fs::write(
            fixture.0.join(".gitignore"),
            "target/\n*.toml\n!src/nested/config.toml\n",
        )
        .unwrap();
        fs::write(fixture.0.join("src/.gitignore"), "lib.rs\n").unwrap();
        let options = Options {
            cwd: fixture.0.clone(),
            positive: vec!["**/*".to_owned()],
            gitignore: true,
            sort: true,
            ..Options::default()
        };
        let plan = QueryPlan::compile(&options).unwrap();
        let mut runner = Runner::new(&plan, Vec::new());
        runner.run().unwrap();

        assert_eq!(
            runner.output,
            [
                PathBuf::from("src/config[1].rs"),
                PathBuf::from("src/nested/config.toml")
            ]
        );
        assert_eq!(runner.stats.matches, 2);
    }

    #[test]
    fn json_escaping_produces_valid_string_content() {
        assert_eq!(
            json_escape("quote\" slash\\ newline\n"),
            "quote\\\" slash\\\\ newline\\n"
        );
    }

    #[test]
    fn exec_command_parser_honors_quotes_and_placeholders() {
        assert_eq!(
            parse_command_line("tool --name 'hello world' {}").unwrap(),
            ["tool", "--name", "hello world", "{}"]
        );
        assert!(parse_command_line("tool 'unterminated").is_err());
    }

    #[test]
    fn count_mode_honors_limit_without_collecting_paths() {
        let fixture = Fixture::new();
        let options = Options {
            cwd: fixture.0.clone(),
            positive: vec!["**/*.rs".to_owned()],
            count: true,
            limit: Some(1),
            ..Options::default()
        };
        let plan = QueryPlan::compile(&options).unwrap();
        let mut runner = Runner::new(&plan, Vec::new());
        runner.run().unwrap();

        assert_eq!(runner.stats.matches, 1);
        assert!(runner.output.is_empty());
        assert!(runner.stopped);
    }

    struct BrokenWriter;

    impl Write for BrokenWriter {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn streaming_propagates_broken_pipe_without_panicking() {
        let fixture = Fixture::new();
        let options = Options {
            cwd: fixture.0.clone(),
            positive: vec!["**/*.rs".to_owned()],
            ..Options::default()
        };
        let plan = QueryPlan::compile(&options).unwrap();
        let mut runner = Runner::new(&plan, BrokenWriter);

        let error = runner.run().unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn traverses_deep_trees_without_losing_matches() {
        let fixture = Fixture::new();
        let mut directory = fixture.0.clone();
        let mut relative = PathBuf::new();
        for _ in 0..64 {
            directory.push("d");
            relative.push("d");
        }
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("leaf.rs"), b"").unwrap();
        relative.push("leaf.rs");

        let options = Options {
            cwd: fixture.0.clone(),
            positive: vec!["**/leaf.rs".to_owned()],
            sort: true,
            ..Options::default()
        };
        let plan = QueryPlan::compile(&options).unwrap();
        let mut runner = Runner::new(&plan, Vec::new());
        runner.run().unwrap();

        assert_eq!(runner.output, [relative]);
    }

    #[cfg(unix)]
    #[test]
    fn reports_symlinks_without_following_directory_links() {
        use std::os::unix::fs::symlink;

        let fixture = Fixture::new();
        symlink(fixture.0.join("src/lib.rs"), fixture.0.join("lib-link")).unwrap();
        symlink(fixture.0.join("src"), fixture.0.join("src-link")).unwrap();

        let links = Options {
            cwd: fixture.0.clone(),
            positive: vec!["**/*".to_owned()],
            wanted_type: WantedType::Symlink,
            sort: true,
            ..Options::default()
        };
        let link_plan = QueryPlan::compile(&links).unwrap();
        let mut link_runner = Runner::new(&link_plan, Vec::new());
        link_runner.run().unwrap();
        assert_eq!(
            link_runner.output,
            [PathBuf::from("lib-link"), PathBuf::from("src-link")]
        );

        let files = Options {
            wanted_type: WantedType::File,
            ..links
        };
        let file_plan = QueryPlan::compile(&files).unwrap();
        let mut file_runner = Runner::new(&file_plan, Vec::new());
        file_runner.run().unwrap();
        assert!(
            file_runner
                .output
                .iter()
                .all(|path| !path.starts_with("src-link"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn preserves_non_utf8_unix_paths() {
        use std::os::unix::ffi::OsStringExt;

        let fixture = Fixture::new();
        let name = OsString::from_vec(b"invalid-\\xff.rs".to_vec());
        fs::write(fixture.0.join(&name), b"").unwrap();
        let options = Options {
            cwd: fixture.0.clone(),
            positive: vec!["**/*.rs".to_owned()],
            sort: true,
            ..Options::default()
        };
        let plan = QueryPlan::compile(&options).unwrap();
        let mut runner = Runner::new(&plan, Vec::new());
        runner.run().unwrap();

        assert!(runner.output.contains(&PathBuf::from(name)));
    }

    #[test]
    fn parallel_traversal_matches_sequential_results() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join(".gitignore"), "target/\n").unwrap();
        let options = Options {
            cwd: fixture.0.clone(),
            positive: vec!["**/*.{rs,toml}".to_owned()],
            negative: vec!["**/target/**".to_owned()],
            gitignore: true,
            sort: true,
            ..Options::default()
        };
        let plan = QueryPlan::compile(&options).unwrap();
        let mut sequential = Runner::new(&plan, Vec::new());
        sequential.run().unwrap();
        let (stats, mut parallel) = run_parallel(&plan, 2, false).unwrap();
        parallel.sort_unstable();

        assert_eq!(parallel, sequential.output);
        assert_eq!(stats.matches as usize, parallel.len());
    }
}
