use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "branchcut-workflows-{}-{}",
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
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_branchcut"));
        command.arg("--cwd").arg(&self.0);
        command
    }
    fn run(&self, arguments: &[&str]) -> Output {
        self.command().args(arguments).output().unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn text(output: &Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn quiet_exits_distinguish_matches_no_matches_and_errors_without_output() {
    let fixture = Fixture::new();
    fixture.write("one.rs", b"");
    let found = fixture.run(&["--glob", "*.rs", "--quiet", "--threads", "4"]);
    assert_eq!(found.status.code(), Some(0));
    assert!(found.stdout.is_empty());
    let absent = fixture.run(&["--glob", "*.toml", "--has-results"]);
    assert_eq!(absent.status.code(), Some(1));
    assert!(absent.stdout.is_empty());
    let error = fixture.run(&["--glob", "[invalid", "--quiet"]);
    assert_eq!(error.status.code(), Some(2));
    assert!(error.stdout.is_empty());
}

#[test]
fn path_format_templates_cover_components_and_literal_braces() {
    let fixture = Fixture::new();
    fixture.write("dir/spaced name.rs", b"");
    let output = fixture.run(&[
        "--glob",
        "**/*.rs",
        "--format",
        "{/}|{//}|{.}|{/.}|{{literal}}",
    ]);
    let rendered = text(&output).replace('\\', "/");
    assert_eq!(
        rendered,
        "spaced name.rs|dir|dir/spaced name|spaced name|{literal}\n"
    );
    let invalid = fixture.run(&["--format", "{unknown}"]);
    assert_eq!(invalid.status.code(), Some(2));
}

#[test]
fn preview_expands_native_arguments_without_launching_the_executable() {
    let fixture = Fixture::new();
    fixture.write("dir/spaced name.rs", b"");
    let output = fixture.run(&[
        "--glob",
        "**/*.rs",
        "--exec",
        "this-executable-does-not-exist '{/}' '{//}' '{.}' '{/.}' ''",
        "--exec-preview",
    ]);
    let rendered = text(&output);
    assert!(
        rendered
            .contains("\"argv\":[\"this-executable-does-not-exist\",\"spaced name.rs\",\"dir\","),
        "{rendered}"
    );
    assert!(rendered.contains("\"spaced name\",\"\"]}"), "{rendered}");
    assert_eq!(rendered.lines().count(), 1);
}

#[test]
fn batch_preview_preserves_bounds_and_expands_basename_arguments() {
    let fixture = Fixture::new();
    for name in ["a.rs", "b.rs", "c.rs"] {
        fixture.write(name, b"");
    }
    let output = fixture.run(&[
        "--glob",
        "*.rs",
        "--sort",
        "--exec-batch",
        "missing-tool {/}",
        "--batch-size",
        "2",
        "--exec-preview",
    ]);
    let rendered = text(&output);
    let lines: Vec<_> = rendered.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("\"argv\":[\"missing-tool\",\"a.rs\",\"b.rs\"]"));
    assert!(lines[1].contains("\"argv\":[\"missing-tool\",\"c.rs\"]"));
}

#[test]
fn metadata_json_is_opt_in_and_machine_modes_never_emit_decoration() {
    let fixture = Fixture::new();
    fixture.write("file.rs", b"abc");
    let ordinary = fixture.run(&[
        "--glob",
        "*.rs",
        "--json",
        "--color",
        "always",
        "--hyperlink",
        "always",
    ]);
    assert_eq!(text(&ordinary), "{\"path\":\"file.rs\"}\n");
    let metadata = fixture.run(&["--glob", "*.rs", "--json-metadata", "--color", "always"]);
    let rendered = text(&metadata);
    assert!(
        rendered.contains("\"type\":\"file\",\"size\":3,"),
        "{rendered}"
    );
    assert!(rendered.contains("\"modified_unix_ms\":"));
    assert!(!rendered.contains('\x1b'));
    let nul = fixture.run(&[
        "--glob",
        "*.rs",
        "--print0",
        "--color",
        "always",
        "--hyperlink",
        "always",
    ]);
    assert!(nul.status.success());
    assert_eq!(nul.stdout, b"file.rs\0");
}

#[test]
fn requested_plain_decorations_use_escaped_file_urls() {
    let fixture = Fixture::new();
    fixture.write("space name.rs", b"");
    let rendered = text(&fixture.run(&[
        "--glob",
        "*.rs",
        "--color",
        "always",
        "--hyperlink",
        "always",
    ]));
    assert!(rendered.contains("\x1b[32m"));
    assert!(rendered.contains("file:///"));
    assert!(rendered.contains("space%20name.rs"));
    assert!(rendered.ends_with("\x1b[0m\x1b]8;;\x1b\\\n"));
}

#[test]
fn explicit_parallel_limits_are_exact_and_sorted_limits_stay_global() {
    let fixture = Fixture::new();
    for index in 0..120 {
        fixture.write(&format!("d{}/file{index:03}.rs", index % 9), b"");
    }
    for limit in [1, 3, 17] {
        let argument = limit.to_string();
        let rendered =
            text(&fixture.run(&["--glob", "**/*.rs", "--threads", "4", "--limit", &argument]));
        let paths: Vec<_> = rendered.lines().collect();
        assert_eq!(paths.len(), limit);
        let unique: std::collections::HashSet<_> = paths.iter().collect();
        assert_eq!(unique.len(), limit);
        let all = text(&fixture.run(&["--glob", "**/*.rs", "--sort", "--threads", "1"]));
        let sorted = text(&fixture.run(&[
            "--glob",
            "**/*.rs",
            "--sort",
            "--threads",
            "4",
            "--limit",
            &argument,
        ]));
        assert_eq!(
            sorted.lines().collect::<Vec<_>>(),
            all.lines().take(limit).collect::<Vec<_>>()
        );
    }
}

#[test]
fn shell_completion_generators_are_standalone_and_validate_the_shell() {
    let fixture = Fixture::new();
    for shell in ["bash", "zsh", "fish", "powershell"] {
        let output = fixture.run(&["--config", "does-not-exist", "--completions", shell]);
        let script = text(&output);
        assert!(script.contains("branchcut"));
        assert!(script.contains("--exec-preview") || script.contains("-l exec-preview"));
    }
    assert_eq!(
        fixture.run(&["--completions", "unknown"]).status.code(),
        Some(2)
    );
}

#[test]
fn workflow_child() {
    let Some(root) = std::env::var_os("BRANCHCUT_WORKFLOW_CHILD") else {
        return;
    };
    let root = PathBuf::from(root);
    let args: Vec<_> = std::env::args_os().collect();
    let index = args.iter().position(|arg| arg == "--skip").unwrap();
    let file = Path::new(&args[index + 1]);
    if std::env::var_os("BRANCHCUT_WORKFLOW_FAIL").is_some() {
        panic!("requested child failure");
    }
    let active = root.join("active");
    fs::create_dir_all(&active).unwrap();
    let marker = active.join(std::process::id().to_string());
    fs::write(&marker, b"").unwrap();
    let count = fs::read_dir(&active).unwrap().count();
    assert!(count <= 2, "too many concurrent children: {count}");
    std::thread::sleep(std::time::Duration::from_millis(80));
    let mut capture = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("capture"))
        .unwrap();
    writeln!(capture, "{}:{count}", file.display()).unwrap();
    fs::remove_file(marker).unwrap();
}

