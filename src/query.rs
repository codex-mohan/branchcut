use crate::error::{AppError, Result};
use crate::filters::contains_bytes_with_case;
use crate::path::{os_string_from_pattern_bytes, with_os_bytes};
use crate::pattern::{
    Pattern, PatternProgram, Segment, SegmentMatcher, expand_braces, expand_braces_escaped,
};
use crate::{MetadataFilters, ModifiedBefore, SizeFilter};
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use std::time::SystemTime;
use std::{fs, io};

/// Entry kinds returned by a query. Directory symlinks are never followed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryType {
    File,
    Dir,
    Symlink,
    Socket,
    Fifo,
    BlockDevice,
    CharDevice,
}

impl EntryType {
    pub(crate) fn matches_type(self, kind: &std::fs::FileType) -> bool {
        match self {
            Self::File => kind.is_file(),
            Self::Dir => kind.is_dir(),
            Self::Symlink => kind.is_symlink(),
            #[cfg(unix)]
            other => {
                use std::os::unix::fs::FileTypeExt;
                match other {
                    Self::Socket => kind.is_socket(),
                    Self::Fifo => kind.is_fifo(),
                    Self::BlockDevice => kind.is_block_device(),
                    Self::CharDevice => kind.is_char_device(),
                    _ => false,
                }
            }
            #[cfg(not(unix))]
            _ => false,
        }
    }
}

/// ASCII case folding preserves non-ASCII native filename bytes exactly.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CaseMode {
    #[default]
    Sensitive,
    Insensitive,
    Smart,
}

/// Target of literal terms. Globs always match relative path components.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SearchTarget {
    #[default]
    Name,
    Path,
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
    /// Nonempty selections override entry_type and form a union.
    pub entry_types: Vec<EntryType>,
    pub and_terms: Vec<String>,
    pub require_empty: bool,
    pub require_executable: bool,
    pub owner: Option<u32>,
    pub group: Option<u32>,
    pub unicode: bool,
    pub escape_patterns: bool,
    pub follow_symlinks: bool,
    pub same_file_system: bool,
    pub max_open: usize,
    pub contents_first: bool,
    pub cancellation: Option<crate::CancellationToken>,
    pub hidden: bool,
    pub gitignore: bool,
    /// Discover both Git sources and hierarchical .ignore files.
    pub ignore: bool,
    /// Explicit Git-style ignore files, relative to cwd unless absolute.
    /// These override discovered ignore sources, in the order supplied.
    pub ignore_files: Vec<PathBuf>,
    pub case_mode: CaseMode,
    pub search_target: SearchTarget,
    pub min_depth: usize,
    pub max_depth: Option<usize>,
    pub sizes: Vec<SizeFilter>,
    pub modified_within: Option<Duration>,
    pub modified_before: Option<ModifiedBefore>,
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
            entry_types: Vec::new(),
            and_terms: Vec::new(),
            require_empty: false,
            require_executable: false,
            owner: None,
            group: None,
            unicode: false,
            escape_patterns: false,
            follow_symlinks: false,
            same_file_system: false,
            max_open: 64,
            contents_first: false,
            cancellation: None,
            hidden: false,
            gitignore: false,
            ignore: false,
            ignore_files: Vec::new(),
            case_mode: CaseMode::Sensitive,
            search_target: SearchTarget::Name,
            min_depth: 0,
            max_depth: None,
            sizes: Vec::new(),
            modified_within: None,
            modified_before: None,
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
    source_query: Query,
    pub(crate) cwd: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) root_relative: PathBuf,
    pub(crate) positives: Vec<Pattern>,
    pub(crate) positive_program: PatternProgram,
    pub(crate) positive_matches_all: bool,
    pub(crate) selection_all_non_root: bool,
    pub(crate) negative_program: PatternProgram,
    pub(crate) negatives: Vec<Pattern>,
    pub(crate) simple_terms: Vec<Vec<u8>>,
    pub(crate) extensions: Vec<Vec<u8>>,
    pub(crate) wanted_type: EntryType,
    pub(crate) entry_types: Vec<EntryType>,
    pub(crate) and_terms: Vec<Vec<u8>>,
    pub(crate) require_empty: bool,
    pub(crate) require_executable: bool,
    pub(crate) owner: Option<u32>,
    pub(crate) group: Option<u32>,
    pub(crate) unicode: bool,
    pub(crate) escape_patterns: bool,
    pub(crate) follow_symlinks: bool,
    pub(crate) same_file_system: bool,
    pub(crate) max_open: usize,
    pub(crate) contents_first: bool,
    pub(crate) hidden: bool,
    pub(crate) gitignore: bool,
    pub(crate) dot_ignore: bool,
    pub(crate) ignore_files: Vec<PathBuf>,
    pub(crate) case_insensitive: bool,
    pub(crate) search_target: SearchTarget,
    pub(crate) min_depth: usize,
    pub(crate) max_depth: Option<usize>,
    pub(crate) metadata: MetadataFilters,
}

