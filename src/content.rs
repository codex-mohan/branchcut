//! Opt-in literal content search over files selected by a query plan.
//!
//! The scanner reads a file once, line by line, and reports the first byte
//! offset of the earliest pattern match on each matching line. It never
//! decodes text: patterns and file bytes are compared as bytes, so invalid
//! UTF-8 and platform-native names are preserved. ASCII case folding is
//! available when requested.
//!
//! Files are treated as binary when the first [`BINARY_PROBE`] bytes contain a
//! NUL byte. Binary files are skipped unless text mode is requested.
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Cursor, Read};
use std::ops::ControlFlow;
use std::path::Path;

/// Number of leading bytes inspected for a NUL byte.
pub const BINARY_PROBE: usize = 8 * 1024;

/// One matching line. `column` is a one-based byte offset, matching the
/// offset convention used by `grep`-style tools.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentMatch<'a> {
    pub line_number: u64,
    pub column: u64,
    pub line: &'a [u8],
}

/// Work performed while scanning one file.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScanStats {
    pub matched_lines: u64,
    pub bytes_read: u64,
    pub binary_skipped: bool,
}

/// A compiled literal content query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentQuery {
    patterns: Vec<Vec<u8>>,
    case_insensitive: bool,
    text: bool,
}

impl ContentQuery {
    /// Compile literal patterns. Empty pattern lists and empty patterns are
    /// rejected because an empty needle would match every line at column one.
    pub fn new(
        patterns: Vec<Vec<u8>>,
        case_insensitive: bool,
        text: bool,
    ) -> Result<Self, io::Error> {
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
        Ok(Self {
            patterns,
            case_insensitive,
            text,
        })
    }

    /// Build from UTF-8 patterns.
    pub fn from_strings<S: AsRef<str>>(
        patterns: &[S],
        case_insensitive: bool,
        text: bool,
    ) -> Result<Self, io::Error> {
        Self::new(
            patterns
                .iter()
                .map(|pattern| pattern.as_ref().as_bytes().to_vec())
                .collect(),
            case_insensitive,
            text,
        )
    }

    pub fn patterns(&self) -> &[Vec<u8>] {
        &self.patterns
    }

    pub fn case_insensitive(&self) -> bool {
        self.case_insensitive
    }

    pub fn text(&self) -> bool {
        self.text
    }

    /// Scan one file, reporting matches in line order.
    ///
    /// `stop_after` bounds the number of matching lines the callback receives.
    /// Returning [`ControlFlow::Break`] from the callback ends the scan early;
    /// the file is not read further. Read errors are returned as `io::Error`.
    pub fn scan_file<F>(
        &self,
        path: &Path,
        stop_after: Option<u64>,
        mut on_match: F,
    ) -> io::Result<ScanStats>
    where
        F: FnMut(ContentMatch<'_>) -> io::Result<ControlFlow<()>>,
    {
        if stop_after == Some(0) {
            return Ok(ScanStats::default());
        }
        let mut file = File::open(path)?;
        let mut probe = vec![0u8; BINARY_PROBE];
        let mut filled = 0;
        while filled < probe.len() {
            match file.read(&mut probe[filled..]) {
                Ok(0) => break,
                Ok(read) => filled += read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        let mut stats = ScanStats::default();
        if !self.text && probe[..filled].contains(&0) {
            stats.bytes_read = filled as u64;
            stats.binary_skipped = true;
            return Ok(stats);
        }
        let mut reader =
            BufReader::with_capacity(64 * 1024, Cursor::new(probe[..filled].to_vec()).chain(file));
        let mut buffer = Vec::with_capacity(256);
        let mut line_number = 0u64;
        loop {
            buffer.clear();
            let read = match reader.read_until(b'\n', &mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            // The chained cursor replays the probe, so every file byte is
            // counted exactly once across the line reads.
            stats.bytes_read = stats.bytes_read.saturating_add(read as u64);
            line_number += 1;
            let end = trimmed_line_end(&buffer, read);
            let line = &buffer[..end];
            let Some(column) = self.first_match(line) else {
                continue;
            };
            stats.matched_lines += 1;
            let control = on_match(ContentMatch {
                line_number,
                column: column as u64 + 1,
                line,
            })?;
            if control.is_break() || stop_after.is_some_and(|limit| stats.matched_lines >= limit) {
                break;
            }
        }
        Ok(stats)
    }

    fn first_match(&self, line: &[u8]) -> Option<usize> {
        self.patterns
            .iter()
            .filter_map(|pattern| find_literal(line, pattern, self.case_insensitive))
            .min()
    }
}

impl fmt::Display for ContentQuery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let patterns: Vec<String> = self
            .patterns
            .iter()
            .map(|pattern| String::from_utf8_lossy(pattern).into_owned())
            .collect();
        write!(
            formatter,
            "{} pattern(s), case {}, binary {}",
            patterns.len(),
            if self.case_insensitive {
                "insensitive"
            } else {
                "sensitive"
            },
            if self.text {
                "read as text"
            } else {
                "auto-skipped"
            }
        )
    }
}

fn trimmed_line_end(buffer: &[u8], read: usize) -> usize {
    let mut end = read;
    if buffer[end - 1] == b'\n' {
        end -= 1;
        if end > 0 && buffer[end - 1] == b'\r' {
            end -= 1;
        }
    }
    end
}

/// Return the first byte offset of `needle` in `haystack`.
fn find_literal(haystack: &[u8], needle: &[u8], case_insensitive: bool) -> Option<usize> {
    let first = needle[0];
    let haystack_len = haystack.len();
    if needle.len() > haystack_len {
        return None;
    }
    let last_start = haystack_len - needle.len();
    if case_insensitive {
        let first = first.to_ascii_lowercase();
        for (index, byte) in haystack.iter().enumerate().take(last_start + 1) {
            if byte.to_ascii_lowercase() != first {
                continue;
            }
            if haystack[index..index + needle.len()]
                .iter()
                .zip(needle)
                .all(|(found, wanted)| found.eq_ignore_ascii_case(wanted))
            {
                return Some(index);
            }
        }
    } else {
        for (index, byte) in haystack.iter().enumerate().take(last_start + 1) {
            if *byte == first && haystack[index..].starts_with(needle) {
                return Some(index);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

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
