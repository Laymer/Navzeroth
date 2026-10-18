use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
}

#[test]
fn no_arguments_reports_current_capabilities() {
    let output = cli().output().unwrap();
    assert!(output.status.success());
    let banner = String::from_utf8(output.stdout).unwrap();
    assert!(banner.contains("export-schemas") && banner.contains("prepare-atlas"));
    assert!(banner.contains("validate-fixtures"));
}

#[test]
fn human_doctor_keeps_native_and_live_checks_unverified() {
    let output = cli().arg("doctor").output().unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("No selected file contents read"));
    assert!(text.contains("Native prerequisites and live client unverified"));
}

#[test]
fn schema_write_failure_and_bad_inventory_arguments_have_distinct_exit_codes() {
    let file = std::env::temp_dir().join(format!("nav-schema-refusal-{}", std::process::id()));
    std::fs::write(&file, b"preserve existing file").unwrap();
    let output = cli()
        .args(["export-schemas", "--output"])
        .arg(&file)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("Could not write the schema file")
    );
    assert_eq!(std::fs::read(&file).unwrap(), b"preserve existing file");
    std::fs::remove_file(file).unwrap();
    let output = cli().arg("inventory-report").output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn unsupported_commands_and_options_fail_without_echoing_input() {
    for args in [
        &["validate-fixtures", "extra-sentinel"][..],
        &["validate-fixtures", "--output", "dir-sentinel"],
        &["validate-fixtures", ""],
        &["--help"],
        &["private-input-sentinel"],
        &["export-schemas"],
        &["export-schemas", "--private-input-sentinel", "dir"],
        &["export-schemas", "--output", "dir", "extra-sentinel"],
        &["prepare-atlas"],
        &["prepare-atlas", "--manifest"],
        &[
            "prepare-atlas",
            "--manifest",
            "m-sentinel",
            "--content-root",
        ],
        &[
            "prepare-atlas",
            "--manifest",
            "m",
            "--content-root",
            "c",
            "positional-sentinel",
        ],
        &[
            "prepare-atlas",
            "--manifest",
            "m",
            "--content-root",
            "c",
            "--output",
            "o",
            "--check-pointer",
        ],
        &[
            "prepare-atlas",
            "--manifest",
            "m",
            "--manifest",
            "n",
            "--content-root",
            "c",
        ],
        &["prepare-atlas", "--manifest", "--content-root", "c"],
    ] {
        let output = cli().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let message = String::from_utf8(output.stderr).unwrap();
        assert!(message.contains("Unsupported"));
        assert!(!message.contains("sentinel"));
    }
}

#[test]
fn export_schemas_writes_the_committed_schema_bytes() {
    let dir = std::env::temp_dir().join(format!("azeroth-cli-schemas-{}", std::process::id()));
    let output = cli()
        .args(["export-schemas", "--output"])
        .arg(&dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    let written = std::fs::read_to_string(dir.join("exchange.schema.json")).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        written,
        azeroth_core::contract::schema::exchange_schema_text()
    );
}
