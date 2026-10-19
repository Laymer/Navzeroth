//! The packaging command validates signed resources without activating a database.
#[allow(dead_code)]
#[path = "../../app/tests/support/bundled_content_fixture.rs"]
mod support;
use azeroth_cli::bundled_content::execute;
use azeroth_local_data::bundled_receipts as receipts;
use std::{ffi::OsString, fs, process::Command};
use support::Fixture;
#[test]
fn packaging_command_checks_explicit_build_identity_and_exact_signed_resources() {
    let f = Fixture::new();
    let identity = f.root.with_extension("identity.json");
    let policy = f.root.with_extension("policy.json");
    fs::write(&identity, receipts::encode(&f.payload).unwrap()).unwrap();
    fs::write(&policy, receipts::encode(&f.policy).unwrap()).unwrap();
    let args = vec![
        OsString::from("--input"),
        f.root.clone().into_os_string(),
        OsString::from("--identity"),
        identity.clone().into_os_string(),
        OsString::from("--policy"),
        policy.clone().into_os_string(),
    ];
    let output = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .arg("check-bundled-content")
        .args(&args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["verified"], true);
    assert_eq!(json["verification"], "signed-resource-closure");
    assert_eq!(json["app_version"], "1.0.0");
    assert!(
        execute(&args[..4]).is_err(),
        "compiled production keys cannot trust synthetic receipt"
    );
    fs::write(f.root.join("review.json"), b"altered").unwrap();
    assert!(execute(&args).is_err());
    let failed = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .arg("check-bundled-content")
        .args(&args)
        .output()
        .unwrap();
    assert!(!failed.status.success());
    for malformed in [
        vec![],
        vec![OsString::from("--input")],
        vec![
            OsString::from("--input"),
            OsString::from("."),
            OsString::from("--input"),
            OsString::from("."),
        ],
        vec![
            OsString::from("--unknown"),
            OsString::from("."),
            OsString::from("--identity"),
            OsString::from("."),
        ],
    ] {
        assert!(execute(&malformed).is_err());
    }
    fs::remove_file(identity).unwrap();
    fs::remove_file(policy).unwrap();
}
