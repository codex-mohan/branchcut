use super::args::Options;
use crate::error::{AppError, Result};
use crate::{CaseMode, EntryType, ModifiedBefore, SearchTarget, SizeFilter, parse_duration};
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::PathBuf;

pub(crate) struct ConfigSelection {
    path: PathBuf,
    profile: Option<String>,
}

pub(crate) fn find_selection(args: &[OsString]) -> Result<Option<ConfigSelection>> {
    let mut path = None;
    let mut profile = None;
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        if argument == OsStr::new("--") {
            break;
        }
        if argument == OsStr::new("--config") {
            if path.is_some() {
                return Err(AppError("--config may be specified only once".to_owned()));
            }
            index += 1;
            path =
                Some(PathBuf::from(args.get(index).ok_or_else(|| {
                    AppError("missing value for --config".to_owned())
                })?));
        } else if argument == OsStr::new("--profile") {
            if profile.is_some() {
                return Err(AppError("--profile may be specified only once".to_owned()));
            }
            index += 1;
            let value = args
                .get(index)
                .ok_or_else(|| AppError("missing value for --profile".to_owned()))?;
            profile = Some(
                value
                    .to_str()
                    .ok_or_else(|| AppError("--profile must be valid UTF-8".to_owned()))?
                    .to_owned(),
            );
        }
        index += 1;
    }
    match (path, profile) {
        (Some(path), profile) => Ok(Some(ConfigSelection { path, profile })),
        (None, Some(_)) => Err(AppError(
            "--profile requires an explicit --config file".to_owned(),
        )),
        (None, None) => Ok(None),
    }
}

pub(crate) fn load(selection: &ConfigSelection) -> Result<Options> {
    let contents = fs::read_to_string(&selection.path).map_err(|error| {
        AppError(format!(
            "cannot read config {}: {error}",
            selection.path.display()
        ))
    })?;
    let selected_profile = selection.profile.as_deref().unwrap_or("default");
    let config_directory = selection
        .path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_owned();
    let config_root = if config_directory.is_absolute() {
        config_directory
    } else {
        std::env::current_dir()
            .map_err(|error| AppError(format!("cannot resolve config directory: {error}")))?
            .join(config_directory)
    };
    let mut options = Options {
        cwd: config_root.clone(),
        ..Options::default()
    };
    let mut current_profile = "default".to_owned();
    let mut seen_profiles = HashSet::new();
    let mut seen_scalars = HashSet::new();
    let mut selected_found = false;

    for (index, raw_line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = parse_profile_header(line) {
            if !seen_profiles.insert(name.to_owned()) {
                return Err(config_error(
                    &selection.path,
                    line_number,
                    format!("profile {name:?} is defined more than once"),
                ));
            }
            current_profile = name.to_owned();
            selected_found |= current_profile == selected_profile;
            continue;
        }
        seen_profiles.insert(current_profile.clone());
        if current_profile != selected_profile {
            continue;
        }
        selected_found = true;
        apply_line(
            &mut options,
            &mut seen_scalars,
            line,
            &selection.path,
            &config_root,
            line_number,
        )?;
    }

    if !selected_found {
        let mut profiles = seen_profiles.into_iter().collect::<Vec<_>>();
        profiles.sort_unstable();
        return Err(config_error(
            &selection.path,
            1,
            format!(
                "profile {selected_profile:?} was not found{}",
                if profiles.is_empty() {
                    String::new()
                } else {
                    format!("; available profiles: {}", profiles.join(", "))
                }
            ),
        ));
    }

    Ok(options)
}

fn parse_profile_header(line: &str) -> Option<&str> {
    line.strip_prefix('[')
        .and_then(|line| line.strip_suffix(']'))
        .map(str::trim)
        .filter(|name| !name.is_empty())
}

