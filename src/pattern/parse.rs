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

pub(crate) fn parse_unicode_segment(raw: &str, whole_pattern: &str) -> Result<Vec<Token>> {
    let characters: Vec<char> = raw.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        match characters[index] {
            '*' => {
                if !matches!(tokens.last(), Some(Token::Star)) {
                    tokens.push(Token::Star);
                }
                index += 1;
            }
            '?' => {
                tokens.push(Token::Question);
                index += 1;
            }
            '[' => {
                let (class, consumed) = parse_unicode_class(&characters[index..], whole_pattern)?;
                tokens.push(class);
                index += consumed;
            }
            '\\' if index + 1 < characters.len() => {
                for byte in characters[index + 1].to_string().bytes() {
                    tokens.push(Token::Literal(byte));
                }
                index += 2;
            }
            character => {
                for byte in character.to_string().bytes() {
                    tokens.push(Token::Literal(byte));
                }
                index += 1;
            }
        }
    }
    Ok(tokens)
}

pub(crate) fn parse_unicode_class(
    characters: &[char],
    whole_pattern: &str,
) -> Result<(Token, usize)> {
    let mut index = 1;
    let negated = matches!(characters.get(index), Some('!' | '^'));
    if negated {
        index += 1;
    }
    let mut ranges = Vec::new();
    if characters.get(index) == Some(&']') {
        ranges.push((']', ']'));
        index += 1;
    }
    while let Some(&character) = characters.get(index) {
        if character == ']' && !ranges.is_empty() {
            return Ok((Token::ScalarClass { negated, ranges }, index + 1));
        }
        if character == '[' && characters.get(index + 1) == Some(&':') {
            let end = characters[index + 2..]
                .windows(2)
                .position(|pair| pair == [':', ']'])
                .map(|offset| offset + index + 2)
                .ok_or_else(|| {
                    AppError(format!(
                        "unterminated POSIX character class in pattern: {whole_pattern}"
                    ))
                })?;
            let name: String = characters[index + 2..end].iter().collect();
            let byte_ranges = ascii_named_class_ranges(name.as_bytes()).map_err(|_| {
                AppError(format!(
                    "unknown POSIX character class {name:?} in pattern: {whole_pattern}"
                ))
            })?;
            ranges.extend(
                byte_ranges
                    .into_iter()
                    .map(|(start, end)| (char::from(start), char::from(end))),
            );
            index = end + 2;
            continue;
        }
        let (start, after_start) = unicode_class_character(characters, index, whole_pattern)?;
        index = after_start;
        if characters.get(index) == Some(&'-')
            && characters.get(index + 1).is_some_and(|end| *end != ']')
        {
            let (end, after_end) = unicode_class_character(characters, index + 1, whole_pattern)?;
            if start > end {
                return Err(AppError(format!(
                    "invalid descending character range in pattern: {whole_pattern}"
                )));
            }
            ranges.push((start, end));
            index = after_end;
        } else {
            ranges.push((start, start));
        }
    }
    Err(AppError(format!(
        "unterminated character class in pattern: {whole_pattern}"
    )))
}

fn unicode_class_character(
    characters: &[char],
    index: usize,
    whole_pattern: &str,
) -> Result<(char, usize)> {
    match characters.get(index) {
        Some('\\') => characters
            .get(index + 1)
            .copied()
            .map(|character| (character, index + 2))
            .ok_or_else(|| {
                AppError(format!(
                    "trailing escape in character class in pattern: {whole_pattern}"
                ))
            }),
        Some(character) => Ok((*character, index + 1)),
        None => Err(AppError(format!(
            "unterminated character class in pattern: {whole_pattern}"
        ))),
    }
}

/// Ignore-file patterns are data, not query syntax errors. A malformed class
/// does not match; it must not reject the rest of the ignore file.
pub(crate) fn parse_ignore_segment(raw: &str, whole_pattern: &str) -> Vec<Token> {
    parse_ignore_segment_bytes(raw.as_bytes(), whole_pattern)
}

