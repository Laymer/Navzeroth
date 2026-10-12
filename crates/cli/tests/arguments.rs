use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
}

#[test]
fn no_arguments_reports_current_capabilities() {
    let output = cli().output().unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("export-schemas")
    );
}

#[test]
fn unsupported_commands_and_options_fail_without_echoing_input() {
    for args in [
        &["doctor"][..],
        &["validate-fixtures"],
        &["--help"],
        &["private-input-sentinel"],
        &["export-schemas"],
        &["export-schemas", "--private-input-sentinel", "dir"],
        &["export-schemas", "--output", "dir", "extra-sentinel"],
    ] {
        let output = cli().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let message = String::from_utf8(output.stderr).unwrap();
        assert!(message.contains("Unsupported"));
        assert!(!message.contains("sentinel") && !message.contains("doctor"));
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
