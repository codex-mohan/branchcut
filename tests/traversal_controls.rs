use branchcut::{CancellationToken, Completion, EntryType, Query, Visitor};
use std::fs;
use std::io;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "branchcut-traversal-controls-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn write(&self, relative: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"fixture").unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Default)]
struct Capture {
    paths: Vec<PathBuf>,
}
impl Visitor for Capture {
    fn matched(&mut self, path: &Path) -> io::Result<ControlFlow<()>> {
        self.paths.push(path.to_owned());
        Ok(ControlFlow::Continue(()))
    }
}

fn query(fixture: &Fixture, pattern: &str) -> Query {
    Query {
        cwd: fixture.0.clone(),
        patterns: vec![pattern.into()],
        ..Query::default()
    }
}

#[test]
fn max_open_budget_spills_ancestors_without_losing_matches() {
    let fixture = Fixture::new();
    for depth in 0..12 {
        fixture.write(&format!("d{depth}/leaf-{depth}.rs"));
    }
    let mut query = query(&fixture, "**/*.rs");
    query.max_open = 1;
    let compiled = query.compile().unwrap();
    let mut sequential = Capture::default();
    let report = compiled.visit(&mut sequential).unwrap();
    let mut parallel = Capture::default();
    let parallel_report = compiled.visit_parallel(4, true, &mut parallel).unwrap();
    sequential.paths.sort();
    parallel.paths.sort();
    assert_eq!(sequential.paths, parallel.paths);
    assert_eq!(sequential.paths.len(), 12);
    assert_eq!(report.completion, Completion::Complete);
    assert_eq!(report.stats.entries_seen, 24);
    assert_eq!(parallel_report.completion, Completion::Complete);
}

#[test]
fn max_open_budget_handles_nested_descent_iteratively() {
    let fixture = Fixture::new();
    let mut path = String::new();
    for depth in 0..12 {
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(&format!("d{depth}"));
        fixture.write(&format!("{path}/leaf-{depth}.rs"));
    }
    let mut query = query(&fixture, "**/*.rs");
    query.max_open = 1;
    let compiled = query.compile().unwrap();
    let mut sequential = Capture::default();
    compiled.visit(&mut sequential).unwrap();
    let mut parallel = Capture::default();
    compiled.visit_parallel(4, true, &mut parallel).unwrap();
    sequential.paths.sort();
    parallel.paths.sort();
    assert_eq!(sequential.paths, parallel.paths);
    assert_eq!(sequential.paths.len(), 12);
}

#[test]
fn parallel_queue_overflow_uses_iterative_local_frames() {
    let fixture = Fixture::new();
    for index in 0..600 {
        fixture.write(&format!("d{index:04}/leaf.rs"));
    }
    let compiled = query(&fixture, "**/*.rs").compile().unwrap();
    let matches = compiled.collect_parallel(1, true).unwrap();
    assert_eq!(matches.paths.len(), 600);
    assert_eq!(matches.report.completion, Completion::Complete);
}

#[test]
fn pre_cancelled_queries_report_stopped_even_when_there_are_no_matches() {
    let fixture = Fixture::new();
    fixture.write("present.txt");
    let token = CancellationToken::default();
    token.cancel();
    let mut query = query(&fixture, "**/*.missing");
    query.cancellation = Some(token);
    let compiled = query.compile().unwrap();
    assert_eq!(
        compiled.visit(&mut Capture::default()).unwrap().completion,
        Completion::Stopped
    );
    assert_eq!(
        compiled
            .visit_parallel(2, true, &mut Capture::default())
            .unwrap()
            .completion,
        Completion::Stopped
    );
    assert_eq!(
        compiled
            .collect_parallel(2, true)
            .unwrap()
            .report
            .completion,
        Completion::Stopped
    );
}

#[test]
fn default_traversal_stops_before_scanning_a_wide_directory() {
    let fixture = Fixture::new();
    for index in 0..1_000 {
        fixture.write(&format!("file-{index:04}.rs"));
    }
    let compiled = query(&fixture, "*.rs").compile().unwrap();
    let report = compiled
        .visit(&mut |_path: &Path| Ok(ControlFlow::Break(())))
        .unwrap();
    assert_eq!(report.completion, Completion::Stopped);
    assert_eq!(report.stats.matches, 1);
    assert_eq!(report.stats.entries_seen, 1);
}

#[test]
fn contents_first_emits_directories_after_their_descendants() {
    let fixture = Fixture::new();
    fixture.write("a/nested/leaf.rs");
    let mut query = query(&fixture, "**/*");
    query.entry_type = EntryType::Dir;
    query.contents_first = true;
    let compiled = query.compile().unwrap();
    let mut capture = Capture::default();
    compiled.visit_parallel(3, true, &mut capture).unwrap();
    assert_eq!(
        capture.paths,
        [PathBuf::from("a/nested"), PathBuf::from("a")]
    );
}

#[cfg(unix)]
#[test]
fn followed_symlink_cycles_are_pruned_at_the_ancestor() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    fixture.write("tree/leaf.rs");
    symlink(&fixture.0, fixture.0.join("tree/loop")).unwrap();
    let mut query = query(&fixture, "**/leaf.rs");
    query.follow_symlinks = true;
    let compiled = query.compile().unwrap();
    let mut capture = Capture::default();
    let report = compiled.visit_parallel(4, true, &mut capture).unwrap();
    assert_eq!(capture.paths, [PathBuf::from("tree/leaf.rs")]);
    assert_eq!(report.completion, Completion::Complete);
}

#[cfg(unix)]
#[test]
fn same_file_system_keeps_matches_on_the_root_device() {
    let fixture = Fixture::new();
    fixture.write("a/keep.rs");
    let mut query = query(&fixture, "**/*.rs");
    query.same_file_system = true;
    let compiled = query.compile().unwrap();
    let mut capture = Capture::default();
    let report = compiled.visit(&mut capture).unwrap();
    assert_eq!(capture.paths, [PathBuf::from("a/keep.rs")]);
    assert_eq!(report.completion, Completion::Complete);
}

#[cfg(not(unix))]
#[test]
fn same_file_system_reports_unsupported_platforms() {
    let fixture = Fixture::new();
    let mut query = query(&fixture, "**/*");
    query.same_file_system = true;
    let error = query
        .compile()
        .unwrap()
        .visit(&mut Capture::default())
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    assert!(error.to_string().contains("one-file-system"));
}
