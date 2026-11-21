//! Opt-in content search over files and streams selected by a query plan.
mod encoding;
mod literal;

pub use encoding::ContentEncoding;

use encoding::PreparedInput;
use std::collections::VecDeque;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::ops::ControlFlow;
use std::path::Path;
use std::sync::Arc;

/// Number of leading bytes inspected for a NUL byte.
pub const BINARY_PROBE: usize = 8 * 1024;
pub(super) const MAX_SPANS: usize = 65_536;
const MAX_CONTEXT_LINES: usize = 1024;
const MAX_CONTEXT_BYTES: usize = 32 * 1024 * 1024;
const MAX_LINE_BYTES: usize = 64 * 1024 * 1024;
const DEFAULT_LINE_BYTES: usize = 8 * 1024 * 1024;

/// One matching line. The column is a one-based byte offset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentMatch<'a> {
    pub line_number: u64,
    pub column: u64,
    pub line: &'a [u8],
}

/// A zero-based, half-open span within the decoded line and its input pattern ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentSpan {
    pub start: usize,
    pub end: usize,
    pub pattern: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentRecordKind {
    Match,
    Context,
}

/// A line in the logical byte stream, after any decoding and BOM removal.
/// The line number is one-based. The stream byte offset is zero-based.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentRecord<'a> {
    pub line_number: u64,
    pub byte_offset: u64,
    pub line: &'a [u8],
    pub spans: &'a [ContentSpan],
    pub kind: ContentRecordKind,
}

/// Work performed while scanning one input.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScanStats {
    pub matched_lines: u64,
    /// Logical bytes consumed by the scanner, or probe bytes for a skipped input.
    pub bytes_read: u64,
    pub binary_skipped: bool,
    pub encoding: Option<ContentEncoding>,
}

/// How multiple patterns combine within a single line.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ContentBoolean {
    #[default]
    Any,
    All,
    None,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ContentMatchOptions {
    pub case_insensitive: bool,
    /// Enable Unicode lowercasing in insensitive mode and Unicode word boundaries.
    pub unicode_case: bool,
    pub whole_word: bool,
    pub whole_line: bool,
    pub invert: bool,
    pub mode: ContentBoolean,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentScanOptions {
    pub before_context: usize,
    pub after_context: usize,
    /// Report every pattern occurrence, including overlaps.
    pub all_spans: bool,
    pub encoding: ContentEncoding,
    pub max_line_bytes: usize,
}

impl Default for ContentScanOptions {
    fn default() -> Self {
        Self {
            before_context: 0,
            after_context: 0,
            all_spans: false,
            encoding: ContentEncoding::Auto,
            max_line_bytes: DEFAULT_LINE_BYTES,
        }
    }
}

/// Reports candidate spans in a logical input line. With `all` disabled,
/// report the earliest span, breaking ties by input pattern order. With
/// `all` enabled, report overlaps and limit output to 65,537 spans so the
/// scanner can report that the line exceeded its output budget.
pub trait ContentMatcher: Send + Sync {
    fn find_spans(&self, line: &[u8], all: bool, out: &mut Vec<ContentSpan>);

    fn is_match(&self, input: &[u8]) -> bool {
        let mut spans = Vec::new();
        self.find_spans(input, false, &mut spans);
        !spans.is_empty()
    }
}

/// A compiled content query shared by the workers that scan selected files.
#[derive(Clone)]
pub struct ContentQuery {
    patterns: Arc<[Vec<u8>]>,
    matcher: Arc<dyn ContentMatcher>,
    options: ContentMatchOptions,
    text: bool,
}

impl fmt::Debug for ContentQuery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ContentQuery")
            .field("patterns", &self.patterns)
            .field("options", &self.options)
            .field("text", &self.text)
            .finish_non_exhaustive()
    }
}

impl ContentQuery {
    pub fn new(patterns: Vec<Vec<u8>>, case_insensitive: bool, text: bool) -> io::Result<Self> {
        Self::with_options(
            patterns,
            ContentMatchOptions {
                case_insensitive,
                ..ContentMatchOptions::default()
            },
            text,
        )
    }

