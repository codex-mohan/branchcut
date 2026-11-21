use super::exec::render_template;
use super::output::{display_path, file_url, json_escape, write_path};
use crate::{ContentRecord, ContentRecordKind, ContentSpan};
use std::borrow::Cow;
use std::fmt::{self, Write as _};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::str::FromStr;
use std::time::UNIX_EPOCH;

const SCHEMA_VERSION: &str = "1";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ContentFormat {
    #[default]
    Text,
    Jsonl,
    Json,
    Csv,
    Tsv,
}

impl FromStr for ContentFormat {
    type Err = io::Error;

    fn from_str(input: &str) -> io::Result<Self> {
        match input {
            "text" | "plain" => Ok(Self::Text),
            "jsonl" | "ndjson" => Ok(Self::Jsonl),
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            "tsv" => Ok(Self::Tsv),
            _ => Err(invalid_input(format!(
                "unknown output format {input:?}; expected text, jsonl, json, csv, or tsv"
            ))),
        }
    }
}

impl fmt::Display for ContentFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Text => "text",
            Self::Jsonl => "jsonl",
            Self::Json => "json",
            Self::Csv => "csv",
            Self::Tsv => "tsv",
        })
    }
}

#[derive(Default)]
pub(crate) struct ContentFormatState {
    started: bool,
    has_records: bool,
    path_buffer: Vec<u8>,
}

#[derive(Clone, Copy)]
pub(crate) struct Row<'a> {
    pub(crate) path: &'a Path,
    pub(crate) record: Option<ContentRecord<'a>>,
    pub(crate) metadata: Option<&'a fs::Metadata>,
    pub(crate) eligibility: Option<&'a str>,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct FormatOptions<'a> {
    pub(crate) color: bool,
    pub(crate) hyperlink: bool,
    pub(crate) only_matching: bool,
    pub(crate) template: Option<&'a str>,
    pub(crate) link_path: Option<&'a Path>,
}

#[derive(Default)]
pub(crate) struct ContentSummary {
    pub(crate) matched: u64,
    pub(crate) files: u64,
    pub(crate) bytes: u64,
    pub(crate) binary: u64,
    pub(crate) errors: u64,
    pub(crate) dirs_pruned: u64,
    pub(crate) stopped: bool,
}

impl ContentFormatState {
    pub(crate) fn write(
        &mut self,
        writer: &mut impl Write,
        format: ContentFormat,
        row: Row<'_>,
        options: FormatOptions<'_>,
    ) -> io::Result<()> {
        if options.only_matching
            && let Some(record) = row.record
        {
            if record.kind == ContentRecordKind::Context {
                return Ok(());
            }
            for span in record.spans {
                let Some(text) = record.line.get(span.start..span.end) else {
                    continue;
                };
                if text.is_empty() {
                    continue;
                }
                self.write_row(
                    writer,
                    format,
                    Row {
                        record: Some(ContentRecord {
                            line: text,
                            spans: std::slice::from_ref(span),
                            ..record
                        }),
                        ..row
                    },
                    options,
                    span.start,
                )?;
            }
            return Ok(());
        }
        self.write_row(writer, format, row, options, 0)
    }

    fn write_row(
        &mut self,
        writer: &mut impl Write,
        format: ContentFormat,
        row: Row<'_>,
        options: FormatOptions<'_>,
        text_start: usize,
    ) -> io::Result<()> {
        if format == ContentFormat::Text {
            self.write_text_record(writer, row, options, text_start)
        } else {
            self.write_machine(writer, format, row, None, text_start)
        }
    }

    pub(crate) fn write_error(
        &mut self,
        writer: &mut impl Write,
        format: ContentFormat,
        path: &Path,
        error: &io::Error,
    ) -> io::Result<()> {
        if format == ContentFormat::Text {
            return Ok(());
        }
        self.write_machine(
            writer,
            format,
            Row {
                path,
                record: None,
                metadata: None,
                eligibility: None,
            },
            Some(error),
            0,
        )
    }