pub(crate) fn parse_ignore_segment_bytes(bytes: &[u8], whole_pattern: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'[' => match parse_class_impl(&bytes[index..], whole_pattern, true) {
                Ok((class, consumed)) => {
                    tokens.push(class);
                    index += consumed;
                }
                Err(_) => {
                    return vec![Token::Class {
                        negated: false,
                        ranges: Vec::new(),
                    }];
                }
            },
            b'\\' => {
                let Some(&quoted) = bytes.get(index + 1) else {
                    return vec![Token::Class {
                        negated: false,
                        ranges: Vec::new(),
                    }];
                };
                tokens.push(Token::Literal(quoted));
                index += 2;
            }
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
            byte => {
                tokens.push(Token::Literal(byte));
                index += 1;
            }
        }
    }
    // An empty/dot component cannot occur in a native descendant path. Keep
    // it unsatisfiable instead of letting path normalization erase it.
    if bytes.is_empty() || bytes == b"." || bytes == b".." {
        vec![Token::Class {
            negated: false,
            ranges: Vec::new(),
        }]
    } else {
        tokens
    }
}

pub(crate) fn parse_class(bytes: &[u8], whole_pattern: &str) -> Result<(Token, usize)> {
    parse_class_impl(bytes, whole_pattern, false)
}

fn parse_class_impl(bytes: &[u8], whole_pattern: &str, ignore: bool) -> Result<(Token, usize)> {
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
        if byte == b'[' && bytes.get(index + 1) == Some(&b':') {
            let Some((class_ranges, consumed)) = parse_named_class(&bytes[index..])? else {
                return Err(AppError(format!(
                    "unterminated POSIX character class in pattern: {whole_pattern}"
                )));
            };
            ranges.extend(class_ranges);
            index += consumed;
            continue;
        }
        let start = if byte == b'\\' {
            index += 1;
            let Some(&quoted) = bytes.get(index) else {
                break;
            };
            quoted
        } else {
            byte
        };
        index += 1;
        if bytes.get(index) == Some(&b'-') {
            let Some(&end) = bytes.get(index + 1) else {
                break;
            };
            if end == b']' {
                ranges.push((start, start));
                ranges.push((b'-', b'-'));
                index += 1;
                continue;
            }
            index += 1;
            let end = if end == b'\\' {
                index += 1;
                let Some(&quoted) = bytes.get(index) else {
                    break;
                };
                quoted
            } else {
                end
            };
            if start > end {
                if ignore {
                    // Git retains the range's starting literal even when its
                    // endpoint precedes it; the descending interval is empty.
                    ranges.push((start, start));
                } else {
                    return Err(AppError(format!(
                        "invalid descending character range in pattern: {whole_pattern}"
                    )));
                }
            } else {
                ranges.push((start, end));
            }
            index += 1;
        } else {
            ranges.push((start, start));
        }
    }
    Err(AppError(format!(
        "unterminated character class in pattern: {whole_pattern}"
    )))
}

/// Parse the POSIX form `[:name:]` beginning at its opening `[`. The returned
/// ranges are the ASCII byte members of the class, compressed into intervals
/// so the matcher can keep using its ordinary character-class representation.
type NamedClass = (Vec<(u8, u8)>, usize);
fn parse_named_class(bytes: &[u8]) -> Result<Option<NamedClass>> {
    let Some(end) = bytes.windows(2).position(|pair| pair == b":]") else {
        return Ok(None);
    };
    let name = &bytes[2..end];
    let ranges = ascii_named_class_ranges(name)?;
    Ok(Some((ranges, end + 2)))
}

fn ascii_named_class_ranges(name: &[u8]) -> Result<Vec<(u8, u8)>> {
    let predicate: fn(u8) -> bool = match name {
        b"alnum" => |byte| byte.is_ascii_alphanumeric(),
        b"alpha" => |byte| byte.is_ascii_alphabetic(),
        b"blank" => |byte| matches!(byte, b' ' | b'\t'),
        b"cntrl" => |byte| byte.is_ascii_control(),
        b"digit" => |byte| byte.is_ascii_digit(),
        b"graph" => |byte| byte.is_ascii_graphic(),
        b"lower" => |byte| byte.is_ascii_lowercase(),
        b"print" => |byte| byte.is_ascii_graphic() || byte == b' ',
        b"punct" => |byte| byte.is_ascii_punctuation(),
        b"space" => |byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c),
        b"upper" => |byte| byte.is_ascii_uppercase(),
        b"xdigit" => |byte| byte.is_ascii_hexdigit(),
        _ => {
            return Err(AppError(format!(
                "unknown POSIX character class {:?}",
                String::from_utf8_lossy(name)
            )));
        }
    };
    let mut ranges = Vec::new();
    let mut current = None;
    for value in 0u16..=u8::MAX as u16 {
        let byte = value as u8;
        if predicate(byte) {
            match &mut current {
                Some((_, end)) if *end + 1 == byte => *end = byte,
                Some(_) => {
                    ranges.push(current.take().expect("current range exists"));
                    current = Some((byte, byte));
                }
                None => current = Some((byte, byte)),
            }
        } else if let Some(range) = current.take() {
            ranges.push(range);
        }
    }
    if let Some(range) = current {
        ranges.push(range);
    }
    Ok(ranges)
}

