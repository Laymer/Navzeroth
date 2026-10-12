use std::process::Command;

#[test]
fn no_arguments_reports_current_capabilities() {
    let output = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("No subcommands")
    );
}

#[test]
fn unsupported_commands_and_options_fail_without_echoing_input() {
    for arg in [
        "doctor",
        "validate-fixtures",
        "--help",
        "private-input-sentinel",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
            .arg(arg)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let message = String::from_utf8(output.stderr).unwrap();
        assert!(message.contains("not") || message.contains("No subcommands"));
        assert!(!message.contains(arg));
    }
}
