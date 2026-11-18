#[cfg(unix)]
#[path = "support/native_fixture.rs"]
mod native_fixture;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    home: PathBuf,
    xdg: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "branchcut-ignore-sources-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let home = root.join("isolated-home");
        let xdg = root.join("isolated-xdg");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&xdg).unwrap();
        fs::write(home.join(".gitconfig"), b"").unwrap();
        Self { root, home, xdg }
    }

    fn init_repo(&self, relative: &str) -> PathBuf {
        let path = self.root.join(relative);
        fs::create_dir_all(&path).unwrap();
        let result = self.git(&path, ["init", "--quiet"]);
        assert!(result.status.success(), "git init failed: {result:?}");
        path
    }

    fn write(&self, base: &Path, relative: &str, contents: &[u8]) {
        let path = base.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn git<const N: usize>(&self, cwd: &Path, args: [&str; N]) -> Output {
        Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("XDG_CONFIG_HOME", &self.xdg)
            .env("GIT_CONFIG_GLOBAL", self.home.join(".gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("GIT_CONFIG_SYSTEM")
            .env_remove("GIT_CONFIG_COUNT")
            .env_remove("GIT_CONFIG_PARAMETERS")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .output()
            .unwrap()
    }

    fn run_branchcut(&self, cwd: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_branchcut"))
            .arg("--cwd")
            .arg(cwd)
            .args(args)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("XDG_CONFIG_HOME", &self.xdg)
            .env("GIT_CONFIG_GLOBAL", self.home.join(".gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("GIT_CONFIG_SYSTEM")
            .env_remove("GIT_CONFIG_COUNT")
            .env_remove("GIT_CONFIG_PARAMETERS")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .output()
            .unwrap()
    }

    fn ignored_by_git(&self, repo: &Path, relative: &str) -> bool {
        let result = self.git(
            repo,
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
            matches!(result.status.code(), Some(0 | 1)),
            "git check-ignore failed: {result:?}"
        );
        result.status.code() == Some(0)
    }

    fn assert_results_match_git(&self, repo: &Path, cwd: &Path, relative_files: &[&str]) {
        let expected: Vec<_> = relative_files
            .iter()
            .filter(|path| !self.ignored_by_git(repo, path))
            .copied()
            .collect();
        for threads in [1, 4] {
            let output = self.run_branchcut(
                cwd,
                &[
                    "--gitignore",
                    "--hidden",
                    "--glob",
                    "**/*.rs",
                    "--sort",
                    "--threads",
                    if threads == 1 { "1" } else { "4" },
                ],
            );
            assert!(
                output.status.success(),
                "Branchcut failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let actual: Vec<_> = String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .map(str::to_owned)
                .collect();
            let expected_from_cwd: Vec<_> = expected
                .iter()
                .filter_map(|path| {
                    Path::new(path)
                        .strip_prefix(cwd.strip_prefix(repo).unwrap_or(Path::new("")))
                        .ok()
                })
                .map(|path| path.to_string_lossy().replace('\\', "/"))
                .collect();
            assert_eq!(
                actual, expected_from_cwd,
                "result mismatch with Git using {threads} worker(s)"
            );
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn set_local_config(fixture: &Fixture, repo: &Path, key: &str, value: &Path) {
    let value = value.to_string_lossy().into_owned();
    let result = fixture.git(repo, ["config", "--local", key, &value]);
    assert!(result.status.success(), "git config failed: {result:?}");
}

fn set_global_config(fixture: &Fixture, key: &str, value: &Path) {
    let value = value.to_string_lossy().into_owned();
    let result = fixture.git(&fixture.root, ["config", "--global", key, &value]);
    assert!(
        result.status.success(),
        "global git config failed: {result:?}"
    );
}

#[test]
fn repository_info_exclude_is_used_by_gitignore_search() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(&repo, ".git/info/exclude", b"private.rs\n");
    fixture.write(&repo, "private.rs", b"");
    fixture.write(&repo, "visible.rs", b"");
    fixture.assert_results_match_git(&repo, &repo, &["private.rs", "visible.rs"]);
}

#[test]
fn repository_core_excludes_file_is_used() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    let excludes = fixture.root.join("repo-excludes");
    fs::write(&excludes, b"repo-private.rs\n").unwrap();
    set_local_config(&fixture, &repo, "core.excludesFile", &excludes);
    fixture.write(&repo, "repo-private.rs", b"");
    fixture.write(&repo, "visible.rs", b"");
    fixture.assert_results_match_git(&repo, &repo, &["repo-private.rs", "visible.rs"]);
}

#[test]
fn isolated_user_gitconfig_excludes_file_is_used() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    let excludes = fixture.root.join("user-excludes");
    fs::write(&excludes, b"user-private.rs\n").unwrap();
    set_global_config(&fixture, "core.excludesFile", &excludes);
    fixture.write(&repo, "user-private.rs", b"");
    fixture.write(&repo, "visible.rs", b"");
    fixture.assert_results_match_git(&repo, &repo, &["user-private.rs", "visible.rs"]);
}

#[test]
fn default_xdg_git_ignore_file_is_used_without_core_excludes_file() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    let global_ignore = fixture.xdg.join("git/ignore");
    fs::create_dir_all(global_ignore.parent().unwrap()).unwrap();
    fs::write(&global_ignore, b"xdg-private.rs\n").unwrap();
    fixture.write(&repo, "xdg-private.rs", b"");
    fixture.write(&repo, "visible.rs", b"");
    fixture.assert_results_match_git(&repo, &repo, &["xdg-private.rs", "visible.rs"]);
}

#[test]
fn repository_gitignore_has_precedence_over_lower_ignore_sources() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(&repo, ".git/info/exclude", b"!keep.rs\n");
    fs::create_dir_all(fixture.xdg.join("git")).unwrap();
    fs::write(fixture.xdg.join("git/ignore"), b"*.rs\n").unwrap();
    fixture.write(&repo, ".gitignore", b"keep.rs\n");
    fixture.write(&repo, "keep.rs", b"");
    fixture.write(&repo, "visible.txt", b"");
    assert!(fixture.ignored_by_git(&repo, "keep.rs"));
    let output = fixture.run_branchcut(&repo, &["--gitignore", "--hidden", "--why", "keep.rs"]);
    assert!(
        output.status.success(),
        "--why failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let explanation = String::from_utf8(output.stdout).unwrap();
    assert!(
        explanation.contains(".gitignore"),
        "missing winning rule provenance: {explanation}"
    );
    assert!(
        explanation.contains("keep.rs"),
        "missing winning rule text: {explanation}"
    );
}

#[test]
fn nested_cwd_inherits_parent_rules_after_parent_reinclusion() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(
        &repo,
        ".gitignore",
        b"packages/\n!packages/\npackages/app/\n!packages/app/\n",
    );
    fixture.write(&repo, "packages/app/.gitignore", b"private.rs\n");
    fixture.write(&repo, "packages/app/src/private.rs", b"");
    fixture.write(&repo, "packages/app/src/public.rs", b"");
    let cwd = repo.join("packages/app");
    fixture.assert_results_match_git(
        &repo,
        &cwd,
        &["packages/app/src/private.rs", "packages/app/src/public.rs"],
    );
}

#[test]
fn nested_cwd_inside_excluded_parent_remains_excluded() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(&repo, ".gitignore", b"packages/\n");
    fixture.write(&repo, "packages/app/src/private.rs", b"");
    let cwd = repo.join("packages/app");
    assert!(fixture.ignored_by_git(&repo, "packages/app/src/private.rs"));
    fixture.assert_results_match_git(&repo, &cwd, &["packages/app/src/private.rs"]);
    let why = fixture.run_branchcut(
        &cwd,
        &["--gitignore", "--hidden", "--why", "src/private.rs"],
    );
    assert!(
        why.status.success(),
        "--why failed: {}",
        String::from_utf8_lossy(&why.stderr)
    );
    assert!(String::from_utf8(why.stdout).unwrap().contains("packages/"));
}

#[test]
fn ignore_discovery_stops_at_nearest_nested_repository() {
    let fixture = Fixture::new();
    let outer = fixture.init_repo("outer");
    fixture.write(&outer, ".gitignore", b"*.rs\n");
    let inner = outer.join("nested");
    fs::create_dir_all(&inner).unwrap();
    let init = fixture.git(&inner, ["init", "--quiet"]);
    assert!(init.status.success(), "nested git init failed: {init:?}");
    fixture.write(&inner, "keep.rs", b"");
    fixture.write(&inner, "nested/.gitignore", b"drop.rs\n");
    fixture.write(&inner, "nested/keep.rs", b"");
    fixture.write(&inner, "nested/drop.rs", b"");
    assert!(!fixture.ignored_by_git(&inner, "keep.rs"));
    assert!(fixture.ignored_by_git(&inner, "nested/drop.rs"));
    fixture.assert_results_match_git(
        &inner,
        &inner,
        &["keep.rs", "nested/keep.rs", "nested/drop.rs"],
    );
}

#[test]
fn linked_worktree_reads_common_git_info_exclude() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("main-repo");
    let config = fixture.git(&repo, ["config", "user.name", "Branchcut test"]);
    assert!(config.status.success());
    let config = fixture.git(&repo, ["config", "user.email", "branchcut@example.invalid"]);
    assert!(config.status.success());
    fixture.write(&repo, "seed.rs", b"");
    let added = fixture.git(&repo, ["add", "seed.rs"]);
    assert!(added.status.success());
    let committed = fixture.git(&repo, ["commit", "--quiet", "-m", "seed"]);
    assert!(
        committed.status.success(),
        "git commit failed: {committed:?}"
    );
    let worktree = fixture.root.join("linked-worktree");
    let added = fixture.git(
        &repo,
        [
            "worktree",
            "add",
            "--quiet",
            "--detach",
            worktree.to_str().unwrap(),
            "HEAD",
        ],
    );
    assert!(added.status.success(), "git worktree add failed: {added:?}");
    fixture.write(&repo, ".git/info/exclude", b"worktree-private.rs\n");
    fixture.write(&worktree, "worktree-private.rs", b"");
    fixture.write(&worktree, "visible.rs", b"");
    assert!(fixture.ignored_by_git(&worktree, "worktree-private.rs"));
    fixture.assert_results_match_git(
        &worktree,
        &worktree,
        &["seed.rs", "worktree-private.rs", "visible.rs"],
    );
}

#[test]
fn explicit_ignore_files_work_without_discovery_and_last_file_wins() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(&repo, ".gitignore", b"visible.rs\n");
    fixture.write(&repo, "first.ignore", b"*.rs\n");
    fixture.write(&repo, "second.ignore", b"!keep.rs\n");
    fixture.write(&repo, "keep.rs", b"");
    fixture.write(&repo, "drop.rs", b"");
    fixture.write(&repo, "visible.rs", b"");
    for threads in ["1", "4"] {
        let output = fixture.run_branchcut(
            &repo,
            &[
                "--glob",
                "**/*.rs",
                "--ignore-file",
                "first.ignore",
                "--ignore-file",
                "second.ignore",
                "--sort",
                "--threads",
                threads,
            ],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "keep.rs");
    }
    let why = fixture.run_branchcut(
        &repo,
        &[
            "--ignore-file",
            "first.ignore",
            "--ignore-file",
            "second.ignore",
            "--why",
            "keep.rs",
        ],
    );
    let why = String::from_utf8(why.stdout).unwrap();
    assert!(
        why.contains("MATCHED") && why.contains("second.ignore:1"),
        "{why}"
    );
}

