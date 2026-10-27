use crate::error::{AppError, Result};
use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::Command;
pub(crate) fn run_command(command: &[String], path: &Path, cwd: &Path) -> io::Result<()> {
    let mut process = Command::new(&command[0]);
    process.current_dir(cwd);
    for argument in &command[1..] {
        let mut native = OsString::new();
        let mut parts = argument.split("{}");
        native.push(parts.next().unwrap_or_default());
        for part in parts {
            native.push(path.as_os_str());
            native.push(part);
        }
        process.arg(native);
    }
    let status = process.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "command exited unsuccessfully for {}: {status}",
            path.display()
        )))
    }
}
pub(crate) fn parse_command_line(command: &str) -> Result<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in command.chars() {
        if escaped {
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
        } else if character.is_whitespace() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(character);
        }
    }
    if escaped || quote.is_some() {
        return Err(AppError(
            "unterminated quote or escape in --exec command".to_owned(),
        ));
    }
    if !word.is_empty() {
        words.push(word);
    }
    if words.is_empty() {
        return Err(AppError("--exec command cannot be empty".to_owned()));
    }
    Ok(words)
}