impl CompiledQuery {
    pub(crate) fn cancelled(&self) -> bool {
        self.source_query
            .cancellation
            .as_ref()
            .is_some_and(crate::CancellationToken::is_cancelled)
    }
    /// Recompile this selection against another root; relative ignore files
    /// are resolved there, as they would be in a fresh Query.
    pub fn for_root(&self, root: impl Into<PathBuf>) -> Result<Self> {
        let mut query = self.source_query.clone();
        query.cwd = root.into();
        query.compile()
    }

    pub fn query(&self) -> &Query {
        &self.source_query
    }
    pub(crate) fn type_matches(&self, kind: &fs::FileType) -> bool {
        self.entry_types
            .iter()
            .any(|wanted| wanted.matches_type(kind))
    }

    pub(crate) fn needs_metadata(&self) -> bool {
        self.metadata.needs_metadata()
            || self.require_empty
            || self.require_executable
            || self.owner.is_some()
            || self.group.is_some()
    }

    pub(crate) fn matches_metadata(
        &self,
        metadata: &fs::Metadata,
        path: &Path,
        now: SystemTime,
    ) -> io::Result<bool> {
        if !self.metadata.matches(metadata, now)? {
            return Ok(false);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            if self.owner.is_some_and(|owner| metadata.uid() != owner)
                || self.group.is_some_and(|group| metadata.gid() != group)
                || self.require_executable
                    && (!metadata.is_file() || metadata.permissions().mode() & 0o111 == 0)
            {
                return Ok(false);
            }
        }
        #[cfg(not(unix))]
        if self.require_executable
            && (!metadata.is_file()
                || !path.extension().is_some_and(|ext| {
                    ["exe", "com", "bat", "cmd"]
                        .iter()
                        .any(|wanted| ext.eq_ignore_ascii_case(wanted))
                }))
        {
            return Ok(false);
        }
        if self.require_empty {
            return if metadata.is_dir() {
                match fs::read_dir(path)?.next() {
                    None => Ok(true),
                    Some(Ok(_)) => Ok(false),
                    Some(Err(error)) => Err(error),
                }
            } else {
                Ok(metadata.is_file() && metadata.len() == 0)
            };
        }
        Ok(true)
    }

    pub(crate) fn uses_ignores(&self) -> bool {
        self.gitignore || !self.ignore_files.is_empty()
    }

