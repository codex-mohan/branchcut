use branchcut::{CheckOutcome, Completion, EntryType, Query, Visitor};
use std::fs;
use std::io;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "branchcut-library-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create library test fixture root");
        Self(root)
    }

    fn write(&self, relative: impl AsRef<Path>) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().expect("fixture path has a parent"))
            .expect("create fixture parent");
        fs::write(path, b"").expect("write fixture file");
    }

    fn query(&self, patterns: &[&str]) -> Query {
        Query {
            cwd: self.0.clone(),
            patterns: patterns
                .iter()
                .map(|pattern| (*pattern).to_owned())
                .collect(),
            ..Query::default()
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stream(query: &branchcut::CompiledQuery) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    query
        .visit(&mut |path: &Path| {
            paths.push(path.to_owned());
            Ok(ControlFlow::Continue(()))
        })
        .expect("visit fixture tree");
    paths.sort_unstable();
    paths
}

#[test]
fn one_compiled_query_observes_filesystem_changes_between_runs() {
    let fixture = Fixture::new();
    fixture.write("src/first.rs");
    let compiled = fixture.query(&["src/**/*.rs"]).compile().unwrap();

    assert_eq!(stream(&compiled), [PathBuf::from("src/first.rs")]);

    fixture.write("src/second.rs");

    assert_eq!(
        stream(&compiled),
        [
            PathBuf::from("src/first.rs"),
            PathBuf::from("src/second.rs")
        ]
    );
}

#[test]
fn compiled_query_explains_a_candidate_without_traversing() {
    let fixture = Fixture::new();
    fixture.write("src/first.rs");
    fixture.write("src/first.txt");
    let compiled = fixture.query(&["src/**/*.rs"]).compile().unwrap();

    let matching = compiled.explain_path("src/first.rs").unwrap();
    assert!(matching.selected);
    assert_eq!(matching.path, PathBuf::from("src/first.rs"));
    assert!(matching.checks.iter().any(|check| {
        check.label == "positive patterns" && check.outcome == CheckOutcome::Passed
    }));

    let nonmatching = compiled.explain_path("src/first.txt").unwrap();
    assert!(!nonmatching.selected);
    assert!(nonmatching.checks.iter().any(|check| {
        check.label == "positive patterns" && check.outcome == CheckOutcome::Failed
    }));
}

#[test]
fn visitor_can_stop_early_without_visiting_every_entry() {
    let fixture = Fixture::new();
    for index in 0..128 {
        fixture.write(format!("src/file-{index:03}.rs"));
    }
    let compiled = fixture.query(&["src/**/*.rs"]).compile().unwrap();
    let mut matched = 0;

    let report = compiled
        .visit(&mut |_path: &Path| {
            matched += 1;
            Ok(ControlFlow::Break(()))
        })
        .unwrap();

    assert_eq!(matched, 1);
    assert_eq!(report.stats.matches, 1);
    assert_eq!(report.completion, Completion::Stopped);
    assert!(report.stats.entries_seen < 128);
}

#[test]
fn visitor_errors_are_propagated_to_the_caller() {
    let fixture = Fixture::new();
    fixture.write("src/file.rs");
    let compiled = fixture.query(&["src/**/*.rs"]).compile().unwrap();

    let error = compiled
        .visit(&mut |_path: &Path| {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "consumer closed"))
        })
        .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(error.to_string(), "consumer closed");
}

#[test]
fn native_query_combines_search_terms_and_extensions() {
    let fixture = Fixture::new();
    fixture.write("src/lib.rs");
    fixture.write("src/guide.txt");
    fixture.write("src/unrelated.rs");
    let query = Query {
        cwd: fixture.0.clone(),
        patterns: vec!["src/**/*".to_owned()],
        search_terms: vec!["missing".to_owned(), "lib".to_owned()],
        extensions: vec![".rs".to_owned()],
        entry_type: EntryType::File,
        ..Query::default()
    };

    assert_eq!(
        stream(&query.compile().unwrap()),
        [PathBuf::from("src/lib.rs")]
    );
}

