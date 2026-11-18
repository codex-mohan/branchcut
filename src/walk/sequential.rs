use super::{BufferedEntry, Visitor, metadata_matches, prepare_root};
use crate::ignore::{IgnoreRules, directory_rule_sources, ignored_child_by_rules};
use crate::path::is_hidden;
use crate::pattern::PathMatchScratch;
use crate::{CompiledQuery, SearchTarget, Stats};
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

struct DirectoryFrame {
    entries: DirectoryEntries,
    absolute: PathBuf,
    relative: PathBuf,
    positive: Vec<usize>,
    negative: Vec<usize>,
    ignore_rules: IgnoreRules,
    depth: usize,
    identity: Option<PathBuf>,
    postorder: Option<PathBuf>,
}

enum DirectoryEntries {
    Open(Box<fs::ReadDir>),
    Spilled(VecDeque<BufferedEntry>),
}

struct WalkEntry {
    entry: EntryStorage,
}

// Keep the Windows DirEntry inline: boxing it would allocate for every entry.
// This enum is transient; spill buffers contain only compact owned records.
#[allow(clippy::large_enum_variant)]
enum EntryStorage {
    Live(fs::DirEntry),
    Spilled(BufferedEntry),
}

impl WalkEntry {
    fn name(&self) -> std::ffi::OsString {
        match &self.entry {
            EntryStorage::Live(entry) => entry.file_name(),
            EntryStorage::Spilled(entry) => entry.name.clone(),
        }
    }

    fn file_type(&self) -> io::Result<fs::FileType> {
        match &self.entry {
            EntryStorage::Live(entry) => entry.file_type(),
            EntryStorage::Spilled(entry) => Ok(entry.file_type),
        }
    }

    fn path(&self) -> PathBuf {
        match &self.entry {
            EntryStorage::Live(entry) => entry.path(),
            EntryStorage::Spilled(entry) => entry.absolute.clone(),
        }
    }
}

impl DirectoryEntries {
    fn next(&mut self) -> Option<(io::Result<WalkEntry>, bool)> {
        match self {
            Self::Open(entries) => entries.next().map(|entry| {
                let converted = entry.map(|entry| WalkEntry {
                    entry: EntryStorage::Live(entry),
                });
                (converted, false)
            }),
            Self::Spilled(entries) => entries.pop_front().map(|entry| {
                (
                    Ok(WalkEntry {
                        entry: EntryStorage::Spilled(entry),
                    }),
                    true,
                )
            }),
        }
    }
}

pub(crate) struct Runner<'a, V: Visitor> {
    plan: &'a CompiledQuery,
    visitor: &'a mut V,
    pub(crate) stats: Stats,
    pub(crate) stopped: bool,
    now: SystemTime,
    ignore_scratch: PathMatchScratch,
    #[cfg(unix)]
    root_device: Option<u64>,
}

impl<'a, V: Visitor> Runner<'a, V> {
    pub(crate) fn new(plan: &'a CompiledQuery, visitor: &'a mut V, now: SystemTime) -> Self {
        Self {
            plan,
            visitor,
            stats: Stats::default(),
            stopped: false,
            now,
            ignore_scratch: PathMatchScratch::default(),
            #[cfg(unix)]
            root_device: None,
        }
    }

