use branchcut::{CaseMode, EntryType, Query, SearchTarget, Stats, Visitor};
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
            "branchcut-universal-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for (name, contents) in [
            ("keep.rs", "keep"),
            ("UPPER.RS", "upper"),
            ("empty.rs", ""),
            ("src/nested/deep.rs", "deep"),
            ("src/hidden.log", "log"),
            ("skip/drop.rs", "drop"),
            ("ignored/drop.rs", "ignored"),
            (".hidden/keep.rs", "hidden"),
            ("notes.tmp", "temp"),
            (".gitignore", "ignored/\n*.tmp\n"),
            ("src/.gitignore", "*.log\n"),
        ] {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
        #[cfg(unix)]
        {
            use std::ffi::OsString;
            use std::os::unix::{ffi::OsStringExt, fs::symlink};
            fs::write(
                root.join(OsString::from_vec(b"native-\xff.rs".to_vec())),
                b"native",
            )
            .unwrap();
            symlink("src", root.join("alias")).unwrap();
        }
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn select(query: &Query, workers: usize) -> (Vec<PathBuf>, Stats) {
    let plan = query.compile().unwrap();
    let mut paths = Vec::new();
    let mut visitor = |path: &Path| -> io::Result<ControlFlow<()>> {
        paths.push(path.to_owned());
        Ok(ControlFlow::Continue(()))
    };
    let report = if workers == 1 {
        plan.visit(&mut visitor)
    } else {
        plan.visit_parallel(workers, true, &mut visitor)
    }
    .unwrap();
    assert_eq!(report.stats.errors, 0);
    paths.sort();
    (paths, report.stats)
}

#[test]
fn universal_selection_preserves_filters_native_names_and_filesystem_work() {
    let fixture = Fixture::new();
    let base = Query {
        cwd: fixture.0.clone(),
        ..Query::default()
    };
    let mut cases = vec![base.clone()];
    for hidden in [false, true] {
        for entry_type in [EntryType::File, EntryType::Dir, EntryType::Symlink] {
            cases.push(Query {
                hidden,
                entry_type,
                ..base.clone()
            });
        }
    }
    cases.extend([
        Query {
            exclusions: vec!["**/skip/**".into()],
            gitignore: true,
            ..base.clone()
        },
        Query {
            extensions: vec!["rs".into()],
            case_mode: CaseMode::Insensitive,
            ..base.clone()
        },
        Query {
            min_depth: 2,
            max_depth: Some(3),
            ..base.clone()
        },
        Query {
            max_depth: Some(0),
            ..base.clone()
        },
        Query {
            sizes: vec![">=1B".parse().unwrap()],
            ..base.clone()
        },
        Query {
            modified_within: Some(std::time::Duration::from_secs(36500 * 86400)),
            ..base.clone()
        },
        Query {
            search_terms: vec!["nested".into()],
            search_target: SearchTarget::Path,
            ..base.clone()
        },
        Query {
            search_terms: vec!["keep".into()],
            ..base.clone()
        },
        Query {
            exclusions: vec!["!(keep.rs)".into()],
            ..base.clone()
        },
    ]);
    for mut query in cases {
        for workers in [1, 4] {
            // ?* independently expresses any non-empty native component while
            // retaining the ordinary compiled-state traversal path.
            query.patterns = vec!["**/?*".into()];
            let (expected, reference) = select(&query, workers);
            query.patterns = vec!["**/*".into(), "src/nested/*.rs".into()];
            let (actual, optimized) = select(&query, workers);
            assert_eq!(actual, expected, "query={query:?}, workers={workers}");
            assert_eq!(optimized.metadata_calls, reference.metadata_calls);
            assert_eq!(optimized.dirs_opened, reference.dirs_opened);
            assert_eq!(optimized.entries_seen, reference.entries_seen);
            assert_eq!(optimized.matches, reference.matches);
        }
    }
}

#[test]
fn universal_selection_still_validates_patterns_and_does_not_select_cwd_itself() {
    let fixture = Fixture::new();
    assert!(
        Query {
            cwd: fixture.0.clone(),
            patterns: vec!["**/*".into(), "[broken".into()],
            ..Query::default()
        }
        .compile()
        .is_err()
    );
    let query = Query {
        cwd: fixture.0.join("keep.rs"),
        patterns: vec!["**/*".into()],
        ..Query::default()
    };
    for workers in [1, 4] {
        assert!(select(&query, workers).0.is_empty());
    }
}

struct CountOnlyVisitor {
    count: u64,
    limit: Option<u64>,
}

impl Visitor for CountOnlyVisitor {
    fn matched(&mut self, _path: &Path) -> io::Result<ControlFlow<()>> {
        panic!("pathless count visitor received a materialized path");
    }

    fn requires_path(&self) -> bool {
        false
    }

    fn matched_without_path(&mut self) -> io::Result<ControlFlow<()>> {
        self.count += 1;
        Ok(if self.limit.is_some_and(|limit| self.count >= limit) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        })
    }
}

#[test]
fn pathless_visitors_preserve_filters_limits_and_count_semantics() {
    let fixture = Fixture::new();
    for (query, limit) in [
        (
            Query {
                cwd: fixture.0.clone(),
                patterns: vec!["**/*".into()],
                ..Query::default()
            },
            None,
        ),
        (
            Query {
                cwd: fixture.0.clone(),
                patterns: vec!["**/*.rs".into()],
                exclusions: vec!["**/skip/**".into()],
                gitignore: true,
                extensions: vec!["rs".into()],
                ..Query::default()
            },
            None,
        ),
        (
            Query {
                cwd: fixture.0.clone(),
                patterns: vec!["**/*".into()],
                ..Query::default()
            },
            Some(3),
        ),
    ] {
        let expected = select(&query, 1).0.len();
        let expected = limit.map_or(expected as u64, |limit| (expected as u64).min(limit));
        let plan = query.compile().unwrap();
        for workers in [1, 4] {
            let mut visitor = CountOnlyVisitor { count: 0, limit };
            let report = if workers == 1 {
                plan.visit(&mut visitor)
            } else {
                plan.visit_parallel(workers, true, &mut visitor)
            }
            .unwrap();
            assert_eq!(visitor.count, expected, "workers={workers}");
            assert_eq!(report.stats.matches, expected, "workers={workers}");
            assert_eq!(
                report.completion == branchcut::Completion::Stopped,
                limit.is_some(),
                "workers={workers}"
            );
        }
    }
}