fn apply_line(
    options: &mut Options,
    seen_scalars: &mut HashSet<&'static str>,
    line: &str,
    path: &std::path::Path,
    config_root: &std::path::Path,
    line_number: usize,
) -> Result<()> {
    let (key, value) = split_entry(line);
    match key {
        "glob" => options
            .positive
            .push(required_value(value, path, line_number, key)?),
        "exclude" => options
            .negative
            .push(required_value(value, path, line_number, key)?),
        "ignore-file" => {
            let value = required_value(value, path, line_number, key)?;
            options.ignore_files.push(config_root.join(value));
        }
        "extension" | "ext" => {
            let extension = required_value(value, path, line_number, key)?;
            options
                .extensions
                .push(extension.trim_start_matches('.').as_bytes().to_vec());
        }
        "search" => options
            .simple_terms
            .push(required_value(value, path, line_number, key)?.into_bytes()),
        "cwd" => {
            scalar_once(seen_scalars, "cwd", path, line_number)?;
            let value = required_value(value, path, line_number, key)?;
            let cwd = PathBuf::from(value);
            options.cwd = if cwd.is_absolute() {
                cwd
            } else {
                config_root.join(cwd)
            };
        }
        "type" => {
            options.wanted_type = parse_entry_type(
                &required_value(value, path, line_number, key)?,
                path,
                line_number,
            )?;
            if !options.entry_types.contains(&options.wanted_type) {
                options.entry_types.push(options.wanted_type);
            }
        }
        "case" => {
            scalar_once(seen_scalars, "case", path, line_number)?;
            options.case_mode = parse_case_mode(
                &required_value(value, path, line_number, key)?,
                path,
                line_number,
            )?;
        }
        "search-target" => {
            scalar_once(seen_scalars, "search-target", path, line_number)?;
            options.search_target = match required_value(value, path, line_number, key)?.as_str() {
                "filename" | "name" => SearchTarget::Name,
                "path" | "full-path" => SearchTarget::Path,
                other => {
                    return Err(config_error(
                        path,
                        line_number,
                        format!("unknown search target {other:?}; expected filename or path"),
                    ));
                }
            };
        }
        "hidden" => {
            scalar_once(seen_scalars, "hidden", path, line_number)?;
            options.hidden = parse_boolean(value, path, line_number, key)?;
        }
        "gitignore" => {
            scalar_once(seen_scalars, "gitignore", path, line_number)?;
            options.gitignore = parse_boolean(value, path, line_number, key)?;
        }
        "ignore" => {
            scalar_once(seen_scalars, "ignore", path, line_number)?;
            options.ignore = parse_boolean(value, path, line_number, key)?;
        }
        "and" => options
            .and_terms
            .push(required_value(value, path, line_number, key)?),
        "search-path" => options.search_paths.push(PathBuf::from(required_value(
            value,
            path,
            line_number,
            key,
        )?)),
        "escaped-globs" => {
            scalar_once(seen_scalars, "escaped-globs", path, line_number)?;
            options.escape_patterns = parse_boolean(value, path, line_number, key)?;
        }
        "unicode" | "follow" | "one-file-system" | "contents-first" | "empty" | "executable" => {
            let static_key = match key {
                "unicode" => "unicode",
                "follow" => "follow",
                "one-file-system" => "one-file-system",
                "contents-first" => "contents-first",
                "empty" => "empty",
                _ => "executable",
            };
            scalar_once(seen_scalars, static_key, path, line_number)?;
            let enabled = parse_boolean(value, path, line_number, key)?;
            match key {
                "unicode" => options.unicode = enabled,
                "follow" => options.follow_symlinks = enabled,
                "one-file-system" => options.same_file_system = enabled,
                "contents-first" => options.contents_first = enabled,
                "empty" => options.require_empty = enabled,
                _ => options.require_executable = enabled,
            }
        }
        "owner" => {
            scalar_once(seen_scalars, "owner", path, line_number)?;
            let value = required_value(value, path, line_number, key)?;
            let (owner, group) = super::args::parse_owner(&value)
                .map_err(|error| config_error(path, line_number, error.0))?;
            options.owner = owner;
            options.group = group;
        }
        "max-open" => {
            scalar_once(seen_scalars, "max-open", path, line_number)?;
            let value = required_value(value, path, line_number, key)?;
            options.max_open = value
                .parse()
                .ok()
                .filter(|value| *value > 0)
                .ok_or_else(|| {
                    config_error(
                        path,
                        line_number,
                        "max-open expects a positive integer".to_owned(),
                    )
                })?;
        }
        "size" => {
            let value = required_value(value, path, line_number, key)?;
            options
                .sizes
                .push(value.parse::<SizeFilter>().map_err(|error| {
                    config_error(
                        path,
                        line_number,
                        format!("invalid size {value:?}: {error}"),
                    )
                })?);
        }
        "modified-within" => {
            scalar_once(seen_scalars, "modified-within", path, line_number)?;
            let value = required_value(value, path, line_number, key)?;
            options.modified_within = Some(parse_duration(&value).map_err(|error| {
                config_error(
                    path,
                    line_number,
                    format!("invalid --modified-within {value:?}: {error}"),
                )
            })?);
        }
        "modified-before" => {
            scalar_once(seen_scalars, "modified-before", path, line_number)?;
            let value = required_value(value, path, line_number, key)?;
            options.modified_before = Some(value.parse::<ModifiedBefore>().map_err(|error| {
                config_error(
                    path,
                    line_number,
                    format!("invalid --modified-before {value:?}: {error}"),
                )
            })?);
        }
        "min-depth" => {
            scalar_once(seen_scalars, "min-depth", path, line_number)?;
            let value = required_value(value, path, line_number, key)?;
            options.min_depth = value.parse::<usize>().map_err(|_| {
                config_error(
                    path,
                    line_number,
                    format!("invalid min-depth {value:?}; expected a non-negative integer"),
                )
            })?;
        }
        "max-depth" => {
            scalar_once(seen_scalars, "max-depth", path, line_number)?;
            let value = required_value(value, path, line_number, key)?;
            options.max_depth = if value == "none" {
                None
            } else {
                Some(value.parse::<usize>().map_err(|_| {
                    config_error(
                        path,
                        line_number,
                        format!(
                            "invalid max-depth {value:?}; expected a non-negative integer or none"
                        ),
                    )
                })?)
            };
        }
        other => {
            return Err(config_error(
                path,
                line_number,
                format!("unknown config option {other:?}"),
            ));
        }
    }
    Ok(())
}