    pub fn from_strings<S: AsRef<str>>(
        patterns: &[S],
        case_insensitive: bool,
        text: bool,
    ) -> io::Result<Self> {
        Self::new(
            patterns
                .iter()
                .map(|s| s.as_ref().as_bytes().to_vec())
                .collect(),
            case_insensitive,
            text,
        )
    }

    pub fn with_options(
        patterns: Vec<Vec<u8>>,
        options: ContentMatchOptions,
        text: bool,
    ) -> io::Result<Self> {
        validate_patterns(&patterns)?;
        let patterns: Arc<[Vec<u8>]> = patterns.into();
        let matcher = literal::matcher(&patterns, options.case_insensitive, options.unicode_case)?;
        Ok(Self {
            patterns,
            matcher,
            options,
            text,
        })
    }

    /// Attach a separately compiled matcher. Report original, half-open
    /// byte spans and zero-based IDs in the order supplied in `patterns`.
    pub fn with_matcher(
        patterns: Vec<Vec<u8>>,
        matcher: Arc<dyn ContentMatcher>,
        options: ContentMatchOptions,
        text: bool,
    ) -> io::Result<Self> {
        validate_patterns(&patterns)?;
        Ok(Self {
            patterns: patterns.into(),
            matcher,
            options,
            text,
        })
    }

    pub fn patterns(&self) -> &[Vec<u8>] {
        &self.patterns
    }

    pub fn case_insensitive(&self) -> bool {
        self.options.case_insensitive
    }

    pub fn text(&self) -> bool {
        self.text
    }

