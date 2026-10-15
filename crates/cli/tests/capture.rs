use serde_json::{Value, json};
use std::{path::PathBuf, process::Command};

struct Temp(PathBuf);
impl Temp {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("anav-capture-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn diagnostic() -> Value {
    json!({"format":"anav-diagnostic", "version":1,"addon_version":"0.8.1",
        "build":{"version":"1.60.1","build":"70374","interface":"16001", "edition":"wow_forever"},
        "capabilities":[{"status":"untested","detail":"Private-Name"},{"status":"failed"}],
        "private":"Private-Name"})
}
fn saved(value: &str) -> String {
    // JSON quotes/backslash escapes are in the accepted Lua string subset.
    format!(
        "AzerothNavigatorDB = {{ diagnostic_export = {}, session_export = 'Private-Name', private = 'Private-Name' }}",
        serde_json::to_string(value).unwrap()
    )
}
fn run(path: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .args(["inspect-capture", "--input"])
        .arg(path)
        .output()
        .unwrap()
}
#[test]
fn allowlist_never_echoes_raw_exports_paths_details_or_unknown_fields() {
    let temp = Temp::new("Private-Name");
    let path = temp.0.join("Private-Name.lua");
    let original = saved(&diagnostic().to_string());
    std::fs::write(&path, &original).unwrap();
    let result = run(&path);
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    assert!(!String::from_utf8_lossy(&result.stdout).contains("Private-Name"));
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["diagnostic"]["target_tuple_matches"], true);
    assert_eq!(report["diagnostic"]["untested_capabilities"], 1);
    assert_eq!(report["diagnostic"]["failed_capabilities"], 1);
    assert_eq!(report["snapshot_export_present"], false);
    assert_eq!(report["session_export_present"], true);
    assert_eq!(report["live_acceptance_verified"], false);
    assert_eq!(report["installed_revision_verified"], false);
    assert_eq!(std::fs::read_to_string(path).unwrap(), original);
}
#[test]
fn mismatched_unknown_build_and_missing_exports_remain_explicit() {
    let mut value = diagnostic();
    value["build"]["build"] = json!("70338");
    let report = azeroth_cli::capture::inspect(&saved(&value.to_string())).unwrap();
    assert!(!report.diagnostic.unwrap().target_tuple_matches);
    value["build"]["build"] = json!("unknown");
    let report = azeroth_cli::capture::inspect(&saved(&value.to_string())).unwrap();
    assert_eq!(report.diagnostic.unwrap().client_build, "unknown");
    let report = azeroth_cli::capture::inspect("AzerothNavigatorDB = {}").unwrap();
    assert!(report.diagnostic.is_none());
    assert!(!report.snapshot_export_present);
    assert!(!report.session_export_present);
}
#[test]
fn malformed_untrusted_inputs_fail_without_echoing_contents() {
    let temp = Temp::new("invalid");
    let path = temp.0.join("Private-Name.lua");
    let mut cases = vec![
        "AzerothNavigatorDB = loadstring('Private-Name')()".into(),
        "AzerothNavigatorDB = {export='x', export='Private-Name'}".into(),
        saved("{\"version\":1,\"version\":1,\"private\":\"Private-Name\"}"),
        saved(&"Private-Name".repeat(1300)),
        "AzerothNavigatorDB = {snapshot_export = false}".into(),
    ];
    for (key, bad) in [
        ("version", json!(2)),
        ("addon_version", json!("Private-Name")),
    ] {
        let mut value = diagnostic();
        value[key] = bad;
        cases.push(saved(&value.to_string()));
    }
    let mut value = diagnostic();
    value["capabilities"][0]["status"] = json!("verified available");
    cases.push(saved(&value.to_string()));
    for case in cases {
        std::fs::write(&path, case).unwrap();
        let result = run(&path);
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("Private-Name"));
    }
}
#[test]
fn oversized_files_directories_and_extra_arguments_refuse() {
    let temp = Temp::new("bounds");
    let path = temp.0.join("large.lua");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(azeroth_ingest::savedvars::MAX_FILE_BYTES as u64 + 1)
        .unwrap();
    assert!(!run(&path).status.success());
    assert!(!run(&temp.0).status.success());
    assert!(
        !Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
            .args(["inspect-capture", "--input", "none", "--execute"])
            .output()
            .unwrap()
            .status
            .success()
    );
}
#[cfg(unix)]
#[test]
fn linked_file_and_linked_parent_refuse() {
    let temp = Temp::new("links");
    let real = temp.0.join("real");
    std::fs::create_dir(&real).unwrap();
    let file = real.join("capture.lua");
    std::fs::write(&file, "AzerothNavigatorDB = {}").unwrap();
    let linked_file = temp.0.join("link.lua");
    std::os::unix::fs::symlink(&file, &linked_file).unwrap();
    assert!(!run(&linked_file).status.success());
    let linked_dir = temp.0.join("link");
    std::os::unix::fs::symlink(real, &linked_dir).unwrap();
    assert!(!run(&linked_dir.join("capture.lua")).status.success());
}

#[test]
fn actual_snapshot_contract_is_validated_without_exposing_identities() {
    let json = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
    let text = format!(
        "AzerothNavigatorDB = {{ snapshot_export = {} }}",
        serde_json::to_string(&format!("ANAV1:{json}")).unwrap()
    );
    let report = azeroth_cli::capture::inspect(&text).unwrap();
    let summary = report.snapshot.unwrap();
    assert!(summary.fixture);
    assert_eq!(summary.level, Some(22));
    assert!(!summary.target_tuple_matches);
    assert!(
        azeroth_cli::capture::inspect("AzerothNavigatorDB = {snapshot_export='Private-Name'}")
            .is_err()
    );
}
