use std::ffi::OsStr;
use std::fmt;
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
    segments: Vec<Segment>,
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
        Ok(Self { segments })
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

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(pattern) = args.next() else {
        eprintln!("usage: branchcut PATTERN PATH");
        return;
    };
    let Some(path) = args.next() else {
        eprintln!("usage: branchcut PATTERN PATH");
        return;
    };
    let components: Vec<&OsStr> = path.split('/').map(OsStr::new).collect();
    match expand_braces(&pattern).and_then(|expanded| {
        expanded
            .into_iter()
            .map(Pattern::compile)
            .collect::<Result<Vec<_>>>()
    }) {
        Ok(patterns) => println!(
            "{}",
            patterns
                .iter()
                .any(|compiled| compiled.matches(&components))
        ),
        Err(error) => eprintln!("branchcut: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(pattern: &str, path: &str) -> bool {
        let components: Vec<&OsStr> = path.split('/').map(OsStr::new).collect();
        Pattern::compile(pattern.to_owned())
            .unwrap()
            .matches(&components)
    }

    #[test]
    fn matches_basic_segment_syntax() {
        assert!(matches("src/*.rs", "src/main.rs"));
        assert!(matches("src/ma?n.rs", "src/main.rs"));
        assert!(matches("src/[a-z]ain.rs", "src/main.rs"));
        assert!(matches("src/[!0-9]*.rs", "src/main.rs"));
        assert!(!matches("src/*.rs", "src/nested/main.rs"));
    }

    #[test]
    fn globstar_matches_zero_or_many_components() {
        assert!(matches("src/**/mod.rs", "src/mod.rs"));
        assert!(matches("src/**/mod.rs", "src/a/b/mod.rs"));
        assert!(!matches("src/**/mod.rs", "other/mod.rs"));
    }

    #[test]
    fn expands_common_brace_alternatives() {
        assert_eq!(
            expand_braces("**/*.{rs,toml}").unwrap(),
            ["**/*.rs", "**/*.toml"]
        );
        assert!(expand_braces("*.{rs}").is_err());
    }

    #[test]
    fn rejects_invalid_patterns() {
        assert!(Pattern::compile("src/[abc".to_owned()).is_err());
        assert!(Pattern::compile("../*.rs".to_owned()).is_err());
        assert!(Pattern::compile("/tmp/*.rs".to_owned()).is_err());
    }
}
