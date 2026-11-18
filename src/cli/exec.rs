use crate::error::{AppError, Result};
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
const PLACEHOLDERS: [&str; 5] = ["{/.}", "{//}", "{/}", "{.}", "{}"];

pub(crate) fn validate_format(template: &str) -> Result<()> {
    let mut remaining = template;
    while !remaining.is_empty() {
        if remaining.starts_with("{{") || remaining.starts_with("}}") {
            remaining = &remaining[2..];
        } else if let Some(placeholder) = PLACEHOLDERS
            .iter()
            .find(|item| remaining.starts_with(**item))
        {
            remaining = &remaining[placeholder.len()..];
        } else {
            let character = remaining.chars().next().expect("nonempty");
            if matches!(character, '{' | '}') {
                return Err(AppError(
                    "unknown format placeholder; use {}, {/}, {//}, {.}, {/.}, {{ or }}".to_owned(),
                ));
            }
            remaining = &remaining[character.len_utf8()..];
        }
    }
    Ok(())
}

pub(crate) fn render_template(template: &str, path: &Path) -> OsString {
    let mut rendered = OsString::new();
    let mut remaining = template;
    while !remaining.is_empty() {
        if remaining.starts_with("{{") || remaining.starts_with("}}") {
            rendered.push(&remaining[..1]);
            remaining = &remaining[2..];
        } else if let Some(placeholder) = PLACEHOLDERS
            .iter()
            .find(|item| remaining.starts_with(**item))
        {
            let value = match *placeholder {
                "{/}" => path.file_name().unwrap_or_default().to_owned(),
                "{//}" => path
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                    .unwrap_or_else(|| Path::new("."))
                    .as_os_str()
                    .to_owned(),
                "{.}" => path.with_extension("").into_os_string(),
                "{/.}" => path.file_stem().unwrap_or_default().to_owned(),
                _ => path.as_os_str().to_owned(),
            };
            rendered.push(value);
            remaining = &remaining[placeholder.len()..];
        } else {
            let next = remaining
                .char_indices()
                .skip(1)
                .find(|(_, character)| matches!(character, '{' | '}'))
                .map_or(remaining.len(), |(index, _)| index);
            rendered.push(&remaining[..next]);
            remaining = &remaining[next..];
        }
    }
    rendered
}

pub(crate) fn command_arguments(command: &[String], path: &Path) -> Vec<OsString> {
    std::iter::once(OsString::from(&command[0]))
        .chain(
            command[1..]
                .iter()
                .map(|argument| render_template(argument, path)),
        )
        .collect()
}

