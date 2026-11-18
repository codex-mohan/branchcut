//! Component-local extended globs. Evaluation memoizes reachable byte offsets
//! rather than backtracking through alternative/repetition combinations.
use super::{Token, parse, token_matches_byte_with_case, token_matches_char_with_case};
use crate::error::{AppError, Result};
use crate::path::with_os_bytes;
use std::collections::{HashMap, VecDeque};
use std::ffi::OsStr;
use std::rc::Rc;

const MAX_PATTERN_BYTES: usize = 4096;
const MAX_NODES: usize = 512;
const MAX_NESTING: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExtGlob {
    nodes: Vec<Node>,
    root: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Node {
    Byte(Token),
    Scalar(char),
    Star,
    Sequence(Vec<usize>),
    Choice(Vec<usize>),
    Repeat { child: usize, minimum: usize },
    Not(usize),
}

impl ExtGlob {
    pub(crate) fn contains(raw: &str) -> bool {
        let bytes = raw.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'\\' => index += 2,
                b'[' => {
                    // A class may contain extglob punctuation literally.
                    index += 1;
                    if matches!(bytes.get(index), Some(b'!' | b'^')) {
                        index += 1;
                    }
                    if bytes.get(index) == Some(&b']') {
                        index += 1;
                    }
                    while index < bytes.len() && bytes[index] != b']' {
                        index += 1;
                    }
                    index += 1;
                }
                b'@' | b'?' | b'+' | b'*' | b'!' if bytes.get(index + 1) == Some(&b'(') => {
                    return true;
                }
                _ => index += 1,
            }
        }
        false
    }

    #[cfg(test)]
    pub(crate) fn compile(raw: &str, whole_pattern: &str) -> Result<Self> {
        Self::compile_with_unicode(raw, whole_pattern, false)
    }

    pub(crate) fn compile_with_unicode(
        raw: &str,
        whole_pattern: &str,
        unicode: bool,
    ) -> Result<Self> {
        if raw.len() > MAX_PATTERN_BYTES {
            return Err(AppError(format!(
                "extglob component exceeds {MAX_PATTERN_BYTES} bytes in pattern: {whole_pattern}"
            )));
        }
        if raw.contains('/') {
            return Err(AppError(format!(
                "extglobs must stay within one path component: {whole_pattern}"
            )));
        }
        let mut parser = Parser {
            bytes: raw.as_bytes(),
            index: 0,
            nodes: Vec::new(),
            whole_pattern,
            unicode,
        };
        let root = parser.sequence(false, 0)?;
        if parser.index != parser.bytes.len() {
            return Err(parser.error("unmatched closing extglob parenthesis"));
        }
        Ok(Self {
            nodes: parser.nodes,
            root,
        })
    }

    pub(crate) fn matches(&self, name: &OsStr, insensitive: bool) -> bool {
        self.matches_with_mode(name, insensitive, false)
    }

    pub(crate) fn matches_with_mode(&self, name: &OsStr, insensitive: bool, unicode: bool) -> bool {
        with_os_bytes(name, |bytes| {
            let use_unicode = unicode && std::str::from_utf8(bytes).is_ok();
            let input: Vec<u32> = if use_unicode {
                match std::str::from_utf8(bytes) {
                    Ok(text) => text.chars().map(u32::from).collect(),
                    Err(_) => bytes.iter().map(|byte| u32::from(*byte)).collect(),
                }
            } else {
                bytes.iter().map(|byte| u32::from(*byte)).collect()
            };
            let mut evaluator = Evaluator {
                nodes: &self.nodes,
                input: &input,
                insensitive,
                unicode: use_unicode,
                memo: HashMap::new(),
            };
            evaluator.ends(self.root, 0).contains(input.len())
        })
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    index: usize,
    nodes: Vec<Node>,
    whole_pattern: &'a str,
    unicode: bool,
}

