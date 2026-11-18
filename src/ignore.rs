use crate::error::Result;
use crate::pattern::{PathMatchScratch, Pattern, PatternProgram};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
mod sources;
pub(crate) use sources::initial_ignore_rules;

#[derive(Clone, Default)]
pub(crate) struct IgnoreRules {
    rules: Arc<Vec<IgnoreRule>>,
    case_insensitive: bool,
}

impl IgnoreRules {
    pub(crate) fn as_slice(&self) -> &[IgnoreRule] {
        self.rules.as_slice()
    }

    pub(crate) fn extend(&mut self, mut rules: Vec<IgnoreRule>) {
        if rules.is_empty() {
            return;
        }
        for rule in &mut rules {
            rule.case_insensitive = self.case_insensitive;
        }
        let mut combined = self.rules.as_ref().clone();
        for rule in rules {
            let insert_at = combined.partition_point(|existing| existing.priority <= rule.priority);
            combined.insert(insert_at, rule);
        }
        self.rules = Arc::new(combined);
    }

    pub(crate) fn extend_overrides(&mut self, mut rules: Vec<IgnoreRule>) {
        for rule in &mut rules {
            rule.priority = 2;
        }
        self.extend(rules);
    }
}
#[derive(Clone, Debug)]
pub(crate) struct IgnoreRule {
    pub(crate) program: PatternProgram,
    pub(crate) base: PathBuf,
    pub(crate) negated: bool,
    pub(crate) source: PathBuf,
    pub(crate) line: usize,
    pub(crate) text: String,
    directory_only: bool,
    contents_only: bool,
    // Inherited sources are anchored above cwd. Prepend cwd's location under
    // that ancestor when matching a cwd-relative candidate.
    candidate_prefix: PathBuf,
    case_insensitive: bool,
    priority: u8,
}

impl IgnoreRule {
    #[cfg(test)]
    pub(crate) fn parse(
        line: &str,
        base: &Path,
        source_path: &Path,
        line_number: usize,
    ) -> Option<Result<Self>> {
        Self::parse_bytes(line.as_bytes(), base, source_path, line_number)
    }

    fn parse_bytes(
        line: &[u8],
        base: &Path,
        source_path: &Path,
        line_number: usize,
    ) -> Option<Result<Self>> {
        let line = trim_unescaped_trailing_spaces(line);
        if line.is_empty() {
            return None;
        }
        // Git treats unescaped leading # and ! as comment and negation
        // markers. Quoting either marker makes it part of the filename.
        let (negated, pattern) = if line.starts_with(b"\\#") || line.starts_with(b"\\!") {
            (false, &line[1..])
        } else if line.starts_with(b"#") {
            return None;
        } else if let Some(pattern) = line.strip_prefix(b"!") {
            (true, pattern)
        } else {
            (false, line)
        };
        if pattern.is_empty() {
            return None;
        }
        // `compile_ignore` uses Git's backslash quoting rules, including
        // escaped spaces and wildcard characters. Preserve escape runs here:
        // reducing them before compilation would change even/odd slash parity.
        let mut pattern_source = pattern.to_owned();
        let directory_only = pattern_source.ends_with(b"/");
        let contents_only = pattern_source.ends_with(b"/**");
        if directory_only {
            pattern_source.pop();
        }
        let anchored = pattern_source.starts_with(b"/");
        if anchored {
            pattern_source.remove(0);
        }
        let has_separator = pattern_source.contains(&b'/');
        // A trailing slash marks a directory, not an anchoring separator.
        // Detect anchoring before appending the descendant-matching suffix.
        if directory_only {
            pattern_source.extend_from_slice(b"/**");
        }
        let compiled_source = if anchored || has_separator {
            pattern_source
        } else {
            [b"**/".as_slice(), pattern_source.as_slice()].concat()
        };
        Some(
            Pattern::compile_ignore_bytes(compiled_source).map(|pattern| Self {
                program: PatternProgram::compile(std::slice::from_ref(&pattern)),
                base: base.to_owned(),
                negated,
                source: source_path.to_owned(),
                line: line_number,
                text: String::from_utf8_lossy(line).into_owned(),
                directory_only,
                contents_only,
                candidate_prefix: PathBuf::new(),
                case_insensitive: false,
                priority: 0,
            }),
        )
    }

