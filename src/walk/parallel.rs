use super::{CollectedMatches, ErrorCollector, FilesystemError, Visitor, prepare_root, report};
use crate::ignore::{IgnoreRule, ignored_by_rules, load_ignore_rules};
use crate::path::is_hidden;
use crate::{CompiledQuery, EntryType, Stats};
use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
#[derive(Clone)]
pub(crate) struct ParallelTask {
    pub(crate) absolute: PathBuf,
    pub(crate) relative: PathBuf,
    pub(crate) positive: Vec<usize>,
    pub(crate) negative: Vec<usize>,
    pub(crate) ignore_rules: Vec<IgnoreRule>,
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
    pub(crate) entries: Vec<fs::DirEntry>,
    pub(crate) output: Vec<PathBuf>,
    errors: Vec<FilesystemError>,
    pub(crate) stats: Stats,
}

impl<'a> ParallelWorker<'a> {
    pub(crate) fn new(
        id: usize,
        plan: &'a CompiledQuery,
        queue: Arc<ParallelQueue>,
        strict: bool,
    ) -> Self {
        Self {
            id,
            plan,
            queue,
            strict,
            entries: Vec::with_capacity(128),
            output: Vec::new(),
            errors: Vec::new(),
            stats: Stats::default(),
        }
    }

    pub(crate) fn run(mut self) -> ParallelWorkerResult {
        while let Some(task) = self.queue.pop(self.id) {
            let path = task.absolute.clone();
            if let Err(error) = self.process(task) {
                self.stats.errors += 1;
                self.errors.push(FilesystemError {
                    path,
                    error: io::Error::new(error.kind(), error.to_string()),
                });
                if self.strict {
                    self.queue.cancel(error);
                }
            }
            self.queue.task_done();
        }
        ParallelWorkerResult {
            stats: self.stats,
            output: self.output,
            errors: self.errors,
        }
    }

    pub(crate) fn process(&mut self, task: ParallelTask) -> io::Result<()> {
        if self.queue.cancelled.load(Ordering::Acquire) {
            return Ok(());
        }
        self.stats.dirs_seen += 1;
        if self
            .plan
            .negative_program
            .states_exclude_subtree(&task.negative)
        {
            self.stats.dirs_pruned_exclude += 1;
            return Ok(());
        }
        if !self
            .plan
            .positive_program
            .states_descendant_possible(&task.positive)
        {
            self.stats.dirs_pruned_positive += 1;
            return Ok(());
        }
        let mut ignore_rules = task.ignore_rules.clone();
        if self.plan.gitignore {
            ignore_rules.extend(load_ignore_rules(
                &task.absolute.join(".gitignore"),
                &task.relative,
            )?);
        }
        self.entries.clear();
        for entry in fs::read_dir(&task.absolute)? {
            self.entries.push(entry?);
        }
        self.stats.dirs_opened += 1;
        self.stats.entries_seen += self.entries.len() as u64;
        let entries = std::mem::take(&mut self.entries);
        for entry in entries {
            if self.queue.cancelled.load(Ordering::Acquire) {
                break;
            }
            self.process_entry(&task, &ignore_rules, entry)?;
        }
        self.entries.clear();
        Ok(())
    }

    pub(crate) fn process_entry(
        &mut self,
        parent: &ParallelTask,
        ignore_rules: &[IgnoreRule],
        entry: fs::DirEntry,
    ) -> io::Result<()> {
        let name = entry.file_name();
        if !self.plan.hidden && is_hidden(&name) {
            return Ok(());
        }
        let file_type = entry.file_type()?;
        let relative = parent.relative.join(&name);
        if self.plan.gitignore {
            let (ignored, may_reinclude) = ignored_by_rules(ignore_rules, &relative);
            if ignored && !(file_type.is_dir() && may_reinclude) {
                if file_type.is_dir() {
                    self.stats.dirs_pruned_ignore += 1;
                }
                return Ok(());
            }
        }
        let positive = self.plan.positive_program.advance(&parent.positive, &name);
        let negative = self.plan.negative_program.advance(&parent.negative, &name);
        if file_type.is_dir() {
            if self.plan.wanted_type == EntryType::Dir
                && self.plan.states_match(&positive, &negative, &name)
            {
                self.output.push(relative.clone());
                self.stats.matches += 1;
            }
            let child = ParallelTask {
                absolute: entry.path(),
                relative,
                positive,
                negative,
                ignore_rules: ignore_rules.to_vec(),
            };
            if let ParallelPush::Full(child) = self.queue.push(self.id, child) {
                self.process(child)?;
            }
        } else {
            self.stats.candidate_files += 1;
            let type_matches = match self.plan.wanted_type {
                EntryType::File => file_type.is_file(),
                EntryType::Dir => false,
                EntryType::Symlink => file_type.is_symlink(),
            };
            if type_matches
                && self.plan.extension_matches(&name)
                && self.plan.states_match(&positive, &negative, &name)
            {
                self.output.push(relative);
                self.stats.matches += 1;
            }
        }
        Ok(())
    }
}

pub(crate) struct ParallelWorkerResult {
    pub(crate) stats: Stats,
    pub(crate) output: Vec<PathBuf>,
    errors: Vec<FilesystemError>,
}

pub(crate) fn run_parallel(
    plan: &CompiledQuery,
    requested_threads: usize,
    strict: bool,
) -> io::Result<CollectedMatches> {
    let mut initial_stats = Stats::default();
    let mut errors = ErrorCollector::default();
    let Some(start) = prepare_root(plan, &mut initial_stats, &mut errors)? else {
        return Ok(CollectedMatches {
            paths: Vec::new(),
            report: report(initial_stats, false),
            errors: errors.0,
        });
    };
    if !start.file_type.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "parallel traversal requires a directory root",
        ));
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
    let mut main_worker = ParallelWorker::new(0, plan, Arc::clone(&queue), strict);
    if !plan.root_relative.as_os_str().is_empty()
        && !start.ignored
        && plan.wanted_type == EntryType::Dir
        && plan.states_match(
            &root.positive,
            &root.negative,
            plan.root.file_name().unwrap_or_default(),
        )
    {
        main_worker.output.push(plan.root_relative.clone());
        main_worker.stats.matches += 1;
    }
    let root_path = root.absolute.clone();
    if let Err(error) = main_worker.process(root) {
        main_worker.stats.errors += 1;
        errors.error(&root_path, &error)?;
        if strict {
            queue.cancel(error);
        }
    }
    while worker_count > 1 && queue.queued_count() <= 2 && !queue.cancelled.load(Ordering::Acquire)
    {
        let Some(task) = queue.try_pop(0) else {
            break;
        };
        let task_path = task.absolute.clone();
        if let Err(error) = main_worker.process(task) {
            main_worker.stats.errors += 1;
            errors.error(&task_path, &error)?;
            if strict {
                queue.cancel(error);
            }
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
                scope.spawn(move || ParallelWorker::new(id, plan, queue, strict).run())
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
        report: report(stats, false),
        errors: errors.0,
    })
}