#[test]
fn explicit_ignore_rules_override_nested_discovered_rules() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(&repo, "custom.ignore", b"!nested/keep.rs\nnested/drop.rs\n");
    fixture.write(&repo, "nested/.gitignore", b"keep.rs\n!drop.rs\n");
    fixture.write(&repo, "nested/keep.rs", b"");
    fixture.write(&repo, "nested/drop.rs", b"");
    for threads in ["1", "4"] {
        let output = fixture.run_branchcut(
            &repo,
            &[
                "--glob",
                "**/*.rs",
                "--gitignore",
                "--ignore-file",
                "custom.ignore",
                "--sort",
                "--threads",
                threads,
            ],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "nested/keep.rs"
        );
    }
}

#[test]
fn missing_explicit_ignore_file_is_reported_and_strict_mode_fails() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(&repo, "visible.rs", b"");
    for threads in ["1", "4"] {
        let output = fixture.run_branchcut(
            &repo,
            &[
                "--ignore-file",
                "missing.ignore",
                "--strict",
                "--threads",
                threads,
            ],
        );
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains("missing.ignore"));
    }
}

#[test]
fn malformed_git_boolean_is_not_silently_treated_as_false() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(&repo, "visible.rs", b"");
    let result = fixture.git(&repo, ["config", "core.ignoreCase", "maybe"]);
    assert!(result.status.success());
    let output = fixture.run_branchcut(&repo, &["--gitignore", "--strict"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid Git boolean"));
}

#[test]
fn numeric_git_boolean_controls_ignore_case_independently_of_query_case() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(&repo, ".gitignore", b"PRIVATE.RS\n");
    fixture.write(&repo, "private.rs", b"");
    fixture.write(&repo, "visible.rs", b"");
    assert!(
        fixture
            .git(&repo, ["config", "core.ignoreCase", "2"])
            .status
            .success()
    );
    assert_eq!(
        fixture
            .git(
                &repo,
                ["check-ignore", "--no-index", "--quiet", "private.rs"]
            )
            .status
            .code(),
        Some(0)
    );
    for threads in ["1", "4"] {
        let output = fixture.run_branchcut(
            &repo,
            &[
                "--gitignore",
                "--glob",
                "**/*.rs",
                "--case-sensitive",
                "--threads",
                threads,
            ],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "visible.rs"
        );
    }
}

