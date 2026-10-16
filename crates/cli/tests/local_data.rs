use std::{
    fs,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn unsupported_duplicate_and_missing_options_fail_as_json_without_echoing_inputs() {
    for args in [
        vec!["local-data"],
        vec!["local-data", "extract"],
        vec!["local-data", "inspect", "--installation", "private-input"],
        vec!["local-data", "inspect", "--product", "a", "--product", "b"],
        vec!["local-data", "inspect", "--installation", "--product"],
        vec!["local-data", "plan", "--preset", "unlimited"],
        vec!["local-data", "inspect", "--destination", "private-input"],
    ] {
        let output = cli(&args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stderr.is_empty());
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["error"], "usage");
        assert!(
            !String::from_utf8(output.stdout)
                .unwrap()
                .contains("private-input")
        );
    }
}

#[test]
fn inspect_and_plan_exit_codes_preserve_unknown_and_unavailable_capabilities() {
    let root = std::env::temp_dir().join(format!(
        "navzeroth-local-cli-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    let client = root.join("client");
    let output = root.join("output");
    fs::create_dir(&client).unwrap();
    fs::create_dir(&output).unwrap();
    let info = format!(
        "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70425|{}|{}|enUS text\n",
        "a".repeat(32),
        "b".repeat(32)
    );
    fs::write(client.join(".build.info"), info).unwrap();
    let selected = client.to_str().unwrap();
    let dest = output.to_str().unwrap();
    let inspected = cli(&[
        "local-data",
        "inspect",
        "--installation",
        selected,
        "--product",
        "wow_classic_beta",
    ]);
    assert!(inspected.status.success());
    let value: serde_json::Value = serde_json::from_slice(&inspected.stdout).unwrap();
    assert_eq!(value["metadata"]["build"], 70425);
    let planned = cli(&[
        "local-data",
        "plan",
        "--product",
        "wow_classic_beta",
        "--installation",
        selected,
        "--destination",
        dest,
    ]);
    assert_eq!(planned.status.code(), Some(3));
    let value: serde_json::Value = serde_json::from_slice(&planned.stdout).unwrap();
    assert_eq!(value["preset"], "max");
    assert_eq!(value["extraction_ready"], false);
    assert!(value["required_storage_bytes"].is_null());
    assert_eq!(value["actual_workers"], 0);
    assert_eq!(fs::read_dir(output).unwrap().count(), 0);
    let bad = cli(&[
        "local-data",
        "inspect",
        "--product",
        "missing",
        "--installation",
        selected,
    ]);
    assert_eq!(bad.status.code(), Some(1));
    assert!(!String::from_utf8(bad.stdout).unwrap().contains(selected));
    fs::remove_dir_all(root).unwrap();
}
