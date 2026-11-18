use super::{
    BufferedEntry, CollectedMatches, ErrorCollector, FilesystemError, SearchReport, Visitor,
    metadata_matches, prepare_root, report,
};
use crate::ignore::{IgnoreRules, directory_rule_sources, ignored_child_by_rules};
use crate::path::is_hidden;
use crate::pattern::PathMatchScratch;
use crate::{CompiledQuery, Stats};
use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::SystemTime;

const RESULT_BATCH_PATHS: usize = 32;
#[derive(Clone)]
pub(crate) struct ParallelTask {
    pub(crate) absolute: PathBuf,
    pub(crate) relative: PathBuf,
    pub(crate) positive: Vec<usize>,
    pub(crate) negative: Vec<usize>,
    pub(crate) ignore_rules: IgnoreRules,
}

// Inline live entries avoid a heap allocation per filesystem entry. Spill
// buffers hold compact BufferedEntry records, not this transient enum.
#[allow(clippy::large_enum_variant)]
enum ParallelEntry {
    Live(fs::DirEntry),
    Spilled(BufferedEntry),
}

impl ParallelEntry {
    fn name(&self) -> OsString {
        match self {
            Self::Live(entry) => entry.file_name(),
            Self::Spilled(entry) => entry.name.clone(),
        }
    }

    fn file_type(&self) -> io::Result<fs::FileType> {
        match self {
            Self::Live(entry) => entry.file_type(),
            Self::Spilled(entry) => Ok(entry.file_type),
        }
    }

    fn path(&self) -> PathBuf {
        match self {
            Self::Live(entry) => entry.path(),
            Self::Spilled(entry) => entry.absolute.clone(),
        }
    }
}

enum ParallelEntries {
    Open(Box<fs::ReadDir>),
    Spilled(VecDeque<BufferedEntry>),
}

impl ParallelEntries {
    fn next(&mut self) -> Option<io::Result<(ParallelEntry, bool)>> {
        match self {
            Self::Open(entries) => entries
                .next()
                .map(|result| result.map(|entry| (ParallelEntry::Live(entry), false))),
            Self::Spilled(entries) => entries
                .pop_front()
                .map(|entry| Ok((ParallelEntry::Spilled(entry), true))),
        }
    }
}

struct ParallelFrame {
    task: ParallelTask,
    ignore_rules: IgnoreRules,
    entries: ParallelEntries,
}

#[derive(Default)]
pub(crate) struct ParallelQueueState {
    pub(crate) queued: usize,
    pub(crate) outstanding: usize,
    pub(crate) closed: bool,
    pub(crate) failure: Option<io::Error>,
}

pub(crate) enum ParallelPush {
    Queued,
    Full(ParallelTask),
}

pub(crate) struct ParallelQueue {
    pub(crate) locals: Vec<Mutex<VecDeque<ParallelTask>>>,
    pub(crate) state: Mutex<ParallelQueueState>,
    pub(crate) ready: Condvar,
    pub(crate) cancelled: AtomicBool,
    pub(crate) max_queued: usize,
}

impl ParallelQueue {
    pub(crate) fn new(workers: usize) -> Self {
        Self {
            locals: (0..workers).map(|_| Mutex::new(VecDeque::new())).collect(),
            state: Mutex::new(ParallelQueueState::default()),
            ready: Condvar::new(),
            cancelled: AtomicBool::new(false),
            max_queued: workers.saturating_mul(256).max(256),
        }
    }

    pub(crate) fn push(&self, worker: usize, task: ParallelTask) -> ParallelPush {
        let mut state = self.state.lock().expect("parallel queue state poisoned");
        if state.closed || self.cancelled.load(Ordering::Acquire) || state.queued >= self.max_queued
        {
            return ParallelPush::Full(task);
        }
        state.queued += 1;
        state.outstanding += 1;
        drop(state);
        self.locals[worker % self.locals.len()]
            .lock()
            .expect("parallel local queue poisoned")
            .push_back(task);
        self.ready.notify_one();
        ParallelPush::Queued
    }

