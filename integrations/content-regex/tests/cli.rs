use std::{path::Path, process::Command};

#[test]
fn integration_selects_regex_only_when_requested() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for (regex, expected) in [
        (false, "sample.txt:3:8:prefix CODE=[0-9]+\n"),
        (true, "sample.txt:1:8:prefix CODE=123\n"),
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_branchcut-regex"));
        command.arg("--cwd").arg(&fixtures).args([
            "--glob",
            "sample.txt",
            "--content",
            "CODE=[0-9]+",
        ]);
        if regex {
            command.arg("--regex");
        }

        let result = command.output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);
    }
}

#[test]
fn regex_subcommands_share_the_pattern_and_input_order() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    for args in [
        &["find", "-E", r"^sam.*[.]txt$", "fixtures"][..],
        &["find", "--full-path", "-E", r"^fixtures/sam.*[.]txt$"][..],
        &["find", "-E", r"^sam.*[.]txt$", "fixtures/sample.txt"][..],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_branchcut-regex"))
            .current_dir(&fixtures)
            .args(args)
            .output()
            .unwrap();
        assert!(result.status.success(), "{:?}", result);
        assert_eq!(
            String::from_utf8(result.stdout).unwrap(),
            "fixtures/sample.txt\n",
            "{args:?}"
        );
    }
    let result = Command::new(env!("CARGO_BIN_EXE_branchcut-regex"))
        .current_dir(&fixtures)
        .args(["grep", "-E", "CODE=[0-9]+", "fixtures/sample.txt"])
        .output()
        .unwrap();
    assert!(result.status.success(), "{:?}", result);
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "fixtures/sample.txt:1:8:prefix CODE=123\n"
    );
}