#[cfg(test)]
mod named_class_tests {
    use super::*;
    use crate::pattern::segment_matches_with_case;

    fn matches(pattern: &str, candidate: &[u8], insensitive: bool) -> bool {
        let tokens = parse_segment(pattern, pattern).unwrap();
        segment_matches_with_case(&tokens, candidate, insensitive)
    }

    #[test]
    fn supports_all_ascii_posix_classes() {
        let cases: &[(&str, &[u8], &[u8])] = &[
            ("[[:alnum:]]", b"az09", b"-_"),
            ("[[:alpha:]]", b"azAZ", b"09"),
            ("[[:blank:]]", b" \t", b"\n"),
            ("[[:cntrl:]]", b"\0\x7f", b"a"),
            ("[[:digit:]]", b"09", b"a"),
            ("[[:graph:]]", b"!~", b" \n"),
            ("[[:lower:]]", b"az", b"AZ"),
            ("[[:print:]]", b" ~", b"\x7f"),
            ("[[:punct:]]", b"!_", b"a0"),
            ("[[:space:]]", b" \t\n\r\x0b\x0c", b"a"),
            ("[[:upper:]]", b"AZ", b"az"),
            ("[[:xdigit:]]", b"09aAfF", b"g"),
        ];
        for (pattern, accepted, rejected) in cases {
            for byte in *accepted {
                assert!(
                    matches(pattern, &[*byte], false),
                    "{pattern:?} should match {byte:#04x}"
                );
            }
            for byte in *rejected {
                assert!(
                    !matches(pattern, &[*byte], false),
                    "{pattern:?} should reject {byte:#04x}"
                );
            }
        }
    }

    #[test]
    fn named_classes_compose_inside_ranges_and_negation() {
        assert!(matches("[a[:digit:]_]", b"7", false));
        assert!(matches("[a[:digit:]_]", b"_", false));
        assert!(!matches("[a[:digit:]_]", b"z", false));
        assert!(!matches("[![:digit:]]", b"7", false));
        assert!(matches("[![:digit:]]", b"a", false));
        assert!(matches("[[:lower:]]", b"A", true));
    }

    #[test]
    fn unknown_or_unterminated_named_classes_are_diagnostic() {
        let unknown = parse_segment("[[:greek:]]", "[[:greek:]]").unwrap_err();
        assert!(
            unknown
                .to_string()
                .contains("unknown POSIX character class")
        );
        let unterminated = parse_segment("[[:digit]", "[[:digit]").unwrap_err();
        assert!(
            unterminated
                .to_string()
                .contains("unterminated POSIX character class")
        );
    }
}
pub(crate) fn has_drive_prefix(pattern: &str) -> bool {
    pattern.as_bytes().get(1) == Some(&b':')
}

const MAX_BRACE_EXPANSIONS: usize = 256;
const MAX_BRACE_DEPTH: usize = 8;
const MAX_EXPANDED_BYTES: usize = 1024 * 1024;

pub(crate) fn expand_braces(pattern: &str) -> Result<Vec<String>> {
    expand_braces_mode(pattern, false)
}

pub(crate) fn expand_braces_escaped(pattern: &str) -> Result<Vec<String>> {
    expand_braces_mode(pattern, true)
}

fn expand_braces_mode(pattern: &str, backslash_escapes: bool) -> Result<Vec<String>> {
    let mut expanded = Vec::new();
    let mut total_bytes = 0;
    expand_braces_into(
        pattern,
        0,
        &mut total_bytes,
        &mut expanded,
        backslash_escapes,
    )?;
    Ok(expanded)
}