    /// Scan a path in legacy raw mode. Use `scan_path` to decode input or
    /// request context and precise span records.
    pub fn scan_file<F>(
        &self,
        path: &Path,
        stop_after: Option<u64>,
        mut on_match: F,
    ) -> io::Result<ScanStats>
    where
        F: FnMut(ContentMatch<'_>) -> io::Result<ControlFlow<()>>,
    {
        self.scan_path(
            path,
            &ContentScanOptions {
                encoding: ContentEncoding::Bytes,
                ..ContentScanOptions::default()
            },
            stop_after,
            |record| {
                on_match(ContentMatch {
                    line_number: record.line_number,
                    column: record.spans.first().map_or(1, |span| span.start as u64 + 1),
                    line: record.line,
                })
            },
        )
    }

    pub fn scan_path<F>(
        &self,
        path: &Path,
        options: &ContentScanOptions,
        stop_after: Option<u64>,
        on_record: F,
    ) -> io::Result<ScanStats>
    where
        F: FnMut(ContentRecord<'_>) -> io::Result<ControlFlow<()>>,
    {
        if stop_after == Some(0) {
            return Ok(ScanStats::default());
        }
        self.scan_reader(File::open(path)?, options, stop_after, on_record)
    }

    /// Scan a reader in line order. Limits count matching records. The final
    /// match can be followed by trailing context, and a callback break stops
    /// immediately.
    pub fn scan_reader<R, F>(
        &self,
        reader: R,
        options: &ContentScanOptions,
        stop_after: Option<u64>,
        mut on_record: F,
    ) -> io::Result<ScanStats>
    where
        R: Read,
        F: FnMut(ContentRecord<'_>) -> io::Result<ControlFlow<()>>,
    {
        if stop_after == Some(0) {
            return Ok(ScanStats::default());
        }
        validate_scan_options(options)?;
        let prepared = encoding::prepare(reader, options.encoding, self.text, BINARY_PROBE)?;
        let input = match prepared {
            PreparedInput::Text(input) => input,
            PreparedInput::Binary { bytes_read } => {
                return Ok(ScanStats {
                    bytes_read,
                    binary_skipped: true,
                    ..ScanStats::default()
                });
            }
        };
        let stats = ScanStats {
            encoding: Some(input.encoding()),
            ..ScanStats::default()
        };
        let mut reader = BufReader::with_capacity(64 * 1024, input);
        if options.before_context == 0 && options.after_context == 0 {
            self.scan_borrowed_lines(&mut reader, options, stop_after, stats, &mut on_record)
        } else {
            self.scan_lines(&mut reader, options, stop_after, stats, &mut on_record)
        }
    }

    // Keep complete lines borrowed from the read buffer. Only lines crossing
    // a refill boundary need a copy, and the same scratch allocation is reused.
    fn scan_borrowed_lines<F>(
        &self,
        reader: &mut impl BufRead,
        options: &ContentScanOptions,
        stop_after: Option<u64>,
        mut stats: ScanStats,
        on_record: &mut F,
    ) -> io::Result<ScanStats>
    where
        F: FnMut(ContentRecord<'_>) -> io::Result<ControlFlow<()>>,
    {
        let mut pending = Vec::new();
        let mut spans = Vec::new();
        let mut seen = vec![
            false;
            if self.options.mode == ContentBoolean::All {
                self.patterns.len()
            } else {
                0
            }
        ];
        let mut line_number = 0;
        loop {
            let bytes = match reader.fill_buf() {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            let eof = bytes.is_empty();
            if eof && pending.is_empty() {
                break;
            }
            let newline = literal::find_byte(bytes, b'\n');
            let consumed = newline.map_or(bytes.len(), |index| index + 1);
            if pending.len().saturating_add(consumed) > options.max_line_bytes + 2 {
                return Err(line_limit_error(line_number + 1, options.max_line_bytes));
            }
            if newline.is_none() && !eof {
                pending.extend_from_slice(bytes);
                reader.consume(consumed);
                continue;
            }
            let raw = if pending.is_empty() {
                &bytes[..consumed]
            } else {
                pending.extend_from_slice(&bytes[..consumed]);
                &pending
            };
            let line = trimmed_line(raw);
            line_number += 1;
            if line.len() > options.max_line_bytes {
                return Err(line_limit_error(line_number, options.max_line_bytes));
            }
            let byte_offset = stats.bytes_read;
            stats.bytes_read = stats.bytes_read.saturating_add(raw.len() as u64);
            if self.match_line(line, options.all_spans, &mut spans, &mut seen)? {
                stats.matched_lines += 1;
                if on_record(ContentRecord {
                    line_number,
                    byte_offset,
                    line,
                    spans: &spans,
                    kind: ContentRecordKind::Match,
                })?
                .is_break()
                    || stop_after.is_some_and(|limit| stats.matched_lines >= limit)
                {
                    break;
                }
            }
            reader.consume(consumed);
            pending.clear();
            if eof {
                break;
            }
        }
        Ok(stats)
    }

    fn scan_lines<F>(
        &self,
        reader: &mut impl BufRead,
        options: &ContentScanOptions,
        stop_after: Option<u64>,
        mut stats: ScanStats,
        on_record: &mut F,
    ) -> io::Result<ScanStats>
    where
        F: FnMut(ContentRecord<'_>) -> io::Result<ControlFlow<()>>,
    {
        let mut buffer = Vec::with_capacity(256);
        let mut spans = Vec::new();
        let mut seen_patterns = vec![
            false;
            if self.options.mode == ContentBoolean::All {
                self.patterns.len()
            } else {
                0
            }
        ];
        let mut context = Context::default();
        let mut line_number = 0u64;
        let mut match_limit_reached = false;

        while !match_limit_reached || context.after > 0 {
            buffer.clear();
            // Take one extra payload byte plus CRLF to distinguish an
            // oversized line from a line exactly at the configured limit.
            let read = (&mut *reader)
                .take((options.max_line_bytes + 3) as u64)
                .read_until(b'\n', &mut buffer)?;
            if read == 0 {
                break;
            }
            let byte_offset = stats.bytes_read;
            stats.bytes_read = stats.bytes_read.saturating_add(read as u64);
            line_number += 1;
            let line = trimmed_line(&buffer);
            if line.len() > options.max_line_bytes {
                return Err(line_limit_error(line_number, options.max_line_bytes));
            }
            let found = !match_limit_reached
                && self.match_line(line, options.all_spans, &mut spans, &mut seen_patterns)?;
            if found {
                stats.matched_lines += 1;
                if context.flush(on_record)?.is_break()
                    || on_record(ContentRecord {
                        line_number,
                        byte_offset,
                        line,
                        spans: &spans,
                        kind: ContentRecordKind::Match,
                    })?
                    .is_break()
                {
                    break;
                }
                context.after = options.after_context;
                match_limit_reached = stop_after.is_some_and(|limit| stats.matched_lines >= limit);
            } else if context.after > 0 {
                context.after -= 1;
                if on_record(ContentRecord {
                    line_number,
                    byte_offset,
                    line,
                    spans: &[],
                    kind: ContentRecordKind::Context,
                })?
                .is_break()
                {
                    break;
                }
            } else if options.before_context > 0 {
                context.push(line_number, byte_offset, line, options.before_context)?;
            }
        }
        Ok(stats)
    }

    fn match_line(
        &self,
        line: &[u8],
        all_spans: bool,
        spans: &mut Vec<ContentSpan>,
        seen_patterns: &mut [bool],
    ) -> io::Result<bool> {
        spans.clear();
        let candidates_required = all_spans
            || self.options.mode == ContentBoolean::All
            || self.options.whole_word
            || self.options.whole_line;
        self.matcher.find_spans(line, candidates_required, spans);
        if spans.len() > MAX_SPANS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "a line exceeds the 65,536-match span limit",
            ));
        }
        if spans.iter().any(|span| {
            span.start > span.end || span.end > line.len() || span.pattern >= self.patterns.len()
        }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "content matcher returned an invalid span",
            ));
        }
        if self.options.whole_line {
            spans.retain(|span| span.start == 0 && span.end == line.len());
        }
        if self.options.whole_word {
            let utf8 = if self.options.unicode_case {
                std::str::from_utf8(line).ok()
            } else {
                None
            };
            spans.retain(|span| is_whole_word(line, utf8, *span));
        }
        let matches = match self.options.mode {
            ContentBoolean::Any => !spans.is_empty(),
            ContentBoolean::None => spans.is_empty(),
            ContentBoolean::All => {
                seen_patterns.fill(false);
                for span in spans.iter() {
                    seen_patterns[span.pattern] = true;
                }
                seen_patterns.iter().all(|seen| *seen)
            }
        } ^ self.options.invert;
        if matches {
            if all_spans && spans.len() > 1 {
                spans.sort_unstable_by_key(|span| span_key(*span));
                spans.dedup();
            } else if !all_spans && spans.len() > 1 {
                let first = *spans
                    .iter()
                    .min_by_key(|span| span_key(**span))
                    .expect("multiple spans");
                spans.clear();
                spans.push(first);
            }
        }
        Ok(matches)
    }
}

impl fmt::Display for ContentQuery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} pattern(s), case {}, binary {}",
            self.patterns.len(),
            if self.options.case_insensitive {
                "insensitive"
            } else {
                "sensitive"
            },
            if self.text {
                "read as text"
            } else {
                "auto-skipped"
            },
        )
    }
}

