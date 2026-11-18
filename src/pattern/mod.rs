#[cfg(test)]
mod case_tests;
mod extglob;
mod parse;
mod program;
use crate::error::{AppError, Result};
use crate::path::with_os_bytes;
use extglob::ExtGlob;
pub(crate) use parse::{expand_braces, expand_braces_escaped};
pub(crate) use program::{PathMatchScratch, PatternProgram};
use std::ffi::OsStr;
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Token {
    Literal(u8),
    Star,
    Question,
    Class {
        negated: bool,
        ranges: Vec<(u8, u8)>,
    },
    ScalarClass {
        negated: bool,
        ranges: Vec<(char, char)>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SegmentMatcher {
    Literal(Vec<u8>),
    Prefix(Vec<u8>),
    Suffix(Vec<u8>),
    General(Vec<Token>),
    ExtGlob(ExtGlob),
}

impl SegmentMatcher {
    pub(crate) fn compile(tokens: Vec<Token>) -> Self {
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

    pub(crate) fn literal(&self) -> Option<&[u8]> {
        match self {
            Self::Literal(literal) => Some(literal),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn matches(&self, name: &OsStr) -> bool {
        self.matches_with_case(name, false)
    }

    #[cfg(test)]
    pub(crate) fn matches_with_case(&self, name: &OsStr, insensitive: bool) -> bool {
        self.matches_with_mode(name, insensitive, false)
    }

    pub(crate) fn matches_with_mode(&self, name: &OsStr, insensitive: bool, unicode: bool) -> bool {
        with_os_bytes(name, |bytes| match self {
            _ if unicode && std::str::from_utf8(bytes).is_ok() => match self {
                Self::Literal(literal) => unicode_literal_matches(literal, bytes, insensitive),
                Self::Prefix(prefix) => unicode_affix_matches(prefix, bytes, insensitive, true),
                Self::Suffix(suffix) => unicode_affix_matches(suffix, bytes, insensitive, false),
                Self::General(tokens) => segment_matches_unicode(tokens, bytes, insensitive),
                Self::ExtGlob(pattern) => pattern.matches_with_mode(name, insensitive, true),
            },
            Self::Literal(literal) => bytes_equal(literal, bytes, insensitive),
            Self::Prefix(prefix) => {
                bytes.starts_with(prefix)
                    || (insensitive
                        && bytes.len() >= prefix.len()
                        && bytes_equal(prefix, &bytes[..prefix.len()], true))
            }
            Self::Suffix(suffix) => {
                bytes.ends_with(suffix)
                    || (insensitive
                        && bytes.len() >= suffix.len()
                        && bytes_equal(suffix, &bytes[bytes.len() - suffix.len()..], true))
            }
            Self::General(tokens) => {
                if insensitive {
                    segment_matches_with_case(tokens, bytes, true)
                } else {
                    segment_matches(tokens, bytes)
                }
            }
            Self::ExtGlob(pattern) => pattern.matches(name, insensitive),
        })
    }
}

fn char_equal(left: char, right: char, insensitive: bool) -> bool {
    left == right || (insensitive && simple_lower(left) == simple_lower(right))
}

pub(crate) fn simple_lower(character: char) -> char {
    let mut lowered = character.to_lowercase();
    let first = lowered.next().unwrap_or(character);
    if lowered.next().is_none() {
        first
    } else {
        character
    }
}

fn unicode_literal_matches(pattern: &[u8], candidate: &[u8], insensitive: bool) -> bool {
    let (Ok(pattern), Ok(candidate)) =
        (std::str::from_utf8(pattern), std::str::from_utf8(candidate))
    else {
        return false;
    };
    let pattern: Vec<char> = pattern.chars().collect();
    let candidate: Vec<char> = candidate.chars().collect();
    pattern.len() == candidate.len()
        && pattern
            .iter()
            .zip(candidate)
            .all(|(a, b)| char_equal(*a, b, insensitive))
}

fn unicode_affix_matches(
    pattern: &[u8],
    candidate: &[u8],
    insensitive: bool,
    prefix: bool,
) -> bool {
    let (Ok(pattern), Ok(candidate)) =
        (std::str::from_utf8(pattern), std::str::from_utf8(candidate))
    else {
        return false;
    };
    let pattern: Vec<char> = pattern.chars().collect();
    let candidate: Vec<char> = candidate.chars().collect();
    if pattern.len() > candidate.len() {
        return false;
    }
    let slice = if prefix {
        &candidate[..pattern.len()]
    } else {
        &candidate[candidate.len() - pattern.len()..]
    };
    pattern
        .iter()
        .zip(slice)
        .all(|(a, b)| char_equal(*a, *b, insensitive))
}

enum UnicodeToken {
    Literal(char),
    Star,
    Question,
    Class {
        negated: bool,
        ranges: Vec<(char, char)>,
    },
}

fn segment_matches_unicode(tokens: &[Token], candidate: &[u8], insensitive: bool) -> bool {
    let Ok(candidate_text) = std::str::from_utf8(candidate) else {
        return segment_matches_with_case(tokens, candidate, insensitive);
    };
    let mut units = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        match &tokens[index] {
            Token::Star => {
                units.push(UnicodeToken::Star);
                index += 1;
            }
            Token::Question => {
                units.push(UnicodeToken::Question);
                index += 1;
            }
            Token::Class { negated, ranges } => {
                units.push(UnicodeToken::Class {
                    negated: *negated,
                    ranges: ranges
                        .iter()
                        .map(|(start, end)| (char::from(*start), char::from(*end)))
                        .collect(),
                });
                index += 1;
            }
            Token::ScalarClass { negated, ranges } => {
                units.push(UnicodeToken::Class {
                    negated: *negated,
                    ranges: ranges.clone(),
                });
                index += 1;
            }
            Token::Literal(_) => {
                let start = index;
                while index < tokens.len() && matches!(tokens[index], Token::Literal(_)) {
                    index += 1;
                }
                let bytes: Vec<u8> = tokens[start..index]
                    .iter()
                    .filter_map(|token| {
                        if let Token::Literal(byte) = token {
                            Some(*byte)
                        } else {
                            None
                        }
                    })
                    .collect();
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    units.extend(text.chars().map(UnicodeToken::Literal));
                } else {
                    return segment_matches_with_case(tokens, candidate, insensitive);
                }
            }
        }
    }
    let chars: Vec<char> = candidate_text.chars().collect();
    let mut current = vec![false; chars.len() + 1];
    current[0] = true;
    for unit in units {
        let mut next = vec![false; chars.len() + 1];
        match unit {
            UnicodeToken::Star => {
                let mut seen = false;
                for i in 0..=chars.len() {
                    seen |= current[i];
                    next[i] = seen;
                }
            }
            UnicodeToken::Literal(expected) => {
                for i in 0..chars.len() {
                    if current[i] && char_equal(expected, chars[i], insensitive) {
                        next[i + 1] = true;
                    }
                }
            }
            UnicodeToken::Question => {
                for i in 0..chars.len() {
                    if current[i] {
                        next[i + 1] = true;
                    }
                }
            }
            UnicodeToken::Class { negated, ranges } => {
                for i in 0..chars.len() {
                    if current[i] {
                        let contained = ranges.iter().any(|(start, end)| {
                            *start <= chars[i] && chars[i] <= *end
                                || (insensitive
                                    && chars[i].is_ascii_alphabetic()
                                    && [
                                        chars[i].to_ascii_lowercase(),
                                        chars[i].to_ascii_uppercase(),
                                    ]
                                    .into_iter()
                                    .any(|variant| *start <= variant && variant <= *end))
                        });
                        if contained != negated {
                            next[i + 1] = true;
                        }
                    }
                }
            }
        }
        current = next;
    }
    current[chars.len()]
}

fn bytes_equal(left: &[u8], right: &[u8], insensitive: bool) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| *left == *right || (insensitive && ascii_case_pair(*left, *right)))
}

fn ascii_case_pair(left: u8, right: u8) -> bool {
    left.is_ascii_alphabetic() && left.eq_ignore_ascii_case(&right)
}

pub(crate) fn tokens_as_literal(tokens: &[Token]) -> Option<Vec<u8>> {
    tokens
        .iter()
        .map(|token| match token {
            Token::Literal(byte) => Some(*byte),
            _ => None,
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PatternClass {
    Literal,
    SingleDirectory,
    FixedPrefixRecursive,
    UnboundedRecursive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Segment {
    GlobStar,
    Match(SegmentMatcher),
}

#[derive(Clone, Debug)]
pub(crate) struct Pattern {
    pub(crate) source: String,
    pub(crate) segments: Vec<Segment>,
    pub(crate) literal_prefix: Vec<Vec<u8>>,
    pub(crate) class: PatternClass,
}

impl Pattern {
    pub(crate) fn compile(source: String) -> Result<Self> {
        Self::compile_impl_with_options(source, true, true, false)
    }

    pub(crate) fn compile_escaped(source: String) -> Result<Self> {
        Self::compile_impl_with_options(source, true, false, false)
    }

    pub(crate) fn compile_unicode(source: String, escaped: bool) -> Result<Self> {
        Self::compile_impl_with_options(source, true, !escaped, true)
    }

    pub(crate) fn compile_ignore(source: String) -> Result<Self> {
        Self::compile_impl_with_options(source, false, false, false)
    }

    pub(crate) fn compile_ignore_bytes(source: Vec<u8>) -> Result<Self> {
        if let Ok(text) = String::from_utf8(source.clone()) {
            return Self::compile_ignore(text);
        }
        let display = String::from_utf8_lossy(&source).into_owned();
        if source.first() == Some(&b'/') {
            return Err(AppError(format!(
                "pattern must be relative to --cwd: {display}"
            )));
        }
        let mut components: Vec<Vec<u8>> = Vec::new();
        let mut current = Vec::new();
        let mut index = 0;
        while index < source.len() {
            match source[index] {
                b'/' => {
                    components.push(std::mem::take(&mut current));
                    index += 1;
                }
                b'\\' if source.get(index + 1) == Some(&b'/') => {
                    components.push(std::mem::take(&mut current));
                    index += 2;
                }
                byte => {
                    current.push(byte);
                    index += 1;
                }
            }
        }
        components.push(current);
        let segments: Vec<Segment> = components
            .iter()
            .map(|part| {
                if part == b"**" {
                    Segment::GlobStar
                } else {
                    Segment::Match(SegmentMatcher::compile(parse::parse_ignore_segment_bytes(
                        part, &display,
                    )))
                }
            })
            .collect();
        if segments.is_empty() {
            return Err(AppError(format!("empty pattern: {display}")));
        }
        Self::from_segments(display, segments)
    }

    fn from_segments(source: String, segments: Vec<Segment>) -> Result<Self> {
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

    fn compile_impl_with_options(
        source: String,
        query_syntax: bool,
        windows_separators: bool,
        unicode: bool,
    ) -> Result<Self> {
        // Query paths accept Windows separators. Gitignore uses forward slashes
        // and reserves backslashes for quoting literal pattern characters.
        let normalized = if query_syntax && windows_separators {
            source.replace('\\', "/")
        } else if query_syntax {
            source.clone()
        } else {
            // A quoted slash is still a path separator in Git wildmatch.
            // Preserve pairs of backslashes, which quote a literal backslash.
            let mut normalized = String::with_capacity(source.len());
            for character in source.chars() {
                if character == '/'
                    && normalized
                        .bytes()
                        .rev()
                        .take_while(|byte| *byte == b'\\')
                        .count()
                        % 2
                        == 1
                {
                    normalized.pop();
                }
                normalized.push(character);
            }
            normalized
        };
        if normalized.starts_with('/') || (query_syntax && parse::has_drive_prefix(&normalized)) {
            return Err(AppError(format!(
                "pattern must be relative to --cwd: {source}"
            )));
        }
        let mut segments = Vec::new();
        for raw in normalized
            .split('/')
            .filter(|part| !query_syntax || (!part.is_empty() && *part != "."))
        {
            if query_syntax && raw == ".." {
                return Err(AppError(format!(
                    "pattern cannot traverse above --cwd: {source}"
                )));
            }
            if raw == "**" {
                if !matches!(segments.last(), Some(Segment::GlobStar)) {
                    segments.push(Segment::GlobStar);
                }
            } else if query_syntax && ExtGlob::contains(raw) {
                segments.push(Segment::Match(SegmentMatcher::ExtGlob(
                    ExtGlob::compile_with_unicode(raw, &source, unicode)?,
                )));
            } else if !query_syntax {
                segments.push(Segment::Match(SegmentMatcher::compile(
                    parse::parse_ignore_segment(raw, &source),
                )));
            } else {
                let tokens = if unicode {
                    parse::parse_unicode_segment(raw, &source)?
                } else {
                    parse::parse_segment(raw, &source)?
                };
                segments.push(Segment::Match(SegmentMatcher::compile(tokens)));
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

pub(crate) fn segment_matches(tokens: &[Token], bytes: &[u8]) -> bool {
    segment_matches_with_case(tokens, bytes, false)
}

pub(crate) fn segment_matches_with_case(tokens: &[Token], bytes: &[u8], insensitive: bool) -> bool {
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
            Some(token)
                if if insensitive {
                    token_matches_byte_with_case(token, bytes[byte_index], true)
                } else {
                    token_matches_byte(token, bytes[byte_index])
                } =>
            {
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

pub(crate) fn token_matches_byte(token: &Token, byte: u8) -> bool {
    token_matches_byte_with_case(token, byte, false)
}

fn token_matches_byte_with_case(token: &Token, byte: u8, insensitive: bool) -> bool {
    match token {
        Token::Literal(expected) => {
            byte == *expected || (insensitive && ascii_case_pair(byte, *expected))
        }
        Token::Question => true,
        Token::Class { negated, ranges } => {
            let contained = ranges.iter().any(|(start, end)| {
                (*start <= byte && byte <= *end)
                    || (insensitive
                        && byte.is_ascii_alphabetic()
                        && [byte.to_ascii_lowercase(), byte.to_ascii_uppercase()]
                            .into_iter()
                            .any(|variant| *start <= variant && variant <= *end))
            });
            contained != *negated
        }
        Token::ScalarClass { negated, ranges } => {
            let character = char::from(byte);
            let contained = ranges.iter().any(|(start, end)| {
                *start <= character && character <= *end
                    || (insensitive
                        && character.is_ascii_alphabetic()
                        && [
                            character.to_ascii_lowercase(),
                            character.to_ascii_uppercase(),
                        ]
                        .into_iter()
                        .any(|variant| *start <= variant && variant <= *end))
            });
            contained != *negated
        }
        Token::Star => false,
    }
}

pub(crate) fn token_matches_char_with_case(
    token: &Token,
    character: char,
    insensitive: bool,
) -> bool {
    match token {
        Token::Literal(byte) => {
            character == char::from(*byte)
                || (insensitive && character.is_ascii() && ascii_case_pair(character as u8, *byte))
        }
        Token::Question => true,
        Token::Class { negated, ranges } => {
            let contained = character.is_ascii()
                && ranges.iter().any(|(start, end)| {
                    *start <= character as u8 && (character as u8) <= *end
                        || (insensitive
                            && character.is_ascii_alphabetic()
                            && [
                                character.to_ascii_lowercase(),
                                character.to_ascii_uppercase(),
                            ]
                            .into_iter()
                            .any(|variant| *start <= variant as u8 && (variant as u8) <= *end))
                });
            contained != *negated
        }
        Token::ScalarClass { negated, ranges } => {
            let contained = ranges.iter().any(|(start, end)| {
                *start <= character && character <= *end
                    || (insensitive
                        && character.is_ascii_alphabetic()
                        && [
                            character.to_ascii_lowercase(),
                            character.to_ascii_uppercase(),
                        ]
                        .into_iter()
                        .any(|variant| *start <= variant && variant <= *end))
            });
            contained != *negated
        }
        Token::Star => false,
    }
}