impl Parser<'_> {
    fn error(&self, message: &str) -> AppError {
        AppError(format!(
            "{message} at component byte {} in pattern: {}",
            self.index, self.whole_pattern
        ))
    }

    fn push(&mut self, node: Node) -> Result<usize> {
        if self.nodes.len() >= MAX_NODES {
            return Err(self.error("extglob exceeds the 512-node complexity limit"));
        }
        let index = self.nodes.len();
        self.nodes.push(node);
        Ok(index)
    }

    fn sequence(&mut self, in_group: bool, depth: usize) -> Result<usize> {
        let mut sequence = Vec::new();
        while let Some(&byte) = self.bytes.get(self.index) {
            if byte == b')' || in_group && byte == b'|' {
                break;
            }
            let node = if matches!(byte, b'@' | b'?' | b'+' | b'*' | b'!')
                && self.bytes.get(self.index + 1) == Some(&b'(')
            {
                if depth >= MAX_NESTING {
                    return Err(self.error("extglob nesting exceeds 16 groups"));
                }
                self.index += 2;
                let mut choices = Vec::new();
                loop {
                    choices.push(self.sequence(true, depth + 1)?);
                    match self.bytes.get(self.index) {
                        Some(b'|') => self.index += 1,
                        Some(b')') => {
                            self.index += 1;
                            break;
                        }
                        _ => return Err(self.error("unterminated extglob group")),
                    }
                }
                let child = self.push(Node::Choice(choices))?;
                match byte {
                    b'@' => child,
                    b'?' => {
                        let empty = self.push(Node::Sequence(Vec::new()))?;
                        self.push(Node::Choice(vec![empty, child]))?
                    }
                    b'*' => self.push(Node::Repeat { child, minimum: 0 })?,
                    b'+' => self.push(Node::Repeat { child, minimum: 1 })?,
                    b'!' => self.push(Node::Not(child))?,
                    _ => unreachable!(),
                }
            } else {
                self.index += 1;
                match byte {
                    b'*' => self.push(Node::Star)?,
                    b'?' => self.push(Node::Byte(Token::Question))?,
                    b'[' => {
                        let (token, consumed_bytes) = if self.unicode {
                            let tail = std::str::from_utf8(&self.bytes[self.index - 1..])
                                .map_err(|_| self.error("invalid UTF-8 in Unicode extglob"))?;
                            let chars: Vec<char> = tail.chars().collect();
                            let (token, consumed_chars) =
                                parse::parse_unicode_class(&chars, self.whole_pattern)?;
                            let byte_count = chars
                                .iter()
                                .take(consumed_chars)
                                .map(|character| character.len_utf8())
                                .sum();
                            (token, byte_count)
                        } else {
                            parse::parse_class(&self.bytes[self.index - 1..], self.whole_pattern)?
                        };
                        self.index = self.index - 1 + consumed_bytes;
                        self.push(Node::Byte(token))?
                    }
                    b'\\' => {
                        let Some(&escaped) = self.bytes.get(self.index) else {
                            return Err(self.error("trailing escape in extglob"));
                        };
                        if escaped >= 0x80 {
                            let tail = std::str::from_utf8(&self.bytes[self.index..])
                                .ok()
                                .and_then(|s| s.chars().next());
                            if let Some(character) = tail {
                                self.index += character.len_utf8();
                                self.push(Node::Scalar(character))?
                            } else {
                                self.index += 1;
                                self.push(Node::Byte(Token::Literal(escaped)))?
                            }
                        } else {
                            self.index += 1;
                            self.push(Node::Byte(Token::Literal(escaped)))?
                        }
                    }
                    literal if literal >= 0x80 => {
                        let start = self.index - 1;
                        if let Some(character) = std::str::from_utf8(&self.bytes[start..])
                            .ok()
                            .and_then(|s| s.chars().next())
                        {
                            self.index = start + character.len_utf8();
                            self.push(Node::Scalar(character))?
                        } else {
                            self.push(Node::Byte(Token::Literal(literal)))?
                        }
                    }
                    literal => self.push(Node::Byte(Token::Literal(literal)))?,
                }
            };
            sequence.push(node);
        }
        self.push(Node::Sequence(sequence))
    }
}

#[derive(Clone)]
struct Ends {
    words: Vec<u64>,
    length: usize,
}

impl Ends {
    fn new(length: usize) -> Self {
        Self {
            words: vec![0; length.div_ceil(64)],
            length,
        }
    }

    fn insert(&mut self, position: usize) -> bool {
        let mask = 1 << (position % 64);
        let word = &mut self.words[position / 64];
        let was_present = *word & mask != 0;
        *word |= mask;
        !was_present
    }

    fn contains(&self, position: usize) -> bool {
        self.words[position / 64] & (1 << (position % 64)) != 0
    }

    fn union(&mut self, other: &Self) {
        for (left, right) in self.words.iter_mut().zip(&other.words) {
            *left |= right;
        }
    }

    fn positions(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.length).filter(|&position| self.contains(position))
    }
}

struct Evaluator<'a> {
    nodes: &'a [Node],
    input: &'a [u32],
    insensitive: bool,
    unicode: bool,
    memo: HashMap<(usize, usize), Rc<Ends>>,
}

