use branchcut::{CaseMode, EntryType, Query, SearchTarget};
use std::fs::{self, File, FileTimes};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

#[test]
fn slashless_directory_ignores_apply_at_every_depth_without_hiding_files() {
    let fixture = Fixture::new();
    fixture.write(".gitignore", b"target/\n/root-only/\n");
    fixture.write("target/generated.rs", b"");
    fixture.write("packages/core/target/generated.rs", b"");
    fixture.write("root-only/generated.rs", b"");
    fixture.write("packages/core/root-only/keep.rs", b"");
    fixture.write("packages/core/src/lib.rs", b"");
    fixture.write(
        "packages/other/target",
        b"regular file, not an ignored directory",
    );
    let expected = [
        "packages/core/root-only/keep.rs",
        "packages/core/src/lib.rs",
        "packages/other/target",
    ];
    for threads in ["1", "2"] {
        let output = fixture.run(&["--gitignore", "--sort", "--threads", threads]);
        assert_success(&output);
        assert_eq!(output_lines(&output), expected);
    }
    let narrowed = fixture.run(&["--gitignore", "--glob", "packages/core/target/*.rs"]);
    assert_success(&narrowed);
    assert!(output_lines(&narrowed).is_empty());
    let why = fixture.run(&["--gitignore", "--why", "packages/core/target/generated.rs"]);
    assert_success(&why);
    let why = String::from_utf8_lossy(&why.stdout);
    assert!(why.contains("EXCLUDED") && why.contains("target/"), "{why}");
}

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "branchcut-everyday-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create everyday-search fixture");
        Self(root)
    }

    fn write(&self, relative: impl AsRef<Path>, contents: &[u8]) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().expect("fixture file has a parent"))
            .expect("create fixture parent");
        fs::write(path, contents).expect("write fixture file");
    }

    fn set_modified(&self, relative: impl AsRef<Path>, modified: SystemTime) {
        let file = File::options()
            .write(true)
            .open(self.0.join(relative))
            .expect("open fixture file for timestamp update");
        file.set_times(FileTimes::new().set_modified(modified))
            .expect("set fixture modification time");
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_branchcut"))
            .arg("--cwd")
            .arg(&self.0)
            .args(args)
            .output()
            .expect("run branchcut CLI")
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

fn output_lines(output: &Output) -> Vec<String> {
    String::from_utf8(output.stdout.clone())
        .expect("fixture paths are UTF-8")
        .lines()
        .map(str::to_owned)
        .collect()
}

fn stat(output: &Output, label: &str) -> u64 {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .find_map(|line| {
            line.strip_prefix(label)
                .and_then(|value| value.trim().parse().ok())
        })
        .unwrap_or_else(|| {
            panic!(
                "missing {label:?} in stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        })
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "branchcut failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn sorted(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.sort_unstable();
    paths
}

#[test]
fn size_filters_compose_and_require_metadata_only_for_name_candidates() {
    let fixture = Fixture::new();
    fixture.write("four.dat", b"1234");
    fixture.write("seven.dat", b"1234567");
    fixture.write("eight.dat", b"12345678");
    fixture.write("ignored.bin", b"1234567");

    let output = fixture.run(&[
        "--glob", "*.dat", "--size", ">=4", "--size", "<8", "--stats",
    ]);
    assert_success(&output);
    let mut actual = output_lines(&output);
    actual.sort();
    assert_eq!(actual, ["four.dat", "seven.dat"]);
    assert_eq!(
        stat(&output, "metadata calls"),
        4,
        "one root lookup plus one per surviving name candidate"
    );
    assert_eq!(stat(&output, "candidate files"), 4);
}

#[test]
fn time_filters_support_relative_and_absolute_cutoffs() {
    let fixture = Fixture::new();
    fixture.write("old.log", b"old");
    fixture.write("recent.log", b"recent");
    let now = SystemTime::now();
    fixture.set_modified("old.log", now - Duration::from_secs(10 * 24 * 60 * 60));
    fixture.set_modified("recent.log", now - Duration::from_secs(60 * 60));

    let within = fixture.run(&["--glob", "*.log", "--modified-within", "2d"]);
    assert_success(&within);
    assert_eq!(output_lines(&within), ["recent.log"]);

    let before = fixture.run(&["--glob", "*.log", "--modified-before", "2d"]);
    assert_success(&before);
    assert_eq!(output_lines(&before), ["old.log"]);

    let absolute = fixture.run(&[
        "--glob",
        "*.log",
        "--modified-before",
        "2000-01-01T00:00:00Z",
    ]);
    assert_success(&absolute);
    assert!(output_lines(&absolute).is_empty());
}

#[test]
fn no_metadata_predicates_avoid_per_file_metadata_calls() {
    let fixture = Fixture::new();
    fixture.write("one.txt", b"");
    fixture.write("two.txt", b"");
    fixture.write("three.bin", b"");

    let plain = fixture.run(&["--glob", "**/*", "--stats"]);
    assert_success(&plain);
    assert_eq!(
        stat(&plain, "metadata calls"),
        1,
        "only the traversal root is inspected for metadata"
    );

    let filtered = fixture.run(&[
        "--glob",
        "*.txt",
        "--size",
        ">=0",
        "--modified-within",
        "1w",
        "--stats",
    ]);
    assert_success(&filtered);
    assert_eq!(stat(&filtered, "candidate files"), 3);
    assert_eq!(
        stat(&filtered, "metadata calls"),
        3,
        "metadata should be fetched once per surviving candidate, shared by all metadata filters"
    );
}

#[test]
fn case_modes_apply_to_globs_terms_and_extensions() {
    let fixture = Fixture::new();
    fixture.write("Readme-UPPER.TXT", b"");
    fixture.write("readme-lower.txt", b"");

    let sensitive = fixture.run(&["--glob", "*.txt"]);
    assert_success(&sensitive);
    assert_eq!(output_lines(&sensitive), ["readme-lower.txt"]);

    let insensitive = fixture.run(&["--glob", "*.txt", "-i", "-e", "TXT"]);
    assert_success(&insensitive);
    let mut paths = output_lines(&insensitive);
    paths.sort();
    assert_eq!(paths, ["Readme-UPPER.TXT", "readme-lower.txt"]);

    let smart_lower = fixture.run(&["--smart-case", "readme"]);
    assert_success(&smart_lower);
    let mut paths = output_lines(&smart_lower);
    paths.sort();
    assert_eq!(paths, ["Readme-UPPER.TXT", "readme-lower.txt"]);

    let smart_upper = fixture.run(&["--smart-case", "Readme"]);
    assert_success(&smart_upper);
    assert_eq!(output_lines(&smart_upper), ["Readme-UPPER.TXT"]);
}

#[test]
fn full_path_search_checks_parent_components_while_filename_mode_does_not() {
    let fixture = Fixture::new();
    fixture.write("needle/ordinary.txt", b"");

    let names = fixture.run(&["needle"]);
    assert_success(&names);
    assert!(output_lines(&names).is_empty());

    let full_path = fixture.run(&["--full-path", "needle"]);
    assert_success(&full_path);
    assert_eq!(output_lines(&full_path), ["needle/ordinary.txt"]);

    let filename = fixture.run(&["--full-path", "--filename", "needle"]);
    assert!(!filename.status.success());
}

#[test]
fn depth_is_cwd_relative_even_when_a_literal_prefix_narrows_the_walk() {
    let fixture = Fixture::new();
    fixture.write("root.rs", b"");
    fixture.write("src/direct.rs", b"");
    fixture.write("src/nested/deep.rs", b"");

    let root_only = fixture.run(&["--glob", "**/*.rs", "--max-depth", "0", "--stats"]);
    assert_success(&root_only);
    assert!(output_lines(&root_only).is_empty());
    assert_eq!(stat(&root_only, "directories opened"), 0);
    let immediate = fixture.run(&["--glob", "**/*.rs", "--max-depth", "1"]);
    assert_success(&immediate);
    assert_eq!(output_lines(&immediate), ["root.rs"]);
    assert!(
        stat(&root_only, "  depth") > 0,
        "directories beyond max depth should be pruned before opening"
    );

    let before_narrowed_root =
        fixture.run(&["--glob", "src/**/*.rs", "--max-depth", "1", "--stats"]);
    assert_success(&before_narrowed_root);
    assert!(output_lines(&before_narrowed_root).is_empty());
    assert_eq!(stat(&before_narrowed_root, "directories opened"), 0);

    let min_depth = fixture.run(&["--glob", "**/*.rs", "--min-depth", "2"]);
    assert_success(&min_depth);
    let mut paths = output_lines(&min_depth);
    paths.sort();
    assert_eq!(paths, ["src/direct.rs", "src/nested/deep.rs"]);
}

#[test]
fn sequential_and_parallel_query_results_match_with_everyday_filters() {
    let fixture = Fixture::new();
    fixture.write("src/Readme.RS", b"12345");
    fixture.write("src/child/readme.rs", b"123456");
    fixture.write("src/child/no.txt", b"1234567");
    fixture.write("other/readme.rs", b"12345678");

    let query = Query {
        patterns: vec!["src/**/*.rs".into()],
        case_mode: CaseMode::Insensitive,
        search_target: SearchTarget::Name,
        search_terms: vec!["readme".into()],
        extensions: vec!["RS".into()],
        min_depth: 3,
        max_depth: Some(3),
        sizes: vec![">=5".parse().unwrap(), "<7".parse().unwrap()],
        ..fixture.query(&[])
    };
    assert_eq!(query.entry_type, EntryType::File);
    let plan = query.compile().expect("compile everyday query");
    let mut sequential = Vec::new();
    plan.visit(&mut |path: &Path| {
        sequential.push(path.to_owned());
        Ok(std::ops::ControlFlow::Continue(()))
    })
    .expect("visit sequentially");
    let parallel = plan.collect_parallel(2, true).expect("collect in parallel");
    let parallel_paths = sorted(parallel.paths);
    assert_eq!(sorted(sequential), parallel_paths);
    assert_eq!(parallel_paths, [PathBuf::from("src/child/readme.rs")]);
}

#[test]
fn parser_rejects_conflicts_invalid_ranges_and_size_on_directories() {
    let fixture = Fixture::new();
    for args in [
        vec!["-i", "-s", "*.rs"],
        vec!["--smart-case", "-i", "*.rs"],
        vec!["--min-depth", "4", "--max-depth", "3", "*.rs"],
        vec!["--min-depth", "18446744073709551616", "*.rs"],
        vec!["--size", "-1", "*.rs"],
        vec!["--size", "18446744073709551615T", "*.rs"],
        vec!["--modified-within", "-2d", "*.rs"],
        vec!["--modified-within", "999999999999999999999d", "*.rs"],
        vec!["--modified-before", "2025-02-30", "*.rs"],
        vec!["--size", ">0", "--type", "dir", "*.rs"],
    ] {
        let output = fixture.run(&args);
        assert!(
            !output.status.success(),
            "expected invalid query to fail: {args:?}"
        );
    }
}

#[test]
fn explain_reports_search_case_depth_and_metadata_decisions() {
    let fixture = Fixture::new();
    let output = fixture.run(&[
        "--glob",
        "src/**/*.rs",
        "--smart-case",
        "--full-path",
        "--min-depth",
        "2",
        "--max-depth",
        "4",
        "--size",
        ">10K",
        "--explain",
    ]);
    assert_success(&output);
    let explanation = String::from_utf8_lossy(&output.stdout);
    assert!(explanation.contains("case: insensitive"), "{explanation}");
    assert!(
        explanation.contains("simple search target: cwd-relative path"),
        "{explanation}"
    );
    assert!(
        explanation.contains("depth from cwd: 2..4"),
        "{explanation}"
    );
    assert!(
        explanation.contains("evaluation: after type, name, extension, and glob checks"),
        "{explanation}"
    );
}

#[test]
fn why_explains_ignore_provenance_and_explicit_exclusions() {
    let fixture = Fixture::new();
    fixture.write(".gitignore", b"generated/*\n!generated/keep.rs\n");
    fixture.write("generated/drop.rs", b"");
    fixture.write("generated/keep.rs", b"");
    fixture.write("skip.rs", b"");

    let included = fixture.run(&[
        "--glob",
        "**/*.rs",
        "--gitignore",
        "--why",
        "generated/keep.rs",
    ]);
    assert_success(&included);
    let included = String::from_utf8_lossy(&included.stdout);
    assert!(included.contains("RESULT\n  MATCHED"), "{included}");
    assert!(
        included.contains("re-included by `!generated/keep.rs` at"),
        "{included}"
    );
    assert!(included.contains(".gitignore:2"), "{included}");

    let ignored = fixture.run(&[
        "--glob",
        "**/*.rs",
        "--gitignore",
        "--why",
        "generated/drop.rs",
    ]);
    assert_success(&ignored);
    let ignored = String::from_utf8_lossy(&ignored.stdout);
    assert!(ignored.contains("RESULT\n  EXCLUDED"), "{ignored}");
    assert!(
        ignored.contains("excluded by `generated/*` at"),
        "{ignored}"
    );
    assert!(ignored.contains(".gitignore:1"), "{ignored}");

    let excluded = fixture.run(&[
        "--glob",
        "**/*.rs",
        "--exclude",
        "**/skip.rs",
        "--why",
        "skip.rs",
    ]);
    assert_success(&excluded);
    let excluded = String::from_utf8_lossy(&excluded.stdout);
    assert!(excluded.contains("RESULT\n  EXCLUDED"), "{excluded}");
    assert!(excluded.contains("[FAIL] exclusions"), "{excluded}");
    assert!(excluded.contains("`**/skip.rs`"), "{excluded}");
}

#[test]
fn excluded_parents_block_reinclusion_until_the_directory_is_reincluded() {
    let fixture = Fixture::new();
    fixture.write(
        ".gitignore",
        b"blocked/\n!blocked/keep.rs\nreopened/\n!reopened/\nreopened/*\n!reopened/keep.rs\n",
    );
    fixture.write("blocked/keep.rs", b"");
    fixture.write("blocked/nested/keep.rs", b"");
    fixture.write("reopened/drop.rs", b"");
    fixture.write("reopened/keep.rs", b"");
    let expected = ["reopened/keep.rs"];

    for threads in ["1", "2"] {
        let output = fixture.run(&[
            "--gitignore",
            "--glob",
            "**/*",
            "--sort",
            "--threads",
            threads,
        ]);
        assert_success(&output);
        assert_eq!(output_lines(&output), expected);
    }

    let why = fixture.run(&["--gitignore", "--why", "blocked/keep.rs"]);
    assert_success(&why);
    let why = String::from_utf8_lossy(&why.stdout);
    assert!(why.contains("RESULT\n  EXCLUDED"), "{why}");
    assert!(why.contains("excluded by `blocked/`"), "{why}");

    // Compare the same fixture against Git's local implementation. Skipping
    // is only for environments without Git; CI installs it with the checkout.
    let init = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&fixture.0)
        .output();
    let Ok(init) = init else {
        eprintln!("skipping Git ignore oracle because git is unavailable");
        return;
    };
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let candidates = [
        "blocked/keep.rs",
        "blocked/nested/keep.rs",
        "reopened/drop.rs",
        "reopened/keep.rs",
    ];
    let mut child = Command::new("git")
        .args(["check-ignore", "--verbose", "--no-index", "--stdin"])
        .current_dir(&fixture.0)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("run local Git ignore oracle");
    {
        let stdin = child.stdin.as_mut().unwrap();
        for candidate in candidates {
            writeln!(stdin, "{candidate}").unwrap();
        }
    }
    let oracle = child.wait_with_output().expect("read Git ignore results");
    assert!(
        oracle.status.success(),
        "{}",
        String::from_utf8_lossy(&oracle.stderr)
    );
    let oracle_stdout = String::from_utf8_lossy(&oracle.stdout);
    let ignored: Vec<_> = oracle_stdout
        .lines()
        .filter_map(|line| {
            let (source, path) = line.split_once('\t')?;
            let rule = source.rsplit_once(':')?.1;
            (!rule.starts_with('!')).then_some(path)
        })
        .collect();
    assert_eq!(
        ignored,
        [
            "blocked/keep.rs",
            "blocked/nested/keep.rs",
            "reopened/drop.rs"
        ],
        "oracle stdout: {}",
        oracle_stdout
    );
}

#[test]
fn named_config_profiles_are_resolved_before_cli_overrides() {
    let fixture = Fixture::new();
    fixture.write("src/main.rs", b"");
    fixture.write("docs/GUIDE.MD", b"");
    fixture.write("notes.md", b"");
    let config = fixture.0.join("queries.conf");
    fs::write(
        &config,
        b"[default]\nglob src/**/*.rs\n\n[docs]\nglob docs/**/*.md\ncase insensitive\ntype file\n",
    )
    .unwrap();
    let config = config.to_str().unwrap();

    let resolved = fixture.run(&[
        "--config",
        config,
        "--profile",
        "docs",
        "--type",
        "file",
        "--max-depth",
        "2",
        "--explain",
    ]);
    assert_success(&resolved);
    let resolved = String::from_utf8_lossy(&resolved.stdout);
    assert!(resolved.contains("docs/**/*.md"), "{resolved}");
    assert!(resolved.contains("type: File"), "{resolved}");
    assert!(resolved.contains("depth from cwd: 0..2"), "{resolved}");

    let selected = fixture.run(&[
        "--config",
        config,
        "--profile",
        "docs",
        "--glob",
        "src/**/*.rs",
    ]);
    assert_success(&selected);
    let mut paths = output_lines(&selected);
    paths.sort();
    assert_eq!(paths, ["docs/GUIDE.MD", "src/main.rs"]);
}

#[test]
fn invalid_config_selection_and_keys_fail_without_traversal() {
    let fixture = Fixture::new();
    fixture.write("file.rs", b"");

    let no_config = fixture.run(&["--profile", "docs"]);
    assert!(!no_config.status.success());
    assert!(String::from_utf8_lossy(&no_config.stderr).contains("requires an explicit --config"));

    let config = fixture.0.join("queries.conf");
    fs::write(&config, b"[docs]\nunknown-option value\n").unwrap();
    let config = config.to_str().unwrap();
    let missing_profile = fixture.run(&["--config", config, "--profile", "other"]);
    assert!(!missing_profile.status.success());
    assert!(
        String::from_utf8_lossy(&missing_profile.stderr)
            .contains("profile \"other\" was not found")
    );

    let unknown = fixture.run(&["--config", config, "--profile", "docs"]);
    assert!(!unknown.status.success());
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("unknown config option"));
}

#[test]
fn config_directory_and_relative_cwd_anchor_saved_queries() {
    let fixture = Fixture::new();
    fixture.write("root.rs", b"");
    fixture.write("src/nested.rs", b"");
    let config = fixture.0.join("queries.conf");
    fs::write(
        &config,
        b"[root]\nglob *.rs\n\n[nested]\ncwd src\nglob *.rs\n",
    )
    .unwrap();

    let root = Command::new(env!("CARGO_BIN_EXE_branchcut"))
        .args([
            "--config",
            config.to_str().unwrap(),
            "--profile",
            "root",
            "--sort",
        ])
        .output()
        .expect("run root profile without an explicit --cwd");
    assert_success(&root);
    assert_eq!(output_lines(&root), ["root.rs"]);

    let nested = Command::new(env!("CARGO_BIN_EXE_branchcut"))
        .args([
            "--config",
            config.to_str().unwrap(),
            "--profile",
            "nested",
            "--sort",
        ])
        .output()
        .expect("run nested profile without an explicit --cwd");
    assert_success(&nested);
    assert_eq!(output_lines(&nested), ["nested.rs"]);
}
