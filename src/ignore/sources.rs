//! Resolve ignore sources once per execution, without invoking Git or walking
//! unrelated subtrees. Rules remain in increasing precedence order.
use super::{IgnoreRule, IgnoreRules, deciding_rule, directory_rule_sources, load_ignore_rules};
use std::path::{Path, PathBuf};
use std::{env, fs, io};

pub(crate) struct InitialIgnoreRules {
    pub(crate) rules: IgnoreRules,
    pub(crate) blocked: Option<IgnoreRule>,
    pub(crate) errors: Vec<(PathBuf, io::Error)>,
}

struct Repository {
    root: PathBuf,
    git_dir: PathBuf,
    common_dir: PathBuf,
}

#[derive(Default)]
struct GitConfig {
    excludes_file: Option<PathBuf>,
    ignore_case: bool,
    worktree_config: bool,
}

pub(crate) fn initial_ignore_rules(plan: &crate::CompiledQuery) -> io::Result<InitialIgnoreRules> {
    let mut initial = if plan.gitignore {
        discovered_ignore_rules(plan)?
    } else {
        InitialIgnoreRules {
            rules: IgnoreRules::default(),
            blocked: None,
            errors: Vec::new(),
        }
    };
    for path in &plan.ignore_files {
        // Explicitly named files are required; discovered sources may be absent.
        let loaded = fs::metadata(path).and_then(|_| load_ignore_rules(path, Path::new("")));
        match loaded {
            Ok(rules) => initial.rules.extend_overrides(rules),
            Err(error) => initial.errors.push((path.to_owned(), error)),
        }
    }
    Ok(initial)
}

fn discovered_ignore_rules(plan: &crate::CompiledQuery) -> io::Result<InitialIgnoreRules> {
    let cwd = std::path::absolute(&plan.cwd)?;
    let mut errors = Vec::new();
    let repository = discover_repository(&cwd, &mut errors);
    let base = repository
        .as_ref()
        .map_or(cwd.as_path(), |repo| repo.root.as_path());
    let mut config = GitConfig::default();
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    let xdg = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home.as_ref().map(|home| home.join(".config")));

    {
        let mut loader = ConfigLoader {
            home: home.as_deref(),
            cwd: &cwd,
            repository: repository.as_ref(),
            errors: &mut errors,
            stack: Vec::new(),
        };
        // Do not guess a Git installation prefix. Explicit system config paths
        // are honored; user/repository configuration does not require Git.
        let use_system = match env::var_os("GIT_CONFIG_NOSYSTEM") {
            None => true,
            Some(value) => match value.to_str().and_then(config_boolean) {
                Some(disabled) => !disabled,
                None => {
                    loader.invalid(
                        Path::new("GIT_CONFIG_NOSYSTEM"),
                        "invalid Git boolean in GIT_CONFIG_NOSYSTEM",
                    );
                    false
                }
            },
        };
        if use_system && let Some(system) = env::var_os("GIT_CONFIG_SYSTEM") {
            loader.read(&PathBuf::from(system), &mut config, false);
        }
        if let Some(global) = env::var_os("GIT_CONFIG_GLOBAL") {
            loader.read(&PathBuf::from(global), &mut config, false);
        } else {
            if let Some(xdg) = &xdg {
                loader.read(&xdg.join("git/config"), &mut config, false);
            }
            if let Some(home) = &home {
                loader.read(&home.join(".gitconfig"), &mut config, false);
            }
        }
        if let Some(repo) = &repository {
            loader.read(&repo.common_dir.join("config"), &mut config, true);
            if config.worktree_config {
                loader.read(&repo.git_dir.join("config.worktree"), &mut config, false);
            }
        }
    }
    let mut rules = IgnoreRules {
        case_insensitive: config.ignore_case,
        ..IgnoreRules::default()
    };
    let global_ignore = config
        .excludes_file
        .or_else(|| xdg.map(|path| path.join("git/ignore")));
    if let Some(path) = global_ignore.filter(|path| !path.as_os_str().is_empty()) {
        append_rules(&mut rules, &path, Path::new(""), &mut errors);
    }
    if let Some(repo) = &repository {
        append_rules(
            &mut rules,
            &repo.common_dir.join("info/exclude"),
            Path::new(""),
            &mut errors,
        );
    }

    // Load parent .gitignore files before cwd. The walk loads cwd's own file
    // when entering it; do not duplicate that work or reset nested precedence.
    let ancestors: Vec<_> = if repository.is_some() {
        cwd.ancestors().take_while(|path| *path != base).collect()
    } else {
        vec![cwd.as_path()]
    };
    let mut blocked = None;
    for directory in std::iter::once(base).chain(ancestors.iter().rev().copied()) {
        let relative = directory.strip_prefix(base).unwrap_or(Path::new(""));
        if !relative.as_os_str().is_empty()
            && let Some(rule) = deciding_rule(rules.as_slice(), relative, true)
            && !rule.negated
        {
            blocked = Some(rule.clone());
            break;
        }
        if directory != cwd {
            for (path, loaded) in directory_rule_sources(plan, directory, relative) {
                match loaded {
                    Ok(loaded) => rules.extend(loaded),
                    Err(error) => errors.push((path, error)),
                }
            }
        }
    }
    let prefix = cwd.strip_prefix(base).unwrap_or(Path::new(""));
    if !prefix.as_os_str().is_empty() {
        let adjusted = rules
            .rules
            .iter()
            .cloned()
            .map(|mut rule| {
                rule.candidate_prefix = prefix.to_owned();
                rule
            })
            .collect();
        rules.rules = std::sync::Arc::new(adjusted);
    }
    Ok(InitialIgnoreRules {
        rules,
        blocked,
        errors,
    })
}

