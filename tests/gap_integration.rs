use branchcut::{
    CaseMode, Completion, EntryType, MatchOptions, PathMatcher, Query, escape_path_pattern,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    fs, io,
    ops::ControlFlow,
    path::{Path, PathBuf},
    process::Command,
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "bc-gap-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn write(&self, path: &str, bytes: &[u8]) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn query(&self) -> Query {
        Query {
            cwd: self.0.clone(),
            ..Query::default()
        }
    }
    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_branchcut"))
            .arg("--cwd")
            .arg(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn collect(query: Query) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    query
        .compile()
        .unwrap()
        .visit(&mut |path: &Path| {
            paths.push(path.to_owned());
            Ok(ControlFlow::Continue(()))
        })
        .unwrap();
    paths.sort();
    paths
}

#[test]
fn public_matcher_retains_input_ids_without_accessing_filesystem() {
    let matcher = PathMatcher::new(&["src/**/*.{rs,toml}", "**/lib.rs", "tests/**/*.rs"]).unwrap();
    assert_eq!(matcher.matching_patterns("src/deep/lib.rs"), [0, 1]);
    assert_eq!(matcher.matching_patterns("src/Cargo.toml"), [0]);
    assert!(!matcher.is_match("src/no.txt"));
}
#[test]
fn public_escape_helper_roundtrips_literal_metacharacters() {
    let name = Path::new("dir[1]/report{final}(v2).rs");
    let escaped = escape_path_pattern(name).unwrap();
    let matcher = PathMatcher::with_options(
        &[escaped],
        MatchOptions {
            escaped_patterns: true,
            ..MatchOptions::default()
        },
    )
    .unwrap();
    assert!(matcher.is_match(name));
    assert!(!matcher.is_match("dir1/reportfinalv2.rs"));
}
#[test]
fn public_matcher_supports_unicode_scalar_case_and_wildcards() {
    let matcher = PathMatcher::with_options(
        &["Ä/?.rs"],
        MatchOptions {
            unicode: true,
            case_mode: CaseMode::Insensitive,
            ..MatchOptions::default()
        },
    )
    .unwrap();
    assert!(matcher.is_match("ä/é.rs"));
    assert!(!matcher.is_match("ä/ab.rs"));
}

#[test]
fn unicode_classes_and_metadata_records_work_through_the_public_api() {
    let matcher = PathMatcher::with_options(
        &["[α-ω].rs", "[é].rs"],
        MatchOptions {
            unicode: true,
            ..MatchOptions::default()
        },
    )
    .unwrap();
    assert_eq!(matcher.matching_patterns("λ.rs"), [0]);
    assert_eq!(matcher.matching_patterns("é.rs"), [1]);
    let fixture = Fixture::new();
    fixture.write("selected.rs", b"123");
    fixture.write("other.txt", b"x");
    let mut query = fixture.query();
    query.patterns = vec!["*.rs".into()];
    let compiled = query.compile().unwrap();
    let mut seen = Vec::new();
    let report = compiled
        .visit_entries_parallel(4, true, &mut |entry: &branchcut::EntryRecord| {
            seen.push((entry.path.clone(), entry.metadata.len()));
            Ok(ControlFlow::Continue(()))
        })
        .unwrap();
    assert_eq!(seen, [(PathBuf::from("selected.rs"), 3)]);
    assert_eq!(report.stats.matches, 1);
    assert_eq!(report.stats.metadata_calls, 2);
}

#[test]
fn unicode_simple_search_uses_the_same_single_scalar_case_rule_as_globs() {
    let fixture = Fixture::new();
    fixture.write("İ.txt", b"");
    let mut query = fixture.query();
    query.unicode = true;
    query.case_mode = CaseMode::Insensitive;
    query.search_terms = vec!["i".into()];
    assert!(collect(query).is_empty());
}

#[cfg(unix)]
#[test]
fn why_and_traversal_agree_for_followed_and_unfollowed_directory_symlinks() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    fixture.write("tree/keep.rs", b"");
    symlink(fixture.0.join("tree"), fixture.0.join("alias")).unwrap();
    symlink(&fixture.0, fixture.0.join("tree/loop")).unwrap();
    let mut query = fixture.query();
    query.patterns = vec!["alias/keep.rs".into()];
    assert!(
        !query
            .compile()
            .unwrap()
            .explain_path("alias/keep.rs")
            .unwrap()
            .selected
    );
    assert!(collect(query.clone()).is_empty());
    query.follow_symlinks = true;
    assert!(
        query
            .compile()
            .unwrap()
            .explain_path("alias/keep.rs")
            .unwrap()
            .selected
    );
    assert_eq!(collect(query.clone()), [PathBuf::from("alias/keep.rs")]);
    query.patterns = vec!["tree/loop/tree/keep.rs".into()];
    assert!(
        !query
            .compile()
            .unwrap()
            .explain_path("tree/loop/tree/keep.rs")
            .unwrap()
            .selected
    );
    assert!(collect(query).is_empty());
}

