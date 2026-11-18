use crate::ignore::{
    IgnoreRule, IgnoreRules, deciding_rule, directory_rule_sources, initial_ignore_rules,
};
use crate::path::path_has_hidden_component;
use crate::{CompiledQuery, SearchTarget};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

/// Outcome of one semantic check used to explain a candidate path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckOutcome {
    Passed,
    Failed,
    NotApplicable,
}

/// One readable decision in a candidate-path explanation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathCheck {
    pub label: &'static str,
    pub outcome: CheckOutcome,
    pub detail: Option<String>,
}

/// Selection result and ordered checks for one candidate path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathExplanation {
    pub path: PathBuf,
    pub selected: bool,
    pub checks: Vec<PathCheck>,
}

impl CompiledQuery {
    /// Explain whether one cwd-relative candidate satisfies this compiled query.
    ///
    /// Absolute candidates are accepted when they are lexically below the
    /// query's cwd. This method reads the candidate's metadata and any relevant
    /// `.gitignore` files, but does not traverse unrelated directories.
    pub fn explain_path(&self, candidate: impl AsRef<Path>) -> io::Result<PathExplanation> {
        explain_path(self, candidate.as_ref(), SystemTime::now())
    }
}

fn explain_path(
    plan: &CompiledQuery,
    candidate: &Path,
    now: SystemTime,
) -> io::Result<PathExplanation> {
    let relative = relative_candidate(&plan.cwd, candidate)?;
    let absolute = plan.cwd.join(&relative);
    let link_metadata = fs::symlink_metadata(&absolute)?;
    let metadata = if plan.follow_symlinks && link_metadata.file_type().is_symlink() {
        fs::metadata(&absolute)?
    } else {
        link_metadata
    };
    let file_type = metadata.file_type();
    let name = relative
        .file_name()
        .unwrap_or_else(|| OsStr::new(""))
        .to_owned();
    let mut checks = Vec::new();
    let traversal = traversable_parent(plan, &relative)?;
    record(
        &mut checks,
        "traversal policy",
        traversal.is_none(),
        traversal.unwrap_or_else(|| "candidate ancestors satisfy the traversal policy".into()),
    );

    record(
        &mut checks,
        "hidden path",
        plan.hidden || !path_has_hidden_component(&relative),
        if plan.hidden {
            "hidden paths are enabled".to_owned()
        } else if path_has_hidden_component(&relative) {
            "a path component is hidden; use --hidden to include it".to_owned()
        } else {
            "no hidden path component".to_owned()
        },
    );

    if plan.uses_ignores() {
        let (rules, blocked) = load_candidate_ignore_rules(plan, &relative)?;
        if let Some(rule) = blocked
            .as_ref()
            .or_else(|| deciding_rule(rules.as_slice(), &relative, file_type.is_dir()))
        {
            record(
                &mut checks,
                ".gitignore",
                rule.negated,
                format!(
                    "{} `{}` at {}:{}",
                    if rule.negated {
                        "re-included by"
                    } else {
                        "excluded by"
                    },
                    rule.text,
                    rule.source.display(),
                    rule.line
                ),
            );
        } else {
            record(
                &mut checks,
                ".gitignore",
                true,
                "no ignore rule matches the candidate".to_owned(),
            );
        }
    } else {
        checks.push(PathCheck {
            label: ".gitignore",
            outcome: CheckOutcome::NotApplicable,
            detail: Some("hierarchical ignore rules are disabled".to_owned()),
        });
    }

    let (positive, negative) = states_for_path(plan, &relative);
    let positive_match = plan.positive_program.states_match(&positive);
    record(
        &mut checks,
        "positive patterns",
        positive_match,
        if positive_match {
            format!(
                "matched one of {}",
                format_sources(plan.positives.iter().map(|pattern| pattern.source.as_str()))
            )
        } else {
            format!(
                "matched none of {}",
                format_sources(plan.positives.iter().map(|pattern| pattern.source.as_str()))
            )
        },
    );

    let excluded = plan.negative_program.states_match(&negative);
    let subtree_excluded =
        file_type.is_dir() && plan.negative_program.states_exclude_subtree(&negative);
    record(
        &mut checks,
        "exclusions",
        !excluded,
        if excluded {
            format!(
                "matched by {}",
                format_sources(plan.negatives.iter().map(|pattern| pattern.source.as_str()))
            )
        } else if subtree_excluded {
            "the directory itself is not excluded, but this rule prunes its descendants".to_owned()
        } else if plan.negatives.is_empty() {
            "no exclusions configured".to_owned()
        } else {
            "not excluded".to_owned()
        },
    );

    let actual_type = if file_type.is_file() {
        "file"
    } else if file_type.is_dir() {
        "directory"
    } else if file_type.is_symlink() {
        "symlink"
    } else {
        "special entry"
    };
    record(
        &mut checks,
        "entry type",
        plan.type_matches(&file_type),
        format!(
            "candidate is {}; query requests {:?}",
            actual_type, plan.entry_types
        ),
    );

    let depth = relative.components().count();
    record(
        &mut checks,
        "depth",
        plan.depth_matches(depth),
        format!(
            "candidate depth is {depth}; query range is {}..{}",
            plan.min_depth,
            plan.max_depth
                .map_or_else(|| "unbounded".to_owned(), |maximum| maximum.to_string())
        ),
    );

    if plan.extensions.is_empty() {
        checks.push(PathCheck {
            label: "extension",
            outcome: CheckOutcome::NotApplicable,
            detail: Some("no extension filter configured".to_owned()),
        });
    } else {
        record(
            &mut checks,
            "extension",
            plan.extension_matches(&name),
            "extension filters follow the query case mode".to_owned(),
        );
    }

    if plan.simple_terms.is_empty() && plan.and_terms.is_empty() {
        checks.push(PathCheck {
            label: "simple search",
            outcome: CheckOutcome::NotApplicable,
            detail: Some("no literal search term configured".to_owned()),
        });
    } else {
        record(
            &mut checks,
            "simple search",
            plan.simple_matches(&name, &relative),
            match plan.search_target {
                SearchTarget::Name => "searched the entry name".to_owned(),
                SearchTarget::Path => "searched the cwd-relative path".to_owned(),
            },
        );
    }

    if plan.needs_metadata() {
        record(
            &mut checks,
            "metadata predicates",
            plan.matches_metadata(&metadata, &absolute, now)?,
            metadata_detail(plan),
        );
    } else {
        checks.push(PathCheck {
            label: "metadata predicates",
            outcome: CheckOutcome::NotApplicable,
            detail: Some("size and modification-time filters are not configured".to_owned()),
        });
    }

    Ok(PathExplanation {
        path: relative,
        selected: checks
            .iter()
            .all(|check| check.outcome != CheckOutcome::Failed),
        checks,
    })
}