fn append_rules(
    rules: &mut IgnoreRules,
    path: &Path,
    base: &Path,
    errors: &mut Vec<(PathBuf, io::Error)>,
) {
    match load_ignore_rules(path, base) {
        Ok(loaded) => rules.extend(loaded),
        Err(error) => errors.push((path.to_owned(), error)),
    }
}

fn discover_repository(cwd: &Path, errors: &mut Vec<(PathBuf, io::Error)>) -> Option<Repository> {
    for root in cwd.ancestors() {
        let marker = root.join(".git");
        let metadata = match fs::symlink_metadata(&marker) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => {
                errors.push((marker, error));
                return None;
            }
        };
        let git_dir = if metadata.is_dir() {
            marker
        } else if metadata.is_file() {
            let text = read_optional(&marker, errors)?;
            let Some(path) = text
                .trim()
                .strip_prefix("gitdir:")
                .map(str::trim)
                .filter(|path| !path.is_empty())
            else {
                errors.push((
                    marker,
                    io::Error::new(io::ErrorKind::InvalidData, "invalid gitdir file"),
                ));
                return None;
            };
            root.join(path)
        } else {
            continue;
        };
        let common_dir = read_optional(&git_dir.join("commondir"), errors)
            .map_or_else(|| git_dir.clone(), |text| git_dir.join(text.trim()));
        return Some(Repository {
            root: root.to_owned(),
            git_dir,
            common_dir,
        });
    }
    None
}

fn read_optional(path: &Path, errors: &mut Vec<(PathBuf, io::Error)>) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            errors.push((path.to_owned(), error));
            None
        }
    }
}

struct ConfigLoader<'a> {
    home: Option<&'a Path>,
    cwd: &'a Path,
    repository: Option<&'a Repository>,
    errors: &'a mut Vec<(PathBuf, io::Error)>,
    stack: Vec<PathBuf>,
}