    #[cfg(test)]
    pub(crate) fn matches(&self, relative: &Path, is_dir: bool) -> bool {
        self.matches_with_scratch(relative, is_dir, &mut PathMatchScratch::default())
    }

    pub(crate) fn matches_with_scratch(
        &self,
        relative: &Path,
        is_dir: bool,
        scratch: &mut PathMatchScratch,
    ) -> bool {
        let scoped;
        let relative = if self.candidate_prefix.as_os_str().is_empty() {
            relative
        } else {
            scoped = self.candidate_prefix.join(relative);
            &scoped
        };
        let Ok(local) = relative.strip_prefix(&self.base) else {
            return false;
        };
        // An ignore file controls entries below its directory, never that
        // directory itself. In particular, ** must not match the empty base.
        if local.as_os_str().is_empty() {
            return false;
        }
        // A directory-only rule may cover a file's parent, but must never
        // exclude a regular file or symlink solely because its name matches.
        let candidate = if self.contents_only || (self.directory_only && !is_dir) {
            local.parent().unwrap_or_else(|| Path::new(""))
        } else {
            local
        };
        if self.case_insensitive {
            self.program
                .matches_path_with_case_and_scratch(candidate, true, scratch)
        } else {
            self.program.matches_path_with_scratch(candidate, scratch)
        }
    }
}

/// Unescaped trailing ASCII spaces are insignificant in gitignore patterns.
/// An odd run of backslashes quotes the final space, so preserve it for the
/// pattern parser to consume as a literal.
fn trim_unescaped_trailing_spaces(line: &[u8]) -> &[u8] {
    let bytes = line;
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1] == b' ' {
        let mut slashes = 0;
        let mut cursor = end - 1;
        while cursor > 0 && bytes[cursor - 1] == b'\\' {
            cursor -= 1;
            slashes += 1;
        }
        if slashes % 2 == 1 {
            break;
        }
        end -= 1;
    }
    &line[..end]
}

pub(crate) fn load_ignore_rules(path: &Path, base: &Path) -> io::Result<Vec<IgnoreRule>> {
    let contents = match fs::read(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut rules = Vec::new();
    let contents = contents.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&contents);
    for (index, line) in contents.split(|byte| *byte == b'\n').enumerate() {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if let Some(rule) = IgnoreRule::parse_bytes(line, base, path, index + 1) {
            rules.push(rule.map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.0))?);
        }
    }
    Ok(rules)
}

/// Git does not follow a worktree .gitignore symlink. Explicit external ignore
/// sources use the ordinary file loader instead.
pub(crate) fn load_worktree_rules(path: &Path, base: &Path) -> io::Result<Vec<IgnoreRule>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Ok(Vec::new()),
        Ok(_) => load_ignore_rules(path, base),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

/// Enumerate per-directory sources in load order. IgnoreRules preserves their
/// cross-directory priority: .ignore overrides Git rules, explicit files win.
pub(crate) fn directory_rule_sources(
    plan: &crate::CompiledQuery,
    absolute: &Path,
    base: &Path,
) -> Vec<(PathBuf, io::Result<Vec<IgnoreRule>>)> {
    let mut sources = Vec::new();
    if plan.gitignore {
        let path = absolute.join(".gitignore");
        let rules = load_worktree_rules(&path, base);
        sources.push((path, rules));
    }
    if plan.dot_ignore {
        let path = absolute.join(".ignore");
        let rules = load_worktree_rules(&path, base).map(|mut rules| {
            for rule in &mut rules {
                rule.priority = 1;
            }
            rules
        });
        sources.push((path, rules));
    }
    sources
}

pub(crate) fn deciding_rule<'a>(
    rules: &'a [IgnoreRule],
    relative: &Path,
    is_dir: bool,
) -> Option<&'a IgnoreRule> {
    let component_count = relative
        .components()
        .filter(|component| matches!(component, Component::Normal(_)))
        .count();
    let mut prefix = PathBuf::new();
    let mut last_matching_rule = None;
    let mut scratch = PathMatchScratch::default();
    for (index, component) in relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(component) => Some(component),
            _ => None,
        })
        .enumerate()
    {
        prefix.push(component);
        let prefix_is_dir = index + 1 < component_count || is_dir;
        if let Some(rule) = rules
            .iter()
            .rev()
            .find(|rule| rule.matches_with_scratch(&prefix, prefix_is_dir, &mut scratch))
        {
            last_matching_rule = Some(rule);
            if rule.negated {
                continue;
            }
            // An ignored parent cannot be re-included by a later child rule.
            // Returning it first also gives --why the rule Git uses to omit
            // the candidate without descending into that directory.
            return Some(rule);
        }
    }
    last_matching_rule
}

