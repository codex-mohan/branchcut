//! Bounded file scanning for the content CLI. The caller controls output order.

use crate::{
    ContentQuery, ContentRecord, ContentRecordKind, ContentScanOptions, ContentSpan, ScanStats,
};
use std::fs::File;
use std::io::{self, Read};
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle, Thread};

// A later file may finish first, but only a few records per file can wait
// behind an earlier file. In particular, dense results cannot collect an
// entire file while the coordinator processes its predecessor.
const RECORDS_PER_FILE: usize = 2;

pub(crate) struct OwnedContentRecord {
    line_number: u64,
    byte_offset: u64,
    line: Vec<u8>,
    spans: Vec<ContentSpan>,
    kind: ContentRecordKind,
}

impl OwnedContentRecord {
    pub(crate) fn as_borrowed(&self) -> ContentRecord<'_> {
        ContentRecord {
            line_number: self.line_number,
            byte_offset: self.byte_offset,
            line: &self.line,
            spans: &self.spans,
            kind: self.kind,
        }
    }
}

impl From<ContentRecord<'_>> for OwnedContentRecord {
    fn from(record: ContentRecord<'_>) -> Self {
        Self {
            line_number: record.line_number,
            byte_offset: record.byte_offset,
            line: record.line.to_vec(),
            spans: record.spans.to_vec(),
            kind: record.kind,
        }
    }
}

pub(crate) enum ContentEvent {
    Record(OwnedContentRecord),
    Done(io::Result<ScanStats>),
}

#[derive(Default)]
struct JobState {
    cancelled: AtomicBool,
    worker: OnceLock<Thread>,
}

impl JobState {
    fn wake_worker(&self) {
        if let Some(worker) = self.worker.get() {
            worker.unpark();
        }
    }

    fn stopped(&self, pool_stopped: &AtomicBool) -> bool {
        pool_stopped.load(Ordering::Acquire) || self.cancelled.load(Ordering::Acquire)
    }
}

pub(crate) struct ContentReceiver {
    state: Arc<JobState>,
    events: Receiver<ContentEvent>,
}

impl ContentReceiver {
    pub(crate) fn recv(&self) -> Result<ContentEvent, mpsc::RecvError> {
        let event = self.events.recv()?;
        self.state.wake_worker();
        Ok(event)
    }
}

impl Drop for ContentReceiver {
    fn drop(&mut self) {
        self.state.cancelled.store(true, Ordering::Release);
        self.state.wake_worker();
    }
}

struct Job {
    path: PathBuf,
    stop_after: Option<u64>,
    state: Arc<JobState>,
    events: SyncSender<ContentEvent>,
}

impl Job {
    fn send(&self, mut event: ContentEvent, pool_stopped: &AtomicBool) -> bool {
        while !self.state.stopped(pool_stopped) {
            match self.events.try_send(event) {
                Ok(()) => return true,
                Err(TrySendError::Disconnected(_)) => return false,
                Err(TrySendError::Full(pending)) => {
                    event = pending;
                    // A recv, cancellation, or shutdown gives this thread an
                    // unpark permit even if it happens just before park().
                    thread::park();
                }
            }
        }
        false
    }

    fn scan(&self, query: &ContentQuery, options: &ContentScanOptions, pool_stopped: &AtomicBool) {
        if self.state.stopped(pool_stopped) {
            return;
        }
        let _ = self.state.worker.set(thread::current());
        let result = File::open(&self.path).and_then(|file| {
            let reader = CancellableReader {
                file,
                state: &self.state,
                pool_stopped,
            };
            query.scan_reader(reader, options, self.stop_after, |record| {
                Ok(
                    if self.send(ContentEvent::Record(record.into()), pool_stopped) {
                        ControlFlow::Continue(())
                    } else {
                        ControlFlow::Break(())
                    },
                )
            })
        });
        self.send(ContentEvent::Done(result), pool_stopped);
    }
}

struct CancellableReader<'a> {
    file: File,
    state: &'a JobState,
    pool_stopped: &'a AtomicBool,
}

impl Read for CancellableReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.state.stopped(self.pool_stopped) {
            // Interrupted would tell BufRead to retry the read.
            return Err(io::Error::other("content scan cancelled"));
        }
        self.file.read(buffer)
    }
}

