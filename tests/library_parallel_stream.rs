use branchcut::{Completion, EntryType, Query, Visitor};
use std::cell::Cell;
use std::fs;
use std::io;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::Duration;

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "branchcut-parallel-stream-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn write(&self, relative: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"fixture").unwrap();
    }
    fn query(&self, pattern: &str) -> Query {
        Query {
            cwd: self.0.clone(),
            patterns: vec![pattern.into()],
            ..Query::default()
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn within_deadline<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || send.send(work()).unwrap());
    receive
        .recv_timeout(Duration::from_secs(15))
        .expect("parallel streaming did not finish; possible cancellation deadlock")
}

#[test]
fn parallel_streaming_matches_sequential_and_allows_a_non_send_visitor() {
    let fixture = Fixture::new();
    for directory in 0..20 {
        fixture.write(&format!("src/d{directory}/keep.rs"));
        fixture.write(&format!("src/d{directory}/skip.txt"));
        fixture.write(&format!("target/d{directory}/ignored.rs"));
    }
    let mut query = fixture.query("**/*.rs");
    query.exclusions.push("**/target/**".into());
    let compiled = query.compile().unwrap();
    let mut expected = Vec::new();
    compiled
        .visit(&mut |path: &Path| {
            expected.push(path.to_owned());
            Ok(ControlFlow::Continue(()))
        })
        .unwrap();
    expected.sort();
    for threads in [1, 2, 4] {
        let count = Rc::new(Cell::new(0));
        let captured = Rc::clone(&count);
        let mut actual = Vec::new();
        let result = compiled
            .visit_parallel(threads, true, &mut |path: &Path| {
                captured.set(captured.get() + 1);
                actual.push(path.to_owned());
                Ok(ControlFlow::Continue(()))
            })
            .unwrap();
        actual.sort();
        assert_eq!(actual, expected);
        assert_eq!(count.get(), expected.len());
        assert_eq!(result.completion, Completion::Complete);
        assert_eq!(result.stats.matches as usize, expected.len());
    }
}

#[test]
fn stopping_with_a_full_result_buffer_releases_workers() {
    let fixture = Fixture::new();
    for index in 0..2000 {
        fixture.write(&format!("file{index:04}.rs"));
    }
    within_deadline(move || {
        let compiled = fixture.query("**/*.rs").compile().unwrap();
        let result = compiled
            .visit_parallel(4, true, &mut |_: &Path| {
                // Give producers time to fill the bounded result channel.
                std::thread::sleep(Duration::from_millis(40));
                Ok(ControlFlow::Break(()))
            })
            .unwrap();
        assert_eq!(result.completion, Completion::Stopped);
        assert_eq!(result.stats.matches, 1);
        assert!(result.stats.entries_seen < 2000);
    });
}

#[test]
fn visitor_error_with_a_full_result_buffer_is_returned_without_deadlock() {
    let fixture = Fixture::new();
    for index in 0..1000 {
        fixture.write(&format!("file{index:04}.rs"));
    }
    within_deadline(move || {
        let compiled = fixture.query("**/*.rs").compile().unwrap();
        let error = compiled
            .visit_parallel(2, false, &mut |_: &Path| {
                std::thread::sleep(Duration::from_millis(40));
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "visitor stopped reading",
                ))
            })
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
        assert!(error.to_string().contains("visitor stopped reading"));
    });
}

#[test]
fn stopping_or_failing_mid_batch_delivers_only_the_requested_callbacks() {
    for fail in [false, true] {
        let fixture = Fixture::new();
        for index in 0..200 {
            fixture.write(&format!("file{index:04}.rs"));
        }
        within_deadline(move || {
            let compiled = fixture.query("**/*.rs").compile().unwrap();
            let mut delivered = 0;
            let result = compiled.visit_parallel(2, true, &mut |_: &Path| {
                delivered += 1;
                if delivered == 7 {
                    if fail {
                        return Err(io::Error::new(io::ErrorKind::BrokenPipe, "mid-batch error"));
                    }
                    return Ok(ControlFlow::Break(()));
                }
                Ok(ControlFlow::Continue(()))
            });
            assert_eq!(delivered, 7);
            if fail {
                assert_eq!(result.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
            } else {
                let report = result.unwrap();
                assert_eq!(report.completion, Completion::Stopped);
                assert_eq!(report.stats.matches, 7);
            }
        });
    }
}

#[derive(Default)]
struct Capture {
    paths: Vec<PathBuf>,
    errors: Vec<PathBuf>,
}
impl Visitor for Capture {
    fn matched(&mut self, path: &Path) -> io::Result<ControlFlow<()>> {
        self.paths.push(path.to_owned());
        Ok(ControlFlow::Continue(()))
    }
    fn error(&mut self, path: &Path, _: &io::Error) -> io::Result<()> {
        self.errors.push(path.to_owned());
        Ok(())
    }
}

#[test]
fn best_effort_errors_stream_to_the_visitor_and_strict_errors_abort() {
    let fixture = Fixture::new();
    fixture.write("src/keep.rs");
    fs::create_dir(fixture.0.join(".gitignore")).unwrap();
    let mut query = fixture.query("**/*.rs");
    query.gitignore = true;
    let compiled = query.compile().unwrap();
    let mut capture = Capture::default();
    let report = compiled.visit_parallel(3, false, &mut capture).unwrap();
    assert_eq!(capture.paths, [PathBuf::from("src/keep.rs")]);
    assert_eq!(capture.errors, [fixture.0.join(".gitignore")]);
    assert_eq!(report.stats.errors, 1);
    assert_eq!(report.completion, Completion::Incomplete);
    let error = compiled
        .visit_parallel(3, true, &mut Capture::default())
        .unwrap_err();
    assert_ne!(error.kind(), io::ErrorKind::NotFound);
}

#[test]
fn parallel_streaming_preserves_literal_file_directory_and_missing_roots() {
    let fixture = Fixture::new();
    fixture.write("src/keep.rs");
    for (pattern, entry_type, expected) in [
        (
            "src/keep.rs",
            EntryType::File,
            vec![PathBuf::from("src/keep.rs")],
        ),
        ("src", EntryType::Dir, vec![PathBuf::from("src")]),
        ("missing", EntryType::File, vec![]),
    ] {
        let mut query = fixture.query(pattern);
        query.entry_type = entry_type;
        let mut capture = Capture::default();
        let report = query
            .compile()
            .unwrap()
            .visit_parallel(2, true, &mut capture)
            .unwrap();
        assert_eq!(capture.paths, expected);
        assert_eq!(report.completion, Completion::Complete);
    }
}

#[test]
fn parallel_streaming_rejects_invalid_worker_counts() {
    let fixture = Fixture::new();
    let compiled = fixture.query("**/*.rs").compile().unwrap();
    for threads in [0, 65] {
        assert_eq!(
            compiled
                .visit_parallel(threads, false, &mut Capture::default())
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