pub(crate) fn ignored_by_rules(
    rules: &[IgnoreRule],
    relative: &Path,
    is_dir: bool,
) -> (bool, bool) {
    let ignored = deciding_rule(rules, relative, is_dir).is_some_and(|rule| !rule.negated);
    (ignored, false)
}

/// Traversal has already admitted every parent directory. Rechecking ancestors
/// for each entry is redundant; explanations and root preparation still use
/// the full ancestor-aware decision above.
pub(crate) fn ignored_child_by_rules(
    rules: &[IgnoreRule],
    relative: &Path,
    is_dir: bool,
    scratch: &mut PathMatchScratch,
) -> bool {
    rules
        .iter()
        .rev()
        .find(|rule| rule.matches_with_scratch(relative, is_dir, scratch))
        .is_some_and(|rule| !rule.negated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(source: &str) -> Option<IgnoreRule> {
        IgnoreRule::parse(source, Path::new(""), Path::new(".gitignore"), 1).map(Result::unwrap)
    }

    #[test]
    fn comments_and_escaped_leading_markers_follow_gitignore_rules() {
        assert!(rule("# comment").is_none());
        assert!(
            rule("\\#literal")
                .unwrap()
                .matches(Path::new("#literal"), false)
        );
        assert!(!rule("\\#literal").unwrap().negated);
        assert!(
            rule("\\!literal")
                .unwrap()
                .matches(Path::new("!literal"), false)
        );
        assert!(!rule("\\!literal").unwrap().negated);
        assert!(rule("!literal").unwrap().negated);
        assert!(rule(" ").is_none());
    }

    #[test]
    fn escaped_spaces_are_literal_and_only_unescaped_trailing_spaces_are_trimmed() {
        assert!(
            rule("file\\ name")
                .unwrap()
                .matches(Path::new("file name"), false)
        );
        assert!(
            rule("file\\ with\\ spaces  ")
                .unwrap()
                .matches(Path::new("file with spaces"), false)
        );
        assert!(rule("name\\ ").unwrap().matches(Path::new("name "), false));
        assert!(
            rule("name\\   ")
                .unwrap()
                .matches(Path::new("name "), false)
        );
        assert!(rule("name   ").unwrap().matches(Path::new("name"), false));
        assert!(!rule("name   ").unwrap().matches(Path::new("name "), false));
        assert!(
            rule("literal\\*.rs")
                .unwrap()
                .matches(Path::new("literal*.rs"), false)
        );
        #[cfg(unix)]
        {
            assert!(
                rule("back\\\\ space")
                    .unwrap()
                    .matches(Path::new("back\\ space"), false)
            );
            assert!(
                rule(r"back\\\ space")
                    .unwrap()
                    .matches(Path::new(r"back\ space"), false)
            );
        }
    }

    #[test]
    fn slash_anchoring_directory_only_and_ignore_file_base_are_distinct() {
        let anchored = IgnoreRule::parse(
            "/target/",
            Path::new("packages/core"),
            Path::new("packages/core/.gitignore"),
            1,
        )
        .unwrap()
        .unwrap();
        assert!(anchored.matches(Path::new("packages/core/target"), true));
        assert!(!anchored.matches(Path::new("packages/core/nested/target"), true));
        assert!(!anchored.matches(Path::new("packages/other/target"), true));

        let unanchored = IgnoreRule::parse(
            "target/",
            Path::new("packages/core"),
            Path::new("packages/core/.gitignore"),
            1,
        )
        .unwrap()
        .unwrap();
        assert!(unanchored.matches(Path::new("packages/core/nested/target"), true));
        assert!(!unanchored.matches(Path::new("packages/other/target"), true));
        assert!(!unanchored.matches(Path::new("packages/core/target"), false));
        assert!(unanchored.matches(Path::new("packages/core/target/file.rs"), false));
    }
}
