use super::{Visitor, prepare_root};
use crate::ignore::{IgnoreRule, ignored_by_rules, load_ignore_rules};
use crate::path::is_hidden;
use crate::{CompiledQuery, EntryType, Stats};
use std::fs;
use std::io;
use std::path::PathBuf;
pub(crate) struct Runner<'a, V: Visitor> {
    plan: &'a CompiledQuery,
    visitor: &'a mut V,
    pub(crate) stats: Stats,
    pub(crate) stopped: bool,
}

impl<'a, V: Visitor> Runner<'a, V> {
    pub(crate) fn new(plan: &'a CompiledQuery, visitor: &'a mut V) -> Self {
        Self {
            plan,
            visitor,
            stats: Stats::default(),
            stopped: false,
        }
    }

    pub(crate) fn run(&mut self) -> io::Result<()> {
        let Some(root) = prepare_root(self.plan, &mut self.stats, self.visitor)? else {
            return Ok(());
        };
        let root_type = root.file_type;
        let (positive_states, negative_states) = self.plan.root_states();
        let root_name = self.plan.root.file_name().unwrap_or_default();
        if !root_type.is_dir() {
            let type_matches = match self.plan.wanted_type {
                EntryType::File => root_type.is_file(),
                EntryType::Dir => false,
                EntryType::Symlink => root_type.is_symlink(),
            };
            if type_matches
                && self
                    .plan
                    .states_match(&positive_states, &negative_states, root_name)
                && self.plan.extension_matches(root_name)
            {
                self.emit(self.plan.root_relative.clone())?;
            }
            return Ok(());
        }
        if !self.plan.root_relative.as_os_str().is_empty()
            && !root.ignored
            && self.plan.wanted_type == EntryType::Dir
            && self
                .plan
                .states_match(&positive_states, &negative_states, root_name)
        {
            self.emit(self.plan.root_relative.clone())?;
        }
        self.visit_directory(
            self.plan.root.clone(),
            self.plan.root_relative.clone(),
            positive_states,
            negative_states,
            root.ignore_rules,
        )?;
        Ok(())
    }

    fn visit_directory(
        &mut self,
        absolute: PathBuf,
        relative: PathBuf,
        positive_states: Vec<usize>,
        negative_states: Vec<usize>,
        mut ignore_rules: Vec<IgnoreRule>,
    ) -> io::Result<()> {
        if self.stopped {
            return Ok(());
        }
        self.stats.dirs_seen += 1;
        if self.plan.gitignore {
            match load_ignore_rules(&absolute.join(".gitignore"), &relative) {
                Ok(rules) => ignore_rules.extend(rules),
                Err(error) => {
                    self.stats.errors += 1;
                    self.visitor.error(&absolute.join(".gitignore"), &error)?;
                }
            }
        }
        if self
            .plan
            .negative_program
            .states_exclude_subtree(&negative_states)
        {
            self.stats.dirs_pruned_exclude += 1;
            return Ok(());
        }
        if !self
            .plan
            .positive_program
            .states_descendant_possible(&positive_states)
        {
            self.stats.dirs_pruned_positive += 1;
            return Ok(());
        }
        let entries = match fs::read_dir(&absolute) {
            Ok(entries) => entries,
            Err(error) => {
                self.stats.errors += 1;
                self.visitor.error(&absolute, &error)?;
                return Ok(());
            }
        };
        self.stats.dirs_opened += 1;
        for result in entries {
            if self.stopped {
                break;
            }
            let entry = match result {
                Ok(entry) => entry,
                Err(error) => {
                    self.stats.errors += 1;
                    self.visitor.error(&absolute, &error)?;
                    continue;
                }
            };
            self.stats.entries_seen += 1;
            let name = entry.file_name();
            if !self.plan.hidden && is_hidden(&name) {
                continue;
            }
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    self.stats.errors += 1;
                    self.visitor.error(&entry.path(), &error)?;
                    continue;
                }
            };
            let child_relative = relative.join(&name);
            if self.plan.gitignore {
                let (ignored, may_reinclude) = ignored_by_rules(&ignore_rules, &child_relative);
                if ignored && !(file_type.is_dir() && may_reinclude) {
                    if file_type.is_dir() {
                        self.stats.dirs_pruned_ignore += 1;
                    }
                    continue;
                }
            }
            let child_positive = self.plan.positive_program.advance(&positive_states, &name);
            let child_negative = self.plan.negative_program.advance(&negative_states, &name);
            if file_type.is_dir() {
                if self.plan.wanted_type == EntryType::Dir
                    && self
                        .plan
                        .states_match(&child_positive, &child_negative, &name)
                {
                    self.emit(child_relative.clone())?;
                }
                self.visit_directory(
                    entry.path(),
                    child_relative,
                    child_positive,
                    child_negative,
                    ignore_rules.clone(),
                )?;
            } else {
                self.stats.candidate_files += 1;
                let type_matches = match self.plan.wanted_type {
                    EntryType::File => file_type.is_file(),
                    EntryType::Dir => false,
                    EntryType::Symlink => file_type.is_symlink(),
                };
                if type_matches
                    && self.plan.extension_matches(&name)
                    && self
                        .plan
                        .states_match(&child_positive, &child_negative, &name)
                {
                    self.emit(child_relative)?;
                }
            }
        }
        Ok(())
    }

    fn emit(&mut self, path: PathBuf) -> io::Result<()> {
        self.stats.matches += 1;
        self.stopped = self.visitor.matched(&path)?.is_break();
        Ok(())
    }
}