fn expand_braces_into(
    pattern: &str,
    depth: usize,
    total_bytes: &mut usize,
    expanded: &mut Vec<String>,
    backslash_escapes: bool,
) -> Result<()> {
    let Some((open, close, separators)) = find_brace_group(pattern, backslash_escapes)? else {
        if expanded.len() >= MAX_BRACE_EXPANSIONS {
            return Err(brace_error(pattern, "brace expansion exceeds 256 results"));
        }
        *total_bytes = (*total_bytes)
            .checked_add(pattern.len())
            .filter(|size| *size <= MAX_EXPANDED_BYTES)
            .ok_or_else(|| brace_error(pattern, "expanded patterns exceed 1 MiB"))?;
        expanded.push(pattern.to_owned());
        return Ok(());
    };
    if depth >= MAX_BRACE_DEPTH {
        return Err(brace_error(pattern, "brace nesting exceeds 8 levels"));
    }

    let body = &pattern[open + 1..close];
    let alternatives = if separators.is_empty() {
        parse_brace_range(body, pattern)?
    } else {
        if separators.len() + 1 > MAX_BRACE_EXPANSIONS {
            return Err(brace_error(pattern, "brace group exceeds 256 alternatives"));
        }
        let mut alternatives = Vec::with_capacity(separators.len() + 1);
        let mut start = open + 1;
        for separator in separators {
            alternatives.push(&pattern[start..separator]);
            start = separator + 1;
        }
        alternatives.push(&pattern[start..close]);
        if alternatives
            .iter()
            .any(|alternative| alternative.is_empty())
        {
            return Err(brace_error(pattern, "brace alternatives must not be empty"));
        }
        alternatives.into_iter().map(str::to_owned).collect()
    };

    for alternative in alternatives {
        let combined = format!(
            "{}{}{}",
            &pattern[..open],
            alternative,
            &pattern[close + 1..]
        );
        expand_braces_into(
            &combined,
            depth + 1,
            total_bytes,
            expanded,
            backslash_escapes,
        )?;
    }
    Ok(())
}

fn find_brace_group(
    pattern: &str,
    backslash_escapes: bool,
) -> Result<Option<(usize, usize, Vec<usize>)>> {
    let mut open = None;
    let mut depth = 0usize;
    let mut separators = Vec::new();
    let mut in_class = false;
    let bytes = pattern.as_bytes();
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if backslash_escapes && byte == b'\\' {
            index += if index + 1 < bytes.len() { 2 } else { 1 };
            continue;
        }
        if in_class {
            if byte == b']' {
                in_class = false;
            }
            index += 1;
            continue;
        }
        match byte {
            b'[' => in_class = true,
            b'{' => {
                if open.is_none() {
                    open = Some(index);
                }
                depth += 1;
                if depth > MAX_BRACE_DEPTH {
                    return Err(brace_error(pattern, "brace nesting exceeds 8 levels"));
                }
            }
            b',' if depth == 1 => separators.push(index),
            b'}' if depth == 0 => {
                return Err(brace_error(pattern, "unmatched closing brace"));
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(Some((
                        open.expect("an open brace precedes its close"),
                        index,
                        separators,
                    )));
                }
            }
            _ => {}
        }
        index += 1;
    }
    if depth != 0 {
        return Err(brace_error(pattern, "unterminated brace group"));
    }
    Ok(None)
}

fn parse_brace_range(body: &str, pattern: &str) -> Result<Vec<String>> {
    let fields: Vec<_> = body.split("..").collect();
    if !(2..=3).contains(&fields.len()) {
        return Err(brace_error(
            pattern,
            "brace groups require alternatives or a range such as {1..5}",
        ));
    }
    if fields.iter().any(|field| field.is_empty()) {
        return Err(brace_error(pattern, "brace range values must not be empty"));
    }
    let step = if fields.len() == 3 {
        fields[2]
            .parse::<u64>()
            .ok()
            .filter(|step| *step > 0)
            .ok_or_else(|| brace_error(pattern, "brace range step must be a positive integer"))?
    } else {
        1
    };

    if fields[0].len() == 1
        && fields[1].len() == 1
        && fields[0].as_bytes()[0].is_ascii_alphabetic()
        && fields[1].as_bytes()[0].is_ascii_alphabetic()
    {
        let start = fields[0].as_bytes()[0];
        let end = fields[1].as_bytes()[0];
        if start.is_ascii_lowercase() != end.is_ascii_lowercase() {
            return Err(brace_error(pattern, "brace letter range must use one case"));
        }
        let distance = start.abs_diff(end) as u64;
        let count = distance / step + 1;
        check_range_count(u128::from(count), pattern)?;
        let direction = if start <= end { 1_i16 } else { -1_i16 };
        return Ok((0..count)
            .map(|index| {
                let value = i16::from(start) + direction * (index * step) as i16;
                char::from(value as u8).to_string()
            })
            .collect());
    }

    let start = fields[0]
        .parse::<i64>()
        .map_err(|_| brace_error(pattern, "brace range must use integers or ASCII letters"))?;
    let end = fields[1]
        .parse::<i64>()
        .map_err(|_| brace_error(pattern, "brace range must use integers or ASCII letters"))?;
    let distance = (i128::from(end) - i128::from(start)).unsigned_abs();
    let count = distance / u128::from(step) + 1;
    check_range_count(count, pattern)?;
    let padded = fields[..2]
        .iter()
        .any(|field| range_digits(field).len() > 1 && range_digits(field).starts_with('0'));
    let width = fields[..2]
        .iter()
        .map(|field| range_digits(field).len())
        .max()
        .unwrap_or(0);
    let direction = if start <= end { 1_i128 } else { -1_i128 };
    Ok((0..count as u64)
        .map(|index| {
            let value = i128::from(start) + direction * i128::from(step) * i128::from(index);
            if padded {
                if value < 0 {
                    format!("-{:0width$}", value.unsigned_abs())
                } else {
                    format!("{:0width$}", value)
                }
            } else {
                value.to_string()
            }
        })
        .collect::<Vec<_>>())
}

