use std::fmt;
use std::io::{self, BufRead, BufReader, Chain, Cursor, Read};
use std::str::FromStr;

/// How to read file bytes for content matching.
///
/// Text encodings that require transcoding produce UTF-8. Reported offsets
/// refer to the resulting byte stream, after any leading BOM is removed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ContentEncoding {
    /// Detect UTF-8 and UTF-16 BOMs. Without a BOM, preserve all input bytes.
    #[default]
    Auto,
    /// Strip a leading UTF-8 BOM and preserve all remaining bytes.
    Utf8,
    /// Decode UTF-16 little endian, stripping a matching BOM.
    Utf16Le,
    /// Decode UTF-16 big endian, stripping a matching BOM.
    Utf16Be,
    /// Decode ISO-8859-1, including its C1 control characters.
    Latin1,
    /// Decode Windows-1252 using the WHATWG Windows-1252 index.
    Windows1252,
    /// Preserve all bytes, including any BOM.
    Bytes,
}

impl ContentEncoding {
    fn is_utf16(self) -> bool {
        matches!(self, Self::Utf16Le | Self::Utf16Be)
    }
}

impl FromStr for ContentEncoding {
    type Err = io::Error;

    fn from_str(name: &str) -> io::Result<Self> {
        match name.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "utf8" | "utf-8" => Ok(Self::Utf8),
            "utf16le" | "utf-16le" => Ok(Self::Utf16Le),
            "utf16be" | "utf-16be" => Ok(Self::Utf16Be),
            "latin1" | "latin-1" | "iso-8859-1" | "iso8859-1" => Ok(Self::Latin1),
            "windows-1252" | "windows1252" | "cp1252" => Ok(Self::Windows1252),
            "bytes" | "raw" => Ok(Self::Bytes),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "unknown encoding {name:?}; expected auto, utf-8, utf-16le, utf-16be, latin1, windows-1252 or bytes"
                ),
            )),
        }
    }
}

impl fmt::Display for ContentEncoding {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Self::Auto => "auto",
            Self::Utf8 => "utf-8",
            Self::Utf16Le => "utf-16le",
            Self::Utf16Be => "utf-16be",
            Self::Latin1 => "latin1",
            Self::Windows1252 => "windows-1252",
            Self::Bytes => "bytes",
        })
    }
}

pub(super) enum PreparedInput<R> {
    Binary { bytes_read: u64 },
    Text(DecodedInput<R>),
}

type Replay<R> = Chain<Cursor<Vec<u8>>, R>;
const UTF8_BOM: &[u8] = &[0xef, 0xbb, 0xbf];
const UTF16_LE_BOM: &[u8] = &[0xff, 0xfe];
const UTF16_BE_BOM: &[u8] = &[0xfe, 0xff];