impl ConfigLoader<'_> {
    fn invalid(&mut self, path: &Path, message: impl Into<String>) {
        self.errors.push((
            path.to_owned(),
            io::Error::new(io::ErrorKind::InvalidData, message.into()),
        ));
    }

    fn read(&mut self, path: &Path, config: &mut GitConfig, repository_scope: bool) {
        if self.stack.len() >= 10 {
            self.invalid(path, "Git config include depth exceeds 10");
            return;
        }
        let Some(text) = read_optional(path, self.errors) else {
            return;
        };
        self.stack.push(path.to_owned());
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        let mut section = (String::new(), None);
        let mut logical_line = String::new();
        for physical in text.lines() {
            if logical_line.is_empty() {
                logical_line.push_str(physical.trim_start());
            } else {
                logical_line.push_str(physical);
            }
            if logical_line
                .bytes()
                .rev()
                .take_while(|byte| *byte == b'\\')
                .count()
                % 2
                == 1
            {
                logical_line.pop();
                continue;
            }
            let line = logical_line.trim_start();
            if line.starts_with('[') {
                match config_section(line) {
                    Some(parsed) => section = parsed,
                    None => self.invalid(path, "invalid Git config section"),
                }
            } else if !line.is_empty() && !line.starts_with(['#', ';']) {
                let (key, value) = line
                    .split_once('=')
                    .map_or((line.trim(), "true"), |(key, value)| {
                        (key.trim(), value.trim_start())
                    });
                let key = key.to_ascii_lowercase();
                let relevant = matches!(
                    (section.0.as_str(), key.as_str()),
                    ("core", "excludesfile" | "ignorecase")
                        | ("extensions", "worktreeconfig")
                        | ("include" | "includeif", "path")
                );
                if relevant {
                    match config_value(value) {
                        Some(value) => {
                            self.apply(path, &section, &key, &value, config, repository_scope)
                        }
                        None => self.invalid(path, "invalid quoted Git config value"),
                    }
                }
            }
            logical_line.clear();
        }
        if !logical_line.is_empty() {
            self.invalid(path, "unterminated Git config continuation");
        }
        self.stack.pop();
    }

    fn apply(
        &mut self,
        path: &Path,
        section: &(String, Option<String>),
        key: &str,
        value: &str,
        config: &mut GitConfig,
        repository_scope: bool,
    ) {
        match (section.0.as_str(), key) {
            ("core", "excludesfile") if section.1.is_none() => {
                let root = self.repository.map_or(self.cwd, |repo| repo.root.as_path());
                match expand_path(value, self.home, root) {
                    Ok(value) => config.excludes_file = Some(value),
                    Err(error) => self.invalid(path, error),
                }
            }
            ("core", "ignorecase") if section.1.is_none() => match config_boolean(value) {
                Some(value) => config.ignore_case = value,
                None => self.invalid(
                    path,
                    format!("invalid Git boolean for core.ignoreCase: {value:?}"),
                ),
            },
            ("extensions", "worktreeconfig") if section.1.is_none() && repository_scope => {
                match config_boolean(value) {
                    Some(value) => config.worktree_config = value,
                    None => self.invalid(
                        path,
                        format!("invalid Git boolean for extensions.worktreeConfig: {value:?}"),
                    ),
                }
            }
            ("include" | "includeif", "path") => {
                let enabled = section.0 == "include"
                    || section
                        .1
                        .as_ref()
                        .is_some_and(|condition| self.condition_matches(condition, path));
                if enabled {
                    match expand_path(value, self.home, path.parent().unwrap_or(self.cwd)) {
                        Ok(include) => self.read(&include, config, repository_scope),
                        Err(error) => self.invalid(path, error),
                    }
                }
            }
            _ => {}
        }
    }

    fn condition_matches(&mut self, condition: &str, config_path: &Path) -> bool {
        let (gitdir_pattern, insensitive) =
            if let Some(pattern) = condition.strip_prefix("gitdir/i:") {
                (Some(pattern), true)
            } else {
                (condition.strip_prefix("gitdir:"), false)
            };
        if let Some(pattern) = gitdir_pattern {
            let Some(repo) = self.repository else {
                return false;
            };
            let pattern = if let Some(relative) = pattern.strip_prefix("./") {
                config_path
                    .parent()
                    .unwrap_or(self.cwd)
                    .join(relative)
                    .to_string_lossy()
                    .replace('\\', "/")
            } else if pattern.starts_with('~') {
                match expand_path(pattern, self.home, self.cwd) {
                    Ok(path) => path.to_string_lossy().replace('\\', "/"),
                    Err(error) => {
                        self.invalid(config_path, error);
                        return false;
                    }
                }
            } else {
                pattern.to_owned()
            };
            let mut pattern =
                if pattern.starts_with('/') || pattern.as_bytes().get(1) == Some(&b':') {
                    pattern
                } else {
                    format!("**/{pattern}")
                };
            if pattern.ends_with('/') {
                pattern.push_str("**");
            }
            let lexical = format!(
                "{}/",
                repo.git_dir
                    .to_string_lossy()
                    .replace('\\', "/")
                    .trim_end_matches('/')
            );
            if condition_glob_matches(&pattern, &lexical, insensitive) {
                return true;
            }
            // Git matches both the lexical and real gitdir location when a
            // symlink is involved. This runs once during source discovery.
            return fs::canonicalize(&repo.git_dir).ok().is_some_and(|path| {
                let canonical = format!(
                    "{}/",
                    path.to_string_lossy()
                        .replace('\\', "/")
                        .trim_end_matches('/')
                );
                condition_glob_matches(
                    &pattern,
                    canonical.strip_prefix("//?/").unwrap_or(&canonical),
                    insensitive,
                )
            });
        }
        let Some(pattern) = condition.strip_prefix("onbranch:") else {
            self.invalid(config_path, format!("unsupported Git includeIf condition {condition:?}; use --ignore-file for an explicit source"));
            return false;
        };
        let Some(repo) = self.repository else {
            return false;
        };
        let Some(head) = read_optional(&repo.git_dir.join("HEAD"), self.errors) else {
            return false;
        };
        let Some(branch) = head.trim().strip_prefix("ref: refs/heads/") else {
            return false;
        };
        let pattern = if pattern.ends_with('/') {
            format!("{pattern}**")
        } else {
            pattern.to_owned()
        };
        condition_glob_matches(&pattern, branch, false)
    }
}