#[cfg(unix)]
#[test]
fn unix_special_types_ownership_and_execute_permissions_are_composable() {
    use std::os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::UnixListener,
    };
    let fixture = Fixture::new();
    fixture.write("run", b"script");
    fixture.write("data", b"data");
    fs::set_permissions(fixture.0.join("run"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(fixture.0.join("data"), fs::Permissions::from_mode(0o644)).unwrap();
    let socket = UnixListener::bind(fixture.0.join("socket")).unwrap();
    let mut query = fixture.query();
    query.entry_types = vec![EntryType::Socket];
    assert_eq!(collect(query.clone()), [PathBuf::from("socket")]);
    query.entry_types = vec![EntryType::File];
    query.require_executable = true;
    let metadata = fs::metadata(fixture.0.join("run")).unwrap();
    query.owner = Some(metadata.uid());
    query.group = Some(metadata.gid());
    assert_eq!(collect(query), [PathBuf::from("run")]);
    drop(socket);
}
#[test]
fn entry_type_union_and_empty_filter_share_one_selection() {
    let fixture = Fixture::new();
    fixture.write("empty.txt", b"");
    fixture.write("full.txt", b"x");
    fs::create_dir(fixture.0.join("empty-dir")).unwrap();
    let mut query = fixture.query();
    query.entry_types = vec![EntryType::File, EntryType::Dir];
    query.require_empty = true;
    assert_eq!(
        collect(query),
        [PathBuf::from("empty-dir"), PathBuf::from("empty.txt")]
    );
}
#[test]
fn required_terms_intersect_optional_or_terms() {
    let fixture = Fixture::new();
    for name in [
        "config-prod.toml",
        "config-test.toml",
        "settings-prod.toml",
        "other-prod.toml",
    ] {
        fixture.write(name, b"");
    }
    let mut query = fixture.query();
    query.search_terms = vec!["config".into(), "settings".into()];
    query.and_terms = vec!["prod".into()];
    assert_eq!(
        collect(query),
        [
            PathBuf::from("config-prod.toml"),
            PathBuf::from("settings-prod.toml")
        ]
    );
}

#[cfg(target_os = "linux")]
#[test]
fn one_open_directory_budget_survives_a_four_descriptor_process_limit() {
    let fixture = Fixture::new();
    fixture.write("a/b/leaf.rs", b"");
    let output = Command::new("sh")
        .args(["-c", "ulimit -n 4; exec \"$1\" --cwd \"$2\" --glob '**/*.rs' --threads 1 --max-open 1 --strict", "branchcut-fd-limit"])
        .arg(env!("CARGO_BIN_EXE_branchcut"))
        .arg(&fixture.0)
        .output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "a/b/leaf.rs"
    );
}
#[test]
fn multiple_roots_deduplicate_overlaps_and_propagate_cancellation() {
    let fixture = Fixture::new();
    fixture.write("a/one.rs", b"");
    fixture.write("a/nested/two.rs", b"");
    fixture.write("b/three.rs", b"");
    let roots = fixture
        .query()
        .compile_roots(["a", "a/nested", "b"])
        .unwrap();
    for workers in [1, 4] {
        let mut paths = Vec::new();
        let mut visitor = |path: &Path| {
            paths.push(path.to_owned());
            Ok(ControlFlow::Continue(()))
        };
        let report = if workers == 1 {
            roots.visit(&mut visitor)
        } else {
            roots.visit_parallel(workers, true, &mut visitor)
        }
        .unwrap();
        assert_eq!(paths.len(), 3);
        assert_eq!(report.stats.matches, 3);
        assert!(paths.iter().all(|path| path.is_absolute()));
    }
    let mut count = 0;
    let report = roots
        .visit_parallel(
            4,
            true,
            &mut |_path: &Path| -> io::Result<ControlFlow<()>> {
                count += 1;
                Ok(ControlFlow::Break(()))
            },
        )
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(report.completion, Completion::Stopped);
}
#[test]
fn cli_multiple_roots_sort_and_limit_the_union_once() {
    let fixture = Fixture::new();
    fixture.write("a/z.rs", b"");
    fixture.write("b/a.rs", b"");
    fixture.write("b/deep/b.rs", b"");
    let output = fixture.run(&[
        "--search-path",
        "b",
        "--search-path",
        "a",
        "--search-path",
        "b/deep",
        "--sort",
        "--limit",
        "2",
        "--threads",
        "4",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        ["a/z.rs", "b/a.rs"]
    );
}
#[test]
fn and_terms_and_unicode_mode_are_used_by_candidate_explanations() {
    let fixture = Fixture::new();
    fixture.write("Ä-prod.rs", b"");
    fixture.write("Ä-test.rs", b"");
    let mut query = fixture.query();
    query.search_terms = vec!["ä".into()];
    query.and_terms = vec!["prod".into()];
    query.case_mode = CaseMode::Insensitive;
    query.unicode = true;
    let compiled = query.compile().unwrap();
    assert!(compiled.explain_path("Ä-prod.rs").unwrap().selected);
    assert!(!compiled.explain_path("Ä-test.rs").unwrap().selected);
    assert_eq!(collect(query), [PathBuf::from("Ä-prod.rs")]);
}