fn traversable_parent(plan: &CompiledQuery, relative: &Path) -> io::Result<Option<String>> {
    let mut directory = plan.cwd.clone();
    let mut identities = Vec::new();
    #[cfg(unix)]
    let root_device = {
        use std::os::unix::fs::MetadataExt;
        if plan.same_file_system {
            Some(fs::metadata(&plan.cwd)?.dev())
        } else {
            None
        }
    };
    let mut components = relative
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .components();
    loop {
        let metadata = fs::symlink_metadata(&directory)?;
        if metadata.file_type().is_symlink() && !plan.follow_symlinks {
            return Ok(Some(format!(
                "directory symlink {} is not followed; use --follow",
                directory.display()
            )));
        }
        if plan.follow_symlinks {
            let identity = fs::canonicalize(&directory)?;
            if identities.contains(&identity) {
                return Ok(Some(
                    "a followed directory symlink forms an ancestor loop".into(),
                ));
            }
            identities.push(identity);
        }
        #[cfg(unix)]
        if let Some(device) = root_device {
            use std::os::unix::fs::MetadataExt;
            if fs::metadata(&directory)?.dev() != device {
                return Ok(Some("ancestor crosses the starting filesystem".into()));
            }
        }
        let Some(component) = components.next() else {
            break;
        };
        directory.push(component.as_os_str());
    }
    Ok(None)
}