    pub(crate) fn try_pop(&self, worker: usize) -> Option<ParallelTask> {
        if self.cancelled.load(Ordering::Acquire) {
            return None;
        }
        if let Some(task) = self.locals[worker]
            .lock()
            .expect("parallel local queue poisoned")
            .pop_back()
        {
            self.decrement_queued();
            return Some(task);
        }
        for offset in 1..self.locals.len() {
            let index = (worker + offset) % self.locals.len();
            if let Some(task) = self.locals[index]
                .lock()
                .expect("parallel local queue poisoned")
                .pop_front()
            {
                self.decrement_queued();
                return Some(task);
            }
        }
        None
    }

    pub(crate) fn pop(&self, worker: usize) -> Option<ParallelTask> {
        loop {
            if let Some(task) = self.try_pop(worker) {
                return Some(task);
            }
            let mut state = self.state.lock().expect("parallel queue state poisoned");
            while state.queued == 0 && !state.closed && !self.cancelled.load(Ordering::Acquire) {
                state = self
                    .ready
                    .wait(state)
                    .expect("parallel queue state poisoned while waiting");
            }
            if state.closed || self.cancelled.load(Ordering::Acquire) {
                return None;
            }
        }
    }

    pub(crate) fn decrement_queued(&self) {
        let mut state = self.state.lock().expect("parallel queue state poisoned");
        state.queued -= 1;
    }

    pub(crate) fn task_done(&self) {
        let mut state = self.state.lock().expect("parallel queue state poisoned");
        state.outstanding -= 1;
        if state.outstanding == 0 {
            state.closed = true;
            self.ready.notify_all();
        }
    }

    pub(crate) fn cancel(&self, error: io::Error) {
        self.cancelled.store(true, Ordering::Release);
        let mut state = self.state.lock().expect("parallel queue state poisoned");
        if state.failure.is_none() {
            state.failure = Some(error);
        }
        state.closed = true;
        self.ready.notify_all();
    }

    pub(crate) fn stop(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.state
            .lock()
            .expect("parallel queue state poisoned")
            .closed = true;
        self.ready.notify_all();
    }

    pub(crate) fn take_failure(&self) -> Option<io::Error> {
        self.state
            .lock()
            .expect("parallel queue state poisoned")
            .failure
            .take()
    }

    pub(crate) fn queued_count(&self) -> usize {
        self.state
            .lock()
            .expect("parallel queue state poisoned")
            .queued
    }

    pub(crate) fn close_if_idle(&self) {
        let mut state = self.state.lock().expect("parallel queue state poisoned");
        if state.outstanding == 0 {
            state.closed = true;
            self.ready.notify_all();
        }
    }
}

pub(crate) struct ParallelWorker<'a> {
    pub(crate) id: usize,
    pub(crate) plan: &'a CompiledQuery,
    pub(crate) queue: Arc<ParallelQueue>,
    pub(crate) strict: bool,
    pub(crate) needs_path: bool,
    pub(crate) output: Vec<PathBuf>,
    events: Option<SyncSender<ParallelEvent>>,
    pending: Vec<PathBuf>,
    pending_count: usize,
    sent_first: bool,
    positive_scratch: Vec<usize>,
    negative_scratch: Vec<usize>,
    errors: Vec<FilesystemError>,
    pub(crate) stats: Stats,
    now: SystemTime,
    ignore_scratch: PathMatchScratch,
}

impl<'a> ParallelWorker<'a> {
    pub(crate) fn new(
        id: usize,
        plan: &'a CompiledQuery,
        queue: Arc<ParallelQueue>,
        strict: bool,
        now: SystemTime,
        needs_path: bool,
    ) -> Self {
        Self {
            id,
            plan,
            queue,
            strict,
            needs_path,
            output: Vec::new(),
            events: None,
            pending: Vec::new(),
            pending_count: 0,
            sent_first: false,
            positive_scratch: Vec::new(),
            negative_scratch: Vec::new(),
            errors: Vec::new(),
            stats: Stats::default(),
            now,
            ignore_scratch: PathMatchScratch::default(),
        }
    }

    fn with_events(mut self, events: SyncSender<ParallelEvent>) -> Self {
        self.events = Some(events);
        self
    }

    fn emit(&mut self, path: PathBuf) {
        if let Some(events) = &self.events {
            if !self.sent_first {
                // The first result must not wait for a batch to fill.
                if events.send(ParallelEvent::Match(path)).is_err() {
                    self.queue.stop();
                    return;
                }
                self.sent_first = true;
            } else {
                self.pending.push(path);
                if self.pending.len() == RESULT_BATCH_PATHS {
                    self.flush_pending();
                }
            }
        } else {
            self.output.push(path);
        }
        self.stats.matches += 1;
    }

