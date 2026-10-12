//! `validate-fixtures`: success and failure paths of the compiled binary.
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
}
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("azeroth-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
    }
}
fn run_with_root(root: &Path) -> Output {
    cli()
        .arg("validate-fixtures")
        .env("AZEROTH_REPO_ROOT", root)
        .output()
        .unwrap()
}

#[test]
fn validates_both_manifests_and_prints_only_counts() {
    let output = cli().arg("validate-fixtures").output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty(), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert!(
        lines[0].starts_with("fixtures/contracts/v1: ") && lines[0].contains("fixtures checked")
    );
    assert!(lines[1].starts_with("fixtures/transport: ") && lines[1].contains("fixtures checked"));
    // No fixture content, file names or paths are echoed.
    for echoed in [
        "azeroth-navigator",
        "demo-",
        ".json",
        ".lua",
        ".txt",
        "ANAV1",
        env!("CARGO_MANIFEST_DIR"),
    ] {
        assert!(!stdout.contains(echoed), "{echoed} in {stdout}");
    }
}

#[test]
fn does_not_depend_on_the_working_directory() {
    let elsewhere = scratch("cwd");
    let output = cli()
        .arg("validate-fixtures")
        .current_dir(&elsewhere)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&elsewhere).unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
}

#[test]
fn the_root_can_be_overridden_and_only_the_two_manifests_are_read() {
    let root = scratch("override");
    copy_dir(
        &repo_root().join("fixtures/contracts/v1"),
        &root.join("fixtures/contracts/v1"),
    );
    copy_dir(
        &repo_root().join("fixtures/transport"),
        &root.join("fixtures/transport"),
    );
    // A third fixture area is never visited, even when it is broken.
    std::fs::create_dir_all(root.join("fixtures/other")).unwrap();
    std::fs::write(root.join("fixtures/other/manifest.json"), "not json").unwrap();
    let output = run_with_root(&root);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_missing_fixture_tree_fails_safely() {
    let root = scratch("empty");
    let output = run_with_root(&root);
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = text(&output.stderr);
    assert!(stderr.contains("fixtures/contracts/v1: manifest cannot be read"));
    assert!(stderr.contains("fixtures/transport: manifest cannot be read"));
    assert!(stderr.contains("validate-fixtures failed"));
}

#[test]
fn a_mismatch_fails_with_the_file_name_but_never_its_content() {
    let root = scratch("mismatch");
    copy_dir(
        &repo_root().join("fixtures/contracts/v1"),
        &root.join("fixtures/contracts/v1"),
    );
    copy_dir(
        &repo_root().join("fixtures/transport"),
        &root.join("fixtures/transport"),
    );
    let secret = "SENTINEL-private-fixture-content";
    // An unlisted file with secret content, and a valid transport file replaced by one.
    std::fs::write(root.join("fixtures/contracts/v1/extra.json"), secret).unwrap();
    std::fs::write(
        root.join("fixtures/transport/anav1-character-snapshot.txt"),
        format!("ANAV1:{secret}"),
    )
    .unwrap();
    let output = run_with_root(&root);
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("fixtures/contracts/v1: extra.json: file exists but is not listed"),
        "{stderr}"
    );
    assert!(
        stderr.contains(
            "fixtures/transport: anav1-character-snapshot.txt: rejected but declared valid"
        ),
        "{stderr}"
    );
    assert!(!stderr.contains(secret) && !text(&output.stdout).contains(secret));
    // Both manifests fail here, so nothing is reported as passing.
    assert!(output.stdout.is_empty());
}

#[test]
fn extra_arguments_are_a_usage_error_without_echo() {
    for extra in [
        vec!["extra-sentinel"],
        vec!["--help"],
        vec!["--output", "dir-sentinel"],
    ] {
        let output = cli()
            .arg("validate-fixtures")
            .args(&extra)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let stderr = text(&output.stderr);
        assert!(stderr.contains("Unsupported"));
        assert!(!stderr.contains("sentinel") && !stderr.contains("--help"));
    }
}