pub(crate) struct ContentWorkers {
    jobs: Option<SyncSender<Job>>,
    stopped: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

impl ContentWorkers {
    pub(crate) fn new(
        query: ContentQuery,
        options: ContentScanOptions,
        workers: usize,
    ) -> io::Result<Self> {
        if workers == 0 || workers > 64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "content scanning requires 1 to 64 workers",
            ));
        }
        let (sender, jobs) = mpsc::sync_channel::<Job>(workers);
        let jobs = Arc::new(Mutex::new(jobs));
        let mut pool = Self {
            jobs: Some(sender),
            stopped: Arc::new(AtomicBool::new(false)),
            threads: Vec::with_capacity(workers),
        };
        for index in 0..workers {
            let jobs = Arc::clone(&jobs);
            let query = query.clone();
            let stopped = Arc::clone(&pool.stopped);
            pool.threads.push(
                thread::Builder::new()
                    .name(format!("branchcut-content-{index}"))
                    .spawn(move || {
                        while !stopped.load(Ordering::Acquire) {
                            let job = match jobs.lock() {
                                Ok(receiver) => receiver.recv(),
                                Err(_) => break,
                            };
                            match job {
                                Ok(job) => job.scan(&query, &options, &stopped),
                                Err(_) => break,
                            }
                        }
                    })?,
            );
        }
        Ok(pool)
    }

    pub(crate) fn submit(
        &self,
        path: PathBuf,
        stop_after: Option<u64>,
    ) -> io::Result<ContentReceiver> {
        let state = Arc::new(JobState::default());
        let (events, receiver) = mpsc::sync_channel(RECORDS_PER_FILE);
        self.jobs
            .as_ref()
            .ok_or_else(|| io::Error::other("content scanner is closed"))?
            .send(Job {
                path,
                stop_after,
                state: Arc::clone(&state),
                events,
            })
            .map_err(|_| io::Error::other("content scan workers disconnected"))?;
        Ok(ContentReceiver {
            state,
            events: receiver,
        })
    }
}

impl Drop for ContentWorkers {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        drop(self.jobs.take());
        for worker in &self.threads {
            worker.thread().unpark();
        }
        for worker in self.threads.drain(..) {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!(
                "branchcut-content-workers-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            Self(root)
        }

        fn write(&self, name: &str, data: impl AsRef<[u8]>) -> PathBuf {
            let path = self.0.join(Path::new(name));
            fs::write(&path, data).unwrap();
            path
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn workers(count: usize) -> ContentWorkers {
        ContentWorkers::new(
            ContentQuery::from_strings(&["hit"], false, false).unwrap(),
            ContentScanOptions::default(),
            count,
        )
        .unwrap()
    }

    fn next_record(events: &ContentReceiver) -> OwnedContentRecord {
        let ContentEvent::Record(record) = events.recv().unwrap() else {
            panic!("expected a record");
        };
        record
    }

    #[test]
    fn cancelling_a_file_releases_its_worker_for_the_next_submission() {
        let fixture = Fixture::new();
        let workers = workers(1);
        let dense = workers
            .submit(fixture.write("dense.txt", "hit\n".repeat(1024)), None)
            .unwrap();
        assert_eq!(next_record(&dense).as_borrowed().line_number, 1);
        let next = workers
            .submit(
                fixture.write("next.txt", "no\nhit here\nhit again\n"),
                Some(1),
            )
            .unwrap();
        drop(dense);

        let record = next_record(&next);
        let record = record.as_borrowed();
        assert_eq!(record.line_number, 2);
        assert_eq!(record.byte_offset, 3);
        assert_eq!(record.line, b"hit here");
        let ContentEvent::Done(result) = next.recv().unwrap() else {
            panic!("expected exactly one record followed by completion");
        };
        assert_eq!(result.unwrap().matched_lines, 1);
    }

    #[test]
    fn shutdown_wakes_a_worker_even_while_its_receiver_is_retained() {
        let fixture = Fixture::new();
        let workers = workers(1);
        let events = workers
            .submit(fixture.write("dense.txt", "hit\n".repeat(1024)), None)
            .unwrap();
        assert_eq!(next_record(&events).as_borrowed().line_number, 1);
        let (finished, completion) = mpsc::channel();
        let shutdown = thread::spawn(move || {
            drop(workers);
            finished.send(()).unwrap();
        });
        completion
            .recv_timeout(Duration::from_secs(5))
            .expect("shutdown waits on a worker blocked behind its full result queue");
        drop(events);
        shutdown.join().unwrap();
    }
}
