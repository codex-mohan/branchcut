use branchcut::{ContentMatchOptions, ContentMatcher, ContentSpan};
use regex::bytes::{Regex, RegexBuilder};
use std::{io, process::ExitCode, sync::Arc};

// Bound the amount of compiled state a pattern set can keep per thread.
const MAX_PATTERNS: usize = 128;
const REGEX_COMPILED_LIMIT: usize = 2 * 1024 * 1024;
const REGEX_CACHE_LIMIT: usize = 128 * 1024;
// Pass one additional span so the scanner can report its per-line limit.
const SPAN_OVERFLOW: usize = 65_537;

fn main() -> ExitCode {
    branchcut::cli::run_with_content_matcher(compile)
}

struct RegexMatcher(Vec<Regex>);

impl ContentMatcher for RegexMatcher {
    fn is_match(&self, input: &[u8]) -> bool {
        self.0.iter().any(|regex| regex.is_match(input))
    }

    fn find_spans(&self, line: &[u8], all: bool, out: &mut Vec<ContentSpan>) {
        out.clear();
        if all {
            for (pattern, regex) in self.0.iter().enumerate() {
                let room = SPAN_OVERFLOW - out.len();
                out.extend(regex.find_iter(line).take(room).map(|found| ContentSpan {
                    start: found.start(),
                    end: found.end(),
                    pattern,
                }));
                if out.len() == SPAN_OVERFLOW {
                    return;
                }
            }
            out.sort_unstable_by_key(span_order);
        } else if let Some(first) = self
            .0
            .iter()
            .enumerate()
            .filter_map(|(pattern, regex)| {
                regex.find(line).map(|found| ContentSpan {
                    start: found.start(),
                    end: found.end(),
                    pattern,
                })
            })
            .min_by_key(span_order)
        {
            out.push(first);
        }
    }
}

fn span_order(span: &ContentSpan) -> (usize, usize, usize) {
    (span.start, span.pattern, span.end)
}

fn compile(
    patterns: &[Vec<u8>],
    options: ContentMatchOptions,
) -> io::Result<Arc<dyn ContentMatcher>> {
    if patterns.len() > MAX_PATTERNS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("regex mode accepts at most {MAX_PATTERNS} patterns"),
        ));
    }
    patterns
        .iter()
        .enumerate()
        .map(|(index, bytes)| compile_term(index, bytes, &options))
        .collect::<io::Result<Vec<_>>>()
        .map(|patterns| Arc::new(RegexMatcher(patterns)) as Arc<dyn ContentMatcher>)
}

fn compile_term(index: usize, bytes: &[u8], options: &ContentMatchOptions) -> io::Result<Regex> {
    let label = index + 1;
    let pattern = std::str::from_utf8(bytes).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("regex pattern {label} is not valid UTF-8: {error}"),
        )
    })?;
    let build = |source| {
        RegexBuilder::new(source)
            .unicode(options.unicode_case)
            .case_insensitive(options.case_insensitive)
            .size_limit(REGEX_COMPILED_LIMIT)
            .dfa_size_limit(REGEX_CACHE_LIMIT)
            .nest_limit(128)
            .build()
    };
    let boundaries = match (options.whole_line, options.whole_word) {
        (true, _) => Some((r"\A", r"\z")),
        (false, true) => Some((r"\b{start-half}", r"\b{end-half}")),
        (false, false) => None,
    };
    let compiled = if let Some((start, end)) = boundaries {
        match build(&format!("{start}(?:{pattern}){end}")) {
            // A trailing verbose-mode comment can swallow the added closing
            // parenthesis. End the comment before closing the group.
            Err(regex::Error::Syntax(_)) if build(pattern).is_ok() => {
                build(&format!("{start}(?:{pattern}\n){end}"))
            }
            result => result,
        }
    } else {
        build(pattern)
    };
    compiled.map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid regex pattern {label}: {error}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(
        patterns: &[&str],
        options: ContentMatchOptions,
        line: &[u8],
        all: bool,
    ) -> Vec<(usize, usize, usize)> {
        let patterns = patterns
            .iter()
            .map(|term| term.as_bytes().to_vec())
            .collect::<Vec<_>>();
        let matcher = compile(&patterns, options).unwrap();
        let mut spans = Vec::new();
        matcher.find_spans(line, all, &mut spans);
        spans
            .into_iter()
            .map(|span| (span.start, span.end, span.pattern))
            .collect()
    }

    #[test]
    fn all_spans_preserve_pattern_identity_and_zero_length_matches() {
        assert_eq!(
            spans(
                &[r"\d+", "E[0-9]", "$"],
                ContentMatchOptions::default(),
                b"E12 X9",
                true
            ),
            [(0, 2, 1), (1, 3, 0), (5, 6, 0), (6, 6, 2)]
        );
    }

    #[test]
    fn first_span_uses_pattern_order_to_break_ties() {
        assert_eq!(
            spans(
                &["ERROR|ERR", "ERR", "later"],
                ContentMatchOptions::default(),
                b" ERROR later",
                false
            ),
            [(1, 6, 0)]
        );
    }

    #[test]
    fn word_and_line_options_wrap_the_full_expression() {
        let word = ContentMatchOptions {
            whole_word: true,
            ..ContentMatchOptions::default()
        };
        let line = ContentMatchOptions {
            whole_line: true,
            ..ContentMatchOptions::default()
        };
        assert_eq!(spans(&["ERR|WARN"], word, b"xERR WARN!", true), [(5, 9, 0)]);
        assert_eq!(spans(&["ERR|WARN"], line, b"ERR WARN", true), []);
        assert_eq!(spans(&["ERR|WARN"], line, b"WARN", true), [(0, 4, 0)]);
    }

    #[test]
    fn unicode_case_and_raw_bytes_use_the_requested_engine_modes() {
        let insensitive = ContentMatchOptions {
            case_insensitive: true,
            unicode_case: true,
            ..ContentMatchOptions::default()
        };
        assert_eq!(
            spans(&["é.*e"], insensitive, "ÉTÉE".as_bytes(), true),
            [(0, 6, 0)]
        );
        assert_eq!(
            spans(
                &[r"\xFF+"],
                ContentMatchOptions::default(),
                b"a\xff\xffz",
                true
            ),
            [(1, 3, 0)]
        );
    }

    #[test]
    fn invalid_regex_identifies_the_pattern_number() {
        let patterns = vec![b"valid".to_vec(), b"(".to_vec()];
        let error = compile(&patterns, ContentMatchOptions::default())
            .err()
            .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("pattern 2"));
    }

    #[test]
    fn dense_regex_reports_span_overflow_without_growing_the_vector() {
        let line = vec![b'a'; SPAN_OVERFLOW * 2];
        let spans = spans(&["a", "a"], ContentMatchOptions::default(), &line, true);
        assert_eq!(spans.len(), SPAN_OVERFLOW);
    }

    #[test]
    fn boundaries_work_when_verbose_regex_ends_with_a_comment() {
        let options = ContentMatchOptions {
            whole_word: true,
            ..ContentMatchOptions::default()
        };
        assert_eq!(
            spans(&["(?x) WARN # comment"], options, b"SWARN WARN!", true),
            [(6, 10, 0)]
        );
    }
}
