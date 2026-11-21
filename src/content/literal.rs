use super::{ContentMatcher, ContentSpan, MAX_SPANS};
use std::collections::VecDeque;
use std::io;
use std::sync::Arc;

const WORD_LOW_BITS: u64 = 0x0101_0101_0101_0101;
const WORD_HIGH_BITS: u64 = 0x8080_8080_8080_8080;
const MAX_AUTOMATON_CELLS: usize = 8 * 1024 * 1024;
const MAX_PATTERN_BYTES: usize = 256 * 1024;

pub(super) fn matcher(
    patterns: &Arc<[Vec<u8>]>,
    insensitive: bool,
    unicode: bool,
) -> io::Result<Arc<dyn ContentMatcher>> {
    if !insensitive || !unicode {
        return Ok(Arc::new(LiteralMatcher::new(
            Arc::clone(patterns),
            insensitive,
        )?));
    }

    let mut folded_patterns = Vec::with_capacity(patterns.len());
    for pattern in patterns.iter() {
        let value = std::str::from_utf8(pattern).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unicode case matching requires UTF-8 patterns",
            )
        })?;
        folded_patterns.push(
            value
                .chars()
                .flat_map(char::to_lowercase)
                .collect::<String>()
                .into_bytes(),
        );
    }
    let mut matcher = LiteralMatcher::new(folded_patterns.into(), false)?;
    // A match ending inside a folded scalar must be discarded. The default
    // first-match shortcut cannot know whether a later match is valid.
    matcher.allow_first = false;
    Ok(Arc::new(UnicodeMatcher { folded: matcher }))
}

struct LiteralMatcher {
    program: LiteralProgram,
    insensitive: bool,
    needles: Arc<[Vec<u8>]>,
    allow_first: bool,
}

enum LiteralProgram {
    Single(ByteFilter),
    Shared(Box<Automaton>),
}

impl LiteralMatcher {
    fn new(needles: Arc<[Vec<u8>]>, insensitive: bool) -> io::Result<Self> {
        if needles.iter().map(Vec::len).sum::<usize>() > MAX_PATTERN_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "literal content patterns exceed the 256 KiB byte budget",
            ));
        }
        let program = if needles.len() == 1 {
            LiteralProgram::Single(ByteFilter::new([needles[0][0]], insensitive))
        } else {
            LiteralProgram::Shared(Box::new(Automaton::compile(&needles, insensitive)?))
        };
        Ok(Self {
            program,
            insensitive,
            needles,
            allow_first: true,
        })
    }
}

impl ContentMatcher for LiteralMatcher {
    fn may_match_block(&self, input: &[u8]) -> bool {
        let filter = match &self.program {
            LiteralProgram::Single(filter) => filter,
            LiteralProgram::Shared(program) => &program.first,
        };
        filter.candidates.len() > 4 || filter.find(input).is_some()
    }

    fn find_spans(&self, line: &[u8], all: bool, out: &mut Vec<ContentSpan>) {
        let all = all || !self.allow_first;
        match &self.program {
            LiteralProgram::Single(filter) => {
                let needle = &self.needles[0];
                if line.len() < needle.len() {
                    return;
                }
                let last = line.len() - needle.len();
                let mut index = 0;
                while index <= last {
                    let Some(pos) = filter.find(&line[index..=last]) else {
                        break;
                    };
                    let start = index + pos;
                    let end = start + needle.len();
                    let candidate = &line[start..end];
                    if if self.insensitive {
                        candidate.eq_ignore_ascii_case(needle)
                    } else {
                        candidate == needle
                    } {
                        out.push(ContentSpan {
                            start,
                            end,
                            pattern: 0,
                        });
                        if !all || out.len() > MAX_SPANS {
                            break;
                        }
                    }
                    index = start + 1;
                }
            }
            LiteralProgram::Shared(program) => program.scan(line, all, out),
        }
    }
}

struct UnicodeMatcher {
    folded: LiteralMatcher,
}

