use azeroth_core::local_data::PerformancePreset;
use azeroth_local_data::media;
use std::io::Write;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};
#[test]
fn media_worker_refuses_a_versioned_request_over_the_real_framed_transport() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .arg(media::WORKER_ARGUMENT)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let request = media::Request {
        format_version: 999,
        installation: PathBuf::from("unselected-synthetic-source"),
        destination: PathBuf::from("unselected-synthetic-store"),
        catalog: "invalid".into(),
        locale: "enUS".into(),
        ids: vec![900],
        preset: PerformancePreset::Small,
    };
    let bytes = serde_json::to_vec(&request).unwrap();
    let mut input = child.stdin.take().unwrap();
    input
        .write_all(&(bytes.len() as u32).to_le_bytes())
        .unwrap();
    input.write_all(&bytes).unwrap();
    drop(input);
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let size = u32::from_le_bytes(output.stdout[..4].try_into().unwrap()) as usize;
    assert_eq!(size + 4, output.stdout.len());
    let response: serde_json::Value = serde_json::from_slice(&output.stdout[4..]).unwrap();
    assert_eq!(response["format_version"], 1);
    assert!(response["revision"].is_null());
    assert_eq!(response["error"], "Unsupported media request");
    assert!(!request.installation.exists());
    assert!(!request.destination.exists());
}
