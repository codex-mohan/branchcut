//! Filesystem work performed during one execution.
#[derive(Default, Debug)]
pub struct Stats {
    pub dirs_seen: u64,
    pub dirs_opened: u64,
    pub dirs_pruned_positive: u64,
    pub dirs_pruned_exclude: u64,
    pub dirs_pruned_ignore: u64,
    pub dirs_pruned_depth: u64,
    pub dirs_pruned_policy: u64,
    pub entries_seen: u64,
    pub candidate_files: u64,
    pub metadata_calls: u64,
    pub matches: u64,
    pub errors: u64,
}

impl Stats {
    pub(crate) fn merge(&mut self, other: Stats) {
        self.dirs_seen += other.dirs_seen;
        self.dirs_opened += other.dirs_opened;
        self.dirs_pruned_positive += other.dirs_pruned_positive;
        self.dirs_pruned_exclude += other.dirs_pruned_exclude;
        self.dirs_pruned_ignore += other.dirs_pruned_ignore;
        self.dirs_pruned_depth += other.dirs_pruned_depth;
        self.dirs_pruned_policy += other.dirs_pruned_policy;
        self.entries_seen += other.entries_seen;
        self.candidate_files += other.candidate_files;
        self.metadata_calls += other.metadata_calls;
        self.matches += other.matches;
        self.errors += other.errors;
    }
}
