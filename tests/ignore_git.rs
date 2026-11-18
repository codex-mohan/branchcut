use branchcut::Query;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "branchcut-git-ignore-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn write(&self, path: &str, contents: &[u8]) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn contents_globstar_does_not_ignore_its_parent_directory() {
    let fixture = Fixture::new();
    fixture.write(".gitignore", b"generated/**\n!generated/keep.rs\n");
    fixture.write("generated/keep.rs", b"");
    fixture.write("generated/discard.rs", b"");
    fixture.write("generated/deep/discard.rs", b"");
    let query = Query {
        cwd: fixture.0.clone(),
        gitignore: true,
        ..Query::default()
    }
    .compile()
    .unwrap();
    let mut paths = Vec::new();
    query
        .visit(&mut |path: &Path| {
            paths.push(path.to_owned());
            Ok(std::ops::ControlFlow::Continue(()))
        })
        .unwrap();
    assert_eq!(paths, [PathBuf::from("generated/keep.rs")]);
}

#[test]
fn supported_ignore_decisions_agree_with_local_git() {
    if Command::new("git").arg("--version").output().is_err() {
        eprintln!("git unavailable; independent ignore oracle skipped");
        return;
    }
    let fixture = Fixture::new();
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(&fixture.0)
            .status()
            .unwrap()
            .success()
    );
    for file in [
        "visible.rs",
        "#literal",
        "!literal",
        "space name",
        "target/drop.rs",
        "nested/target/drop.rs",
        "generated/keep.rs",
        "generated/drop.rs",
        "blocked/keep.rs",
        "open/keep.rs",
        "open/drop.rs",
        "nested/root-only/keep.rs",
        "root-only/drop.rs",
        "nested/special.tmp",
        "nested/keep.tmp",
    ] {
        fixture.write(file, b"");
    }
    fixture.write(".gitignore", b"target/\n/root-only/\n\\#literal\n\\!literal\nspace\\ name\ngenerated/**\n!generated/keep.rs\nblocked/\n!blocked/keep.rs\nopen/\n!open/\nopen/*\n!open/keep.rs\n");
    fixture.write("nested/.gitignore", b"*.tmp\n!keep.tmp\n");
    fixture.write("wild/nested/keep.rs", b"");
    fixture.write("wild/nested/drop.rs", b"");
    fixture.write("wild/nested/.gitignore", b"**\n!keep.rs\n");
    let nested = Query {
        cwd: fixture.0.clone(),
        gitignore: true,
        patterns: vec!["wild/**/*.rs".into()],
        ..Query::default()
    }
    .compile()
    .unwrap();
    for workers in [1, 4] {
        let mut paths = Vec::new();
        let mut visitor = |path: &Path| {
            paths.push(path.to_owned());
            Ok(std::ops::ControlFlow::Continue(()))
        };
        if workers == 1 {
            nested.visit(&mut visitor)
        } else {
            nested.visit_parallel(workers, true, &mut visitor)
        }
        .unwrap();
        assert_eq!(paths, [PathBuf::from("wild/nested/keep.rs")]);
    }
    let query = Query {
        cwd: fixture.0.clone(),
        gitignore: true,
        hidden: true,
        ..Query::default()
    }
    .compile()
    .unwrap();
    for file in [
        "visible.rs",
        "#literal",
        "!literal",
        "space name",
        "target/drop.rs",
        "nested/target/drop.rs",
        "generated/keep.rs",
        "generated/drop.rs",
        "blocked/keep.rs",
        "open/keep.rs",
        "open/drop.rs",
        "nested/root-only/keep.rs",
        "root-only/drop.rs",
        "nested/special.tmp",
        "nested/keep.tmp",
        "wild/nested/keep.rs",
        "wild/nested/drop.rs",
    ] {
        let result = Command::new("git")
            .current_dir(&fixture.0)
            .args([
                "-c",
                "core.ignorecase=false",
                "check-ignore",
                "--no-index",
                "--quiet",
                "--",
                file,
            ])
            .status()
            .unwrap();
        assert!(
            matches!(result.code(), Some(0 | 1)),
            "git failed for {file}"
        );
        let explanation = query.explain_path(file).unwrap();
        assert_eq!(
            explanation.selected,
            result.code() == Some(1),
            "Git disagrees for {file}: {explanation:?}"
        );
    }
}