fn split_entry(line: &str) -> (&str, Option<&str>) {
    match line.find(char::is_whitespace) {
        Some(index) => (
            &line[..index],
            Some(line[index..].trim()).filter(|value| !value.is_empty()),
        ),
        None => (line, None),
    }
}

fn scalar_once(
    seen: &mut HashSet<&'static str>,
    key: &'static str,
    path: &std::path::Path,
    line_number: usize,
) -> Result<()> {
    if seen.insert(key) {
        Ok(())
    } else {
        Err(config_error(
            path,
            line_number,
            format!("{key:?} may be specified only once per profile"),
        ))
    }
}

fn required_value(
    value: Option<&str>,
    path: &std::path::Path,
    line_number: usize,
    key: &str,
) -> Result<String> {
    value
        .map(str::to_owned)
        .ok_or_else(|| config_error(path, line_number, format!("{key:?} requires a value")))
}

fn parse_boolean(
    value: Option<&str>,
    path: &std::path::Path,
    line_number: usize,
    key: &str,
) -> Result<bool> {
    match value.unwrap_or("true") {
        "true" | "yes" | "on" => Ok(true),
        "false" | "no" | "off" => Ok(false),
        other => Err(config_error(
            path,
            line_number,
            format!("invalid {key:?} value {other:?}; expected true or false"),
        )),
    }
}

fn parse_entry_type(value: &str, path: &std::path::Path, line_number: usize) -> Result<EntryType> {
    match value {
        "file" | "f" => Ok(EntryType::File),
        "dir" | "directory" | "d" => Ok(EntryType::Dir),
        "symlink" | "link" | "l" => Ok(EntryType::Symlink),
        "socket" | "s" => Ok(EntryType::Socket),
        "fifo" | "pipe" | "p" => Ok(EntryType::Fifo),
        "block-device" | "b" => Ok(EntryType::BlockDevice),
        "char-device" | "c" => Ok(EntryType::CharDevice),
        other => Err(config_error(
            path,
            line_number,
            format!(
                "invalid type {other:?}; expected file, dir, symlink, socket, fifo, block-device, or char-device"
            ),
        )),
    }
}

fn parse_case_mode(value: &str, path: &std::path::Path, line_number: usize) -> Result<CaseMode> {
    match value {
        "sensitive" | "case-sensitive" => Ok(CaseMode::Sensitive),
        "insensitive" | "ignore-case" => Ok(CaseMode::Insensitive),
        "smart" | "smart-case" => Ok(CaseMode::Smart),
        other => Err(config_error(
            path,
            line_number,
            format!("invalid case mode {other:?}; expected sensitive, insensitive, or smart"),
        )),
    }
}

fn config_error(path: &std::path::Path, line: usize, message: String) -> AppError {
    AppError(format!("{}:{line}: {message}", path.display()))
}