fn record(checks: &mut Vec<PathCheck>, label: &'static str, passed: bool, detail: String) {
    checks.push(PathCheck {
        label,
        outcome: if passed {
            CheckOutcome::Passed
        } else {
            CheckOutcome::Failed
        },
        detail: Some(detail),
    });
}

fn states_for_path(plan: &CompiledQuery, relative: &Path) -> (Vec<usize>, Vec<usize>) {
    let mut positive = plan.positive_program.initial_states();
    let mut negative = plan.negative_program.initial_states();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            continue;
        };
        positive = plan.positive_program.advance_with_mode(
            &positive,
            component,
            plan.case_insensitive,
            plan.unicode,
        );
        negative = plan.negative_program.advance_with_mode(
            &negative,
            component,
            plan.case_insensitive,
            plan.unicode,
        );
    }
    (positive, negative)
}

fn load_candidate_ignore_rules(
    plan: &CompiledQuery,
    relative: &Path,
) -> io::Result<(IgnoreRules, Option<IgnoreRule>)> {
    let mut absolute = plan.cwd.clone();
    let mut base = PathBuf::new();
    let initial = initial_ignore_rules(plan)?;
    if let Some((_, error)) = initial.errors.into_iter().next() {
        return Err(error);
    }
    let mut rules = initial.rules;
    if initial.blocked.is_some() {
        return Ok((rules, initial.blocked));
    }
    for component in relative.components() {
        let Component::Normal(component) = component else {
            continue;
        };
        for (_, loaded) in directory_rule_sources(plan, &absolute, &base) {
            rules.extend(loaded?);
        }
        absolute.push(component);
        base.push(component);
    }
    Ok((rules, None))
}

fn relative_candidate(cwd: &Path, candidate: &Path) -> io::Result<PathBuf> {
    let relative = if candidate.is_absolute() {
        let absolute_cwd = if cwd.is_absolute() {
            cwd.to_owned()
        } else {
            std::env::current_dir()?.join(cwd)
        };
        candidate.strip_prefix(&absolute_cwd).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "--why path {} is outside query cwd {}",
                    candidate.display(),
                    cwd.display()
                ),
            )
        })?
    } else {
        candidate
    };
    normalize_relative(relative)
}

fn normalize_relative(path: &Path) -> io::Result<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(component) => normalized.push(component),
            Component::ParentDir => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "--why path must stay below --cwd",
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "--why path must be relative to --cwd",
                ));
            }
        }
    }
    Ok(normalized)
}

fn metadata_detail(plan: &CompiledQuery) -> String {
    let mut parts = Vec::new();
    if plan.require_empty {
        parts.push("empty entry".to_owned());
    }
    if plan.require_executable {
        parts.push("executable file".to_owned());
    }
    if let Some(owner) = plan.owner {
        parts.push(format!("owner uid {owner}"));
    }
    if let Some(group) = plan.group {
        parts.push(format!("group gid {group}"));
    }
    if !plan.metadata.sizes.is_empty() {
        parts.push(format!(
            "size {}",
            plan.metadata
                .sizes
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" and ")
        ));
    }
    if let Some(within) = plan.metadata.modified_within {
        parts.push(format!("modified within {within:?}"));
    }
    if let Some(before) = plan.metadata.modified_before {
        parts.push(format!("modified before {before:?}"));
    }
    parts.join("; ")
}

fn format_sources<'a>(sources: impl Iterator<Item = &'a str>) -> String {
    sources
        .map(|source| format!("`{source}`"))
        .collect::<Vec<_>>()
        .join(", ")
}
