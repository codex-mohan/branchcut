use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "branchcut content-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create content fixture root");
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
            .current_dir(&self.0)
            .args(args)
            .output()
            .expect("run branchcut CLI")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn grep_takes_a_pattern_then_files_and_reports_grep_exit_codes() {
    let fixture = Fixture::new();
    fixture.write("src/a file.txt", b"start\nNeedle twice needle\nend\n");
    fixture.write("src/other.txt", b"needle in a different file\n");
    let output = fixture.run(&["grep", "-i", "-w", "-C", "1", "needle", "src/a file.txt"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "src/a file.txt-1-start\nsrc/a file.txt:2:1:Needle twice needle\nsrc/a file.txt-3-end\n"
    );
    let no_match = fixture.run(&["grep", "absent", "src/a file.txt"]);
    assert_eq!(no_match.status.code(), Some(1), "{}", stderr(&no_match));
    let missing = fixture.run(&["grep", "needle", "missing.txt"]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(stderr(&missing).contains("missing.txt"));
}

#[test]
fn grep_reads_multiple_patterns_and_counts_per_file() {
    let fixture = Fixture::new();
    fixture.write("patterns.txt", b"error\r\nwarning\n");
    fixture.write("a.log", b"error; warning\nwarning\nother\n");
    let patterns = fixture.0.join("patterns.txt");
    let output = Command::new(env!("CARGO_BIN_EXE_branchcut"))
        .current_dir(&fixture.0)
        .args(["grep", "-c", "-e", "other", "-f"])
        .arg(patterns)
        .arg("a.log")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "a.log:3\n");
}

#[test]
fn grep_searches_utf16_and_reports_jsonl_from_stdin() {
    let fixture = Fixture::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_branchcut"))
        .current_dir(&fixture.0)
        .args(["grep", "--json", "--summary", "ERR", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let encoded = "a\r\né ERR\r\n".encode_utf16().flat_map(u16::to_le_bytes);
    let data: Vec<u8> = [0xff, 0xfe].into_iter().chain(encoded).collect();
    child.stdin.take().unwrap().write_all(&data).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(
        text.contains("\"path\":\"-\"") || text.contains("\"path\": \"-\""),
        "{text}"
    );
    assert!(
        text.contains("\"line\":2,\"column\":4,\"byte_offset\":3"),
        "{text}"
    );
    assert!(
        text.contains("\"kind\":\"summary\",\"matched\":1"),
        "{text}"
    );
}

#[test]
fn find_accepts_roots_and_keeps_the_legacy_literal_shortcut() {
    let fixture = Fixture::new();
    fixture.write("src/config.json", b"");
    fixture.write("other/config.json", b"");
    fixture.write("find.txt", b"");
    let output = fixture.run(&["find", "config.json", "src"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "src/config.json\n");
    assert_eq!(stdout(&fixture.run(&["find"])), "find.txt\n");
}

#[test]
fn reports_path_line_column_and_text() {
    let fixture = Fixture::new();
    fixture.write("src/a.txt", b"alpha\nbeta needle\ngamma\n");
    fixture.write("src/b.txt", b"nothing here\n");

    let output = fixture.run(&["--glob", "**/*.txt", "--content", "needle"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "src/a.txt:2:6:beta needle\n");
}

#[test]
fn multiple_patterns_report_the_earliest_match() {
    let fixture = Fixture::new();
    fixture.write("a.txt", b"one two three\n");

    let output = fixture.run(&["--glob", "*.txt", "--content", "three", "--content", "two"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "a.txt:1:5:one two three\n");
}

#[test]
fn case_modes_apply_to_content_patterns() {
    let fixture = Fixture::new();
    fixture.write("a.txt", b"Hello NEEDLE\nhello needle\n");

    let sensitive = fixture.run(&["--glob", "*.txt", "--content", "needle"]);
    assert_eq!(stdout(&sensitive), "a.txt:2:7:hello needle\n");

    let insensitive = fixture.run(&["--glob", "*.txt", "-i", "--content", "needle"]);
    assert_eq!(
        stdout(&insensitive),
        "a.txt:1:7:Hello NEEDLE\na.txt:2:7:hello needle\n"
    );

    let smart_lower = fixture.run(&["--glob", "*.txt", "--smart-case", "--content", "needle"]);
    assert_eq!(
        stdout(&smart_lower),
        "a.txt:1:7:Hello NEEDLE\na.txt:2:7:hello needle\n"
    );

    let smart_upper = fixture.run(&["--glob", "*.txt", "--smart-case", "--content", "NEEDLE"]);
    assert_eq!(stdout(&smart_upper), "a.txt:1:7:Hello NEEDLE\n");
}

#[test]
fn respects_exclusions_and_selection_filters() {
    let fixture = Fixture::new();
    fixture.write("src/keep.txt", b"needle\n");
    fixture.write("skip/drop.txt", b"needle\n");
    fixture.write("src/other.md", b"needle\n");

    let output = fixture.run(&[
        "--glob",
        "**/*",
        "--exclude",
        "**/skip/**",
        "-e",
        "txt",
        "--content",
        "needle",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "src/keep.txt:1:1:needle\n");
}

#[test]
fn count_reports_matching_lines() {
    let fixture = Fixture::new();
    fixture.write("a.txt", b"needle\nno\nneedle needle\n");

    let output = fixture.run(&["--glob", "*.txt", "--content", "needle", "--count"]);
    assert_eq!(stdout(&output), "2\n");
}

#[test]
fn files_with_matches_prints_each_path_once() {
    let fixture = Fixture::new();
    fixture.write("a.txt", b"needle\nneedle\n");
    fixture.write("b.txt", b"none\n");
    fixture.write("c.txt", b"needle\n");

    let output = fixture.run(&["--glob", "*.txt", "--content", "needle", "-l"]);
    let text = stdout(&output);
    let mut lines: Vec<&str> = text.lines().collect();
    lines.sort_unstable();
    assert_eq!(lines, vec!["a.txt", "c.txt"]);
}

#[test]
fn limit_and_first_stop_after_enough_lines() {
    let fixture = Fixture::new();
    fixture.write("a.txt", b"needle\nneedle\nneedle\n");
    fixture.write("b.txt", b"needle\n");

    let limited = fixture.run(&["--glob", "*.txt", "--content", "needle", "--limit", "2"]);
    assert_eq!(stdout(&limited), "a.txt:1:1:needle\na.txt:2:1:needle\n");

    let first = fixture.run(&["--glob", "*.txt", "--content", "needle", "--first"]);
    assert_eq!(stdout(&first), "a.txt:1:1:needle\n");
}

#[test]
fn sort_orders_files_before_scanning() {
    let fixture = Fixture::new();
    fixture.write("z.txt", b"needle\n");
    fixture.write("a.txt", b"needle\n");
    fixture.write("m.txt", b"needle\n");

    let output = fixture.run(&[
        "--glob",
        "*.txt",
        "--content",
        "needle",
        "--sort",
        "--limit",
        "2",
    ]);
    assert_eq!(stdout(&output), "a.txt:1:1:needle\nm.txt:1:1:needle\n");
}

#[test]
fn binary_files_are_skipped_unless_text_is_requested() {
    let fixture = Fixture::new();
    fixture.write("bin.dat", b"needle\0binary\n");
    fixture.write("text.dat", b"needle text\n");

    let automatic = fixture.run(&["--glob", "*.dat", "--content", "needle"]);
    assert_eq!(stdout(&automatic), "text.dat:1:1:needle text\n");

    let text = fixture.run(&["--glob", "*.dat", "--content", "needle", "--text"]);
    assert!(stdout(&text).contains("bin.dat:1:1:needle"));
    assert!(stdout(&text).contains("text.dat:1:1:needle text"));
}

#[test]
fn json_lines_include_line_column_and_text() {
    let fixture = Fixture::new();
    fixture.write("a.txt", b"needle \"quoted\"\n");

    let output = fixture.run(&["--glob", "*.txt", "--content", "needle", "--json"]);
    let line = stdout(&output);
    let line = line.trim_end();
    assert!(line.starts_with("{\"path\":\"a.txt\""), "{line}");
    assert!(line.contains(",\"line\":1"), "{line}");
    assert!(line.contains(",\"column\":1"), "{line}");
    assert!(
        line.contains(",\"text\":\"needle \\\"quoted\\\"\""),
        "{line}"
    );
    assert!(line.ends_with('}'), "{line}");
}

#[test]
fn stats_report_content_work() {
    let fixture = Fixture::new();
    fixture.write("a.txt", b"needle\n");
    fixture.write("bin.dat", b"needle\0x");

    let output = fixture.run(&["--glob", "**/*", "--content", "needle", "--stats"]);
    let errors = stderr(&output);
    assert!(errors.contains("content files           2"), "{errors}");
    assert!(errors.contains("binary files skipped    1"), "{errors}");
}

#[test]
fn crlf_endings_are_not_printed() {
    let fixture = Fixture::new();
    fixture.write("a.txt", b"first\r\nneedle here\r\n");

    let output = fixture.run(&["--glob", "*.txt", "--content", "needle"]);
    assert_eq!(stdout(&output), "a.txt:2:1:needle here\n");
}

#[test]
fn invalid_combinations_are_rejected() {
    let fixture = Fixture::new();
    fixture.write("a.txt", b"needle\n");

    let exec = fixture.run(&[
        "--glob",
        "*.txt",
        "--content",
        "needle",
        "--exec",
        "echo {}",
    ]);
    assert_eq!(exec.status.code(), Some(2));
    assert!(stderr(&exec).contains("--content cannot be combined with command execution"));

    let print0 = fixture.run(&["--glob", "*.txt", "--content", "needle", "--print0"]);
    assert_eq!(print0.status.code(), Some(2));

    let dirs = fixture.run(&["--type", "dir", "--content", "needle"]);
    assert_eq!(dirs.status.code(), Some(2));
    assert!(stderr(&dirs).contains("--content searches regular files"));

    let missing = fixture.run(&["-l", "--glob", "*.txt"]);
    assert_eq!(missing.status.code(), Some(2));

    let empty = fixture.run(&["--glob", "*.txt", "--content", ""]);
    assert_eq!(empty.status.code(), Some(2));
}

#[test]
fn parallel_workers_produce_the_same_sorted_result() {
    let fixture = Fixture::new();
    for index in 0..24 {
        fixture.write(format!("f{index:02}.txt"), b"needle\n");
    }

    let sequential = fixture.run(&["--glob", "*.txt", "--content", "needle", "--sort"]);
    let parallel = fixture.run(&[
        "--glob",
        "*.txt",
        "--content",
        "needle",
        "--sort",
        "--threads",
        "4",
    ]);
    assert_eq!(stdout(&sequential), stdout(&parallel));
}

#[test]
fn additional_search_roots_are_searched() {
    let fixture = Fixture::new();
    fixture.write("inside.txt", b"needle\n");
    let external = Fixture::new();
    external.write("outside.txt", b"needle\n");

    let output = Command::new(env!("CARGO_BIN_EXE_branchcut"))
        .arg("--cwd")
        .arg(&fixture.0)
        .arg("--glob")
        .arg("*.txt")
        .arg("--content")
        .arg("needle")
        .arg("--search-path")
        .arg(".")
        .arg("--search-path")
        .arg(&external.0)
        .output()
        .expect("run branchcut with an additional root");
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("inside.txt:1:1:needle"), "{text}");
    assert!(text.contains("outside.txt:1:1:needle"), "{text}");
}

#[test]
fn quiet_content_uses_exit_status_only() {
    let fixture = Fixture::new();
    fixture.write("hit.txt", b"needle\n");
    fixture.write("miss.txt", b"none\n");

    let hit = fixture.run(&["--glob", "hit.txt", "--content", "needle", "--quiet"]);
    assert!(hit.status.success());
    assert!(hit.stdout.is_empty());

    let miss = fixture.run(&["--glob", "hit.txt", "--content", "absent", "--quiet"]);
    assert_eq!(miss.status.code(), Some(1));
    assert!(miss.stdout.is_empty());
}

#[test]
fn json_metadata_adds_file_fields_to_content_records() {
    let fixture = Fixture::new();
    fixture.write("a.txt", b"needle\n");

    let output = fixture.run(&["--glob", "*.txt", "--content", "needle", "--json-metadata"]);
    let text = stdout(&output);
    let line = text.trim_end();
    assert!(line.starts_with("{\"path\":\"a.txt\""), "{line}");
    assert!(line.contains(",\"line\":1"), "{line}");
    assert!(line.contains(",\"type\":\"file\""), "{line}");
    assert!(line.contains(",\"size\":7"), "{line}");
    assert!(line.contains(",\"modified_unix_ms\":"), "{line}");
    assert!(line.ends_with('}'), "{line}");
}

#[test]
fn content_mode_never_emits_directories() {
    let fixture = Fixture::new();
    fixture.write("dir/nested.txt", b"needle\n");

    let output = fixture.run(&["--glob", "**/*", "--content", "needle"]);
    assert_eq!(stdout(&output), "dir/nested.txt:1:1:needle\n");
}
