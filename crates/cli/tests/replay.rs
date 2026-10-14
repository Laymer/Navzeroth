use azeroth_cli::replay::{MAX_FIXTURE_BYTES, MAX_OUTPUT_BYTES, file, replay};
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
const SOLO: &str = include_str!("../../../fixtures/planner/solo-basic.json");
const DUO: &str = include_str!("../../../fixtures/planner/duo-basic.json");
fn fixture(duo: bool) -> Value {
    serde_json::from_str(if duo { DUO } else { SOLO }).unwrap()
}
fn run(value: &Value) -> Value {
    serde_json::from_slice(&replay(&serde_json::to_vec(value).unwrap()).unwrap()).unwrap()
}
fn reject(mutator: impl FnOnce(&mut Value), duo: bool) {
    let mut v = fixture(duo);
    mutator(&mut v);
    assert!(replay(&serde_json::to_vec(&v).unwrap()).is_err());
}
#[test]
fn solo_and_duo_replay_actual_services_with_deterministic_results() {
    for text in [SOLO, DUO] {
        let first = replay(text.as_bytes()).unwrap();
        assert_eq!(first, replay(text.as_bytes()).unwrap());
        assert!(first.len() < MAX_OUTPUT_BYTES);
        let v: Value = serde_json::from_slice(&first).unwrap();
        assert_eq!(v["fixture"], true);
        assert_eq!(v["context"]["namespace"], "demo");
        assert_eq!(v["source_policies"][0]["redistribution_status"], "unknown");
        assert!(v["source_policies"][0]["redistribution"].is_null());
        assert!(!String::from_utf8(first).unwrap().contains("profile_id"));
        let plan = if v["mode"] == "joint" {
            &v["result"]["plan"]["plan"]
        } else {
            &v["result"]["plan"]
        };
        assert_eq!(plan["selected"].as_array().unwrap().len(), 1);
        assert_eq!(plan["expected_total_ms"], 61000);
        assert_eq!(
            plan["selected"][0]["utility"],
            if v["mode"] == "joint" { 8 } else { 10 }
        );
    }
}
#[test]
fn actor_indexes_and_utilities_map_explicitly_without_relabelling() {
    let mut v = fixture(true);
    v["request"]["participants"] = json!([1, 0]);
    v["request"]["candidates"][0]["utilities"] = json!([3, 7]);
    let r = run(&v);
    assert_eq!(r["participants"], json!([1, 0]));
    assert_eq!(r["result"]["plan"]["plan"]["selected"][0]["utility"], 3);
    let participants = r["result"]["plan"]["assessments"][0]["participants"]
        .as_array()
        .unwrap();
    assert!(
        participants
            .iter()
            .any(|p| p["character"] == "demo-character-mage" && p["utility"] == 3)
    );
    assert!(
        participants
            .iter()
            .any(|p| p["character"] == "demo-character-paladin" && p["utility"] == 7)
    );
}
#[test]
fn whole_unique_cohort_context_freshness_and_schema_are_enforced() {
    for field in ["schema_version", "format"] {
        reject(
            |v| {
                v[field] = if field == "schema_version" {
                    json!(2)
                } else {
                    json!("future")
                }
            },
            false,
        );
    }
    reject(|v| v["request"]["participants"] = json!([0, 0]), true);
    reject(|v| v["request"]["participants"] = json!([0, 2]), true);
    reject(|v| v["request"]["participants"] = json!([0]), true);
    reject(
        |v| {
            v["snapshots"][1]["payload"]["character"] =
                v["snapshots"][0]["payload"]["character"].clone()
        },
        true,
    );
    reject(|v| v["snapshots"][0]["fixture"] = json!(false), false);
    reject(
        |v| v["snapshots"][0]["game"]["build"] = json!("other-build"),
        false,
    );
    reject(
        |v| v["snapshots"][0]["payload"]["capture"]["clock_trust"] = json!("uncertain"),
        false,
    );
    reject(
        |v| v["snapshots"][0]["payload"]["capture"]["observed_at_utc_ms"] = json!(1791791989999i64),
        false,
    );
    reject(
        |v| v["request"]["candidates"][0]["utilities"] = json!([1]),
        true,
    );
    reject(
        |v| v["request"]["excluded"] = json!(["synthetic-service", "synthetic-service"]),
        false,
    );
}
#[test]
fn unknown_restricted_false_and_zero_do_not_become_recommendations() {
    for status in ["unknown", "restricted"] {
        let mut v = fixture(false);
        v["snapshots"][0]["payload"]["level"] = json!({"status":status});
        v["content"]["envelope"]["payload"]["objectives"][0]["requirements"] = json!({"status":"known","value":{"op":"level_at_least","level":22},"source":"reference_data"});
        let output = run(&v);
        assert!(
            output["result"]["plan"]["selected"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            serde_json::to_string(&output["result"]["plan"]["trace"])
                .unwrap()
                .contains(status)
        );
    }
    let mut v = fixture(false);
    v["snapshots"][0]["payload"]["preferences"] = json!({"synthetic-preference":{"status":"known","value":false,"observed_at_utc_ms":1791792000000i64,"clock_trust":"trusted"}});
    v["content"]["envelope"]["payload"]["objectives"][0]["requirements"] = json!({"status":"known","value":{"op":"user_preference","key":"synthetic-preference","value":true},"source":"reference_data"});
    let out = run(&v);
    assert!(
        out["result"]["plan"]["selected"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        out["result"]["plan"]["trace"][0]["decision"],
        "eligibility_false"
    );
    v = fixture(false);
    v["request"]["candidates"][0]["utilities"] = json!([0]);
    let out = run(&v);
    assert_eq!(out["result"]["plan"]["trace"][0]["decision"], "no_utility");
    v = fixture(false);
    v["request"]["excluded"] = json!(["synthetic-service"]);
    assert_eq!(
        run(&v)["result"]["plan"]["trace"][0]["decision"],
        "excluded"
    );
}
#[test]
fn nested_unknown_duplicate_and_byte_limits_fail_safely() {
    reject(
        |v| v["snapshots"][0]["producer"]["private_secret"] = json!("private-secret"),
        false,
    );
    reject(
        |v| v["content"]["sources"][0]["private_secret"] = json!("private-secret"),
        false,
    );
    reject(
        |v| v["request"]["profile_id"] = json!("private-secret"),
        false,
    );
    assert!(
        replay(
            SOLO.replacen(
                "\"schema_version\": 1",
                "\"schema_version\": 1, \"schema_version\": 1",
                1
            )
            .as_bytes()
        )
        .is_err()
    );
    assert!(replay(&vec![b'x'; MAX_FIXTURE_BYTES + 1]).is_err());
    assert!(
        replay(b"private-secret-malformed-json")
            .unwrap_err()
            .contains("JSON")
    );
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "anav-replay-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn cli_stdout_is_complete_and_errors_do_not_echo_input() {
    let temp = Temp::new();
    let path = temp.0.join("fixture.json");
    fs::write(&path, DUO).unwrap();
    let command = || Command::new(env!("CARGO_BIN_EXE_azeroth-cli"));
    let out = command()
        .args(["replay", "--fixture"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(out.stdout, replay(DUO.as_bytes()).unwrap());
    fs::write(&path, b"private-secret-malformed-json").unwrap();
    let out = command()
        .args(["replay", "--fixture"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(
        !String::from_utf8(out.stderr)
            .unwrap()
            .contains("private-secret")
    );
    for args in [
        vec!["replay"],
        vec!["replay", "--fixture", "private-secret", "--extra"],
        vec![
            "replay",
            "--fixture",
            "private-secret",
            "--fixture",
            "private-secret",
        ],
    ] {
        let out = command().args(args).output().unwrap();
        assert!(!out.status.success());
        assert!(
            !String::from_utf8(out.stderr)
                .unwrap()
                .contains("private-secret")
        );
    }
}
#[cfg(unix)]
#[test]
fn selected_file_symlink_components_and_nonregular_inputs_fail() {
    use std::os::unix::fs::symlink;
    let temp = Temp::new();
    let p = temp.0.join("fixture.json");
    fs::write(&p, SOLO).unwrap();
    let link = temp.0.join("link.json");
    symlink(&p, &link).unwrap();
    assert!(file(&link).is_err());
    let dir = temp.0.join("linkdir");
    symlink(&temp.0, &dir).unwrap();
    assert!(file(&dir.join("fixture.json")).is_err());
    assert!(file(&temp.0).is_err());
    assert!(file(&temp.0.join("../fixture.json")).is_err());
}