/// Replay the bytes inspected during detection so no read or seek is repeated.
/// A UTF-16 selection takes precedence over the NUL-byte binary heuristic.
pub(super) fn prepare<R: Read>(
    mut reader: R,
    encoding: ContentEncoding,
    text: bool,
    binary_probe: usize,
) -> io::Result<PreparedInput<R>> {
    let bom_len = match encoding {
        ContentEncoding::Auto | ContentEncoding::Utf8 => UTF8_BOM.len(),
        ContentEncoding::Utf16Le | ContentEncoding::Utf16Be => UTF16_LE_BOM.len(),
        ContentEncoding::Latin1 | ContentEncoding::Windows1252 | ContentEncoding::Bytes => 0,
    };
    let probe_limit = if text || encoding.is_utf16() {
        bom_len
    } else {
        binary_probe.max(bom_len)
    };
    let mut probe = vec![0; probe_limit];
    let mut filled = 0;
    while filled < probe.len() {
        match reader.read(&mut probe[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    probe.truncate(filled);
    let encoding = match encoding {
        ContentEncoding::Auto if probe.starts_with(UTF16_LE_BOM) => ContentEncoding::Utf16Le,
        ContentEncoding::Auto if probe.starts_with(UTF16_BE_BOM) => ContentEncoding::Utf16Be,
        ContentEncoding::Auto => ContentEncoding::Utf8,
        explicit => explicit,
    };
    if !text && !encoding.is_utf16() && probe[..filled.min(binary_probe)].contains(&0) {
        return Ok(PreparedInput::Binary {
            bytes_read: filled as u64,
        });
    }
    let bom = match encoding {
        ContentEncoding::Utf8 => UTF8_BOM,
        ContentEncoding::Utf16Le => UTF16_LE_BOM,
        ContentEncoding::Utf16Be => UTF16_BE_BOM,
        _ => &[],
    };
    let skip = if probe.starts_with(bom) { bom.len() } else { 0 };
    let mut prefix = Cursor::new(probe);
    prefix.set_position(skip as u64);
    let replay = prefix.chain(reader);

    let input = match encoding {
        ContentEncoding::Utf16Le
        | ContentEncoding::Utf16Be
        | ContentEncoding::Latin1
        | ContentEncoding::Windows1252 => Input::Decoded(Transcoder::new(replay, encoding)),
        _ => Input::Raw(replay),
    };
    Ok(PreparedInput::Text(DecodedInput { input, encoding }))
}

pub(super) struct DecodedInput<R> {
    input: Input<R>,
    encoding: ContentEncoding,
}

impl<R> DecodedInput<R> {
    pub(super) fn encoding(&self) -> ContentEncoding {
        self.encoding
    }
}

enum Input<R> {
    Raw(Replay<R>),
    Decoded(Transcoder<Replay<R>>),
}

impl<R: Read> Read for DecodedInput<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        match &mut self.input {
            Input::Raw(reader) => reader.read(output),
            Input::Decoded(reader) => reader.read(output),
        }
    }
}

struct Transcoder<R> {
    source: BufReader<R>,
    encoding: ContentEncoding,
    pending_unit: Option<Unit>,
    pending_utf8: [u8; 4],
    pending_start: usize,
    pending_end: usize,
    pending_error: Option<io::Error>,
}

#[derive(Clone, Copy)]
enum Unit {
    Code(u16),
    Incomplete,
    Eof,
}

impl<R: Read> Transcoder<R> {
    fn new(reader: R, encoding: ContentEncoding) -> Self {
        Self {
            source: BufReader::new(reader),
            encoding,
            pending_unit: None,
            pending_utf8: [0; 4],
            pending_start: 0,
            pending_end: 0,
            pending_error: None,
        }
    }

    fn byte(&mut self) -> io::Result<Option<u8>> {
        loop {
            match self.source.fill_buf() {
                Ok(bytes) => {
                    let byte = bytes.first().copied();
                    self.source.consume(usize::from(byte.is_some()));
                    return Ok(byte);
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
    }

    fn unit(&mut self) -> io::Result<Unit> {
        if let Some(unit) = self.pending_unit.take() {
            return Ok(unit);
        }
        let Some(first) = self.byte()? else {
            return Ok(Unit::Eof);
        };
        let Some(second) = self.byte()? else {
            return Ok(Unit::Incomplete);
        };
        let bytes = [first, second];
        Ok(Unit::Code(if self.encoding == ContentEncoding::Utf16Le {
            u16::from_le_bytes(bytes)
        } else {
            u16::from_be_bytes(bytes)
        }))
    }

    fn scalar(&mut self) -> io::Result<Option<char>> {
        if !self.encoding.is_utf16() {
            let windows1252 = self.encoding == ContentEncoding::Windows1252;
            return Ok(self.byte()?.map(|byte| {
                if windows1252 && (0x80..=0x9f).contains(&byte) {
                    WINDOWS_1252_C1[(byte - 0x80) as usize]
                } else {
                    char::from(byte)
                }
            }));
        }

        let first = match self.unit()? {
            Unit::Code(unit) => unit,
            Unit::Incomplete => return Ok(Some(char::REPLACEMENT_CHARACTER)),
            Unit::Eof => return Ok(None),
        };
        let codepoint = if (0xd800..=0xdbff).contains(&first) {
            match self.unit()? {
                Unit::Code(second) if (0xdc00..=0xdfff).contains(&second) => {
                    0x1_0000 + ((u32::from(first) - 0xd800) << 10) + (u32::from(second) - 0xdc00)
                }
                other => {
                    self.pending_unit = Some(other);
                    return Ok(Some(char::REPLACEMENT_CHARACTER));
                }
            }
        } else {
            u32::from(first)
        };
        // An unpaired surrogate or trailing incomplete unit becomes U+FFFD.
        Ok(Some(
            char::from_u32(codepoint).unwrap_or(char::REPLACEMENT_CHARACTER),
        ))
    }
}

impl<R: Read> Read for Transcoder<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        if let Some(error) = self.pending_error.take() {
            return Err(error);
        }
        let mut filled = 0;
        while filled < output.len() {
            if self.pending_start < self.pending_end {
                let count = (self.pending_end - self.pending_start).min(output.len() - filled);
                output[filled..filled + count].copy_from_slice(
                    &self.pending_utf8[self.pending_start..self.pending_start + count],
                );
                filled += count;
                self.pending_start += count;
                continue;
            }
            // Return available data without waiting for another source read.
            if filled > 0 && self.source.buffer().is_empty() && self.pending_unit.is_none() {
                break;
            }
            match self.scalar() {
                Ok(Some(character)) => {
                    self.pending_end = character.encode_utf8(&mut self.pending_utf8).len();
                    self.pending_start = 0;
                }
                Ok(None) => break,
                Err(error) if filled == 0 => return Err(error),
                Err(error) => {
                    self.pending_error = Some(error);
                    break;
                }
            }
        }
        Ok(filled)
    }
}

// WHATWG Encoding Standard, Windows-1252 index, C1 range (0x80–0x9F).
// https://encoding.spec.whatwg.org/index-windows-1252.txt
const WINDOWS_1252_C1: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}',
    '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
];