    fn flush_pending(&mut self) {
        if !self.pending.is_empty() {
            let paths =
                std::mem::replace(&mut self.pending, Vec::with_capacity(RESULT_BATCH_PATHS));
            if let Some(events) = &self.events
                && events.send(ParallelEvent::Matches(paths)).is_err()
            {
                self.queue.stop();
            }
        }
        if self.pending_count != 0 {
            let count = std::mem::take(&mut self.pending_count);
            if let Some(events) = &self.events
                && events.send(ParallelEvent::Counts(count)).is_err()
            {
                self.queue.stop();
            }
        }
    }

    fn emit_without_path(&mut self) {
        if let Some(events) = &self.events {
            if !self.sent_first {
                if events.send(ParallelEvent::Counts(1)).is_err() {
                    self.queue.stop();
                    return;
                }
                self.sent_first = true;
            } else {
                self.pending_count += 1;
                if self.pending_count == RESULT_BATCH_PATHS {
                    self.flush_pending();
                }
            }
        }
        self.stats.matches += 1;
    }

    pub(crate) fn run(mut self) -> ParallelWorkerResult {
        if self.plan.cancelled() {
            self.queue.stop();
        }
        while let Some(task) = self.queue.pop(self.id) {
            if let Err(error) = self.process(task) {
                self.queue.cancel(error);
            }
            self.queue.task_done();
        }
        ParallelWorkerResult {
            stats: self.stats,
            output: self.output,
            errors: self.errors,
        }
    }

    fn record_error(&mut self, path: &std::path::Path, error: io::Error) -> io::Result<()> {
        self.stats.errors += 1;
        if self.strict {
            return Err(io::Error::new(
                error.kind(),
                format!("{}: {error}", path.display()),
            ));
        }
        let error = FilesystemError {
            path: path.to_owned(),
            error,
        };
        self.flush_pending();
        if let Some(events) = &self.events {
            if events.send(ParallelEvent::Error(error)).is_err() {
                self.queue.stop();
            }
        } else {
            self.errors.push(error);
        }
        Ok(())
    }

    pub(crate) fn process(&mut self, task: ParallelTask) -> io::Result<()> {
        let result = self.process_directory(task);
        // Flush partial batches on task completion, including early returns
        // and errors, before task_done can close the queue.
        self.flush_pending();
        result
    }

    fn process_directory(&mut self, task: ParallelTask) -> io::Result<()> {
        let mut frames = Vec::new();
        if let Some(frame) = self.open_frame(task)? {
            frames.push(frame);
        }
        let worker_count = self.queue.locals.len().max(1);
        let per_worker_budget = (self.plan.max_open / worker_count).max(1);
        while !self.queue.cancelled.load(Ordering::Acquire) && !frames.is_empty() {
            if self.plan.cancelled() {
                self.queue.stop();
                break;
            }
            let next = frames.last_mut().and_then(|frame| frame.entries.next());
            let Some(next) = next else {
                frames.pop();
                continue;
            };
            let frame = frames.last().expect("parallel directory frame exists");
            let (entry, already_counted) = match next {
                Ok(entry) => entry,
                Err(error) => {
                    let path = frame.task.absolute.clone();
                    self.record_error(&path, error)?;
                    continue;
                }
            };
            if !already_counted {
                self.stats.entries_seen += 1;
            }
            let parent = frame.task.clone();
            let rules = frame.ignore_rules.clone();
            if let Some(child) = self.process_entry(&parent, &rules, entry)? {
                let open_count = frames
                    .iter()
                    .filter(|frame| matches!(frame.entries, ParallelEntries::Open(_)))
                    .count();
                if open_count >= per_worker_budget
                    && let Some(oldest) = frames
                        .iter_mut()
                        .find(|frame| matches!(frame.entries, ParallelEntries::Open(_)))
                {
                    self.spill_frame(oldest)?;
                }
                if let Some(frame) = self.open_frame(child)? {
                    frames.push(frame);
                }
            }
        }
        Ok(())
    }