    pub(crate) fn finish(
        &mut self,
        writer: &mut impl Write,
        format: ContentFormat,
        summary: Option<&ContentSummary>,
    ) -> io::Result<()> {
        match format {
            ContentFormat::Text => {}
            ContentFormat::Json => {
                self.start(writer, format)?;
                writer.write_all(b"]")?;
                if let Some(summary) = summary {
                    writer.write_all(b",\"summary\":")?;
                    write_summary_json(writer, summary)?;
                }
                writer.write_all(b"}\n")?;
            }
            ContentFormat::Jsonl => {
                if let Some(summary) = summary {
                    write_summary_json(writer, summary)?;
                    writer.write_all(b"\n")?;
                }
            }
            ContentFormat::Csv | ContentFormat::Tsv => {
                self.start(writer, format)?;
                if let Some(summary) = summary {
                    let values = [
                        summary.matched,
                        summary.files,
                        summary.bytes,
                        summary.binary,
                        summary.errors,
                        summary.dirs_pruned,
                    ]
                    .map(|value| value.to_string());
                    let mut fields = blank_fields("summary");
                    let (stopped, summary_fields) = fields[SUMMARY_COLUMN..]
                        .split_last_mut()
                        .expect("summary has a stopped column");
                    for (field, value) in summary_fields.iter_mut().zip(&values) {
                        *field = value;
                    }
                    *stopped = if summary.stopped { "true" } else { "false" };
                    write_fields(writer, format, &fields)?;
                }
            }
        }
        Ok(())
    }

    fn start(&mut self, writer: &mut impl Write, format: ContentFormat) -> io::Result<()> {
        if self.started {
            return Ok(());
        }
        match format {
            ContentFormat::Json => write!(
                writer,
                "{{\"schema_version\":{SCHEMA_VERSION},\"records\":["
            )?,
            ContentFormat::Csv | ContentFormat::Tsv => write_fields(writer, format, &COLUMNS)?,
            ContentFormat::Text | ContentFormat::Jsonl => {}
        }
        self.started = true;
        Ok(())
    }

    fn write_machine(
        &mut self,
        writer: &mut impl Write,
        format: ContentFormat,
        row: Row<'_>,
        error: Option<&io::Error>,
        text_start: usize,
    ) -> io::Result<()> {
        self.start(writer, format)?;
        let kind = if error.is_some() { "error" } else { row.kind() };
        match format {
            ContentFormat::Jsonl | ContentFormat::Json => {
                if format == ContentFormat::Json && self.has_records {
                    writer.write_all(b",")?;
                }
                self.path_buffer.clear();
                write_path(&mut self.path_buffer, true, false, row.path)?;
                // Keep Branchcut's existing JSON path and native bytes fields.
                writer.write_all(&self.path_buffer[..self.path_buffer.len() - 2])?;
                write!(
                    writer,
                    ",\"schema_version\":{SCHEMA_VERSION},\"kind\":\"{kind}\""
                )?;
                if let Some(record) = row.record {
                    let text = String::from_utf8_lossy(record.line);
                    write!(
                        writer,
                        ",\"line\":{},\"column\":{},\"byte_offset\":{},\"text_start\":{text_start},\"text\":\"{}\",\"spans\":",
                        record.line_number,
                        column(&record),
                        record.byte_offset,
                        json_escape(&text)
                    )?;
                    write_spans(writer, record.spans)?;
                    if matches!(text, Cow::Owned(_)) {
                        let bytes = hex(record.line);
                        write!(writer, ",\"text_bytes\":\"{bytes}\"")?;
                    }
                }
                if let Some(eligibility) = row.eligibility {
                    write!(writer, ",\"eligibility\":\"{}\"", json_escape(eligibility))?;
                }
                if let Some(metadata) = row.metadata {
                    write!(
                        writer,
                        ",\"type\":\"file\",\"size\":{},\"readonly\":{}",
                        metadata.len(),
                        metadata.permissions().readonly()
                    )?;
                    if let Some(milliseconds) = modified_ms(metadata) {
                        write!(writer, ",\"modified_unix_ms\":{milliseconds}")?;
                    }
                }
                if let Some(error) = error {
                    write!(
                        writer,
                        ",\"error\":\"{}\",\"error_kind\":\"{:?}\"",
                        json_escape(&error.to_string()),
                        error.kind()
                    )?;
                }
                writer.write_all(b"}")?;
                if format == ContentFormat::Jsonl {
                    writer.write_all(b"\n")?;
                }
            }
            ContentFormat::Csv | ContentFormat::Tsv => {
                write_table_row(writer, format, row, kind, error, text_start)?
            }
            ContentFormat::Text => unreachable!("text is handled before writing machine output"),
        }
        self.has_records = true;
        Ok(())
    }