pub(super) fn span_key(span: ContentSpan) -> (usize, usize, usize) {
    (span.start, span.pattern, span.end)
}

fn validate_patterns(patterns: &[Vec<u8>]) -> io::Result<()> {
    if patterns.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "content search requires at least one pattern",
        ));
    }
    if patterns.iter().any(Vec::is_empty) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "content patterns cannot be empty",
        ));
    }
    Ok(())
}

fn validate_scan_options(options: &ContentScanOptions) -> io::Result<()> {
    if !(1..=MAX_LINE_BYTES).contains(&options.max_line_bytes) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "content line limit must be between 1 byte and 64 MiB",
        ));
    }
    if options.before_context > MAX_CONTEXT_LINES || options.after_context > MAX_CONTEXT_LINES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "content context must be between 0 and 1024 lines",
        ));
    }
    Ok(())
}

fn trimmed_line(buffer: &[u8]) -> &[u8] {
    if let Some(line) = buffer.strip_suffix(b"\n") {
        line.strip_suffix(b"\r").unwrap_or(line)
    } else {
        buffer
    }
}

fn line_limit_error(line_number: u64, limit: usize) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("line {line_number} exceeds the {limit}-byte limit"),
    )
}

fn is_whole_word(line: &[u8], utf8: Option<&str>, span: ContentSpan) -> bool {
    if let Some(text) = utf8 {
        let Some(before) = text.get(..span.start) else {
            return false;
        };
        let Some(after) = text.get(span.end..) else {
            return false;
        };
        let word = |ch: char| ch.is_alphanumeric() || ch == '_';
        return !before.chars().next_back().is_some_and(word)
            && !after.chars().next().is_some_and(word);
    }
    let word = |byte: &u8| byte.is_ascii_alphanumeric() || *byte == b'_';
    !span
        .start
        .checked_sub(1)
        .and_then(|offset| line.get(offset))
        .is_some_and(word)
        && !line.get(span.end).is_some_and(word)
}

