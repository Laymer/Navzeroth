//! Exercise the public bounded enumeration command and its refusal behavior.
use std::{fs, path::Path, process::Command};
#[test]
fn command_enumerates_verified_content_and_preserves_existing_output() {
    let root = std::env::temp_dir().join(format!("nav-enum-command-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("release/v1")).unwrap();
    let repo = fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
    for name in ["accepted-origins.json", "world-data-target.json"] {
        fs::copy(
            repo.join("release/v1").join(name),
            root.join("release/v1").join(name),
        )
        .unwrap();
    }
    azeroth_cli::pack::v2::build(&repo.join("content/authored/slice-v1"), &root.join("pack"))
        .unwrap();
    let call = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
            .args(args)
            .output()
            .unwrap()
    };
    assert_eq!(call(&["enumerate-release-packs"]).status.code(), Some(2));
    let args = [
        "enumerate-release-packs",
        "--root",
        root.to_str().unwrap(),
        "--pack",
        "pack",
        "--output",
        "records.json",
    ];
    let success = call(&args);
    assert!(
        success.status.success(),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let summary: serde_json::Value = serde_json::from_slice(&success.stdout).unwrap();
    assert_eq!(summary["missing_provenance"], 0);
    assert!(
        summary["records_by_domain"]["class_roles"]
            .as_u64()
            .unwrap()
            > 0
    );
    let original = fs::read(root.join("records.json")).unwrap();
    assert_eq!(call(&args).status.code(), Some(1));
    assert_eq!(fs::read(root.join("records.json")).unwrap(), original);
    fs::remove_file(root.join("records.json")).unwrap();
    fs::write(root.join("pack/undeclared.json"), b"{}").unwrap();
    assert_eq!(call(&args).status.code(), Some(1));
    assert!(!root.join("records.json").exists());
    fs::remove_dir_all(root).unwrap();
}