    fn open_frame(&mut self, task: ParallelTask) -> io::Result<Option<ParallelFrame>> {
        if self.queue.cancelled.load(Ordering::Acquire) {
            return Ok(None);
        }
        self.stats.dirs_seen += 1;
        if !self.plan.can_descend(task.relative.components().count()) {
            self.stats.dirs_pruned_depth += 1;
            return Ok(None);
        }
        if self
            .plan
            .negative_program
            .states_exclude_subtree(&task.negative)
        {
            self.stats.dirs_pruned_exclude += 1;
            return Ok(None);
        }
        if !self.plan.positive_matches_all
            && !self
                .plan
                .positive_program
                .states_descendant_possible(&task.positive)
        {
            self.stats.dirs_pruned_positive += 1;
            return Ok(None);
        }
        let mut ignore_rules = task.ignore_rules.clone();
        for (path, loaded) in directory_rule_sources(self.plan, &task.absolute, &task.relative) {
            match loaded {
                Ok(rules) => ignore_rules.extend(rules),
                Err(error) => self.record_error(&path, error)?,
            }
        }
        let entries = match fs::read_dir(&task.absolute) {
            Ok(entries) => entries,
            Err(error) => {
                self.record_error(&task.absolute, error)?;
                return Ok(None);
            }
        };
        self.stats.dirs_opened += 1;
        Ok(Some(ParallelFrame {
            task,
            ignore_rules,
            entries: ParallelEntries::Open(Box::new(entries)),
        }))
    }

    fn spill_frame(&mut self, frame: &mut ParallelFrame) -> io::Result<()> {
        let ParallelEntries::Open(entries) = &mut frame.entries else {
            return Ok(());
        };
        let mut spilled = VecDeque::new();
        let parent = frame.task.absolute.clone();
        for result in entries {
            match result {
                Ok(entry) => {
                    self.stats.entries_seen += 1;
                    match entry.file_type() {
                        Ok(file_type) => spilled.push_back(BufferedEntry {
                            name: entry.file_name(),
                            absolute: entry.path(),
                            file_type,
                        }),
                        Err(error) => self.record_error(&entry.path(), error)?,
                    }
                }
                Err(error) => self.record_error(&parent, error)?,
            }
        }
        frame.entries = ParallelEntries::Spilled(spilled);
        Ok(())
    }

    fn process_entry(
        &mut self,
        parent: &ParallelTask,
        ignore_rules: &IgnoreRules,
        entry: ParallelEntry,
    ) -> io::Result<Option<ParallelTask>> {
        let name = entry.name();
        if !self.plan.hidden && is_hidden(&name) {
            return Ok(None);
        }
        let mut file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => return self.record_error(&entry.path(), error).map(|()| None),
        };
        let followed_metadata = if file_type.is_symlink() && self.plan.follow_symlinks {
            self.stats.metadata_calls += 1;
            match fs::metadata(entry.path()) {
                Ok(metadata) => {
                    file_type = metadata.file_type();
                    Some(metadata)
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return self.record_error(&entry.path(), error).map(|()| None),
            }
        } else {
            None
        };
        let relative = (file_type.is_dir()
            || self.needs_path
            || self.plan.uses_ignores()
            || self.plan.search_target == crate::SearchTarget::Path)
            .then(|| parent.relative.join(&name));
        if self.plan.uses_ignores()
            && ignored_child_by_rules(
                ignore_rules.as_slice(),
                relative
                    .as_deref()
                    .expect("ignore checks require a relative path"),
                file_type.is_dir(),
                &mut self.ignore_scratch,
            )
        {
            if file_type.is_dir() {
                self.stats.dirs_pruned_ignore += 1;
            }
            return Ok(None);
        }
        let mut positive = std::mem::take(&mut self.positive_scratch);
        let mut negative = std::mem::take(&mut self.negative_scratch);
        if !self.plan.selection_all_non_root && !self.plan.positive_matches_all {
            self.plan.positive_program.advance_into_with_mode(
                &parent.positive,
                &name,
                self.plan.case_insensitive,
                self.plan.unicode,
                &mut positive,
            );
        }
        if !self.plan.selection_all_non_root {
            self.plan.negative_program.advance_into_with_mode(
                &parent.negative,
                &name,
                self.plan.case_insensitive,
                self.plan.unicode,
                &mut negative,
            );
        }
        let depth = parent.relative.components().count() + 1;
        if file_type.is_dir() {
            let relative = relative.expect("directory traversal requires a relative path");
            if self.plan.type_matches(&file_type)
                && (self.plan.selection_all_non_root
                    || self
                        .plan
                        .states_match(&positive, &negative, &name, &relative)
                        && self.plan.extension_matches(&name)
                        && self.plan.depth_matches(depth))
                && self.passes_entry_metadata(&entry.path(), followed_metadata.as_ref())?
            {
                if self.needs_path {
                    self.emit(relative.clone());
                } else {
                    self.emit_without_path();
                }
            }
            let child = ParallelTask {
                absolute: entry.path(),
                relative,
                positive,
                negative,
                ignore_rules: ignore_rules.clone(),
            };
            if let ParallelPush::Full(child) = self.queue.push(self.id, child) {
                return Ok(Some(child));
            }
        } else {
            self.stats.candidate_files += 1;
            let type_matches = self.plan.type_matches(&file_type);
            if type_matches
                && (self.plan.selection_all_non_root
                    || self.plan.depth_matches(depth)
                        && self.plan.extension_matches(&name)
                        && relative.as_deref().map_or_else(
                            || self.plan.states_match_name(&positive, &negative, &name),
                            |relative| {
                                self.plan
                                    .states_match(&positive, &negative, &name, relative)
                            },
                        ))
                && self.passes_entry_metadata(&entry.path(), followed_metadata.as_ref())?
            {
                if self.needs_path {
                    self.emit(relative.expect("path consumers require a relative path"));
                } else {
                    self.emit_without_path();
                }
            }
            self.positive_scratch = positive;
            self.negative_scratch = negative;
        }
        Ok(None)
    }