struct ContextLine {
    number: u64,
    byte_offset: u64,
    bytes: Vec<u8>,
}

#[derive(Default)]
struct Context {
    pending: VecDeque<ContextLine>,
    pending_bytes: usize,
    after: usize,
}

impl Context {
    fn push(
        &mut self,
        number: u64,
        byte_offset: u64,
        line: &[u8],
        capacity: usize,
    ) -> io::Result<()> {
        if self.pending.len() == capacity {
            let old = self.pending.pop_front().expect("nonempty context");
            self.pending_bytes -= old.bytes.len();
        }
        if self.pending_bytes + line.len() > MAX_CONTEXT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "leading context exceeds the 32 MiB buffer limit",
            ));
        }
        self.pending_bytes += line.len();
        self.pending.push_back(ContextLine {
            number,
            byte_offset,
            bytes: line.to_vec(),
        });
        Ok(())
    }

    fn flush<F>(&mut self, on_record: &mut F) -> io::Result<ControlFlow<()>>
    where
        F: FnMut(ContentRecord<'_>) -> io::Result<ControlFlow<()>>,
    {
        while let Some(record) = self.pending.pop_front() {
            self.pending_bytes -= record.bytes.len();
            if on_record(ContentRecord {
                line_number: record.number,
                byte_offset: record.byte_offset,
                line: &record.bytes,
                spans: &[],
                kind: ContentRecordKind::Context,
            })?
            .is_break()
            {
                return Ok(ControlFlow::Break(()));
            }
        }
        Ok(ControlFlow::Continue(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn borrowed_scanner_preserves_lines_spans_offsets_and_limits_across_refills() {
        let input = b"\nalpha\r\nneedle needle\nna\xffneedle\r\nlast needle\r";
        let query = ContentQuery::from_strings(&["needle", "eed", "last"], false, true).unwrap();
        for capacity in 1..=40 {
            for limit in [None, Some(1), Some(2)] {
                let options = ContentScanOptions {
                    all_spans: true,
                    ..ContentScanOptions::default()
                };
                let mut expected = Vec::new();
                let mut actual = Vec::new();
                let baseline = query
                    .scan_lines(
                        &mut BufReader::with_capacity(capacity, input.as_slice()),
                        &options,
                        limit,
                        ScanStats::default(),
                        &mut |record| {
                            expected.push((
                                record.line_number,
                                record.byte_offset,
                                record.line.to_vec(),
                                record.spans.to_vec(),
                            ));
                            Ok(ControlFlow::Continue(()))
                        },
                    )
                    .unwrap();
                let optimized = query
                    .scan_borrowed_lines(
                        &mut BufReader::with_capacity(capacity, input.as_slice()),
                        &options,
                        limit,
                        ScanStats::default(),
                        &mut |record| {
                            actual.push((
                                record.line_number,
                                record.byte_offset,
                                record.line.to_vec(),
                                record.spans.to_vec(),
                            ));
                            Ok(ControlFlow::Continue(()))
                        },
                    )
                    .unwrap();
                assert_eq!(actual, expected, "capacity {capacity}, limit {limit:?}");
                assert_eq!(optimized, baseline);
            }
        }
    }

    #[test]
    fn borrowed_scanner_enforces_decoded_line_limits_with_crlf_and_eof() {
        let query = ContentQuery::from_strings(&["x"], false, true).unwrap();
        let options = ContentScanOptions {
            max_line_bytes: 3,
            ..ContentScanOptions::default()
        };
        for capacity in 1..=8 {
            for (input, valid) in [
                (b"xxx\r\n".as_slice(), true),
                (b"xxx".as_slice(), true),
                (b"xxxx\n".as_slice(), false),
                (b"xxx\r".as_slice(), false),
            ] {
                let result = query.scan_borrowed_lines(
                    &mut BufReader::with_capacity(capacity, input),
                    &options,
                    None,
                    ScanStats::default(),
                    &mut |_| Ok(ControlFlow::Continue(())),
                );
                assert_eq!(
                    result.is_ok(),
                    valid,
                    "capacity {capacity}, input {input:?}"
                );
            }
        }
    }

    struct Temp(std::path::PathBuf);

    impl Temp {
        fn file(contents: &[u8]) -> (Self, std::path::PathBuf) {
            let root = std::env::temp_dir().join(format!(
                "branchcut-content-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).expect("create content fixture");
            let path = root.join("sample.txt");
            fs::write(&path, contents).expect("write content fixture");
            (Self(root), path)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn collect(query: &ContentQuery, path: &Path) -> Vec<(u64, u64, Vec<u8>)> {
        let mut matches = Vec::new();
        query
            .scan_file(path, None, |found| {
                matches.push((found.line_number, found.column, found.line.to_vec()));
                Ok(ControlFlow::Continue(()))
            })
            .expect("scan fixture");
        matches
    }

    #[test]
    fn reports_line_column_and_text() {
        let (temp, path) = Temp::file(b"alpha\nbeta needle\ngamma\n");
        let query = ContentQuery::from_strings(&["needle"], false, false).expect("compile");
        assert_eq!(
            collect(&query, &path),
            vec![(2, 6, b"beta needle".to_vec())]
        );
        assert!(temp.0.exists());
    }

    #[test]
    fn supports_multiple_patterns_and_earliest_column() {
        let (temp, path) = Temp::file(b"one two three\n");
        let query = ContentQuery::from_strings(&["three", "two"], false, false).expect("compile");
        assert_eq!(
            collect(&query, &path),
            vec![(1, 5, b"one two three".to_vec())]
        );
        assert!(temp.0.exists());
    }

    #[test]
    fn case_insensitive_matching_folds_ascii() {
        let (temp, path) = Temp::file(b"Hello NEEDLE\n");
        let query = ContentQuery::from_strings(&["needle"], true, false).expect("compile");
        assert_eq!(
            collect(&query, &path),
            vec![(1, 7, b"Hello NEEDLE".to_vec())]
        );
        assert!(temp.0.exists());
    }

    #[test]
    fn strips_crlf_endings() {
        let (temp, path) = Temp::file(b"first\r\nneedle here\r\n");
        let query = ContentQuery::from_strings(&["needle"], false, false).expect("compile");
        assert_eq!(
            collect(&query, &path),
            vec![(2, 1, b"needle here".to_vec())]
        );
        assert!(temp.0.exists());
    }

    #[test]
    fn skips_binary_unless_text_mode() {
        let (temp, path) = Temp::file(b"needle\0binary\n");
        let automatic = ContentQuery::from_strings(&["needle"], false, false).expect("compile");
        let stats = automatic
            .scan_file(&path, None, |_| Ok(ControlFlow::Continue(())))
            .expect("scan");
        assert!(stats.binary_skipped);
        assert_eq!(stats.matched_lines, 0);

        let text = ContentQuery::from_strings(&["needle"], false, true).expect("compile");
        assert_eq!(collect(&text, &path).len(), 1);
        assert!(temp.0.exists());
    }

    #[test]
    fn preserves_non_utf8_bytes() {
        let (temp, path) = Temp::file(b"pre\xffneedle\xfe\n");
        let query = ContentQuery::from_strings(&["needle"], false, false).expect("compile");
        let found = collect(&query, &path);
        assert_eq!(found[0].0, 1);
        assert_eq!(found[0].2, b"pre\xffneedle\xfe".to_vec());
        assert!(temp.0.exists());
    }

    #[test]
    fn stops_at_the_requested_limit() {
        let (temp, path) = Temp::file(b"needle\nneedle\nneedle\n");
        let query = ContentQuery::from_strings(&["needle"], false, false).expect("compile");
        let stats = query
            .scan_file(&path, Some(2), |_| Ok(ControlFlow::Continue(())))
            .expect("scan");
        assert_eq!(stats.matched_lines, 2);
        assert!(temp.0.exists());
    }

    #[test]
    fn rejects_empty_patterns() {
        assert!(ContentQuery::from_strings(&[""], false, false).is_err());
        assert!(ContentQuery::from_strings::<&str>(&[], false, false).is_err());
    }
}