    fn write_text_record(
        &mut self,
        writer: &mut impl Write,
        row: Row<'_>,
        options: FormatOptions<'_>,
        text_start: usize,
    ) -> io::Result<()> {
        let link = options.link_path.filter(|_| options.hyperlink);
        if let Some(template) = options.template {
            self.render_template(writer, template, row, text_start, link)?;
        } else {
            self.write_text_path(writer, row.path, link)?;
            if let Some(record) = row.record {
                match record.kind {
                    ContentRecordKind::Context => {
                        write!(writer, "-{}-", record.line_number)?;
                        writer.write_all(record.line)?;
                    }
                    ContentRecordKind::Match => {
                        write!(writer, ":{}:{}:", record.line_number, column(&record))?;
                        if options.color && options.only_matching {
                            write_highlighted(writer, record.line)?;
                        } else if options.color {
                            write_colored(writer, record.line, record.spans)?;
                        } else {
                            writer.write_all(record.line)?;
                        }
                    }
                }
            }
            if let Some(eligibility) = row.eligibility {
                write!(writer, " [{eligibility}]")?;
            }
        }
        writer.write_all(b"\n")
    }

    fn write_text_path(
        &mut self,
        writer: &mut impl Write,
        path: &Path,
        link: Option<&Path>,
    ) -> io::Result<()> {
        if let Some(link) = link {
            write!(writer, "\x1b]8;;{}\x1b\\", file_url(link))?;
        }
        self.path_buffer.clear();
        write_path(&mut self.path_buffer, false, false, path)?;
        writer.write_all(&self.path_buffer[..self.path_buffer.len() - 1])?;
        if link.is_some() {
            writer.write_all(b"\x1b]8;;\x1b\\")?;
        }
        Ok(())
    }

