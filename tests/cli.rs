use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

const EXEC_HELPER_ENV: &str = "BRANCHCUT_TEST_EXEC_HELPER";
const EXEC_CAPTURE_ENV: &str = "BRANCHCUT_TEST_EXEC_CAPTURE";
const EXEC_FAILURE_ENV: &str = "BRANCHCUT_TEST_EXEC_FAILURE";

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("branchcut cli-{}-{sequence}", std::process::id()));
        fs::create_dir_all(&root).expect("create CLI test fixture root");
        Self(root)
    }

    fn write(&self, relative: impl AsRef<Path>, contents: &[u8]) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().expect("fixture file has a parent"))
            .expect("create fixture parent");
        fs::write(path, contents).expect("write fixture file");
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_branchcut"))
            .arg("--cwd")
            .arg(&self.0)
            .args(args)
            .output()
            .expect("run branchcut CLI")
    }

    fn run_with_env(&self, args: &[&str], key: &str, value: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_branchcut"))
            .arg("--cwd")
            .arg(&self.0)
            .args(args)
            .env(key, value)
            .env(EXEC_HELPER_ENV, "capture")
            .output()
            .expect("run branchcut CLI with a helper environment")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stdout_lines(output: &Output) -> Vec<&str> {
    std::str::from_utf8(&output.stdout)
        .expect("fixture results use UTF-8 paths")
        .lines()
        .collect()
}

fn stat_value(output: &Output, label: &str) -> u64 {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .find_map(|line| {
            line.strip_prefix(label)
                .and_then(|value| value.trim().parse().ok())
        })
        .unwrap_or_else(|| {
            panic!(
                "missing {label:?} in stats:\n{}",
                String::from_utf8_lossy(&output.stderr)
            )
        })
}

fn exec_helper_command(test_name: &str) -> String {
    let executable = std::env::current_exe().expect("locate integration test executable");
    format!(
        "'{}' --exact {test_name} --nocapture --skip {{}}",
        executable.display()
    )
}

#[test]
fn child_exec_capture_helper() {
    if std::env::var_os(EXEC_HELPER_ENV).is_none() {
        return;
    }
    let capture = std::env::var_os(EXEC_CAPTURE_ENV).expect("capture path is configured");
    let args: Vec<_> = std::env::args_os().collect();
    let skip_index = args
        .iter()
        .position(|arg| arg == "--skip")
        .expect("Branchcut passes the selected path after --skip");
    let selected_path = args.get(skip_index + 1).expect("selected path argument");
    let cwd = std::env::current_dir().expect("read child working directory");
    let mut capture = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(capture)
        .expect("open capture log");
    writeln!(capture, "{:?}\t{:?}", cwd.as_os_str(), selected_path)
        .expect("write captured arguments");
}

#[test]
fn child_exec_failure_helper() {
    if std::env::var_os(EXEC_FAILURE_ENV).is_some() {
        panic!("requested child command failure");
    }
}

#[test]
fn globstar_does_not_leak_into_sibling_patterns() {
    let fixture = Fixture::new();
    fixture.write("docs/cli.md", b"");
    fixture.write("docs/reference/cli.md", b"");

    let output = fixture.run(&[
        "--glob",
        "docs/**/__branchcut_never_matches__.md",
        "--glob",
        "docs/cli.md",
        "--sort",
    ]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(stdout_lines(&output), ["docs/cli.md"]);
}

#[test]
fn sorted_limit_returns_the_first_globally_sorted_matches() {
    let fixture = Fixture::new();
    // Create entries in reverse lexical order so enumeration order cannot
    // accidentally satisfy the expected globally sorted result.
    for name in ["z-last.rs", "m-middle.rs", "a-first.rs"] {
        fixture.write(name, b"");
    }

    let all = fixture.run(&["--glob", "**/*.rs", "--sort", "--stats"]);
    let limited = fixture.run(&["--glob", "**/*.rs", "--sort", "--limit", "2", "--stats"]);

    assert!(
        all.status.success(),
        "{}",
        String::from_utf8_lossy(&all.stderr)
    );
    assert!(
        limited.status.success(),
        "{}",
        String::from_utf8_lossy(&limited.stderr)
    );
    assert_eq!(
        stdout_lines(&all),
        ["a-first.rs", "m-middle.rs", "z-last.rs"]
    );
    assert_eq!(stdout_lines(&limited), ["a-first.rs", "m-middle.rs"]);
    assert_eq!(
        stat_value(&all, "entries inspected"),
        stat_value(&limited, "entries inspected"),
        "a sorted limit must inspect every candidate before selecting the first N"
    );
}

#[test]
fn narrowed_roots_inherit_ancestor_gitignore_rules() {
    let fixture = Fixture::new();
    fixture.write(".gitignore", b"packages/\n");
    fixture.write("packages/core/src/lib.rs", b"");

    let output = fixture.run(&["--glob", "packages/core/src/*.rs", "--gitignore", "--sort"]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout_lines(&output).is_empty());
}

#[test]
fn literal_file_roots_obey_ancestor_gitignore_rules() {
    let fixture = Fixture::new();
    fixture.write(".gitignore", b"packages/\n");
    fixture.write("packages/core/src/lib.rs", b"");

    let output = fixture.run(&[
        "--glob",
        "packages/core/src/lib.rs",
        "--gitignore",
        "--sort",
    ]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout_lines(&output).is_empty());
}

#[test]
fn parallel_and_sequential_queries_return_the_same_set() {
    let fixture = Fixture::new();
    fixture.write("src/lib.rs", b"");
    fixture.write("src/nested/test_api.rs", b"");
    fixture.write("src/nested/config.toml", b"");
    fixture.write("target/debug/generated.rs", b"");
    fixture.write(".gitignore", b"target/\n");

    let sequential = fixture.run(&[
        "--glob",
        "**/*.{rs,toml}",
        "--exclude",
        "**/target/**",
        "--gitignore",
        "--sort",
    ]);
    let parallel = fixture.run(&[
        "--glob",
        "**/*.{rs,toml}",
        "--exclude",
        "**/target/**",
        "--gitignore",
        "--threads",
        "2",
        "--sort",
    ]);

    assert!(
        sequential.status.success(),
        "{}",
        String::from_utf8_lossy(&sequential.stderr)
    );
    assert!(
        parallel.status.success(),
        "{}",
        String::from_utf8_lossy(&parallel.stderr)
    );
    assert_eq!(stdout_lines(&sequential), stdout_lines(&parallel));
    assert_eq!(
        stdout_lines(&sequential),
        [
            "src/lib.rs",
            "src/nested/config.toml",
            "src/nested/test_api.rs"
        ]
    );
}

#[test]
fn invalid_patterns_fail_with_a_user_facing_error() {
    let fixture = Fixture::new();
    let output = fixture.run(&["--glob", "src/[unterminated"]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("branchcut:"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
}

#[test]
fn literal_directory_matches_in_sequential_and_parallel_modes() {
    let fixture = Fixture::new();
    fixture.write("src/lib.rs", b"");

    let sequential = fixture.run(&["--glob", "src", "--type", "dir"]);
    let parallel = fixture.run(&["--glob", "src", "--type", "dir", "--threads", "2"]);

    assert!(
        sequential.status.success(),
        "{}",
        String::from_utf8_lossy(&sequential.stderr)
    );
    assert!(
        parallel.status.success(),
        "{}",
        String::from_utf8_lossy(&parallel.stderr)
    );
    assert_eq!(stdout_lines(&sequential), ["src"]);
    assert_eq!(stdout_lines(&parallel), ["src"]);
}

#[test]
fn strict_mode_fails_on_invalid_ignore_rules() {
    let fixture = Fixture::new();
    fixture.write(".gitignore", b"[unterminated\n");
    fixture.write("src/lib.rs", b"");

    let output = fixture.run(&["--glob", "**/*.rs", "--gitignore", "--strict"]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("query incomplete"), "{stderr}");
    assert!(stderr.contains("filesystem error"), "{stderr}");
}

#[test]
fn parallel_option_conflicts_are_reported_for_missing_roots() {
    let fixture = Fixture::new();
    let missing_root = fixture.0.join("does not exist");
    let output = Command::new(env!("CARGO_BIN_EXE_branchcut"))
        .arg("--cwd")
        .arg(missing_root)
        .args(["--glob", "**/*.rs", "--threads", "2", "--limit", "1"])
        .output()
        .expect("run branchcut CLI with a missing root");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--threads cannot be combined"), "{stderr}");
}

#[test]
fn exec_passes_spaced_paths_as_one_argument_and_uses_query_root() {
    let fixture = Fixture::new();
    fixture.write("src/nested folder/selected file.rs", b"");
    let capture = fixture.0.join("captured arguments.txt");
    let command = exec_helper_command("child_exec_capture_helper");

    let output = fixture.run_with_env(
        &["--glob", "**/*.rs", "--exec", &command],
        EXEC_CAPTURE_ENV,
        &capture,
    );

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let records = fs::read_to_string(&capture).expect("read captured child arguments");
    let selected_path = PathBuf::from("src")
        .join("nested folder")
        .join("selected file.rs");
    assert_eq!(
        records.lines().collect::<Vec<_>>(),
        [format!(
            "{:?}\t{:?}",
            fixture.0.as_os_str(),
            selected_path.as_os_str()
        )]
    );
}

#[test]
fn sorted_limited_exec_runs_only_the_selected_sorted_matches() {
    let fixture = Fixture::new();
    for name in ["src/c third.rs", "src/a first.rs", "src/b second.rs"] {
        fixture.write(name, b"");
    }
    let capture = fixture.0.join("captured sorted arguments.txt");
    let command = exec_helper_command("child_exec_capture_helper");

    let output = fixture.run_with_env(
        &[
            "--glob", "**/*.rs", "--sort", "--limit", "2", "--exec", &command,
        ],
        EXEC_CAPTURE_ENV,
        &capture,
    );

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let output_lines = stdout_lines(&output);
    assert!(
        output_lines.ends_with(&["src/a first.rs", "src/b second.rs"]),
        "{}",
        output_lines.join("\n")
    );
    let records = fs::read_to_string(&capture).expect("read captured sorted child arguments");
    let captured_paths: Vec<_> = records
        .lines()
        .map(|line| line.split_once('\t').expect("cwd/path separator").1)
        .collect();
    assert_eq!(
        captured_paths,
        [
            format!("{:?}", PathBuf::from("src").join("a first.rs").as_os_str()),
            format!("{:?}", PathBuf::from("src").join("b second.rs").as_os_str())
        ]
    );
}

#[test]
fn unsuccessful_exec_commands_fail_the_query() {
    let fixture = Fixture::new();
    fixture.write("selected.rs", b"");
    let command = exec_helper_command("child_exec_failure_helper");
    let output = Command::new(env!("CARGO_BIN_EXE_branchcut"))
        .arg("--cwd")
        .arg(&fixture.0)
        .args(["--glob", "**/*.rs", "--exec", &command])
        .env(EXEC_FAILURE_ENV, "1")
        .output()
        .expect("run branchcut CLI with a failing child");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("query failed"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