    fn passes_metadata(
        &mut self,
        absolute: &std::path::Path,
        cached: Option<&fs::Metadata>,
    ) -> io::Result<bool> {
        match metadata_matches(self.plan, absolute, &mut self.stats, self.now, cached) {
            Ok(matches) => Ok(matches),
            Err(error) => {
                self.record_error(absolute, error)?;
                Ok(false)
            }
        }
    }

    fn passes_entry_metadata(
        &mut self,
        absolute: &std::path::Path,
        cached: Option<&fs::Metadata>,
    ) -> io::Result<bool> {
        if !self.plan.needs_metadata() {
            return Ok(true);
        }
        self.passes_metadata(absolute, cached)
    }
}

pub(crate) struct ParallelWorkerResult {
    pub(crate) stats: Stats,
    pub(crate) output: Vec<PathBuf>,
    errors: Vec<FilesystemError>,
}

enum ParallelEvent {
    Match(PathBuf),
    Matches(Vec<PathBuf>),
    Counts(usize),
    Error(FilesystemError),
}

pub(crate) fn visit_parallel<V: Visitor>(
    plan: &CompiledQuery,
    threads: usize,
    strict: bool,
    visitor: &mut V,
) -> io::Result<SearchReport> {
    let visitor_needs_path = visitor.requires_path();
    let now = SystemTime::now();
    plan.metadata.validate_time(now)?;
    let mut stats = Stats::default();
    if plan.cancelled() {
        return Ok(report(stats, true));
    }
    let mut errors = ErrorCollector::default();
    let start = prepare_root(plan, &mut stats, &mut errors)?;
    for error in errors.0 {
        if strict {
            return Err(io::Error::new(
                error.error.kind(),
                format!("{}: {}", error.path.display(), error.error),
            ));
        }
        visitor.error(&error.path, &error.error)?;
    }
    let Some(start) = start else {
        return Ok(report(stats, plan.cancelled()));
    };
    if plan.cancelled() {
        return Ok(report(stats, true));
    }
    let (positive, negative) = plan.root_states();
    let root_name = plan.root.file_name().unwrap_or_default();
    let type_matches = plan.type_matches(&start.file_type);
    if !plan.root_relative.as_os_str().is_empty()
        && !start.ignored
        && type_matches
        && plan.states_match(&positive, &negative, root_name, &plan.root_relative)
        && plan.extension_matches(root_name)
        && plan.depth_matches(plan.root_relative.components().count())
    {
        let selected =
            match metadata_matches(plan, &plan.root, &mut stats, now, Some(&start.metadata)) {
                Ok(selected) => selected,
                Err(error) if strict => return Err(error),
                Err(error) => {
                    stats.errors += 1;
                    visitor.error(&plan.root, &error)?;
                    false
                }
            };
        if selected {
            stats.matches += 1;
            let outcome = if visitor_needs_path {
                visitor.matched(&plan.root_relative)?
            } else {
                visitor.matched_without_path()?
            };
            if outcome.is_break() {
                return Ok(report(stats, true));
            }
        }
    }
    if !start.file_type.is_dir() {
        return Ok(report(stats, plan.cancelled()));
    }
    let queue = Arc::new(ParallelQueue::new(threads));
    let root = ParallelTask {
        absolute: plan.root.clone(),
        relative: plan.root_relative.clone(),
        positive,
        negative,
        ignore_rules: start.ignore_rules,
    };
    if let ParallelPush::Full(_) = queue.push(0, root) {
        return Err(io::Error::other("failed to schedule parallel root"));
    }
    // Only bounded events cross the worker/coordinator boundary. Dropping the
    // receiver before joining is essential: it releases senders blocked by a
    // slow visitor when that visitor stops or returns an error.
    // At most 32 paths/event and one queued event/worker. Together with
    // pending/blocked sends and the current batch, delivery holds at most
    // 64 * workers + 32 paths, independent of the total result count.
    let (sender, receiver) = sync_channel(threads);
    let mut stopped = false;
    let mut visitor_error = None;
    let mut delivered = stats.matches;
    let results = thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|id| {
                let worker = ParallelWorker::new(
                    id,
                    plan,
                    Arc::clone(&queue),
                    strict,
                    now,
                    visitor_needs_path,
                )
                .with_events(sender.clone());
                scope.spawn(move || worker.run())
            })
            .collect();
        drop(sender);
        'receive: while let Ok(event) = receiver.recv() {
            if plan.cancelled() {
                stopped = true;
                queue.stop();
                break;
            }
            if queue.cancelled.load(Ordering::Acquire) {
                break;
            }
            let paths = match event {
                ParallelEvent::Match(path) => vec![path],
                ParallelEvent::Matches(paths) => paths,
                ParallelEvent::Counts(count) => {
                    for _ in 0..count {
                        if queue.cancelled.load(Ordering::Acquire) {
                            break 'receive;
                        }
                        delivered += 1;
                        match visitor.matched_without_path() {
                            Ok(ControlFlow::Continue(())) => {}
                            Ok(ControlFlow::Break(())) => {
                                stopped = true;
                                queue.stop();
                                break 'receive;
                            }
                            Err(error) => {
                                visitor_error = Some(error);
                                queue.stop();
                                break 'receive;
                            }
                        }
                    }
                    continue;
                }
                ParallelEvent::Error(error) => {
                    if let Err(error) = visitor.error(&error.path, &error.error) {
                        visitor_error = Some(error);
                        queue.stop();
                        break;
                    }
                    continue;
                }
            };
            for path in paths {
                if queue.cancelled.load(Ordering::Acquire) {
                    break 'receive;
                }
                delivered += 1;
                match visitor.matched(&path) {
                    Ok(ControlFlow::Continue(())) => {}
                    Ok(ControlFlow::Break(())) => {
                        stopped = true;
                        queue.stop();
                        break 'receive;
                    }
                    Err(error) => {
                        visitor_error = Some(error);
                        queue.stop();
                        break 'receive;
                    }
                }
            }
        }
        drop(receiver);
        handles
            .into_iter()
            .map(|handle| handle.join().expect("parallel worker panicked"))
            .collect::<Vec<_>>()
    });
    if let Some(error) = visitor_error {
        return Err(error);
    }
    if let Some(error) = queue.take_failure() {
        return Err(error);
    }
    for result in results {
        stats.merge(result.stats);
    }
    stats.matches = delivered;
    Ok(report(stats, stopped || plan.cancelled()))
}

