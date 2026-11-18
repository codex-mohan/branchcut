use branchcut::Query;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Repo(PathBuf);

impl Repo {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "branchcut-ignore-edges-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        let init = git(&path, ["init", "--quiet"]);
        assert!(init.status.success(), "git init failed: {:?}", init);
        Self(path)
    }

    fn write(&self, relative: &str, contents: &[u8]) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn ignored_by_git(&self, relative: &str) -> bool {
        let output = git(
            &self.0,
            [
                "-c",
                "core.ignorecase=false",
                "check-ignore",
                "--no-index",
                "--quiet",
                "--",
                relative,
            ],
        );
        assert!(
            matches!(output.status.code(), Some(0 | 1)),
            "git failed for {relative}: {output:?}"
        );
        output.status.code() == Some(0)
    }

    fn ignored_by_branchcut(&self, cwd: &Path, relative: &str) -> bool {
        let query = Query {
            cwd: cwd.to_owned(),
            gitignore: true,
            hidden: true,
            ..Query::default()
        }
        .compile()
        .unwrap();
        !query.explain_path(relative).unwrap().selected
    }

    fn compare(&self, cwd: &Path, repo_relative: &[&str]) {
        let prefix = cwd.strip_prefix(&self.0).unwrap();
        for repo_path in repo_relative {
            let repo_path = Path::new(repo_path);
            let candidate = repo_path.strip_prefix(prefix).unwrap_or(repo_path);
            let git_ignored = self.ignored_by_git(repo_path.to_str().unwrap());
            let branchcut_ignored = self.ignored_by_branchcut(cwd, candidate.to_str().unwrap());
            assert_eq!(
                branchcut_ignored,
                git_ignored,
                "ignore mismatch for {} from cwd {}",
                repo_path.display(),
                cwd.display()
            );
        }
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git<const N: usize>(cwd: &Path, args: [&str; N]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .unwrap()
}

#[test]
fn three_nested_ignore_levels_obey_nearest_rule_precedence() {
    let repo = Repo::new();
    repo.write(".gitignore", b"*.tmp\n!keep.tmp\n");
    repo.write("one/.gitignore", b"!drop.tmp\nkeep.tmp\n");
    repo.write("one/two/.gitignore", b"!drop.tmp\n");
    for file in [
        "drop.tmp",
        "keep.tmp",
        "one/drop.tmp",
        "one/keep.tmp",
        "one/two/drop.tmp",
        "one/two/keep.tmp",
    ] {
        repo.write(file, b"");
    }
    repo.compare(
        &repo.0,
        &[
            "drop.tmp",
            "keep.tmp",
            "one/drop.tmp",
            "one/keep.tmp",
            "one/two/drop.tmp",
            "one/two/keep.tmp",
        ],
    );
}

#[test]
fn narrowed_literal_root_preserves_parent_ignore_context_and_child_overrides() {
    let repo = Repo::new();
    repo.write(".gitignore", b"target/\n!target/visible.rs\n");
    repo.write("target/.gitignore", b"!hidden.rs\n");
    repo.write("target/visible.rs", b"");
    repo.write("target/hidden.rs", b"");
    repo.write("target/drop.rs", b"");

    let query = Query {
        cwd: repo.0.clone(),
        gitignore: true,
        hidden: true,
        patterns: vec!["target/visible.rs".into()],
        ..Query::default()
    }
    .compile()
    .unwrap();
    let explanation = query.explain_path("target/visible.rs").unwrap();
    let git_expected = !repo.ignored_by_git("target/visible.rs");
    assert_eq!(explanation.selected, git_expected);
    assert_eq!(
        repo.ignored_by_branchcut(&repo.0, "target/drop.rs"),
        repo.ignored_by_git("target/drop.rs")
    );
    assert_eq!(
        repo.ignored_by_branchcut(&repo.0, "target/hidden.rs"),
        repo.ignored_by_git("target/hidden.rs")
    );
}

#[test]
fn starting_inside_repository_inherits_ignore_rules_above_query_cwd() {
    let repo = Repo::new();
    repo.write(".gitignore", b"*.secret\n");
    repo.write("packages/app/src/public.rs", b"");
    repo.write("packages/app/src/private.secret", b"");
    let cwd = repo.0.join("packages/app");
    repo.compare(
        &cwd,
        &[
            "packages/app/src/public.rs",
            "packages/app/src/private.secret",
        ],
    );
}

#[test]
fn utf8_bom_does_not_turn_the_first_ignore_rule_into_a_different_name() {
    let repo = Repo::new();
    repo.write(".gitignore", b"\xef\xbb\xbfsecret.tmp\n");
    repo.write("secret.tmp", b"");
    repo.write("visible.tmp", b"");
    repo.compare(&repo.0, &["secret.tmp", "visible.tmp"]);
}

#[test]
fn escaped_trailing_space_and_bracket_class_match_git() {
    let repo = Repo::new();
    repo.write(".gitignore", b"trailing\\ \nfile[0-9].tmp\n!file7.tmp\n");
    repo.write("trailing ", b"");
    repo.write("trailing", b"");
    for file in ["file1.tmp", "file7.tmp", "filex.tmp"] {
        repo.write(file, b"");
    }
    repo.compare(
        &repo.0,
        &[
            "trailing ",
            "trailing",
            "file1.tmp",
            "file7.tmp",
            "filex.tmp",
        ],
    );
}

#[test]
fn malformed_bracket_expression_is_handled_without_panicking() {
    let repo = Repo::new();
    repo.write(".gitignore", b"[broken\n");
    repo.write("[broken", b"");
    let query = Query {
        cwd: repo.0.clone(),
        gitignore: true,
        hidden: true,
        ..Query::default()
    }
    .compile()
    .unwrap();
    let explanation = query.explain_path("[broken").unwrap();
    assert_eq!(explanation.selected, !repo.ignored_by_git("[broken"));
}

#[test]
fn descending_range_is_compared_with_git() {
    compare_case(b"[z-a]\n", &["[z-a]", "z", "a"]);
}

#[test]
fn escaped_closing_bracket_is_compared_with_git() {
    compare_case(b"[\\]]\n", &["name]", "namex"]);
}

#[test]
fn escaped_hyphen_is_compared_with_git() {
    compare_case(b"[\\-]\n", &["-", "x"]);
}

#[test]
fn posix_digit_class_is_compared_with_git() {
    compare_case(b"[[:digit:]]\n", &["7", "x"]);
}

#[test]
fn dot_slash_prefix_is_compared_with_git() {
    compare_case(b"./anchored\n", &["anchored", "nested/anchored"]);
}

#[test]
fn escaped_path_separator_is_compared_with_git() {
    compare_case(b"slash\\/name\n", &["slash/name"]);
}

fn compare_case(rules: &[u8], candidates: &[&str]) {
    let repo = Repo::new();
    repo.write(".gitignore", rules);
    for candidate in candidates {
        repo.write(candidate, b"");
    }
    repo.compare(&repo.0, candidates);
}

#[cfg(unix)]
#[test]
fn terminal_backslash_is_compared_with_git() {
    let repo = Repo::new();
    repo.write(".gitignore", b"trailing\\\n");
    repo.write("trailing", b"");
    repo.write("trailing\\", b"");
    repo.compare(&repo.0, &["trailing", "trailing\\"]);
}