#[test]
fn unconditional_config_include_is_relative_to_its_config_file() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    let excludes = fixture.root.join("rules");
    fs::write(&excludes, b"private.rs\n").unwrap();
    fs::write(
        fixture.home.join(".gitconfig"),
        b"[include]\npath = included.conf\n",
    )
    .unwrap();
    fs::write(
        fixture.home.join("included.conf"),
        format!(
            "[core]\nexcludesFile = {}\n",
            excludes.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();
    fixture.write(&repo, "private.rs", b"");
    fixture.write(&repo, "visible.rs", b"");
    fixture.assert_results_match_git(&repo, &repo, &["private.rs", "visible.rs"]);
}

#[test]
fn conditional_gitdir_and_onbranch_includes_match_git() {
    let fixture = Fixture::new();
    let repo = fs::canonicalize(fixture.init_repo("repo")).unwrap();
    let excludes = fixture.root.join("rules");
    fs::write(&excludes, b"private.rs\n").unwrap();
    fs::write(
        fixture.home.join("included.conf"),
        format!(
            "[core]\nexcludesFile = {}\n",
            excludes.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();
    fixture.write(&repo, "private.rs", b"");
    fixture.write(&repo, "visible.rs", b"");
    let gitdir_path = fs::canonicalize(repo.join(".git")).unwrap();
    let gitdir = gitdir_path.to_string_lossy().replace('\\', "/");
    let gitdir = gitdir.strip_prefix("//?/").unwrap_or(&gitdir);
    for condition in [
        format!("gitdir:{gitdir}"),
        format!("gitdir/i:{}", gitdir.to_uppercase()),
        "onbranch:feature/".to_owned(),
    ] {
        assert!(
            fixture
                .git(&repo, ["symbolic-ref", "HEAD", "refs/heads/feature/topic"])
                .status
                .success()
        );
        fs::write(
            fixture.home.join(".gitconfig"),
            format!("[includeIf \"{condition}\"]\npath = included.conf\n"),
        )
        .unwrap();
        assert!(
            fixture.ignored_by_git(&repo, "private.rs"),
            "condition did not match Git: {condition}"
        );
        fixture.assert_results_match_git(&repo, &repo, &["private.rs", "visible.rs"]);
    }
}

#[test]
fn empty_excludes_file_disables_the_default_global_ignore_file() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(&fixture.xdg, "git/ignore", b"*.rs\n");
    fixture.write(&repo, "visible.rs", b"");
    assert!(
        fixture
            .git(&repo, ["config", "core.excludesFile", ""])
            .status
            .success()
    );
    fixture.assert_results_match_git(&repo, &repo, &["visible.rs"]);
}

#[test]
fn global_worktree_extension_does_not_enable_repository_worktree_config() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(
        &fixture.home,
        ".gitconfig",
        b"[extensions]\nworktreeConfig = true\n",
    );
    let excludes = fixture.root.join("rules");
    fs::write(&excludes, b"*.rs\n").unwrap();
    fixture.write(
        &repo,
        ".git/config.worktree",
        format!(
            "[core]\nexcludesFile = {}\n",
            excludes.to_string_lossy().replace('\\', "/")
        )
        .as_bytes(),
    );
    fixture.write(&repo, "visible.rs", b"");
    fixture.assert_results_match_git(&repo, &repo, &["visible.rs"]);
}

#[test]
fn include_cycle_is_bounded_and_reported() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(
        &fixture.home,
        ".gitconfig",
        b"[include]\npath = .gitconfig\n",
    );
    fixture.write(&repo, "visible.rs", b"");
    let output = fixture.run_branchcut(&repo, &["--gitignore", "--strict"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("include depth"));
}

#[cfg(unix)]
#[test]
fn worktree_gitignore_symlink_is_not_followed() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(&repo, "actual.ignore", b"private.rs\n");
    fixture.write(&repo, "private.rs", b"");
    std::os::unix::fs::symlink("actual.ignore", repo.join(".gitignore")).unwrap();
    fixture.assert_results_match_git(&repo, &repo, &["private.rs"]);
}

