use crate::error::Result;
use crate::filters::contains_bytes;
use crate::path::{os_string_from_pattern_bytes, with_os_bytes};
use crate::pattern::{Pattern, PatternProgram, expand_braces};
use std::ffi::OsStr;
use std::path::{Component, PathBuf};

/// Entry kinds returned by a query. Directory symlinks are never followed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryType {
    File,
    Dir,
    Symlink,
}

/// Filesystem selection, independent of terminal formatting and commands.
#[derive(Clone, Debug)]
pub struct Query {
    pub cwd: PathBuf,
    pub patterns: Vec<String>,
    pub exclusions: Vec<String>,
    pub search_terms: Vec<String>,
    pub extensions: Vec<String>,
    pub entry_type: EntryType,
    pub hidden: bool,
    pub gitignore: bool,
}
impl Default for Query {
    fn default() -> Self {
        Self {
            cwd: PathBuf::from("."),
            patterns: Vec::new(),
            exclusions: Vec::new(),
            search_terms: Vec::new(),
            extensions: Vec::new(),
            entry_type: EntryType::File,
            hidden: false,
            gitignore: false,
        }
    }
}
impl Query {
    /// Compile once and reuse the immutable plan for repeated executions.
    pub fn compile(&self) -> Result<CompiledQuery> {
        CompiledQuery::compile(self)
    }
}
/// Immutable compiled patterns and traversal policy. Executions read fresh
/// filesystem state; this is a query plan, not a cached directory listing.
#[derive(Clone, Debug)]
pub struct CompiledQuery {
    pub(crate) cwd: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) root_relative: PathBuf,
    pub(crate) positives: Vec<Pattern>,
    pub(crate) positive_program: PatternProgram,
    pub(crate) negative_program: PatternProgram,
    pub(crate) negatives: Vec<Pattern>,
    pub(crate) simple_terms: Vec<Vec<u8>>,
    pub(crate) extensions: Vec<Vec<u8>>,
    pub(crate) wanted_type: EntryType,
    pub(crate) hidden: bool,
    pub(crate) gitignore: bool,
}

impl CompiledQuery {
    pub fn compile(options: &Query) -> Result<Self> {
        let mut positives = Vec::new();
        for source in &options.patterns {
            for expanded in expand_braces(source)? {
                positives.push(Pattern::compile(expanded)?);
            }
        }
        if positives.is_empty() {
            positives.push(Pattern::compile("**/*".to_owned())?);
        }
        let mut negatives = Vec::new();
        for source in &options.exclusions {
            let source = source.strip_prefix('!').unwrap_or(source);
            for expanded in expand_braces(source)? {
                negatives.push(Pattern::compile(expanded)?);
            }
        }
        let common = common_literal_prefix(&positives);
        let root_relative = common.iter().fold(PathBuf::new(), |mut path, component| {
            path.push(os_string_from_pattern_bytes(component));
            path
        });
        let root = options.cwd.join(&root_relative);
        let positive_program = PatternProgram::compile(&positives);
        let negative_program = PatternProgram::compile(&negatives);
        Ok(Self {
            cwd: options.cwd.clone(),
            root,
            root_relative,
            positives,
            negatives,
            simple_terms: options
                .search_terms
                .iter()
                .map(|term| term.as_bytes().to_vec())
                .collect(),
            extensions: options
                .extensions
                .iter()
                .map(|ext| ext.trim_start_matches('.').as_bytes().to_vec())
                .collect(),
            wanted_type: options.entry_type,
            hidden: options.hidden,
            positive_program,
            negative_program,
            gitignore: options.gitignore,
        })
    }

    pub(crate) fn root_states(&self) -> (Vec<usize>, Vec<usize>) {
        let mut positive = self.positive_program.initial_states();
        let mut negative = self.negative_program.initial_states();
        for component in self.root_relative.components() {
            let Component::Normal(name) = component else {
                continue;
            };
            positive = self.positive_program.advance(&positive, name);
            negative = self.negative_program.advance(&negative, name);
        }
        (positive, negative)
    }

    pub(crate) fn states_match(
        &self,
        positive: &[usize],
        negative: &[usize],
        name: &OsStr,
    ) -> bool {
        self.positive_program.states_match(positive)
            && !self.negative_program.states_match(negative)
            && self.simple_matches(name)
    }

    pub(crate) fn simple_matches(&self, name: &OsStr) -> bool {
        self.simple_terms.is_empty()
            || with_os_bytes(name, |bytes| {
                self.simple_terms
                    .iter()
                    .any(|term| contains_bytes(bytes, term))
            })
    }

    pub(crate) fn extension_matches(&self, name: &OsStr) -> bool {
        if self.extensions.is_empty() {
            return true;
        }
        with_os_bytes(name, |bytes| {
            bytes
                .iter()
                .rposition(|byte| *byte == b'.')
                .map(|dot| &bytes[dot + 1..])
                .is_some_and(|ext| self.extensions.iter().any(|wanted| wanted == ext))
        })
    }
}

pub(crate) fn common_literal_prefix(patterns: &[Pattern]) -> Vec<Vec<u8>> {
    let Some(first) = patterns.first() else {
        return Vec::new();
    };
    let mut length = first.literal_prefix.len();
    for pattern in &patterns[1..] {
        length = length.min(pattern.literal_prefix.len());
        for index in 0..length {
            if first.literal_prefix[index] != pattern.literal_prefix[index] {
                length = index;
                break;
            }
        }
    }
    first.literal_prefix[..length].to_vec()
}
