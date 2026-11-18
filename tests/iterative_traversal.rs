use branchcut::Completion;
use branchcut::Query;
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
            "branchcut-iterative-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create traversal fixture");
        Self(root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn sequential_frames_preserve_depth_filters_and_metadata_avoidance() {
    let fixture = Fixture::new();
    let mut absolute = fixture.0.clone();
    let mut relative = PathBuf::new();
    for _ in 0..32 {
        absolute.push("d");
        relative.push("d");
        fs::create_dir(&absolute).expect("create nested directory");
    }
    absolute.push("leaf.rs");
    relative.push("leaf.rs");
    fs::write(&absolute, b"").expect("write result");
    let query = Query {
        cwd: fixture.0.clone(),
        patterns: vec!["**/leaf.rs".into()],
        min_depth: 33,
        max_depth: Some(33),
        ..Query::default()
    }
    .compile()
    .expect("compile depth-bounded query");
    let mut matches = Vec::new();
    let report = query
        .visit(&mut |path: &Path| -> io::Result<ControlFlow<()>> {
            matches.push(path.to_owned());
            Ok(ControlFlow::Continue(()))
        })
        .expect("walk depth-bounded fixture");

    assert_eq!(matches, [relative]);
    assert_eq!(report.stats.dirs_opened, 33);
    assert_eq!(report.stats.metadata_calls, 1);
}

#[test]
fn streams_matches_through_a_deep_tree_without_recursive_calls() {
    // 600 one-character components stay below common path length limits,
    // while exercising a traversal depth well beyond the old 64-level test.
    let fixture = Fixture::new();
    let mut absolute = fixture.0.clone();
    let mut relative = PathBuf::new();
    for depth in 0..600 {
        absolute.push("d");
        relative.push("d");
        fs::create_dir(&absolute).expect("create deep directory");
        fs::write(absolute.join(format!("stop-{depth:03}.rs")), b"")
            .expect("write match at every level");
    }
    absolute.push("leaf.rs");
    relative.push("leaf.rs");
    fs::write(&absolute, b"").expect("write deepest result");

    let query = Query {
        cwd: fixture.0.clone(),
        patterns: vec!["**/leaf.rs".into()],
        min_depth: 601,
        max_depth: Some(601),
        ..Query::default()
    }
    .compile()
    .expect("compile deep-tree query");
    let mut matches = Vec::new();
    let report = query
        .visit(&mut |path: &Path| -> io::Result<ControlFlow<()>> {
            matches.push(path.to_owned());
            Ok(ControlFlow::Continue(()))
        })
        .expect("walk deep fixture iteratively");

    assert_eq!(matches, [relative]);
    assert_eq!(report.completion, Completion::Complete);
    assert_eq!(report.stats.dirs_opened, 601);
    assert_eq!(report.stats.entries_seen, 1_201);
    // Only root/ancestor validation uses metadata for this name-only query.
    assert_eq!(report.stats.metadata_calls, 1);
}

#[test]
fn visitor_break_discards_deep_stack_and_reports_stopped() {
    let fixture = Fixture::new();
    let mut absolute = fixture.0.clone();
    for _ in 0..400 {
        absolute.push("d");
        fs::create_dir(&absolute).expect("create nested directory");
        fs::write(absolute.join("match.rs"), b"").expect("write nested match");
    }
    let query = Query {
        cwd: fixture.0.clone(),
        patterns: vec!["**/*.rs".into()],
        ..Query::default()
    }
    .compile()
    .expect("compile early-stop query");
    let mut matches = 0;
    let report = query
        .visit(&mut |_path: &Path| -> io::Result<ControlFlow<()>> {
            matches += 1;
            Ok(ControlFlow::Break(()))
        })
        .expect("stop deep traversal from visitor");

    assert_eq!(matches, 1);
    assert_eq!(report.stats.matches, 1);
    assert_eq!(report.completion, Completion::Stopped);
    assert!(
        report.stats.entries_seen < 800,
        "the callback must stop before every sibling match is visited"
    );
    assert_eq!(report.stats.metadata_calls, 1);
}
