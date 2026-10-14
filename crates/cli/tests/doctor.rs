use serde_json::Value;
use std::{path::PathBuf, process::Command};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
}
struct Temp(PathBuf);
impl Temp {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("azeroth-doctor-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn default_diagnostics_are_bounded_and_do_not_claim_version_or_live_verification() {
    let output = cli()
        .args(["doctor", "--json"])
        .env_remove("PATH")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["report_version"], 1);
    assert_eq!(report["app_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(report["development_tools"].as_array().unwrap().len(), 4);
    for tool in report["development_tools"].as_array().unwrap() {
        assert_eq!(tool["available_on_path"], false);
        assert_eq!(tool["runtime_version_verified"], false);
        assert!(!tool["development_pin"].as_str().unwrap().is_empty());
    }
    for path in report["selected_paths"].as_array().unwrap() {
        assert_eq!(path["state"], "not_selected");
        assert_eq!(path["content_verified"], false);
        assert_eq!(path.as_object().unwrap().len(), 3);
    }
    assert_eq!(report["native_prerequisites_verified"], false);
    assert_eq!(report["live_client_verified"], false);
    assert!(output.stdout.len() < 4096);
}
#[test]
fn explicitly_selected_paths_check_only_kind_without_echoing_names_or_contents() {
    let dir = Temp::new("selected");
    let file = dir.0.join("private-account-sentinel.lua");
    // Content must never be evaluated, parsed or printed, regardless of validity.
    std::fs::write(&file, b"error('private-content-sentinel')").unwrap();
    let output = cli()
        .args(["doctor", "--json", "--saved-variables"])
        .arg(&file)
        .arg("--addons")
        .arg(&dir.0)
        .arg("--content")
        .arg(&dir.0)
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(!text.contains("sentinel") && !text.contains(&dir.0.to_string_lossy().to_string()));
    let report: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(report["selected_paths"][0]["state"], "regular_file");
    assert_eq!(report["selected_paths"][1]["state"], "directory");
    assert_eq!(report["selected_paths"][2]["state"], "directory");
    assert_eq!(
        std::fs::read(&file).unwrap(),
        b"error('private-content-sentinel')"
    );
}
#[test]
fn selected_missing_or_wrong_kind_returns_nonzero_without_input_echo() {
    let dir = Temp::new("refusals");
    let file = dir.0.join("private-file-sentinel");
    std::fs::write(&file, b"unchanged").unwrap();
    for (flag, path, state) in [
        ("--saved-variables", dir.0.clone(), "wrong_kind"),
        ("--addons", file.clone(), "wrong_kind"),
        (
            "--content",
            dir.0.join("private-missing-sentinel"),
            "missing",
        ),
    ] {
        let output = cli()
            .args(["doctor", "--json", flag])
            .arg(&path)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(!text.contains("sentinel"));
        let report: Value = serde_json::from_str(&text).unwrap();
        assert!(
            report["selected_paths"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["state"] == state)
        );
        assert!(output.stderr.is_empty());
    }
}
#[test]
fn malformed_options_fail_without_echoing_inputs() {
    for args in [
        &["doctor", "--json", "--json"][..],
        &["doctor", "--addons"],
        &[
            "doctor",
            "--addons",
            "private-sentinel",
            "--addons",
            "another-sentinel",
        ],
        &["doctor", "--saved-variables", "--json"],
        &["doctor", "--private-sentinel"],
        &["doctor", "private-sentinel"],
        &["doctor", "--content", ""],
    ] {
        let output = cli().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(
            !String::from_utf8(output.stderr)
                .unwrap()
                .contains("sentinel")
        );
    }
}
#[cfg(unix)]
#[test]
fn selected_symlink_is_rejected_and_path_presence_never_executes_tools() {
    use std::os::unix::{fs::PermissionsExt, fs::symlink};
    let dir = Temp::new("symlink-tools");
    let tool = dir.0.join("node");
    let touched = dir.0.join("executed-sentinel");
    std::fs::write(&tool, format!("#!/bin/sh\ntouch '{}'\n", touched.display())).unwrap();
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o700)).unwrap();
    let link = dir.0.join("private-link-sentinel");
    symlink(&tool, &link).unwrap();
    let output = cli()
        .args(["doctor", "--json", "--saved-variables"])
        .arg(&link)
        .env("PATH", &dir.0)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["selected_paths"][0]["state"], "symlink_rejected");
    let node = report["development_tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["tool"] == "node")
        .unwrap();
    assert_eq!(node["available_on_path"], true);
    assert_eq!(node["runtime_version_verified"], false);
    assert!(!touched.exists());
}