fn check_range_count(count: u128, pattern: &str) -> Result<()> {
    if count == 0 || count > MAX_BRACE_EXPANSIONS as u128 {
        Err(brace_error(pattern, "brace range exceeds 256 alternatives"))
    } else {
        Ok(())
    }
}

fn range_digits(field: &str) -> &str {
    field.trim_start_matches(['-', '+'])
}

fn brace_error(pattern: &str, reason: &str) -> AppError {
    AppError(format!("{reason} in pattern: {pattern}"))
}

#[cfg(test)]
mod brace_tests {
    use super::expand_braces;

    #[test]
    fn expands_multiple_nested_groups_in_source_order() {
        assert_eq!(
            expand_braces("{src/{lib,main},tests/{unit,integration}}.{rs,ts}").unwrap(),
            [
                "src/lib.rs",
                "src/lib.ts",
                "src/main.rs",
                "src/main.ts",
                "tests/unit.rs",
                "tests/unit.ts",
                "tests/integration.rs",
                "tests/integration.ts",
            ]
        );
    }

    #[test]
    fn expands_inclusive_stepped_numeric_and_letter_ranges() {
        assert_eq!(
            expand_braces("file{01..05..2}.rs").unwrap(),
            ["file01.rs", "file03.rs", "file05.rs"]
        );
        assert_eq!(
            expand_braces("file{3..1}.rs").unwrap(),
            ["file3.rs", "file2.rs", "file1.rs"]
        );
        assert_eq!(
            expand_braces("file{a..e..2}.rs").unwrap(),
            ["filea.rs", "filec.rs", "filee.rs"]
        );
        assert_eq!(
            expand_braces("file{E..A..2}.rs").unwrap(),
            ["fileE.rs", "fileC.rs", "fileA.rs"]
        );
        assert_eq!(
            expand_braces("file{-03..-01}.rs").unwrap(),
            ["file-03.rs", "file-02.rs", "file-01.rs"]
        );
    }

    #[test]
    fn ignores_brace_characters_inside_classes() {
        assert_eq!(expand_braces("file[{,}].rs").unwrap(), ["file[{,}].rs"]);
    }

    #[test]
    fn rejects_malformed_or_unbounded_brace_expansions() {
        for pattern in [
            "file{a..}.rs",
            "file{}.rs",
            "file{1..3..0}.rs",
            "file{a..Z}.rs",
            "file{01..999}.rs",
            "file{a,b,c",
            "filea,b}",
            "file{a,,b}",
            "file{a,{b,{c,{d,{e,{f,{g,{h,{i,j}}}}}}}}}",
            "file{a,b}{c,d}{e,f}{g,h}{i,j}{k,l}{m,n}{o,p}{q,r}",
        ] {
            assert!(expand_braces(pattern).is_err(), "accepted {pattern:?}");
        }
    }

    #[test]
    fn expansion_byte_budget_is_enforced() {
        let giant = "a".repeat(600_000);
        let pattern = format!("{{x,y}}{giant}");
        assert!(expand_braces(&pattern).is_err());
    }
}