    pub fn compile(options: &Query) -> Result<Self> {
        let entry_types = if options.entry_types.is_empty() {
            vec![options.entry_type]
        } else {
            options.entry_types.clone()
        };
        if options.max_open == 0 {
            return Err(AppError("--max-open must be greater than zero".into()));
        }
        #[cfg(not(unix))]
        if options.owner.is_some()
            || options.group.is_some()
            || entry_types
                .iter()
                .any(|kind| !matches!(kind, EntryType::File | EntryType::Dir | EntryType::Symlink))
        {
            return Err(AppError(
                "ownership and special Unix entry types are unsupported on this platform".into(),
            ));
        }
        if options.max_depth.is_some_and(|max| options.min_depth > max) {
            return Err(AppError("--min-depth cannot exceed --max-depth".into()));
        }
        if !options.sizes.is_empty() && entry_types.iter().any(|kind| *kind != EntryType::File) {
            return Err(AppError("--size requires --type file".into()));
        }
        if options
            .search_terms
            .iter()
            .chain(&options.and_terms)
            .any(String::is_empty)
        {
            return Err(AppError("search terms cannot be empty".into()));
        }
        if options
            .extensions
            .iter()
            .any(|ext| ext.trim_start_matches('.').is_empty())
        {
            return Err(AppError("extension filters cannot be empty".into()));
        }
        let case_insensitive = match options.case_mode {
            CaseMode::Sensitive => false,
            CaseMode::Insensitive => true,
            CaseMode::Smart => !options
                .patterns
                .iter()
                .chain(&options.search_terms)
                .chain(&options.and_terms)
                .any(|text| {
                    if options.unicode {
                        text.chars().any(char::is_uppercase)
                    } else {
                        text.bytes().any(|byte| byte.is_ascii_uppercase())
                    }
                }),
        };
        let mut positives = Vec::new();
        for source in &options.patterns {
            for expanded in if options.escape_patterns {
                expand_braces_escaped(source)?
            } else {
                expand_braces(source)?
            } {
                positives.push(if options.unicode {
                    Pattern::compile_unicode(expanded, options.escape_patterns)?
                } else if options.escape_patterns {
                    Pattern::compile_escaped(expanded)?
                } else {
                    Pattern::compile(expanded)?
                });
            }
        }
        if positives.is_empty() {
            positives.push(Pattern::compile("**/*".to_owned())?);
        }
        let mut negatives = Vec::new();
        for source in &options.exclusions {
            let source = if source.starts_with("!(") {
                source.as_str()
            } else {
                source.strip_prefix('!').unwrap_or(source)
            };
            for expanded in if options.escape_patterns {
                expand_braces_escaped(source)?
            } else {
                expand_braces(source)?
            } {
                negatives.push(if options.unicode {
                    Pattern::compile_unicode(expanded, options.escape_patterns)?
                } else if options.escape_patterns {
                    Pattern::compile_escaped(expanded)?
                } else {
                    Pattern::compile(expanded)?
                });
            }
        }
        // A differently cased literal prefix may be a different directory.
        let common = if case_insensitive || options.follow_symlinks || options.same_file_system {
            Vec::new()
        } else {
            common_literal_prefix(&positives)
        };
        let root_relative = common.iter().fold(PathBuf::new(), |mut path, component| {
            path.push(os_string_from_pattern_bytes(component));
            path
        });
        let root = options.cwd.join(&root_relative);
        let positive_program = PatternProgram::compile(&positives);
        // Only this exact component language is specialized. All patterns are
        // still parsed/validated above; all remaining filters stay active.
        let positive_matches_all = positives.iter().any(|pattern| {
            matches!(pattern.segments.as_slice(), [Segment::GlobStar, Segment::Match(SegmentMatcher::Prefix(prefix))] if prefix.is_empty())
        });
        let selection_all_non_root = positive_matches_all
            && negatives.is_empty()
            && options.search_terms.is_empty()
            && options.and_terms.is_empty()
            && options.extensions.is_empty()
            && options.min_depth == 0
            && options.max_depth.is_none()
            && options.sizes.is_empty()
            && options.modified_within.is_none()
            && options.modified_before.is_none();
        let negative_program = PatternProgram::compile(&negatives);
        Ok(Self {
            source_query: options.clone(),
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
            entry_types,
            and_terms: options
                .and_terms
                .iter()
                .map(|term| term.as_bytes().to_vec())
                .collect(),
            require_empty: options.require_empty,
            require_executable: options.require_executable,
            owner: options.owner,
            group: options.group,
            unicode: options.unicode,
            escape_patterns: options.escape_patterns,
            follow_symlinks: options.follow_symlinks,
            same_file_system: options.same_file_system,
            max_open: options.max_open,
            contents_first: options.contents_first,
            hidden: options.hidden,
            positive_program,
            positive_matches_all,
            selection_all_non_root,
            negative_program,
            gitignore: options.gitignore || options.ignore,
            dot_ignore: options.ignore,
            ignore_files: options
                .ignore_files
                .iter()
                .map(|path| options.cwd.join(path))
                .collect(),
            case_insensitive,
            search_target: options.search_target,
            min_depth: options.min_depth,
            max_depth: options.max_depth,
            metadata: MetadataFilters {
                sizes: options.sizes.clone(),
                modified_within: options.modified_within,
                modified_before: options.modified_before,
            },
        })
    }