impl ContentMatcher for UnicodeMatcher {
    fn find_spans(&self, line: &[u8], all: bool, out: &mut Vec<ContentSpan>) {
        let Ok(text) = std::str::from_utf8(line) else {
            return;
        };
        let mut folded = String::with_capacity(line.len());
        // Keep a sparse map. Most case conversions keep the byte length,
        // including ASCII, so their byte offsets can be computed directly.
        let mut changes = Vec::new();
        for (start, ch) in text.char_indices() {
            let begin = folded.len();
            folded.extend(ch.to_lowercase());
            let end = folded.len();
            if ch.len_utf8() != end - begin {
                changes.push(FoldedScalar {
                    start,
                    begin,
                    end,
                    len: ch.len_utf8(),
                });
            }
        }
        self.folded.find_spans(folded.as_bytes(), all, out);
        out.retain_mut(|span| {
            let Some(start) = original_offset(text, &changes, span.start) else {
                return false;
            };
            let Some(end) = original_offset(text, &changes, span.end) else {
                return false;
            };
            span.start = start;
            span.end = end;
            true
        });
    }
}

struct FoldedScalar {
    start: usize,
    begin: usize,
    end: usize,
    len: usize,
}

fn original_offset(text: &str, changes: &[FoldedScalar], offset: usize) -> Option<usize> {
    let previous = changes.partition_point(|item| item.begin < offset);
    let original = if previous == 0 {
        offset
    } else {
        let item = &changes[previous - 1];
        if offset < item.end {
            return None;
        }
        item.start + item.len + offset - item.end
    };
    text.is_char_boundary(original).then_some(original)
}

fn has_zero_bytes(word: u64) -> bool {
    word.wrapping_sub(WORD_LOW_BITS) & !word & WORD_HIGH_BITS != 0
}

/// Skip blocks of bytes that contain no possible first byte. Verify candidates
/// in the byte loop, since the word-level filter can have false positives.
struct ByteFilter {
    candidates: Vec<u8>,
    words: Vec<u64>,
}

impl ByteFilter {
    fn new(bytes: impl IntoIterator<Item = u8>, insensitive: bool) -> Self {
        let mut candidates = Vec::new();
        for b in bytes {
            candidates.push(b);
            if insensitive && b.is_ascii_alphabetic() {
                candidates.extend([b.to_ascii_lowercase(), b.to_ascii_uppercase()]);
            }
        }
        candidates.sort_unstable();
        candidates.dedup();
        let words = candidates
            .iter()
            .map(|&b| u64::from_ne_bytes([b; 8]))
            .collect();
        Self { candidates, words }
    }

    fn find(&self, bytes: &[u8]) -> Option<usize> {
        match self.candidates.as_slice() {
            [one] => find_word(bytes, [*one], [self.words[0]]),
            [one, two] => find_word(bytes, [*one, *two], [self.words[0], self.words[1]]),
            [a, b, c] => find_word(
                bytes,
                [*a, *b, *c],
                [self.words[0], self.words[1], self.words[2]],
            ),
            [a, b, c, d] => find_word(
                bytes,
                [*a, *b, *c, *d],
                [self.words[0], self.words[1], self.words[2], self.words[3]],
            ),
            _ => bytes
                .iter()
                .position(|b| self.candidates.binary_search(b).is_ok()),
        }
    }
}

fn find_word<const N: usize>(bytes: &[u8], candidates: [u8; N], words: [u64; N]) -> Option<usize> {
    let (chunks, remainder) = bytes.as_chunks::<8>();
    for (index, &chunk) in chunks.iter().enumerate() {
        let word = u64::from_ne_bytes(chunk);
        if words.iter().any(|mask| has_zero_bytes(word ^ mask))
            && let Some(offset) = chunk.iter().position(|b| candidates.contains(b))
        {
            return Some(index * 8 + offset);
        }
    }
    remainder
        .iter()
        .position(|b| candidates.contains(b))
        .map(|index| bytes.len() - remainder.len() + index)
}

pub(super) fn find_byte(bytes: &[u8], byte: u8) -> Option<usize> {
    find_word(bytes, [byte], [u64::from_ne_bytes([byte; 8])])
}

#[derive(Default)]
struct Node {
    edges: Vec<(u8, usize)>,
    outputs: Vec<usize>,
    suffix: usize,
    output_suffix: usize,
}

impl Node {
    fn edge(&self, byte: u8) -> Option<usize> {
        self.edges
            .iter()
            .find(|&&(b, _)| b == byte)
            .map(|&(_, node)| node)
    }
}