pub(crate) fn validate_batch_template(command: &[String]) -> Result<()> {
    if command
        .first()
        .is_some_and(|argument| PLACEHOLDERS.contains(&argument.as_str()))
    {
        return Err(AppError(
            "--exec-batch command must name an executable before a placeholder".to_owned(),
        ));
    }
    let placeholders: Vec<_> = command
        .iter()
        .skip(1)
        .filter(|argument| contains_placeholder(argument))
        .collect();
    if placeholders.len() != 1 {
        return Err(AppError(
            "--exec-batch command must contain exactly one path placeholder".to_owned(),
        ));
    }
    if !PLACEHOLDERS.contains(&placeholders[0].as_str()) {
        return Err(AppError(
            "--exec-batch placeholder must be its own argument".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn batch_arguments(command: &[String], paths: &[PathBuf]) -> Vec<OsString> {
    let mut arguments = vec![OsString::from(&command[0])];
    for argument in &command[1..] {
        if PLACEHOLDERS.contains(&argument.as_str()) {
            arguments.extend(paths.iter().map(|path| render_template(argument, path)));
        } else {
            arguments.push(render_template(argument, Path::new("")));
        }
    }
    arguments
}

fn contains_placeholder(template: &str) -> bool {
    let mut remaining = template;
    while !remaining.is_empty() {
        if remaining.starts_with("{{") || remaining.starts_with("}}") {
            remaining = &remaining[2..];
        } else if PLACEHOLDERS
            .iter()
            .any(|placeholder| remaining.starts_with(placeholder))
        {
            return true;
        } else {
            remaining = &remaining[remaining.chars().next().expect("nonempty").len_utf8()..];
        }
    }
    false
}

/// Owns every spawned child and waits for it, including when a query errors.
/// Capacity bounds concurrent invocations, independently of traversal workers.
pub(crate) struct CommandPool {
    capacity: usize,
    children: std::collections::VecDeque<std::process::Child>,
}
impl CommandPool {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            capacity,
            children: std::collections::VecDeque::new(),
        }
    }
    pub(crate) fn submit(&mut self, arguments: &[OsString], cwd: &Path) -> io::Result<()> {
        if self.children.len() >= self.capacity {
            self.wait_one()?;
        }
        let child = Command::new(&arguments[0])
            .args(&arguments[1..])
            .current_dir(cwd)
            .spawn()?;
        self.children.push_back(child);
        if self.capacity == 1 {
            self.wait_one()?;
        }
        Ok(())
    }
    fn wait_one(&mut self) -> io::Result<()> {
        let Some(mut child) = self.children.pop_front() else {
            return Ok(());
        };
        let status = child.wait()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "command exited unsuccessfully: {status}"
            )))
        }
    }
    pub(crate) fn finish(&mut self) -> io::Result<()> {
        let mut first_error = None;
        while !self.children.is_empty() {
            if let Err(error) = self.wait_one() {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}
impl Drop for CommandPool {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

pub(crate) fn command_batch_base_bytes(command: &[String]) -> usize {
    command
        .iter()
        .filter(|argument| !PLACEHOLDERS.contains(&argument.as_str()))
        .map(|argument| {
            #[cfg(windows)]
            {
                argument.encode_utf16().count() * 2 + 2
            }
            #[cfg(not(windows))]
            {
                argument.len() + 1
            }
        })
        .sum()
}
pub(crate) fn parse_command_line(command: &str) -> Result<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut started = false;
    for character in command.chars() {
        if escaped {
            started = true;
            word.push(character);
            escaped = false;
        } else if character == '\\' && quote != Some('\'') {
            escaped = true;
        } else if let Some(active) = quote {
            if character == active {
                quote = None;
            } else {
                word.push(character);
            }
        } else if character == '\'' || character == '"' {
            quote = Some(character);
            started = true;
        } else if character.is_whitespace() {
            if started {
                words.push(std::mem::take(&mut word));
                started = false;
            }
        } else {
            word.push(character);
            started = true;
        }
    }
    if escaped || quote.is_some() {
        return Err(AppError(
            "unterminated quote or escape in --exec command".to_owned(),
        ));
    }
    if started {
        words.push(word);
    }
    if words.is_empty() || words[0].is_empty() {
        return Err(AppError("--exec command cannot be empty".to_owned()));
    }
    Ok(words)
}

#[cfg(test)]
mod workflow_tests {
    use super::*;

    #[test]
    fn command_parser_preserves_empty_quoted_arguments() {
        assert_eq!(
            parse_command_line("tool '' \"\" 'two words'").unwrap(),
            ["tool", "", "", "two words"]
        );
        assert!(parse_command_line("'' argument").is_err());
    }

    #[test]
    fn templates_preserve_native_components_and_escaped_braces() {
        let path = Path::new("dir").join("a file.tar.gz");
        assert_eq!(
            render_template("{/}|{/.}|{{}}", &path),
            OsString::from("a file.tar.gz|a file.tar|{}")
        );
        assert_eq!(
            render_template("{//}", Path::new("file.rs")),
            OsString::from(".")
        );
        assert_eq!(
            render_template("{.}", &path),
            path.with_extension("").as_os_str()
        );
    }

    #[test]
    fn batch_template_distinguishes_literal_braces_from_expansion() {
        let command = vec![
            "tool".to_owned(),
            "--literal={{}}".to_owned(),
            "{/}".to_owned(),
        ];
        validate_batch_template(&command).unwrap();
        let result = batch_arguments(
            &command,
            &[PathBuf::from("dir/a.rs"), PathBuf::from("dir/b.rs")],
        );
        assert_eq!(
            result,
            ["tool", "--literal={}", "a.rs", "b.rs"].map(OsString::from)
        );
        assert!(validate_batch_template(&["tool".into(), "{{}}".into()]).is_err());
    }
}
