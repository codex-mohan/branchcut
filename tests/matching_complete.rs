use branchcut::{MatchOptions, PathMatcher, Query, escape_path_pattern};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "branchcut-matching-complete-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        for name in [
            "file.rs",
            "filetest.rs",
            "filetests.rs",
            "foo.rs",
            "bar.rs",
            "baz.rs",
            "keep.rs",
        ] {
            fs::write(root.join(name), b"").unwrap();
        }
        Self(root)
    }

    fn run(&self, pattern: &str) -> Vec<String> {
        let query = Query {
            cwd: self.0.clone(),
            patterns: vec![pattern.to_owned()],
            ..Query::default()
        }
        .compile()
        .unwrap();
        let mut found = Vec::new();
        query
            .visit(&mut |path: &std::path::Path| {
                found.push(path.to_owned());
                Ok(std::ops::ControlFlow::Continue(()))
            })
            .unwrap();
        found.sort();
        found
            .into_iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn negated_extglobs_compose_with_prefix_suffix_and_nesting() {
    let fixture = Fixture::new();
    // `!(...)` excludes a matching substring at the point where the group
    // occurs, while adjacent pattern parts still have to match the component.
    assert_eq!(
        fixture.run("file!(s).rs"),
        ["file.rs", "filetest.rs", "filetests.rs"]
    );
    assert_eq!(
        fixture.run("!(foo).rs"),
        [
            "bar.rs",
            "baz.rs",
            "file.rs",
            "filetest.rs",
            "filetests.rs",
            "keep.rs"
        ]
    );
    assert_eq!(
        fixture.run("@(!(foo)|bar).rs"),
        [
            "bar.rs",
            "baz.rs",
            "file.rs",
            "filetest.rs",
            "filetests.rs",
            "keep.rs"
        ]
    );
    assert_eq!(
        fixture.run("!(foo|bar).rs"),
        [
            "baz.rs",
            "file.rs",
            "filetest.rs",
            "filetests.rs",
            "keep.rs"
        ]
    );
}

#[test]
fn escaped_path_helper_round_trips_braces_classes_and_metacharacters() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.0.join("literal/[class]/{cache}/(item)!+@")).unwrap();
    let target = fixture.0.join("literal/[class]/{cache}/(item)!+@/file.rs");
    fs::write(&target, b"").unwrap();
    let relative = target.strip_prefix(&fixture.0).unwrap();
    let escaped = escape_path_pattern(relative).unwrap();
    let matcher = PathMatcher::with_options(
        &[escaped],
        MatchOptions {
            escaped_patterns: true,
            ..MatchOptions::default()
        },
    )
    .unwrap();
    assert!(matcher.is_match(relative));
    assert!(!matcher.is_match("literal/class/{cache}/(item)!+@/file.rs"));
}

#[test]
fn escaped_patterns_keep_globs_while_quoting_selected_characters() {
    let matcher = PathMatcher::with_options(
        &[r"data/\[literal\]/*.rs"],
        MatchOptions {
            escaped_patterns: true,
            ..MatchOptions::default()
        },
    )
    .unwrap();
    assert!(matcher.is_match("data/[literal]/one.rs"));
    assert!(!matcher.is_match("data/xliteralx/one.rs"));
    assert!(!matcher.is_match("data/[literal]/one.txt"));
}