#[test]
fn bounded_child_execution_waits_for_children_and_propagates_failure() {
    let fixture = Fixture::new();
    for index in 0..7 {
        fixture.write(&format!("file{index}.rs"), b"");
    }
    let executable = std::env::current_exe().unwrap();
    let command = format!(
        "'{}' --exact workflow_child --nocapture --skip {{}}",
        executable.display()
    );
    let output = fixture
        .command()
        .args([
            "--glob",
            "*.rs",
            "--count",
            "--exec",
            &command,
            "--exec-jobs",
            "2",
            "--threads",
            "4",
        ])
        .env("BRANCHCUT_WORKFLOW_CHILD", &fixture.0)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let log = fs::read_to_string(fixture.0.join("capture")).unwrap();
    assert_eq!(log.lines().count(), 7);
    assert_eq!(fs::read_dir(fixture.0.join("active")).unwrap().count(), 0);
    let output = fixture
        .command()
        .args([
            "--glob",
            "*.rs",
            "--count",
            "--exec",
            &command,
            "--exec-jobs",
            "2",
        ])
        .env("BRANCHCUT_WORKFLOW_CHILD", &fixture.0)
        .env("BRANCHCUT_WORKFLOW_FAIL", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("command exited unsuccessfully"));
}

#[cfg(unix)]
#[test]
fn formatting_and_execution_preserve_non_utf8_native_names() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let fixture = Fixture::new();
    let name = OsString::from_vec(b"bad\xff.rs".to_vec());
    fs::write(fixture.0.join(name), b"").unwrap();
    let output = fixture.run(&["--glob", "*.rs", "--format", "{/}"]);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"bad\xff.rs\n");
    let preview = fixture.run(&["--glob", "*.rs", "--exec", "missing {}", "--exec-preview"]);
    let preview = text(&preview);
    assert!(preview.contains("\"argv_native\""), "{preview}");
    assert!(preview.contains("626164ff2e7273"), "{preview}");
}