#[cfg(test)]
mod tests {
    use super::*;

    struct OneByte<R>(R);

    impl<R: Read> Read for OneByte<R> {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            let limit = bytes.len().min(1);
            self.0.read(&mut bytes[..limit])
        }
    }

    fn decode(input: &[u8], encoding: ContentEncoding, text: bool) -> (ContentEncoding, Vec<u8>) {
        let PreparedInput::Text(mut reader) = prepare(OneByte(input), encoding, text, 4).unwrap()
        else {
            panic!("unexpected binary skip");
        };
        let effective = reader.encoding();
        // Tiny output as well as tiny input reads exercise pending UTF-8 bytes.
        let mut result = Vec::new();
        let mut byte = [0];
        while reader.read(&mut byte).unwrap() != 0 {
            result.push(byte[0]);
        }
        (effective, result)
    }

    #[test]
    fn replay_and_utf8_bom_policy_preserve_invalid_bytes() {
        let input = b"\xef\xbb\xbfhello\xff\n";
        assert_eq!(
            decode(input, ContentEncoding::Auto, false),
            (ContentEncoding::Utf8, b"hello\xff\n".to_vec())
        );
        assert_eq!(decode(input, ContentEncoding::Bytes, false).1, input);
        assert_eq!(
            decode(b"\xef\xbb", ContentEncoding::Utf8, true).1,
            b"\xef\xbb"
        );
    }

    #[test]
    fn utf16_surrogate_pairs_and_line_breaks_cross_read_boundaries() {
        let symbols = "a🎸\r\né";
        for encoding in [ContentEncoding::Utf16Le, ContentEncoding::Utf16Be] {
            let mut bytes = if encoding == ContentEncoding::Utf16Le {
                UTF16_LE_BOM.to_vec()
            } else {
                UTF16_BE_BOM.to_vec()
            };
            for unit in symbols.encode_utf16() {
                let encoded = if encoding == ContentEncoding::Utf16Le {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                };
                bytes.extend(encoded);
            }
            assert_eq!(
                decode(&bytes, ContentEncoding::Auto, false),
                (encoding, symbols.as_bytes().to_vec())
            );
            assert_eq!(decode(&bytes[2..], encoding, false).1, symbols.as_bytes());
        }
    }

    #[test]
    fn malformed_utf16_preserves_the_lookahead_and_reports_odd_bytes() {
        let input = b"\x00\xd8Q\0\x00\xdc\x00\xd8\x21";
        assert_eq!(
            decode(input, ContentEncoding::Utf16Le, true).1,
            "�Q���".as_bytes()
        );
    }

    #[test]
    fn windows_1252_and_latin1_differ_only_in_the_c1_range() {
        let input = b"\x80\x91\x92\xa3\xe9\n";
        assert_eq!(
            decode(input, ContentEncoding::Windows1252, false).1,
            "€‘’£é\n".as_bytes()
        );
        assert_eq!(
            decode(input, ContentEncoding::Latin1, false).1,
            "\u{80}\u{91}\u{92}£é\n".as_bytes()
        );
    }

    #[test]
    fn binary_probe_skips_only_the_configured_prefix_unless_text_is_forced() {
        let input = &b"abc\0abcd"[..];
        assert!(matches!(
            prepare(OneByte(input), ContentEncoding::Auto, false, 4).unwrap(),
            PreparedInput::Binary { bytes_read: 4 }
        ));
        assert_eq!(decode(input, ContentEncoding::Auto, true).1, input);
        assert_eq!(decode(b"abcd\0", ContentEncoding::Auto, false).1, b"abcd\0");
    }

    #[test]
    fn accepts_encoding_aliases_and_gives_errors_for_unknown_labels() {
        assert_eq!(
            "UTF-16LE".parse::<ContentEncoding>().unwrap(),
            ContentEncoding::Utf16Le
        );
        assert_eq!(
            "cp1252".parse::<ContentEncoding>().unwrap(),
            ContentEncoding::Windows1252
        );
        assert_eq!(
            "banana".parse::<ContentEncoding>().unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