    pub(crate) fn run(&mut self) -> io::Result<()> {
        if self.plan.cancelled() {
            self.stopped = true;
            return Ok(());
        }
        if self.plan.same_file_system && !cfg!(unix) {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "--one-file-system is not supported on this platform",
            ));
        }
        let visitor_needs_path = self.visitor.requires_path();
        let Some(root) = prepare_root(self.plan, &mut self.stats, self.visitor)? else {
            return Ok(());
        };
        if self.plan.cancelled() {
            self.stopped = true;
            return Ok(());
        }
        let root_type = root.file_type;
        #[cfg(unix)]
        if self.plan.same_file_system {
            use std::os::unix::fs::MetadataExt;
            self.root_device = Some(root.metadata.dev());
        }
        let (positive_states, negative_states) = self.plan.root_states();
        let root_name = self.plan.root.file_name().unwrap_or_default();
        if !root_type.is_dir() {
            let type_matches = self.plan.type_matches(&root_type);
            if type_matches
                && self.plan.states_match(
                    &positive_states,
                    &negative_states,
                    root_name,
                    &self.plan.root_relative,
                )
                && self.plan.extension_matches(root_name)
                && self
                    .plan
                    .depth_matches(self.plan.root_relative.components().count())
                && self.passes_metadata(&self.plan.root, Some(&root.metadata))?
            {
                if visitor_needs_path {
                    self.emit(self.plan.root_relative.clone())?;
                } else {
                    self.emit_without_path()?;
                }
            }
            return Ok(());
        }
        let root_matches = !self.plan.root_relative.as_os_str().is_empty()
            && !root.ignored
            && self.plan.type_matches(&root_type)
            && self.plan.states_match(
                &positive_states,
                &negative_states,
                root_name,
                &self.plan.root_relative,
            )
            && self.plan.extension_matches(root_name)
            && self
                .plan
                .depth_matches(self.plan.root_relative.components().count())
            && self.passes_metadata(&self.plan.root, Some(&root.metadata))?;
        let root_postorder = if root_matches && self.plan.contents_first {
            Some(self.plan.root_relative.clone())
        } else {
            None
        };
        if root_matches && !self.plan.contents_first {
            if visitor_needs_path {
                self.emit(self.plan.root_relative.clone())?;
            } else {
                self.emit_without_path()?;
            }
        }
        let mut stack = Vec::new();
        let mut child_positive = Vec::new();
        let mut child_negative = Vec::new();
        let opened = self.push_directory(
            &mut stack,
            self.plan.root.clone(),
            self.plan.root_relative.clone(),
            positive_states,
            negative_states,
            root.ignore_rules,
            root_postorder.clone(),
            &[],
        )?;
        if !opened
            && !self.stopped
            && let Some(path) = root_postorder
        {
            self.emit(path)?;
        }
        while !self.stopped && !stack.is_empty() {
            if self.plan.cancelled() {
                self.stopped = true;
                break;
            }
            let Some((result, counted)) = stack.last_mut().and_then(|frame| frame.entries.next())
            else {
                if let Some(path) = stack.pop().and_then(|frame| frame.postorder) {
                    self.emit(path)?;
                }
                continue;
            };
            let frame = stack.last().expect("a directory iterator is active");
            let entry = match result {
                Ok(entry) => entry,
                Err(error) => {
                    self.stats.errors += 1;
                    self.visitor.error(&frame.absolute, &error)?;
                    continue;
                }
            };
            if !counted {
                self.stats.entries_seen += 1;
            }
            let name = entry.name();
            if !self.plan.hidden && is_hidden(&name) {
                continue;
            }
            let entry_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    self.stats.errors += 1;
                    self.visitor.error(&entry.path(), &error)?;
                    continue;
                }
            };
            let followed_metadata = if entry_type.is_symlink() && self.plan.follow_symlinks {
                self.stats.metadata_calls += 1;
                match fs::metadata(entry.path()) {
                    Ok(metadata) => Some(metadata),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                    Err(error) => {
                        self.stats.errors += 1;
                        self.visitor.error(&entry.path(), &error)?;
                        None
                    }
                }
            } else {
                None
            };
            let file_type = followed_metadata
                .as_ref()
                .map_or(entry_type, fs::Metadata::file_type);
            let child_relative = (file_type.is_dir()
                || self.plan.uses_ignores()
                || self.plan.search_target == SearchTarget::Path)
                .then(|| frame.relative.join(&name));
            if self.plan.uses_ignores()
                && ignored_child_by_rules(
                    frame.ignore_rules.as_slice(),
                    child_relative
                        .as_deref()
                        .expect("ignore checks require a relative path"),
                    file_type.is_dir(),
                    &mut self.ignore_scratch,
                )
            {
                if file_type.is_dir() {
                    self.stats.dirs_pruned_ignore += 1;
                }
                continue;
            }
            if !self.plan.selection_all_non_root && !self.plan.positive_matches_all {
                self.plan.positive_program.advance_into_with_mode(
                    &frame.positive,
                    &name,
                    self.plan.case_insensitive,
                    self.plan.unicode,
                    &mut child_positive,
                );
            }
            if !self.plan.selection_all_non_root {
                self.plan.negative_program.advance_into_with_mode(
                    &frame.negative,
                    &name,
                    self.plan.case_insensitive,
                    self.plan.unicode,
                    &mut child_negative,
                );
            }
            if file_type.is_dir() {
                let child_relative =
                    child_relative.expect("directory traversal requires a relative path");
                let matched_directory = self.plan.type_matches(&file_type)
                    && (self.plan.selection_all_non_root
                        || self.plan.states_match(
                            &child_positive,
                            &child_negative,
                            &name,
                            &child_relative,
                        ) && self.plan.extension_matches(&name)
                            && self.plan.depth_matches(frame.depth + 1))
                    && self.passes_entry_metadata(&entry, followed_metadata.as_ref())?;
                let postorder = if matched_directory && self.plan.contents_first {
                    Some(child_relative.clone())
                } else {
                    None
                };
                if matched_directory && !self.plan.contents_first {
                    if visitor_needs_path {
                        if self.plan.uses_ignores() || self.plan.search_target == SearchTarget::Path
                        {
                            self.emit(child_relative.clone())?;
                        } else {
                            self.emit_child(&frame.relative, &name)?;
                        }
                    } else {
                        self.emit_without_path()?;
                    }
                }
                if self.stopped {
                    continue;
                }
                let child_absolute = entry.path();
                let child_ignore_rules = frame.ignore_rules.clone();
                let ancestors = stack
                    .iter()
                    .filter_map(|frame| frame.identity.clone())
                    .collect::<Vec<_>>();
                // A Unix DirEntry can keep its parent iterator alive after a
                // spill. Release it before opening the child under the budget.
                drop(entry);
                let opened = self.push_directory(
                    &mut stack,
                    child_absolute,
                    child_relative,
                    std::mem::take(&mut child_positive),
                    std::mem::take(&mut child_negative),
                    child_ignore_rules,
                    postorder.clone(),
                    &ancestors,
                )?;
                if !opened
                    && !self.stopped
                    && let Some(path) = postorder
                {
                    self.emit(path)?;
                }
            } else {
                self.stats.candidate_files += 1;
                let type_matches = self.plan.type_matches(&file_type);
                if type_matches
                    && (self.plan.selection_all_non_root
                        || self.plan.depth_matches(frame.depth + 1)
                            && self.plan.extension_matches(&name)
                            && child_relative.as_deref().map_or_else(
                                || {
                                    self.plan.states_match_name(
                                        &child_positive,
                                        &child_negative,
                                        &name,
                                    )
                                },
                                |relative| {
                                    self.plan.states_match(
                                        &child_positive,
                                        &child_negative,
                                        &name,
                                        relative,
                                    )
                                },
                            ))
                    && self.passes_entry_metadata(&entry, followed_metadata.as_ref())?
                {
                    if visitor_needs_path {
                        if let Some(child_relative) = child_relative {
                            self.emit(child_relative)?;
                        } else {
                            self.emit_child(&frame.relative, &name)?;
                        }
                    } else {
                        self.emit_without_path()?;
                    }
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn push_directory(
        &mut self,
        stack: &mut Vec<DirectoryFrame>,
        absolute: PathBuf,
        relative: PathBuf,
        positive_states: Vec<usize>,
        negative_states: Vec<usize>,
        mut ignore_rules: IgnoreRules,
        postorder: Option<PathBuf>,
        ancestors: &[PathBuf],
    ) -> io::Result<bool> {
        if self.stopped {
            return Ok(false);
        }
        if self.plan.cancelled() {
            self.stopped = true;
            return Ok(false);
        }
        self.stats.dirs_seen += 1;
        let depth = relative.components().count();
        if !self.plan.can_descend(depth) {
            self.stats.dirs_pruned_depth += 1;
            return Ok(false);
        }
        for (path, loaded) in directory_rule_sources(self.plan, &absolute, &relative) {
            match loaded {
                Ok(rules) => ignore_rules.extend(rules),
                Err(error) => {
                    self.stats.errors += 1;
                    self.visitor.error(&path, &error)?;
                }
            }
        }
        if self
            .plan
            .negative_program
            .states_exclude_subtree(&negative_states)
        {
            self.stats.dirs_pruned_exclude += 1;
            return Ok(false);
        }
        if !self.plan.positive_matches_all
            && !self
                .plan
                .positive_program
                .states_descendant_possible(&positive_states)
        {
            self.stats.dirs_pruned_positive += 1;
            return Ok(false);
        }
        let identity = if self.plan.follow_symlinks {
            match fs::canonicalize(&absolute) {
                Ok(identity) => {
                    if ancestors.iter().any(|ancestor| ancestor == &identity) {
                        self.stats.dirs_pruned_policy += 1;
                        return Ok(false);
                    }
                    Some(identity)
                }
                Err(error) => {
                    self.stats.errors += 1;
                    self.visitor.error(&absolute, &error)?;
                    return Ok(false);
                }
            }
        } else {
            None
        };
        #[cfg(unix)]
        if self.plan.same_file_system {
            use std::os::unix::fs::MetadataExt;
            self.stats.metadata_calls += 1;
            let metadata = match fs::metadata(&absolute) {
                Ok(metadata) => metadata,
                Err(error) => {
                    self.stats.errors += 1;
                    self.visitor.error(&absolute, &error)?;
                    return Ok(false);
                }
            };
            if self
                .root_device
                .is_some_and(|device| metadata.dev() != device)
            {
                self.stats.dirs_pruned_policy += 1;
                return Ok(false);
            }
        }
        // Preserve lazy enumeration normally. At the configured handle limit,
        // spill the oldest still-open ancestor, retaining its unvisited entries
        // while releasing that directory handle.
        let max_open = self.plan.max_open.max(1);
        let open_count = stack
            .iter()
            .filter(|frame| matches!(&frame.entries, DirectoryEntries::Open(_)))
            .count();
        if open_count >= max_open
            && let Some(frame) = stack
                .iter_mut()
                .find(|frame| matches!(&frame.entries, DirectoryEntries::Open(_)))
            && let DirectoryEntries::Open(open) = &mut frame.entries
        {
            let mut remaining = VecDeque::new();
            for result in open {
                match result {
                    Ok(entry) => {
                        self.stats.entries_seen += 1;
                        match entry.file_type() {
                            Ok(file_type) => remaining.push_back(BufferedEntry {
                                name: entry.file_name(),
                                absolute: entry.path(),
                                file_type,
                            }),
                            Err(error) => {
                                self.stats.errors += 1;
                                self.visitor.error(&entry.path(), &error)?;
                            }
                        }
                    }
                    Err(error) => {
                        self.stats.errors += 1;
                        self.visitor.error(&frame.absolute, &error)?;
                    }
                }
            }
            frame.entries = DirectoryEntries::Spilled(remaining);
        }
        let entries = match fs::read_dir(&absolute) {
            Ok(entries) => entries,
            Err(error) => {
                self.stats.errors += 1;
                self.visitor.error(&absolute, &error)?;
                return Ok(false);
            }
        };
        self.stats.dirs_opened += 1;
        stack.push(DirectoryFrame {
            entries: DirectoryEntries::Open(Box::new(entries)),
            absolute,
            relative,
            positive: positive_states,
            negative: negative_states,
            ignore_rules,
            depth,
            identity,
            postorder,
        });
        Ok(true)
    }

    fn emit(&mut self, path: PathBuf) -> io::Result<()> {
        self.stats.matches += 1;
        self.stopped = self.visitor.matched(&path)?.is_break();
        Ok(())
    }

    fn emit_child(&mut self, parent: &Path, name: &OsStr) -> io::Result<()> {
        self.stats.matches += 1;
        self.stopped = self.visitor.matched_child(parent, name)?.is_break();
        Ok(())
    }

    fn emit_without_path(&mut self) -> io::Result<()> {
        self.stats.matches += 1;
        self.stopped = self.visitor.matched_without_path()?.is_break();
        Ok(())
    }

    fn passes_metadata(
        &mut self,
        absolute: &Path,
        cached: Option<&fs::Metadata>,
    ) -> io::Result<bool> {
        match metadata_matches(self.plan, absolute, &mut self.stats, self.now, cached) {
            Ok(matches) => Ok(matches),
            Err(error) => {
                self.stats.errors += 1;
                self.visitor.error(absolute, &error)?;
                Ok(false)
            }
        }
    }

    fn passes_entry_metadata(
        &mut self,
        entry: &WalkEntry,
        cached: Option<&fs::Metadata>,
    ) -> io::Result<bool> {
        if !self.plan.needs_metadata() {
            return Ok(true);
        }
        self.passes_metadata(&entry.path(), cached)
    }
}

#[cfg(test)]
mod cached_metadata_tests {
    use super::*;
    use crate::{Query, SizeFilter};
    use std::ops::ControlFlow;

    struct Errors {
        strict: bool,
        paths: Vec<PathBuf>,
    }
    impl Visitor for Errors {
        fn matched(&mut self, _: &Path) -> io::Result<ControlFlow<()>> {
            Ok(ControlFlow::Continue(()))
        }
        fn error(&mut self, path: &Path, error: &io::Error) -> io::Result<()> {
            self.paths.push(path.to_owned());
            if self.strict {
                Err(io::Error::new(error.kind(), error.to_string()))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn cached_target_metadata_errors_obey_the_visitor_policy() {
        let directory = std::env::current_dir().unwrap();
        let metadata = fs::metadata(&directory).unwrap();
        let missing = directory.join(format!("missing-cached-target-{}", std::process::id()));
        assert!(!missing.exists());
        let entry = WalkEntry {
            entry: EntryStorage::Spilled(BufferedEntry {
                name: "missing".into(),
                absolute: missing.clone(),
                file_type: metadata.file_type(),
            }),
        };
        let plan = Query {
            require_empty: true,
            ..Query::default()
        }
        .compile()
        .unwrap();
        for strict in [false, true] {
            let mut visitor = Errors {
                strict,
                paths: Vec::new(),
            };
            let mut runner = Runner::new(&plan, &mut visitor, SystemTime::now());
            let result = runner.passes_entry_metadata(&entry, Some(&metadata));
            if strict {
                assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
            } else {
                assert!(!result.unwrap());
            }
            assert_eq!(runner.stats.errors, 1);
            assert_eq!(runner.stats.metadata_calls, 0);
            assert_eq!(visitor.paths, std::slice::from_ref(&missing));
        }
    }

    #[test]
    fn cached_target_metadata_is_not_counted_as_another_filesystem_call() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let metadata = fs::metadata(&path).unwrap();
        let entry = WalkEntry {
            entry: EntryStorage::Spilled(BufferedEntry {
                name: "Cargo.toml".into(),
                absolute: path,
                file_type: metadata.file_type(),
            }),
        };
        let plan = Query {
            sizes: vec![">0".parse::<SizeFilter>().unwrap()],
            ..Query::default()
        }
        .compile()
        .unwrap();
        let mut visitor = Errors {
            strict: false,
            paths: Vec::new(),
        };
        let mut runner = Runner::new(&plan, &mut visitor, SystemTime::now());
        assert!(
            runner
                .passes_entry_metadata(&entry, Some(&metadata))
                .unwrap()
        );
        assert_eq!(runner.stats.metadata_calls, 0);
    }
}