impl Evaluator<'_> {
    fn ends(&mut self, node: usize, start: usize) -> Rc<Ends> {
        if let Some(known) = self.memo.get(&(node, start)) {
            return Rc::clone(known);
        }
        let mut ends = Ends::new(self.input.len() + 1);
        match self.nodes[node].clone() {
            Node::Byte(token) => {
                if self.input.get(start).is_some_and(|&value| {
                    if self.unicode {
                        char::from_u32(value).is_some_and(|character| {
                            token_matches_char_with_case(&token, character, self.insensitive)
                        })
                    } else {
                        value <= u32::from(u8::MAX)
                            && token_matches_byte_with_case(&token, value as u8, self.insensitive)
                    }
                }) {
                    ends.insert(start + 1);
                }
            }
            Node::Scalar(expected) => {
                if self.unicode {
                    if let Some(&value) = self.input.get(start) {
                        let actual = char::from_u32(value).unwrap_or('\0');
                        if actual == expected
                            || (self.insensitive && scalar_lower(actual) == scalar_lower(expected))
                        {
                            ends.insert(start + 1);
                        }
                    }
                } else {
                    let mut buffer = [0; 4];
                    let encoded = expected.encode_utf8(&mut buffer);
                    let wanted = encoded.as_bytes();
                    if self.input.len() >= start + wanted.len()
                        && wanted
                            .iter()
                            .enumerate()
                            .all(|(offset, byte)| self.input[start + offset] == u32::from(*byte))
                    {
                        ends.insert(start + wanted.len());
                    }
                }
            }
            Node::Star => {
                for position in start..=self.input.len() {
                    ends.insert(position);
                }
            }
            Node::Sequence(children) => {
                ends.insert(start);
                for child in children {
                    let mut next = Ends::new(self.input.len() + 1);
                    for position in ends.positions() {
                        next.union(&self.ends(child, position));
                    }
                    ends = next;
                }
            }
            Node::Choice(children) => {
                for child in children {
                    ends.union(&self.ends(child, start));
                }
            }
            Node::Repeat { child, minimum } => {
                let mut pending = VecDeque::new();
                if minimum == 0 {
                    ends.insert(start);
                    pending.push_back(start);
                } else {
                    let first = self.ends(child, start);
                    for position in first.positions() {
                        ends.insert(position);
                        pending.push_back(position);
                    }
                }
                while let Some(position) = pending.pop_front() {
                    let next = self.ends(child, position);
                    for position in next.positions() {
                        if ends.insert(position) {
                            pending.push_back(position);
                        }
                    }
                }
            }
            Node::Not(child) => {
                let excluded = self.ends(child, start);
                for position in start..=self.input.len() {
                    if !excluded.contains(position) {
                        ends.insert(position);
                    }
                }
            }
        }
        let ends = Rc::new(ends);
        self.memo.insert((node, start), Rc::clone(&ends));
        ends
    }
}

