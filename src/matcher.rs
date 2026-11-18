//! Filesystem-independent glob matching with stable input-pattern IDs.
use crate::pattern::{
    PathMatchScratch, Pattern, PatternProgram, expand_braces, expand_braces_escaped,
};
use crate::{CaseMode, QueryError};
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MatchOptions {
    pub case_mode: CaseMode,
    /// Unicode scalars for UTF-8; native non-UTF-8 names retain byte matching.
    /// This does not normalize Unicode text or do multi-character case folding.
    pub unicode: bool,
    /// Backslash quotes a literal rather than separating Windows components.
    pub escaped_patterns: bool,
}

/// Compiled path matching without opening files or enumerating directories.
/// Each input retains its index even when braces expand it.
#[derive(Clone, Debug)]
pub struct PathMatcher {
    programs: Vec<PatternProgram>,
    insensitive: bool,
    unicode: bool,
}

impl PathMatcher {
    pub fn new(patterns: &[impl AsRef<str>]) -> Result<Self, QueryError> {
        Self::with_options(patterns, MatchOptions::default())
    }
    pub fn with_options(
        patterns: &[impl AsRef<str>],
        options: MatchOptions,
    ) -> Result<Self, QueryError> {
        let insensitive = match options.case_mode {
            CaseMode::Sensitive => false,
            CaseMode::Insensitive => true,
            CaseMode::Smart => !patterns.iter().any(|pattern| {
                if options.unicode {
                    pattern.as_ref().chars().any(char::is_uppercase)
                } else {
                    pattern
                        .as_ref()
                        .bytes()
                        .any(|byte| byte.is_ascii_uppercase())
                }
            }),
        };
        let mut programs = Vec::with_capacity(patterns.len());
        for source in patterns {
            let expanded = if options.escaped_patterns {
                expand_braces_escaped(source.as_ref())?
            } else {
                expand_braces(source.as_ref())?
            };
            let patterns: Result<Vec<_>, _> = expanded
                .into_iter()
                .map(|pattern| {
                    if options.unicode {
                        Pattern::compile_unicode(pattern, options.escaped_patterns)
                    } else if options.escaped_patterns {
                        Pattern::compile_escaped(pattern)
                    } else {
                        Pattern::compile(pattern)
                    }
                })
                .collect();
            programs.push(PatternProgram::compile(&patterns?));
        }
        Ok(Self {
            programs,
            insensitive,
            unicode: options.unicode,
        })
    }
    pub fn is_match(&self, path: impl AsRef<Path>) -> bool {
        let mut scratch = PathMatchScratch::default();
        self.programs.iter().any(|program| {
            program.matches_path_with_mode_and_scratch(
                path.as_ref(),
                self.insensitive,
                self.unicode,
                &mut scratch,
            )
        })
    }
    pub fn matching_patterns(&self, path: impl AsRef<Path>) -> Vec<usize> {
        let mut scratch = PathMatchScratch::default();
        self.programs
            .iter()
            .enumerate()
            .filter_map(|(index, program)| {
                program
                    .matches_path_with_mode_and_scratch(
                        path.as_ref(),
                        self.insensitive,
                        self.unicode,
                        &mut scratch,
                    )
                    .then_some(index)
            })
            .collect()
    }
}

/// Quote a UTF-8 path for escaped_patterns matching. Native non-UTF-8 paths
/// can instead be passed directly to search visitors and command execution.
pub fn escape_path_pattern(path: impl AsRef<Path>) -> Result<String, QueryError> {
    let text = path.as_ref().to_str().ok_or_else(|| {
        QueryError("cannot express a non-UTF-8 native name as a text glob".into())
    })?;
    let text = if cfg!(windows) {
        text.replace('\\', "/")
    } else {
        text.to_owned()
    };
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if "*?[]{}()!+@\\".contains(character) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    Ok(escaped)
}
