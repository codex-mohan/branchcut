mod parse;
mod program;
use crate::error::{AppError, Result};
use crate::path::with_os_bytes;
pub(crate) use parse::expand_braces;
pub(crate) use program::PatternProgram;
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SegmentMatcher {
    Literal(Vec<u8>),
    Prefix(Vec<u8>),
    Suffix(Vec<u8>),
    General(Vec<Token>),
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

    pub(crate) fn matches(&self, name: &OsStr) -> bool {
        with_os_bytes(name, |bytes| match self {
            Self::Literal(literal) => bytes == literal,
            Self::Prefix(prefix) => bytes.starts_with(prefix),
            Self::Suffix(suffix) => bytes.ends_with(suffix),
            Self::General(tokens) => segment_matches(tokens, bytes),
        })
    }
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
        let normalized = source.replace('\\', "/");
        if normalized.starts_with('/') || parse::has_drive_prefix(&normalized) {
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
                segments.push(Segment::Match(SegmentMatcher::compile(
                    parse::parse_segment(raw, &source)?,
                )));
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

pub(crate) fn token_matches_byte(token: &Token, byte: u8) -> bool {
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