    pub(crate) fn root_states(&self) -> (Vec<usize>, Vec<usize>) {
        let mut positive = if self.positive_matches_all {
            Vec::new()
        } else {
            self.positive_program.initial_states()
        };
        let mut negative = self.negative_program.initial_states();
        for component in self.root_relative.components() {
            let Component::Normal(name) = component else {
                continue;
            };
            positive = self.positive_program.advance_with_mode(
                &positive,
                name,
                self.case_insensitive,
                self.unicode,
            );
            negative = self.negative_program.advance_with_mode(
                &negative,
                name,
                self.case_insensitive,
                self.unicode,
            );
        }
        (positive, negative)
    }

    pub(crate) fn states_match(
        &self,
        positive: &[usize],
        negative: &[usize],
        name: &OsStr,
        relative: &Path,
    ) -> bool {
        ((self.positive_matches_all && !relative.as_os_str().is_empty())
            || self.positive_program.states_match(positive))
            && !self.negative_program.states_match(negative)
            && self.simple_matches(name, relative)
    }

    pub(crate) fn states_match_name(
        &self,
        positive: &[usize],
        negative: &[usize],
        name: &OsStr,
    ) -> bool {
        (self.positive_matches_all || self.positive_program.states_match(positive))
            && !self.negative_program.states_match(negative)
            && (self.simple_terms.is_empty() && self.and_terms.is_empty()
                || self.search_target == SearchTarget::Name
                    && with_os_bytes(name, |bytes| self.simple_bytes_match(bytes)))
    }

    pub(crate) fn simple_matches(&self, name: &OsStr, relative: &Path) -> bool {
        let target = match self.search_target {
            SearchTarget::Name => name,
            SearchTarget::Path => relative.as_os_str(),
        };
        self.simple_terms.is_empty() && self.and_terms.is_empty()
            || with_os_bytes(target, |bytes| self.simple_bytes_match(bytes))
    }

    fn simple_bytes_match(&self, bytes: &[u8]) -> bool {
        let contains = |term: &[u8]| {
            if self.unicode
                && let (Ok(haystack), Ok(needle)) =
                    (std::str::from_utf8(bytes), std::str::from_utf8(term))
            {
                let normalize = |text: &str| {
                    let text = if cfg!(windows) && self.search_target == SearchTarget::Path {
                        text.replace('\\', "/")
                    } else {
                        text.to_owned()
                    };
                    if self.case_insensitive {
                        text.chars().map(crate::pattern::simple_lower).collect()
                    } else {
                        text
                    }
                };
                return normalize(haystack).contains(&normalize(needle));
            }
            contains_bytes_with_case(
                bytes,
                term,
                self.case_insensitive,
                cfg!(windows) && self.search_target == SearchTarget::Path,
            )
        };
        (self.simple_terms.is_empty() || self.simple_terms.iter().any(|term| contains(term)))
            && self.and_terms.iter().all(|term| contains(term))
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
                .is_some_and(|ext| {
                    self.extensions.iter().any(|wanted| {
                        if self.case_insensitive
                            && self.unicode
                            && let (Ok(wanted), Ok(ext)) =
                                (std::str::from_utf8(wanted), std::str::from_utf8(ext))
                        {
                            wanted
                                .chars()
                                .map(crate::pattern::simple_lower)
                                .eq(ext.chars().map(crate::pattern::simple_lower))
                        } else if self.case_insensitive {
                            wanted.eq_ignore_ascii_case(ext)
                        } else {
                            wanted == ext
                        }
                    })
                })
        })
    }

    pub(crate) fn depth_matches(&self, depth: usize) -> bool {
        depth >= self.min_depth && self.max_depth.is_none_or(|max| depth <= max)
    }

    pub(crate) fn can_descend(&self, depth: usize) -> bool {
        self.max_depth.is_none_or(|max| depth < max)
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
