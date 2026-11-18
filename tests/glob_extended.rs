use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "branchcut-extglob-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        for name in ["a.rs", "aa.rs", "aaa.rs", "b.rs", "keep.rs", "other.txt"] {
            fs::write(root.join(name), b"").unwrap();
        }
        Self(root)
    }
    fn run(&self, args: &[&str]) -> Vec<String> {
        let output = Command::new(env!("CARGO_BIN_EXE_branchcut"))
            .arg("--cwd")
            .arg(&self.0)
            .args(args)
            .arg("--sort")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn extglobs_work_in_cli_shorthand_exclusions_and_parallel_queries() {
    let fixture = Fixture::new();
    assert_eq!(fixture.run(&["@(a|b).rs"]), ["a.rs", "b.rs"]);
    assert_eq!(fixture.run(&["+(a).rs"]), ["a.rs", "aa.rs", "aaa.rs"]);
    assert_eq!(
        fixture.run(&["--glob", "+(a).rs", "--threads", "2"]),
        ["a.rs", "aa.rs", "aaa.rs"]
    );
    assert_eq!(
        fixture.run(&["--glob", "*", "--exclude", "!(keep.rs)"]),
        ["keep.rs"]
    );
    assert_eq!(
        fixture.run(&["--glob", "@(A|B).RS", "--ignore-case"]),
        ["a.rs", "b.rs"]
    );
}

#[test]
fn negated_extglob_can_be_composed_with_a_suffix() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture.run(&["--glob", "!(a).rs"]),
        ["aa.rs", "aaa.rs", "b.rs", "keep.rs"]
    );
}