    fn render_template(
        &mut self,
        writer: &mut impl Write,
        template: &str,
        row: Row<'_>,
        text_start: usize,
        link: Option<&Path>,
    ) -> io::Result<()> {
        for part in TemplateParts(template) {
            match part? {
                Part::Text(text) => writer.write_all(text.as_bytes())?,
                Part::Field("path" | "") => self.write_text_path(writer, row.path, link)?,
                Part::Field(field @ ("/" | "//" | "." | "/.")) => {
                    let component = render_template(&format!("{{{field}}}"), row.path);
                    self.write_text_path(writer, Path::new(&component), None)?;
                }
                Part::Field("kind") => writer.write_all(row.kind().as_bytes())?,
                Part::Field("eligibility") => {
                    writer.write_all(row.eligibility.unwrap_or_default().as_bytes())?
                }
                Part::Field(field) => {
                    if let Some(record) = row.record {
                        match field {
                            "line" => write!(writer, "{}", record.line_number)?,
                            "column" => write!(writer, "{}", column(&record))?,
                            "byte_offset" => write!(writer, "{}", record.byte_offset)?,
                            "text_start" => write!(writer, "{text_start}")?,
                            "text" => writer.write_all(record.line)?,
                            "spans" => write_spans(writer, record.spans)?,
                            "pattern" => {
                                if let Some(span) = record.spans.first() {
                                    write!(writer, "{}", span.pattern)?;
                                }
                            }
                            _ => unreachable!("template fields are validated by TemplateParts"),
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl Row<'_> {
    fn kind(&self) -> &'static str {
        match self.record.as_ref().map(|record| record.kind) {
            Some(ContentRecordKind::Match) => "match",
            Some(ContentRecordKind::Context) => "context",
            None => "file",
        }
    }
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn column(record: &ContentRecord<'_>) -> usize {
    record.spans.first().map_or(1, |span| span.start + 1)
}

fn modified_ms(metadata: &fs::Metadata) -> Option<i128> {
    Some(match metadata.modified().ok()?.duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_millis() as i128,
        Err(error) => -(error.duration().as_millis() as i128),
    })
}

fn write_spans(writer: &mut impl Write, spans: &[ContentSpan]) -> io::Result<()> {
    writer.write_all(b"[")?;
    for (index, span) in spans.iter().enumerate() {
        if index > 0 {
            writer.write_all(b",")?;
        }
        write!(
            writer,
            "{{\"start\":{},\"end\":{},\"pattern\":{}}}",
            span.start, span.end, span.pattern
        )?;
    }
    writer.write_all(b"]")
}

fn hex(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut result, "{byte:02x}").expect("writing to a String cannot fail");
    }
    result
}

fn write_summary_json(writer: &mut impl Write, summary: &ContentSummary) -> io::Result<()> {
    write!(
        writer,
        "{{\"schema_version\":{SCHEMA_VERSION},\"kind\":\"summary\",\"matched\":{},\"files\":{},\"bytes\":{},\"binary\":{},\"errors\":{},\"dirs_pruned\":{},\"stopped\":{}}}",
        summary.matched,
        summary.files,
        summary.bytes,
        summary.binary,
        summary.errors,
        summary.dirs_pruned,
        summary.stopped
    )
}

const SUMMARY_COLUMN: usize = 19;
const COLUMNS: [&str; SUMMARY_COLUMN + 7] = [
    "schema_version",
    "kind",
    "path",
    "path_bytes",
    "path_utf16",
    "line",
    "column",
    "byte_offset",
    "text_start",
    "text",
    "text_bytes",
    "spans",
    "eligibility",
    "error",
    "error_kind",
    "type",
    "size",
    "readonly",
    "modified_unix_ms",
    "matched",
    "files",
    "bytes",
    "binary",
    "errors",
    "dirs_pruned",
    "stopped",
];

fn blank_fields(kind: &str) -> [&str; COLUMNS.len()] {
    let mut fields = [""; COLUMNS.len()];
    fields[0] = SCHEMA_VERSION;
    fields[1] = kind;
    fields
}

fn write_table_row(
    writer: &mut impl Write,
    format: ContentFormat,
    row: Row<'_>,
    kind: &str,
    error: Option<&io::Error>,
    text_start: usize,
) -> io::Result<()> {
    let path = display_path(row.path);
    let (path_bytes, path_utf16) = native_tabular_path(row.path);
    let text = row.record.as_ref().map_or(Cow::Borrowed(""), |record| {
        String::from_utf8_lossy(record.line)
    });
    let mut spans = Vec::new();
    let mut text_bytes = None;
    let [line, col, offset, start] = if let Some(record) = row.record {
        if matches!(text, Cow::Owned(_)) {
            text_bytes = Some(hex(record.line));
        }
        write_spans(&mut spans, record.spans)?;
        [
            record.line_number.to_string(),
            column(&record).to_string(),
            record.byte_offset.to_string(),
            text_start.to_string(),
        ]
    } else {
        Default::default()
    };
    let mut fields = blank_fields(kind);
    let error_text = error.map(ToString::to_string);
    let error_kind = error.map(|error| format!("{:?}", error.kind()));
    let size = row.metadata.map(|metadata| metadata.len().to_string());
    let readonly = row.metadata.map(|metadata| {
        if metadata.permissions().readonly() {
            "true"
        } else {
            "false"
        }
    });
    let modified = row.metadata.and_then(modified_ms).map(|ms| ms.to_string());
    fields[2..SUMMARY_COLUMN].copy_from_slice(&[
        &path,
        &path_bytes,
        &path_utf16,
        &line,
        &col,
        &offset,
        &start,
        &text,
        text_bytes.as_deref().unwrap_or_default(),
        std::str::from_utf8(&spans).expect("match spans encode as ASCII"),
        row.eligibility.unwrap_or_default(),
        error_text.as_deref().unwrap_or_default(),
        error_kind.as_deref().unwrap_or_default(),
        if row.metadata.is_some() { "file" } else { "" },
        size.as_deref().unwrap_or_default(),
        readonly.unwrap_or_default(),
        modified.as_deref().unwrap_or_default(),
    ]);
    write_fields(writer, format, &fields)
}

fn native_tabular_path(path: &Path) -> (String, String) {
    if path.to_str().is_some() {
        return (String::new(), String::new());
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        (hex(path.as_os_str().as_bytes()), String::new())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let mut units = String::from("[");
        for (index, unit) in path.as_os_str().encode_wide().enumerate() {
            if index > 0 {
                units.push(',');
            }
            write!(&mut units, "{unit}").expect("writing to a String cannot fail");
        }
        units.push(']');
        (String::new(), units)
    }
    #[cfg(not(any(unix, windows)))]
    {
        (String::new(), String::new())
    }
}

fn write_fields(writer: &mut impl Write, format: ContentFormat, fields: &[&str]) -> io::Result<()> {
    let (separator, terminator) = if format == ContentFormat::Csv {
        (b",".as_slice(), b"\r\n".as_slice())
    } else {
        (b"\t".as_slice(), b"\n".as_slice())
    };
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            writer.write_all(separator)?;
        }
        if format == ContentFormat::Csv {
            write_csv_field(writer, field)?;
        } else {
            write_tsv_field(writer, field)?;
        }
    }
    writer.write_all(terminator)
}

fn write_csv_field(writer: &mut impl Write, field: &str) -> io::Result<()> {
    if !field
        .bytes()
        .any(|byte| matches!(byte, b',' | b'"' | b'\r' | b'\n'))
    {
        return writer.write_all(field.as_bytes());
    }
    writer.write_all(b"\"")?;
    for (index, part) in field.split('"').enumerate() {
        if index > 0 {
            writer.write_all(b"\"\"")?;
        }
        writer.write_all(part.as_bytes())?;
    }
    writer.write_all(b"\"")
}

fn write_tsv_field(writer: &mut impl Write, field: &str) -> io::Result<()> {
    let mut tail = field.as_bytes();
    while let Some(index) = tail
        .iter()
        .position(|byte| matches!(byte, b'\\' | b'\t' | b'\r' | b'\n' | 0))
    {
        writer.write_all(&tail[..index])?;
        writer.write_all(match tail[index] {
            b'\\' => b"\\\\",
            b'\t' => b"\\t",
            b'\r' => b"\\r",
            b'\n' => b"\\n",
            _ => b"\\0",
        })?;
        tail = &tail[index + 1..];
    }
    writer.write_all(tail)
}

fn write_highlighted(writer: &mut impl Write, text: &[u8]) -> io::Result<()> {
    writer.write_all(b"\x1b[1;32m")?;
    writer.write_all(text)?;
    writer.write_all(b"\x1b[0m")
}

fn write_colored(writer: &mut impl Write, line: &[u8], spans: &[ContentSpan]) -> io::Result<()> {
    let mut written = 0;
    let mut active: Option<(usize, usize)> = None;
    for span in spans {
        if span.start >= span.end || span.end > line.len() {
            continue;
        }
        match &mut active {
            Some((_, end)) if span.start <= *end => *end = (*end).max(span.end),
            _ => {
                if let Some((start, end)) = active {
                    writer.write_all(&line[written..start])?;
                    write_highlighted(writer, &line[start..end])?;
                    written = end;
                }
                active = Some((span.start, span.end));
            }
        }
    }
    if let Some((start, end)) = active {
        writer.write_all(&line[written..start])?;
        write_highlighted(writer, &line[start..end])?;
        written = end;
    }
    writer.write_all(&line[written..])
}

pub(crate) fn validate_template(template: &str) -> io::Result<()> {
    for part in TemplateParts(template) {
        part?;
    }
    Ok(())
}

enum Part<'a> {
    Text(&'a str),
    Field(&'a str),
}

struct TemplateParts<'a>(&'a str);

impl<'a> Iterator for TemplateParts<'a> {
    type Item = io::Result<Part<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        let input = self.0;
        let brace = input.find(['{', '}']).unwrap_or(input.len());
        if input.is_empty() {
            return None;
        }
        if brace > 0 {
            self.0 = &input[brace..];
            return Some(Ok(Part::Text(&input[..brace])));
        }
        if input.starts_with("{{") || input.starts_with("}}") {
            self.0 = &input[2..];
            return Some(Ok(Part::Text(&input[..1])));
        }
        self.0 = "";
        if input.starts_with('}') {
            return Some(Err(invalid_input(
                "unescaped closing brace in content template",
            )));
        }
        let Some(end) = input.find('}') else {
            return Some(Err(invalid_input("unclosed field in content template")));
        };
        let field = &input[1..end];
        if !matches!(
            field,
            "" | "path"
                | "/"
                | "//"
                | "."
                | "/."
                | "kind"
                | "eligibility"
                | "line"
                | "column"
                | "byte_offset"
                | "text_start"
                | "text"
                | "spans"
                | "pattern"
        ) {
            return Some(Err(invalid_input(format!(
                "unknown content template field {{{field}}}"
            ))));
        }
        self.0 = &input[end + 1..];
        Some(Ok(Part::Field(field)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row<'a>(line: &'a [u8], spans: &'a [ContentSpan]) -> Row<'a> {
        Row {
            path: Path::new("src/main.rs"),
            record: Some(ContentRecord {
                kind: ContentRecordKind::Match,
                line_number: 3,
                byte_offset: 11,
                line,
                spans,
            }),
            metadata: None,
            eligibility: None,
        }
    }

    #[test]
    fn json_document_starts_empty_and_separates_rows_from_summary() {
        let mut output = Vec::new();
        ContentFormatState::default()
            .finish(&mut output, ContentFormat::Json, None)
            .unwrap();
        assert_eq!(output, b"{\"schema_version\":1,\"records\":[]}\n");

        output.clear();
        let mut state = ContentFormatState::default();
        let record = row(b"a\xff\"", &[]);
        state
            .write(
                &mut output,
                ContentFormat::Json,
                record,
                FormatOptions::default(),
            )
            .unwrap();
        state
            .write_error(
                &mut output,
                ContentFormat::Json,
                record.path,
                &io::Error::new(io::ErrorKind::PermissionDenied, "cannot\nread"),
            )
            .unwrap();
        state
            .finish(
                &mut output,
                ContentFormat::Json,
                Some(&ContentSummary::default()),
            )
            .unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            concat!(
                "{\"schema_version\":1,\"records\":[",
                "{\"path\":\"src/main.rs\",\"schema_version\":1,\"kind\":\"match\",\"line\":3,\"column\":1,",
                "\"byte_offset\":11,\"text_start\":0,\"text\":\"a�\\\"\",\"spans\":[],\"text_bytes\":\"61ff22\"},",
                "{\"path\":\"src/main.rs\",\"schema_version\":1,\"kind\":\"error\",",
                "\"error\":\"cannot\\nread\",\"error_kind\":\"PermissionDenied\"}],",
                "\"summary\":{\"schema_version\":1,\"kind\":\"summary\",\"matched\":0,\"files\":0,",
                "\"bytes\":0,\"binary\":0,\"errors\":0,\"dirs_pruned\":0,\"stopped\":false}}\n",
            ),
        );
    }

    #[test]
    fn only_matching_keeps_decoded_line_positions_and_skips_context() {
        let span = ContentSpan {
            start: 2,
            end: 5,
            pattern: 1,
        };
        let record = row(b"\xc3\x9fhit!", std::slice::from_ref(&span));
        let mut context = record;
        context.record.as_mut().unwrap().kind = ContentRecordKind::Context;
        let mut output = Vec::new();
        let mut state = ContentFormatState::default();
        let options = FormatOptions {
            only_matching: true,
            ..FormatOptions::default()
        };
        state
            .write(&mut output, ContentFormat::Jsonl, context, options)
            .unwrap();
        state
            .write(&mut output, ContentFormat::Jsonl, record, options)
            .unwrap();
        assert_eq!(output, concat!(
            "{\"path\":\"src/main.rs\",\"schema_version\":1,\"kind\":\"match\",\"line\":3,\"column\":3,",
            "\"byte_offset\":11,\"text_start\":2,\"text\":\"hit\",\"spans\":[{\"start\":2,\"end\":5,\"pattern\":1}]}\n",
        ).as_bytes());
    }

    #[test]
    fn csv_quotes_delimiters_and_tsv_escapes_newlines_and_backslashes() {
        let mut output = Vec::new();
        write_fields(
            &mut output,
            ContentFormat::Csv,
            &["first", "a,\"b\"\r\nc", "last"],
        )
        .unwrap();
        assert_eq!(output, b"first,\"a,\"\"b\"\"\r\nc\",last\r\n");
        output.clear();
        write_fields(
            &mut output,
            ContentFormat::Tsv,
            &["first", "a\t\\\r\n\0", "last"],
        )
        .unwrap();
        assert_eq!(output, b"first\ta\\t\\\\\\r\\n\\0\tlast\n");

        output.clear();
        let mut state = ContentFormatState::default();
        state
            .write(
                &mut output,
                ContentFormat::Tsv,
                row(b"hello", &[]),
                FormatOptions::default(),
            )
            .unwrap();
        state
            .finish(
                &mut output,
                ContentFormat::Tsv,
                Some(&ContentSummary::default()),
            )
            .unwrap();
        for line in String::from_utf8(output).unwrap().lines() {
            assert_eq!(line.split('\t').count(), COLUMNS.len());
        }
    }

    #[test]
    fn templates_validate_and_render_escaped_braces() {
        assert!(validate_template("{unknown}").is_err());
        assert!(validate_template("{path").is_err());
        assert!(validate_template("{path}}").is_err());
        let template = "{{{path}}} {kind}:{line}:{column} {text}";
        validate_template(template).unwrap();
        let mut output = Vec::new();
        ContentFormatState::default()
            .write(
                &mut output,
                ContentFormat::Text,
                row(b"hello", &[]),
                FormatOptions {
                    template: Some(template),
                    ..FormatOptions::default()
                },
            )
            .unwrap();
        assert_eq!(output, b"{src/main.rs} match:3:1 hello\n");
    }
}
