//! `guide verify` (task V2.3). Synthetic guides and facts only.
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

fn root() -> PathBuf {
    // The read boundary refuses `..` components, so resolve the checkout first.
    fs::canonicalize(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}
fn fixture(name: &str) -> PathBuf {
    root().join("fixtures/guides").join(name)
}
fn cli(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .arg("guide")
        .args(args)
        .output()
        .unwrap()
}
fn verify(guide: &Path, facts: &Path, extra: &[&str]) -> Output {
    let mut args = vec![
        "verify".as_ref(),
        "--guide".as_ref(),
        guide.as_os_str(),
        "--facts".as_ref(),
        facts.as_os_str(),
    ];
    args.extend(extra.iter().map(std::ffi::OsStr::new));
    cli(&args)
}
fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("JSON report")
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "anav-guide-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, bytes: impl AsRef<[u8]>) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_synthetic_route_passes_against_its_synthetic_facts() {
    let output = verify(
        &fixture("synthetic-route.lua"),
        &fixture("synthetic-facts.json"),
        &[],
    );
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let r = report(&output);
    assert_eq!(r["format"], "navzeroth-guide-verify");
    assert_eq!(r["replay_clean"], true);
    assert_eq!(r["passes"], false, "synthetic facts never pass");
    assert_eq!(r["facts_edition"], "forever");
    assert_eq!(r["facts_build"], "0.0.0.1-synthetic");
    assert_eq!(r["facts_origin"], "synthetic");
    assert_eq!(r["facts_are_evidence"], false);
    assert_eq!(r["character"]["race"], "Human");
    assert_eq!(r["character"]["class"], "Warrior");
    let totals = &r["totals"];
    assert_eq!(totals["guides"], 2);
    assert_eq!(totals["errors"], 0);
    assert_eq!(totals["unverified"], 0);
    assert_eq!(totals["unsupported"], 0);
    assert!(totals["ok"].as_u64().unwrap() > 50);
    assert_eq!(totals["skipped_steps"], 1, "the Mage step");
    assert_eq!(r["steps"].as_array().unwrap().len(), 15);
}

#[test]
fn another_character_sees_different_steps_and_fails_class_checks() {
    let output = verify(
        &fixture("synthetic-route.lua"),
        &fixture("synthetic-facts.json"),
        &["--class", "Mage"],
    );
    assert_eq!(output.status.code(), Some(1));
    let r = report(&output);
    assert_eq!(r["character"]["class"], "Mage");
    let errors: Vec<_> = r["steps"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["findings"].as_array().unwrap())
        .filter(|f| f["status"] == "error")
        .map(|f| f["check"].as_str().unwrap().to_owned())
        .collect();
    assert!(errors.contains(&"class_allowed".to_owned()), "{errors:?}");
}

#[test]
fn unknown_facts_fail_as_unverified() {
    let temp = Temp::new();
    let facts = temp.write(
        "facts.json",
        r#"{"format":"navzeroth-guide-facts","version":1,"origin":"local_observation","edition":"forever","client_build":"0.0.0.1"}"#,
    );
    let output = verify(&fixture("synthetic-route.lua"), &facts, &[]);
    assert_eq!(output.status.code(), Some(1));
    let r = report(&output);
    assert_eq!(r["passes"], false);
    assert_eq!(r["facts_are_evidence"], true);
    assert_eq!(r["totals"]["errors"], 0);
    assert!(r["totals"]["unverified"].as_u64().unwrap() > 0);
}

#[test]
fn usage_errors_exit_2_without_echoing_input() {
    let g = fixture("synthetic-route.lua");
    let f = fixture("synthetic-facts.json");
    let cases: Vec<Vec<&std::ffi::OsStr>> = vec![
        vec![],
        vec!["inspect-sentinel".as_ref()],
        vec!["verify".as_ref()],
        vec!["verify".as_ref(), "--guide".as_ref(), g.as_os_str()],
        vec![
            "verify".as_ref(),
            "--guide".as_ref(),
            g.as_os_str(),
            "--facts".as_ref(),
            f.as_os_str(),
            "--race".as_ref(),
            "Elf-sentinel".as_ref(),
        ],
        vec![
            "verify".as_ref(),
            "--guide".as_ref(),
            g.as_os_str(),
            "--guide".as_ref(),
            g.as_os_str(),
            "--facts".as_ref(),
            f.as_os_str(),
        ],
        vec![
            "verify".as_ref(),
            "--guide".as_ref(),
            "--facts".as_ref(),
            "--facts".as_ref(),
            f.as_os_str(),
        ],
        vec![
            "verify".as_ref(),
            "--guide".as_ref(),
            g.as_os_str(),
            "--facts".as_ref(),
            f.as_os_str(),
            "positional-sentinel".as_ref(),
        ],
    ];
    for args in cases {
        let output = cli(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains("sentinel"), "{stderr}");
        assert!(stderr.contains("guide verify --guide"));
    }
}

#[test]
fn bad_inputs_are_refused_with_static_messages() {
    let temp = Temp::new();
    let facts = fixture("synthetic-facts.json");
    let big = temp.write("big.lua", vec![b' '; 4 * 1024 * 1024 + 1]);
    let output = verify(&big, &facts, &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("4 MiB"), "{stderr}");

    let no_character = temp.write(
        "plain.lua",
        "RXPGuides.RegisterGuide([[\n#forever\n#name a\n#version 1\n#group g\nstep\n    .hs\n]])\n",
    );
    let output = verify(&no_character, &facts, &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("No character"));
    let output = verify(
        &no_character,
        &facts,
        &["--race", "Human", "--class", "Priest"],
    );
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let bad_facts = temp.write("facts.json", r#"{"format":"navzeroth-guide-facts","version":1,"origin":"synthetic","secret-sentinel":1}"#);
    let output = verify(&fixture("synthetic-route.lua"), &bad_facts, &[]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Facts file") && !stderr.contains("sentinel"),
        "{stderr}"
    );

    let missing = temp.0.join("missing-sentinel.lua");
    let output = verify(&missing, &facts, &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("sentinel"));
}

#[cfg(unix)]
#[test]
fn symlinked_inputs_are_refused() {
    let temp = Temp::new();
    let link = temp.0.join("link.lua");
    std::os::unix::fs::symlink(fixture("synthetic-route.lua"), &link).unwrap();
    let output = verify(&link, &fixture("synthetic-facts.json"), &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Symlink"));
}

#[test]
fn exit_0_needs_facts_that_claim_evidence() {
    let temp = Temp::new();
    let synthetic = fs::read_to_string(fixture("synthetic-facts.json")).unwrap();
    let claimed = synthetic.replace(
        "\"origin\": \"synthetic\"",
        "\"origin\": \"local_observation\"",
    );
    assert_ne!(synthetic, claimed);
    let facts = temp.write("claimed.json", claimed);
    let output = verify(&fixture("synthetic-route.lua"), &facts, &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let r = report(&output);
    assert_eq!(r["passes"], true);
    assert_eq!(r["facts_are_evidence"], true);
}
