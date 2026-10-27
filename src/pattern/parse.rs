use super::Token;
use crate::error::{AppError, Result};
pub(crate) fn parse_segment(raw: &str, whole_pattern: &str) -> Result<Vec<Token>> {
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

pub(crate) fn parse_class(bytes: &[u8], whole_pattern: &str) -> Result<(Token, usize)> {
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
pub(crate) fn has_drive_prefix(pattern: &str) -> bool {
    pattern.as_bytes().get(1) == Some(&b':')
}

pub(crate) fn expand_braces(pattern: &str) -> Result<Vec<String>> {
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
