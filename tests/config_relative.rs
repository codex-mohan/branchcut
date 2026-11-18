use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "branchcut config-relative-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create fixture root");
        Self(root)
    }

    fn write(&self, path: impl AsRef<Path>, contents: impl AsRef<[u8]>) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().expect("fixture path has parent"))
            .expect("create fixture parent");
        fs::write(path, contents).expect("write fixture file");
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_branchcut"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .expect("run Branchcut")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn lines(output: &Output) -> Vec<&str> {
    std::str::from_utf8(&output.stdout)
        .expect("fixture paths are UTF-8")
        .lines()
        .collect()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn relative_config_paths_keep_config_root_for_default_profile_and_ignore_file() {
    let fixture = Fixture::new();
    fixture.write(
        "config/queries",
        "[default]\nglob **/*.rs\nexclude profile-root/**\nignore-file .branchcut-ignore\n\
         [profile]\ncwd profile-root\nglob **/*.rs\nignore-file .branchcut-ignore\n",
    );
    fixture.write("config/.branchcut-ignore", "blocked/\n");
    fixture.write("config/kept.rs", b"");
    fixture.write("config/blocked/hidden.rs", b"");
    fixture.write("config/profile-root/kept.rs", b"");
    fixture.write("config/profile-root/blocked/hidden.rs", b"");
    fixture.write("cli-root/kept.rs", b"");
    fixture.write("cli-root/blocked/hidden.rs", b"");

    let default = fixture.run(&["--config", "config/queries", "--sort"]);
    assert_success(&default);
    assert_eq!(lines(&default), ["kept.rs"]);

    let profile = fixture.run(&[
        "--config",
        "config/queries",
        "--profile",
        "profile",
        "--sort",
    ]);
    assert_success(&profile);
    assert_eq!(lines(&profile), ["kept.rs"]);

    let cli_cwd = fixture.run(&[
        "--config",
        "config/queries",
        "--profile",
        "profile",
        "--cwd",
        "cli-root",
        "--sort",
    ]);
    assert_success(&cli_cwd);
    assert_eq!(lines(&cli_cwd), ["kept.rs"]);
}