#[cfg(unix)]
#[test]
fn non_utf8_ignore_patterns_preserve_native_filename_bytes() {
    use std::os::unix::ffi::OsStringExt;
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(&repo, ".gitignore", b"hidden-\xff.rs\n");
    if !native_fixture::write(
        repo.join(std::ffi::OsString::from_vec(b"hidden-\xff.rs".to_vec())),
        b"",
    ) {
        return;
    }
    fixture.write(&repo, "visible.rs", b"");
    for threads in ["1", "4"] {
        let output = fixture.run_branchcut(
            &repo,
            &[
                "--gitignore",
                "--glob",
                "**/*.rs",
                "--count",
                "--threads",
                threads,
            ],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "1");
    }
}

#[test]
fn relative_excludes_file_resolves_from_repository_root_for_nested_cwd() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(&repo, "shared.ignore", b"private.rs\n");
    assert!(
        fixture
            .git(&repo, ["config", "core.excludesFile", "shared.ignore"])
            .status
            .success()
    );
    fixture.write(&repo, "nested/private.rs", b"");
    fixture.write(&repo, "nested/visible.rs", b"");
    fixture.assert_results_match_git(
        &repo,
        &repo.join("nested"),
        &["nested/private.rs", "nested/visible.rs"],
    );
}

#[test]
fn combined_ignore_prioritizes_dot_ignore_above_nested_gitignore() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(&repo, ".ignore", b"nested/drop.rs\n!nested/keep.rs\n");
    fixture.write(&repo, "nested/.gitignore", b"!drop.rs\nkeep.rs\n");
    fixture.write(&repo, "nested/drop.rs", b"");
    fixture.write(&repo, "nested/keep.rs", b"");
    for threads in ["1", "4"] {
        let output = fixture.run_branchcut(
            &repo,
            &[
                "--ignore",
                "--glob",
                "**/*.rs",
                "--sort",
                "--threads",
                threads,
            ],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "nested/keep.rs"
        );
    }
    let narrowed = fixture.run_branchcut(&repo, &["--ignore", "--glob", "nested/drop.rs"]);
    assert!(narrowed.status.success());
    assert!(narrowed.stdout.is_empty());
    let why = fixture.run_branchcut(&repo, &["--ignore", "--why", "nested/drop.rs"]);
    assert!(why.status.success());
    let why = String::from_utf8(why.stdout).unwrap();
    assert!(
        why.contains("EXCLUDED") && why.contains(".ignore:1"),
        "{why}"
    );
    let only_git = fixture.run_branchcut(&repo, &["--gitignore", "--glob", "**/*.rs"]);
    assert_eq!(
        String::from_utf8(only_git.stdout).unwrap().trim(),
        "nested/drop.rs"
    );
}