fn condition_glob_matches(pattern: &str, candidate: &str, insensitive: bool) -> bool {
    use crate::pattern::{PathMatchScratch, Pattern, PatternProgram};
    // Git's terminal /** requires a slash; traversal globstar also matches its
    // parent. Keep config-condition matching separate at that boundary.
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return candidate
            .char_indices()
            .filter(|(_, character)| *character == '/')
            .any(|(index, _)| condition_glob_matches(prefix, &candidate[..index], insensitive));
    }
    // Absolute paths receive a shared artificial component so the filesystem
    // query parser need not accept an absolute query root.
    let pattern = format!("root/{}", pattern.trim_start_matches('/'));
    let candidate = PathBuf::from(format!("root/{}", candidate.trim_start_matches('/')));
    Pattern::compile_ignore(pattern).is_ok_and(|pattern| {
        PatternProgram::compile(&[pattern]).matches_path_with_case_and_scratch(
            &candidate,
            insensitive,
            &mut PathMatchScratch::default(),
        )
    })
}

fn config_section(line: &str) -> Option<(String, Option<String>)> {
    let mut quoted = false;
    let mut escaped = false;
    let end = line.char_indices().skip(1).find_map(|(index, character)| {
        if escaped {
            escaped = false;
            return None;
        }
        if character == '\\' && quoted {
            escaped = true;
            return None;
        }
        if character == '"' {
            quoted = !quoted;
        }
        (character == ']' && !quoted).then_some(index)
    })?;
    let inside = line[1..end].trim();
    let split = inside.find(char::is_whitespace).unwrap_or(inside.len());
    let name = inside[..split].to_ascii_lowercase();
    let subsection = inside[split..].trim();
    if subsection.is_empty() {
        Some((name, None))
    } else if subsection.starts_with('"') && subsection.ends_with('"') {
        Some((name, Some(config_value(subsection)?)))
    } else {
        None
    }
}

fn config_boolean(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => Some(true),
        "" | "false" | "no" | "off" => Some(false),
        number => {
            let (number, multiplier) = match number.as_bytes().last() {
                Some(b'k') => (&number[..number.len() - 1], 1024i64),
                Some(b'm') => (&number[..number.len() - 1], 1024i64.pow(2)),
                Some(b'g') => (&number[..number.len() - 1], 1024i64.pow(3)),
                _ => (number, 1),
            };
            let (negative, unsigned) = number
                .strip_prefix('-')
                .map_or((false, number), |rest| (true, rest));
            let unsigned = unsigned.strip_prefix('+').unwrap_or(unsigned);
            let (digits, radix) = if let Some(hex) = unsigned.strip_prefix("0x") {
                (hex, 16)
            } else if unsigned.len() > 1 && unsigned.starts_with('0') {
                (&unsigned[1..], 8)
            } else {
                (unsigned, 10)
            };
            let parsed = i64::from_str_radix(digits, radix).ok()?;
            let parsed = if negative {
                parsed.checked_neg()?
            } else {
                parsed
            };
            let parsed = parsed.checked_mul(multiplier)?;
            i32::try_from(parsed).ok().map(|number| number != 0)
        }
    }
}

