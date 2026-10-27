use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(windows)]
use std::ffi::OsString;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "branchcut-native-paths-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create native path test fixture root");
        Self(root)
    }

    fn write(&self, relative: impl AsRef<Path>) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().expect("fixture path has a parent"))
            .expect("create fixture parent");
        fs::write(path, b"").expect("write fixture file");
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_branchcut"))
            .arg("--cwd")
            .arg(&self.0)
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

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "branchcut failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
#[test]
fn plain_output_preserves_non_utf8_filename_bytes() {
    use std::os::unix::ffi::OsStringExt;

    let fixture = Fixture::new();
    let name = OsString::from_vec(b"bad-\xff.rs".to_vec());
    fixture.write(&name);

    let output = fixture.run(&["--glob", "**/*.rs", "--sort"]);

    assert_success(&output);
    assert_eq!(output.stdout, b"bad-\xff.rs\n");
}

#[cfg(unix)]
#[test]
fn json_output_includes_reversible_bytes_for_non_utf8_filename() {
    use std::os::unix::ffi::OsStringExt;

    let fixture = Fixture::new();
    let name = OsString::from_vec(b"bad-\xff.rs".to_vec());
    fixture.write(&name);

    let output = fixture.run(&["--glob", "**/*.rs", "--json", "--sort"]);

    assert_success(&output);
    let json = std::str::from_utf8(&output.stdout).expect("JSON output is UTF-8");
    assert!(json.contains("\u{fffd}"), "{json}");
    assert!(json.contains("\"path_bytes\":\"626164ff2e7273\""), "{json}");
}

#[cfg(unix)]
#[test]
fn plain_and_json_output_preserve_unix_backslashes() {
    let fixture = Fixture::new();
    fixture.write("back\\slash.rs");

    let plain = fixture.run(&["--glob", "**/*.rs", "--sort"]);
    let json = fixture.run(&["--glob", "**/*.rs", "--json", "--sort"]);

    assert_success(&plain);
    assert_success(&json);
    assert_eq!(plain.stdout, b"back\\slash.rs\n");
    assert_eq!(json.stdout, "{\"path\":\"back\\\\slash.rs\"}\n".as_bytes());
}

#[cfg(unix)]
#[test]
fn literal_prefix_symlink_root_is_not_traversed() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    fixture.write("real/target.rs");
    symlink(fixture.0.join("real"), fixture.0.join("alias"))
        .expect("create directory symlink for Unix integration test");

    let output = fixture.run(&["--glob", "alias/*.rs", "--sort"]);

    assert_success(&output);
    assert!(output.stdout.is_empty());
}

#[cfg(windows)]
#[test]
fn windows_json_output_includes_utf16_for_unpaired_surrogate_filename() {
    use std::os::windows::ffi::OsStringExt;

    let fixture = Fixture::new();
    let filename_units = [
        b'b' as u16,
        b'a' as u16,
        b'd' as u16,
        b'-' as u16,
        0xd800,
        b'.' as u16,
        b'r' as u16,
        b's' as u16,
    ];
    let name = OsString::from_wide(&filename_units);
    fixture.write(&name);

    let output = fixture.run(&["--glob", "**/*.rs", "--json", "--sort"]);

    assert_success(&output);
    let json = std::str::from_utf8(&output.stdout).expect("JSON output is UTF-8");
    let marker = "\"path_utf16\":[";
    let start = json.find(marker).expect("lossless UTF-16 path field") + marker.len();
    let end = json[start..]
        .find(']')
        .expect("UTF-16 path array terminator")
        + start;
    let actual = json[start..end]
        .split(',')
        .map(|unit| {
            unit.trim()
                .parse::<u16>()
                .expect("numeric UTF-16 code unit")
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, filename_units);
}