#[test]
fn combined_ignore_reads_nested_dot_ignore_and_explicit_overrides() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(&repo, ".ignore", b"*.rs\n");
    fixture.write(&repo, "nested/.ignore", b"!keep.rs\n");
    fixture.write(
        &repo,
        "explicit.ignore",
        b"nested/keep.rs\n!nested/other.rs\n",
    );
    fixture.write(&repo, "nested/keep.rs", b"");
    fixture.write(&repo, "nested/other.rs", b"");
    for threads in ["1", "4"] {
        let output = fixture.run_branchcut(
            &repo,
            &[
                "--ignore",
                "--ignore-file",
                "explicit.ignore",
                "--glob",
                "**/*.rs",
                "--threads",
                threads,
            ],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "nested/other.rs"
        );
    }
}

#[test]
fn combined_ignore_inherits_dot_ignore_when_cwd_is_nested() {
    let fixture = Fixture::new();
    let repo = fixture.init_repo("repo");
    fixture.write(&repo, ".ignore", b"nested/drop.rs\n");
    fixture.write(&repo, "nested/drop.rs", b"");
    fixture.write(&repo, "nested/keep.rs", b"");
    for threads in ["1", "4"] {
        let output = fixture.run_branchcut(
            &repo.join("nested"),
            &["--ignore", "--glob", "**/*.rs", "--threads", threads],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "keep.rs");
    }
}