struct Automaton {
    nodes: Vec<Node>,
    transitions: Vec<u32>,
    classes: [usize; 256],
    stride: usize,
    max_needle: usize,
    lengths: Vec<usize>,
    first: ByteFilter,
}

impl Automaton {
    fn compile(patterns: &[Vec<u8>], insensitive: bool) -> io::Result<Self> {
        let mut nodes = vec![Node::default()];
        let mut alphabet = [false; 256];
        for (index, pattern) in patterns.iter().enumerate() {
            let mut state = 0;
            for &byte in pattern {
                let byte = fold_ascii(byte, insensitive);
                alphabet[usize::from(byte)] = true;
                state = if let Some(next) = nodes[state].edge(byte) {
                    next
                } else {
                    let next = nodes.len();
                    nodes[state].edges.push((byte, next));
                    nodes.push(Node::default());
                    next
                };
            }
            nodes[state].outputs.push(index);
        }
        let mut classes = [0; 256];
        let mut stride = 1;
        for byte in 0..256 {
            if alphabet[byte] {
                classes[byte] = stride;
                stride += 1;
            }
        }
        for byte in b'A'..=b'Z' {
            if insensitive {
                classes[usize::from(byte)] = classes[usize::from(byte.to_ascii_lowercase())];
            }
        }
        let cells = nodes.len().saturating_mul(stride);
        if cells > MAX_AUTOMATON_CELLS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "literal content automaton exceeds the 32 MiB transition budget",
            ));
        }
        let mut transitions = vec![0; cells];
        let mut queue = VecDeque::new();
        for &(byte, node) in &nodes[0].edges {
            transitions[classes[usize::from(byte)]] = node as u32;
            queue.push_back(node);
        }
        while let Some(state) = queue.pop_front() {
            let fallback = nodes[state].suffix * stride;
            let row = state * stride;
            transitions.copy_within(fallback..fallback + stride, row);
            // The edge list is read while mutating independently indexed
            // nodes. Copy a pair on each iteration without cloning the list.
            for index in 0..nodes[state].edges.len() {
                let (byte, next) = nodes[state].edges[index];
                let class = classes[usize::from(byte)];
                let suffix = transitions[fallback + class] as usize;
                nodes[next].suffix = suffix;
                nodes[next].output_suffix = if nodes[suffix].outputs.is_empty() {
                    nodes[suffix].output_suffix
                } else {
                    suffix
                };
                transitions[row + class] = next as u32;
                queue.push_back(next);
            }
        }
        Ok(Self {
            first: ByteFilter::new(patterns.iter().map(|p| p[0]), insensitive),
            max_needle: patterns.iter().map(Vec::len).max().unwrap_or_default(),
            lengths: patterns.iter().map(Vec::len).collect(),
            nodes,
            transitions,
            classes,
            stride,
        })
    }

    fn scan(&self, line: &[u8], all: bool, out: &mut Vec<ContentSpan>) {
        let mut state = 0;
        let mut i = 0;
        let mut first = None::<ContentSpan>;
        while i < line.len() {
            if state == 0 && self.first.candidates.len() <= 4 {
                let Some(skip) = self.first.find(&line[i..]) else {
                    break;
                };
                i += skip;
            }
            let class = self.classes[usize::from(line[i])];
            i += 1;
            state = self.transitions[state * self.stride + class] as usize;
            if state > 0 {
                let mut found = state;
                while found > 0 {
                    for &pattern in &self.nodes[found].outputs {
                        let span = ContentSpan {
                            start: i - self.lengths[pattern],
                            end: i,
                            pattern,
                        };
                        if all {
                            out.push(span);
                            if out.len() > MAX_SPANS {
                                return;
                            }
                        } else if first
                            .is_none_or(|current| super::span_key(span) < super::span_key(current))
                        {
                            first = Some(span);
                        }
                    }
                    found = self.nodes[found].output_suffix;
                }
            }
            if first.is_some_and(|found| i - found.start >= self.max_needle) {
                break;
            }
        }
        if let Some(span) = first {
            out.push(span);
        }
    }
}

fn fold_ascii(byte: u8, insensitive: bool) -> u8 {
    if insensitive {
        byte.to_ascii_lowercase()
    } else {
        byte
    }
}