fn expand_path(value: &str, home: Option<&Path>, relative_to: &Path) -> Result<PathBuf, String> {
    if value.is_empty() {
        return Ok(PathBuf::new());
    }
    if value == "~" || value.starts_with("~/") {
        return home
            .map(|home| home.join(value.strip_prefix("~/").unwrap_or("")))
            .ok_or_else(|| "cannot expand ~ without a home directory".to_owned());
    }
    if let Some(rest) = value.strip_prefix('~') {
        let (user, tail) = rest.split_once('/').unwrap_or((rest, ""));
        let current = env::var("USER").or_else(|_| env::var("USERNAME")).ok();
        if current.as_deref() == Some(user)
            && let Some(home) = home
        {
            return Ok(home.join(tail));
        }
        #[cfg(unix)]
        if let Ok(passwd) = fs::read_to_string("/etc/passwd") {
            for entry in passwd.lines() {
                let fields: Vec<_> = entry.split(':').collect();
                if fields.len() >= 6 && fields[0] == user {
                    return Ok(PathBuf::from(fields[5]).join(tail));
                }
            }
        }
        return Err(format!(
            "cannot resolve home directory for ~{user}; use an absolute ignore path"
        ));
    }
    if value.starts_with("%(prefix)/") {
        return Err(
            "Git installation-prefix paths are unsupported; use an absolute ignore path".to_owned(),
        );
    }
    Ok(relative_to.join(value))
}

fn config_value(value: &str) -> Option<String> {
    let mut output = String::new();
    let mut pending_space = String::new();
    let mut quoted = false;
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        match character {
            '"' => {
                output.push_str(&pending_space);
                pending_space.clear();
                quoted = !quoted;
            }
            '#' | ';' if !quoted => break,
            '\\' => {
                output.push_str(&pending_space);
                pending_space.clear();
                output.push(match characters.next()? {
                    'n' => '\n',
                    't' => '\t',
                    'b' => '\u{8}',
                    '\\' => '\\',
                    '"' => '"',
                    _ => return None,
                });
            }
            character if !quoted && character.is_whitespace() => pending_space.push(character),
            character => {
                output.push_str(&pending_space);
                pending_space.clear();
                output.push(character);
            }
        }
    }
    (!quoted).then_some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean_values_reject_unknown_values_and_accept_git_numbers() {
        for value in ["true", "YES", "on", "1", "-1", "2", "1k"] {
            assert_eq!(config_boolean(value), Some(true), "{value}");
        }
        for value in ["", "false", "NO", "off", "0"] {
            assert_eq!(config_boolean(value), Some(false), "{value}");
        }
        for value in ["maybe", "trueish", "2147483648"] {
            assert_eq!(config_boolean(value), None, "{value}");
        }
    }

    #[test]
    fn quoted_values_preserve_significant_whitespace_and_comment_characters() {
        assert_eq!(config_value("\"a \"  # comment"), Some("a ".to_owned()));
        assert_eq!(config_value("a   b  ; comment"), Some("a   b".to_owned()));
        assert_eq!(config_value("\"#a;b\""), Some("#a;b".to_owned()));
        assert_eq!(config_value("\"a\\\"b\""), Some("a\"b".to_owned()));
        assert!(config_value("\"unterminated").is_none());
        assert!(config_value("bad\\x").is_none());
    }

    #[test]
    fn conditional_section_preserves_subsection_case_and_brackets() {
        assert_eq!(
            config_section("[includeIf \"gitdir:Work/[abc]/\"] # comment"),
            Some((
                "includeif".to_owned(),
                Some("gitdir:Work/[abc]/".to_owned())
            ))
        );
        assert_eq!(config_section("[Core]"), Some(("core".to_owned(), None)));
    }
}
