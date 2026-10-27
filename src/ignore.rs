use crate::error::Result;
use crate::path::normal_components;
use crate::pattern::{Pattern, PatternProgram};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
#[derive(Clone, Debug)]
pub(crate) struct IgnoreRule {
    pub(crate) program: PatternProgram,
    pub(crate) base: PathBuf,
    pub(crate) negated: bool,
}

impl IgnoreRule {
    pub(crate) fn parse(line: &str, base: &Path) -> Option<Result<Self>> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let negated = line.starts_with('!');
        let mut source = line.strip_prefix('!').unwrap_or(line).to_owned();
        if source.ends_with('/') {
            source.pop();
            source.push_str("/**");
        }
        let anchored = source.starts_with('/');
        if anchored {
            source.remove(0);
        }
        let has_separator = source.contains('/');
        let compiled_source = if anchored || has_separator {
            source
        } else {
            format!("**/{source}")
        };
        Some(Pattern::compile(compiled_source).map(|pattern| Self {
            program: PatternProgram::compile(std::slice::from_ref(&pattern)),
            base: base.to_owned(),
            negated,
        }))
    }

    pub(crate) fn matches(&self, relative: &Path) -> bool {
        let Ok(local) = relative.strip_prefix(&self.base) else {
            return false;
        };
        self.program.matches_path(&normal_components(local))
    }
}

pub(crate) fn load_ignore_rules(path: &Path, base: &Path) -> io::Result<Vec<IgnoreRule>> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut rules = Vec::new();
    for line in contents.lines() {
        if let Some(rule) = IgnoreRule::parse(line, base) {
            rules.push(rule.map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.0))?);
        }
    }
    Ok(rules)
}

pub(crate) fn ignored_by_rules(rules: &[IgnoreRule], relative: &Path) -> (bool, bool) {
    let ignored = rules
        .iter()
        .filter(|rule| rule.matches(relative))
        .fold(false, |_, rule| !rule.negated);
    let may_reinclude = ignored && rules.iter().any(|rule| rule.negated);
    (ignored, may_reinclude)
}