pub(crate) fn run_parallel(
    plan: &CompiledQuery,
    requested_threads: usize,
    strict: bool,
) -> io::Result<CollectedMatches> {
    let now = SystemTime::now();
    plan.metadata.validate_time(now)?;
    let mut initial_stats = Stats::default();
    if plan.cancelled() {
        return Ok(CollectedMatches {
            paths: Vec::new(),
            report: report(initial_stats, true),
            errors: Vec::new(),
        });
    }
    let mut errors = ErrorCollector::default();
    let start = prepare_root(plan, &mut initial_stats, &mut errors)?;
    if strict && let Some(error) = errors.0.first() {
        return Err(io::Error::new(
            error.error.kind(),
            format!("{}: {}", error.path.display(), error.error),
        ));
    }
    let Some(start) = start else {
        return Ok(CollectedMatches {
            paths: Vec::new(),
            report: report(initial_stats, plan.cancelled()),
            errors: errors.0,
        });
    };
    if !start.file_type.is_dir() {
        let (positive, negative) = plan.root_states();
        let name = plan.root.file_name().unwrap_or_default();
        let type_matches = plan.type_matches(&start.file_type);
        let mut paths = Vec::new();
        if type_matches
            && plan.states_match(&positive, &negative, name, &plan.root_relative)
            && plan.extension_matches(name)
            && plan.depth_matches(plan.root_relative.components().count())
        {
            match metadata_matches(
                plan,
                &plan.root,
                &mut initial_stats,
                now,
                Some(&start.metadata),
            ) {
                Ok(true) => {
                    paths.push(plan.root_relative.clone());
                    initial_stats.matches = 1;
                }
                Ok(false) => {}
                Err(error) if strict => return Err(error),
                Err(error) => {
                    initial_stats.errors += 1;
                    errors.0.push(FilesystemError {
                        path: plan.root.clone(),
                        error,
                    });
                }
            }
        }
        return Ok(CollectedMatches {
            paths,
            report: report(initial_stats, plan.cancelled()),
            errors: errors.0,
        });
    }
    let worker_count = requested_threads.clamp(1, 64);
    let queue = Arc::new(ParallelQueue::new(worker_count));
    let (positive, negative) = plan.root_states();
    let root = ParallelTask {
        absolute: plan.root.clone(),
        relative: plan.root_relative.clone(),
        positive,
        negative,
        ignore_rules: start.ignore_rules,
    };
    let mut main_worker = ParallelWorker::new(0, plan, Arc::clone(&queue), strict, now, true);
    if !plan.root_relative.as_os_str().is_empty()
        && !start.ignored
        && plan.type_matches(&start.file_type)
        && plan.states_match(
            &root.positive,
            &root.negative,
            plan.root.file_name().unwrap_or_default(),
            &plan.root_relative,
        )
        && plan.extension_matches(plan.root.file_name().unwrap_or_default())
        && plan.depth_matches(plan.root_relative.components().count())
        && main_worker.passes_metadata(&plan.root, Some(&start.metadata))?
    {
        main_worker.output.push(plan.root_relative.clone());
        main_worker.stats.matches += 1;
    }
    if let Err(error) = main_worker.process(root) {
        queue.cancel(error);
    }
    while worker_count > 1 && queue.queued_count() <= 2 && !queue.cancelled.load(Ordering::Acquire)
    {
        let Some(task) = queue.try_pop(0) else {
            break;
        };
        if let Err(error) = main_worker.process(task) {
            queue.cancel(error);
        }
        queue.task_done();
    }
    queue.close_if_idle();
    let helper_count = if queue.queued_count() > 2 {
        worker_count.saturating_sub(1)
    } else {
        0
    };
    let results = thread::scope(|scope| {
        let handles: Vec<_> = (1..=helper_count)
            .map(|id| {
                let queue = Arc::clone(&queue);
                scope.spawn(move || ParallelWorker::new(id, plan, queue, strict, now, true).run())
            })
            .collect();
        let mut results = vec![main_worker.run()];
        for handle in handles {
            results.push(handle.join().expect("parallel worker panicked"));
        }
        results
    });
    if let Some(error) = queue.take_failure() {
        return Err(error);
    }
    let mut stats = initial_stats;
    let mut output = Vec::new();
    for result in results {
        stats.merge(result.stats);
        output.extend(result.output);
        errors.0.extend(result.errors);
    }
    Ok(CollectedMatches {
        paths: output,
        report: report(stats, plan.cancelled()),
        errors: errors.0,
    })
}