fn scalar_lower(character: char) -> char {
    let mut lowered = character.to_lowercase();
    let first = lowered.next().unwrap_or(character);
    if lowered.next().is_none() {
        first
    } else {
        character
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(pattern: &str, name: &str) -> bool {
        ExtGlob::compile(pattern, pattern)
            .unwrap()
            .matches(OsStr::new(name), false)
    }

    #[test]
    fn positive_extglobs_compose_with_prefixes_suffixes_and_nesting() {
        for (pattern, yes, no) in [
            ("@(foo|bar).rs", "foo.rs", "baz.rs"),
            ("file?(test).rs", "file.rs", "filetests.rs"),
            ("file?(test).rs", "filetest.rs", "filetesttest.rs"),
            ("+(ab|cd).rs", "abcdab.rs", ".rs"),
            ("*(ab|cd).rs", ".rs", "ac.rs"),
            ("@(foo|@(bar|baz))", "baz", "qux"),
            ("@([a-z]?|[0-9]*)", "a7", "A7"),
            ("*(a|aa)b", "aaab", "aaac"),
        ] {
            assert!(matches(pattern, yes), "{pattern} should match {yes}");
            assert!(!matches(pattern, no), "{pattern} should reject {no}");
        }
    }

    #[test]
    fn whole_component_negation_uses_complete_alternative_languages() {
        assert!(!matches("!(foo|bar)", "foo"));
        assert!(!matches("!(foo|bar)", "bar"));
        assert!(matches("!(foo|bar)", "foo.rs"));
        assert!(matches("!(foo|bar)", "baz"));
        assert!(!matches("!(a*)", "abc"));
        assert!(matches("!(a*)", "bbc"));
        assert!(matches("!(!(foo))", "foo"));
        assert!(!matches("!(!(foo))", "bar"));
    }

    #[test]
    fn insensitive_extglobs_fold_ascii_and_negation_after_matching() {
        let matcher = ExtGlob::compile("@(Foo|Bar).RS", "@(Foo|Bar).RS").unwrap();
        assert!(matcher.matches(OsStr::new("foo.rs"), true));
        assert!(!matcher.matches(OsStr::new("foo.rs"), false));
        let matcher = ExtGlob::compile("!(Foo)", "!(Foo)").unwrap();
        assert!(!matcher.matches(OsStr::new("foo"), true));
        assert!(matcher.matches(OsStr::new("foo"), false));
        let matcher = ExtGlob::compile("@(Ä|Z)", "@(Ä|Z)").unwrap();
        assert!(!matcher.matches(OsStr::new("ä"), true));
        assert!(matcher.matches_with_mode(OsStr::new("ä"), true, true));
        let scalar = ExtGlob::compile("?x", "?x").unwrap();
        assert!(scalar.matches_with_mode(OsStr::new("éx"), false, true));
        assert!(!scalar.matches_with_mode(OsStr::new("éx"), false, false));
    }

    #[test]
    fn repetitions_of_empty_languages_terminate() {
        assert!(matches("*(a|)", "aaa"));
        assert!(matches("+(a|)", ""));
        assert!(matches("*(@(a|))b", "aaab"));
        assert!(!matches(
            "*(a|aa|aaa|aaaa|aaaaa)b",
            &format!("{}c", "a".repeat(200))
        ));
    }

    #[test]
    fn generated_small_names_match_explicit_finite_language_sets() {
        use std::collections::HashSet;
        let mut repeated: HashSet<String> = [String::new()].into_iter().collect();
        let mut frontier = vec![String::new()];
        for _ in 0..6 {
            let mut next = Vec::new();
            for prefix in frontier {
                for suffix in ["a", "bb"] {
                    let word = format!("{prefix}{suffix}");
                    if word.len() <= 6 && repeated.insert(word.clone()) {
                        next.push(word);
                    }
                }
            }
            frontier = next;
        }
        for length in 0..=6 {
            for bits in 0..1 << length {
                let word: String = (0..length)
                    .map(|position| {
                        if bits & (1 << position) == 0 {
                            'a'
                        } else {
                            'b'
                        }
                    })
                    .collect();
                assert_eq!(
                    matches("*(a|bb)", &word),
                    repeated.contains(&word),
                    "{word}"
                );
                assert_eq!(
                    matches("+(a|bb)", &word),
                    !word.is_empty() && repeated.contains(&word),
                    "{word}"
                );
                assert_eq!(
                    matches("?(a|bb)", &word),
                    ["", "a", "bb"].contains(&word.as_str())
                );
                assert_eq!(
                    matches("!(a|bb)", &word),
                    !["a", "bb"].contains(&word.as_str())
                );
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn byte_wildcards_preserve_non_utf8_names() {
        use std::os::unix::ffi::OsStrExt;
        let pattern = ExtGlob::compile("@(a|b)?", "@(a|b)?").unwrap();
        assert!(pattern.matches(OsStr::from_bytes(b"a\xff"), false));
        assert!(!pattern.matches(OsStr::from_bytes(b"c\xff"), false));
    }

    #[test]
    fn detection_ignores_classes_and_escaped_punctuation() {
        assert!(ExtGlob::contains("a@(b|c)"));
        assert!(!ExtGlob::contains("[@(]"));
        assert!(!ExtGlob::contains(r"\@(literal)"));
    }

    #[test]
    fn malformed_and_over_budget_patterns_report_errors() {
        for pattern in ["@(foo", "@(foo))", "@(a/b)", "@([a-)"] {
            assert!(ExtGlob::compile(pattern, pattern).is_err(), "{pattern}");
        }
        let nested = format!("{}a{}", "@(".repeat(17), ")".repeat(17));
        assert!(ExtGlob::compile(&nested, &nested).is_err());
        let nodes = format!("@({})", "a".repeat(513));
        assert!(ExtGlob::compile(&nodes, &nodes).is_err());
        let bytes = "a".repeat(4097);
        assert!(ExtGlob::compile(&bytes, &bytes).is_err());
    }
}
