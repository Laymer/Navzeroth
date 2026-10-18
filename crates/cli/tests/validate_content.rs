use std::{path::PathBuf, process::Command};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn run(input: &str, json: bool) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"));
    command
        .args(["validate-content", "--input"])
        .arg(root().join(input));
    if json {
        command.arg("--json");
    }
    command.output().unwrap()
}
#[test]
fn existing_reference_has_separate_gaps_and_deterministic_json() {
    let first = run("content/forever", true);
    assert!(first.status.success());
    assert_eq!(first.stdout, run("content/forever", true).stdout);
    let reports: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(reports[0]["passes"], true);
    assert!(!reports[0]["gaps"].as_array().unwrap().is_empty());
    assert!(reports[0]["errors"].as_array().unwrap().is_empty());
    let human = run("content/forever", false);
    assert!(String::from_utf8(human.stdout).unwrap().contains("gaps"));
}
#[test]
fn validation_errors_fail_without_echoing_content() {
    let output = run("fixtures/coverage/v1/predicate-dangling-quest.json", true);
    // This fixture exists and must fail semantic validation, not input handling.
    assert_eq!(output.status.code(), Some(1));
    assert!(serde_json::from_slice::<serde_json::Value>(&output.stdout).is_ok());
}
#[test]
fn bad_input_and_options_are_rejected() {
    assert_eq!(run("does-not-exist", false).status.code(), Some(1));
    let output = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .args(["validate-content", "--input", "x", "--json", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn selected_directory_shape_capacity_and_decode_failures_are_explicit() {
    let dir = std::env::temp_dir().join(format!(
        "nav-content-directory-boundary-{}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).unwrap();
    assert_eq!(
        azeroth_cli::content::reports(&dir).unwrap_err(),
        "No content datasets found"
    );
    std::fs::write(dir.join("datasets"), b"preserve").unwrap();
    assert_eq!(
        azeroth_cli::content::reports(&dir).unwrap_err(),
        "Content directory must be a regular directory"
    );
    std::fs::remove_file(dir.join("datasets")).unwrap();
    std::fs::create_dir(dir.join("directory.json")).unwrap();
    assert_eq!(
        azeroth_cli::content::reports(&dir).unwrap_err(),
        "Content file must be a regular file"
    );
    std::fs::remove_dir(dir.join("directory.json")).unwrap();
    std::fs::write(dir.join("broken.json"), b"private-sentinel invalid JSON").unwrap();
    let reports = azeroth_cli::content::reports(&dir).unwrap();
    assert!(!reports[0].passes);
    let human = azeroth_cli::content::human(&reports);
    assert!(human.contains("decode failed"));
    assert!(!human.contains("private-sentinel"));
    for i in 0..128 {
        std::fs::write(dir.join(format!("{i}.json")), b"{}").unwrap();
    }
    assert_eq!(
        azeroth_cli::content::reports(&dir).unwrap_err(),
        "Too many content files"
    );
    assert_eq!(
        std::fs::read(dir.join("broken.json")).unwrap(),
        b"private-sentinel invalid JSON"
    );
    std::fs::remove_dir_all(dir).unwrap();
    let invalid = run("fixtures/coverage/v1/predicate-dangling-quest.json", false);
    assert_eq!(invalid.status.code(), Some(1));
    assert!(
        String::from_utf8(invalid.stdout)
            .unwrap()
            .contains("  error:")
    );
}
#[cfg(unix)]
#[test]
fn symlinks_and_oversized_files_are_rejected() {
    use std::os::unix::fs::symlink;
    let dir = std::env::temp_dir().join(format!("navzeroth-content-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("large.json");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(azeroth_core::contract::MAX_DATASET_BYTES as u64 + 1)
        .unwrap();
    assert!(azeroth_cli::content::reports(&path).is_err());
    let link = dir.join("link.json");
    symlink(&path, &link).unwrap();
    assert!(azeroth_cli::content::reports(&link).is_err());
    std::fs::remove_file(link).unwrap();
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