#[test]
fn invalid_patterns_are_reported_during_compilation() {
    let fixture = Fixture::new();
    let query = fixture.query(&["src/[unterminated"]);

    let error = match query.compile() {
        Ok(_) => panic!("unterminated character class unexpectedly compiled"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("unterminated character class"));
}

#[test]
fn parallel_collection_and_streaming_select_the_same_paths() {
    let fixture = Fixture::new();
    for index in 0..24 {
        fixture.write(format!("src/package-{}/file-{index:02}.rs", index % 4));
    }
    fixture.write("src/package-0/generated/skip.rs");
    let query = Query {
        exclusions: vec!["**/generated/**".to_owned()],
        ..fixture.query(&["src/**/*.rs"])
    }
    .compile()
    .unwrap();
    let sequential = stream(&query);
    let parallel = query.collect_parallel(2, false).unwrap();
    let mut parallel_paths = parallel.paths;
    parallel_paths.sort_unstable();

    assert_eq!(parallel_paths, sequential);
    assert_eq!(parallel.report.stats.matches as usize, sequential.len());
    assert_eq!(parallel.report.completion, Completion::Complete);
}

#[test]
fn parallel_collection_rejects_out_of_range_worker_counts() {
    let fixture = Fixture::new();
    fixture.write("src/file.rs");
    let query = fixture.query(&["src/**/*.rs"]).compile().unwrap();

    for threads in [0, 65] {
        let error = query.collect_parallel(threads, false).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
}

#[derive(Default)]
struct RecordingVisitor {
    paths: Vec<PathBuf>,
    errors: Vec<(PathBuf, io::ErrorKind)>,
}

impl Visitor for RecordingVisitor {
    fn matched(&mut self, path: &Path) -> io::Result<ControlFlow<()>> {
        self.paths.push(path.to_owned());
        Ok(ControlFlow::Continue(()))
    }

    fn error(&mut self, path: &Path, error: &io::Error) -> io::Result<()> {
        self.errors.push((path.to_owned(), error.kind()));
        Ok(())
    }
}

#[test]
fn narrowed_query_reports_invalid_ancestor_gitignore_and_incomplete_status() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.0.join(".gitignore")).expect("make unreadable ignore path");
    fixture.write("src/file.rs");
    let query = Query {
        gitignore: true,
        ..fixture.query(&["src/**/*.rs"])
    }
    .compile()
    .unwrap();
    let mut visitor = RecordingVisitor::default();

    let report = query.visit(&mut visitor).unwrap();

    assert_eq!(report.completion, Completion::Incomplete);
    assert_eq!(report.stats.errors, 1);
    assert_eq!(visitor.errors.len(), 1);
    assert_eq!(visitor.errors[0].0, fixture.0.join(".gitignore"));
    assert_ne!(visitor.errors[0].1, io::ErrorKind::NotFound);
    assert_eq!(visitor.paths, [PathBuf::from("src/file.rs")]);

    let parallel = query.collect_parallel(2, false).unwrap();
    assert_eq!(parallel.report.completion, Completion::Incomplete);
    assert_eq!(parallel.report.stats.errors, 1);
    assert_eq!(parallel.errors.len(), 1);
    assert_eq!(parallel.errors[0].path, fixture.0.join(".gitignore"));
}

#[test]
fn parallel_best_effort_keeps_matches_after_an_ignore_file_error() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.0.join(".gitignore")).unwrap();
    fixture.write("src/file.rs");
    let query = Query {
        gitignore: true,
        ..fixture.query(&["**/*.rs"])
    }
    .compile()
    .unwrap();
    let mut visitor = RecordingVisitor::default();
    query.visit(&mut visitor).unwrap();
    let parallel = query.collect_parallel(2, false).unwrap();
    assert_eq!(parallel.paths, visitor.paths);
    assert_eq!(parallel.errors.len(), 1);
    assert_eq!(parallel.errors[0].path, fixture.0.join(".gitignore"));
    assert_eq!(parallel.report.completion, Completion::Incomplete);
}

#[test]
fn parallel_strict_mode_rejects_ancestor_ignore_errors() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.0.join(".gitignore")).unwrap();
    fixture.write("src/file.rs");
    let query = Query {
        gitignore: true,
        ..fixture.query(&["src/**/*.rs"])
    }
    .compile()
    .unwrap();
    assert!(query.collect_parallel(2, true).is_err());
}

#[test]
fn parallel_collection_handles_literal_files_and_missing_roots() {
    let fixture = Fixture::new();
    fixture.write("src/file.rs");
    for pattern in ["src/file.rs", "missing.rs"] {
        let query = fixture.query(&[pattern]).compile().unwrap();
        let parallel = query.collect_parallel(2, false).unwrap();
        assert_eq!(parallel.paths, stream(&query));
        assert_eq!(parallel.report.completion, Completion::Complete);
    }
}
