use azeroth_app::{diagnostics::*, snapshots::SnapshotApplication};
use azeroth_core::{
    contract::Envelope,
    entities::{CharacterSnapshot, GameBuild},
    error::ErrorCode,
    observation::{ClockTrust, FreshnessContext},
    types::{Millis, UtcMillis},
};
use azeroth_storage::content::ContentRevision;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};
const SECRET: &str = "private_secret_account_token";
const SNAPSHOT: &str = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
const CONTENT: &str = include_str!("../../../fixtures/desktop/content/demo-revision.json");
fn planted(value: &mut Value, key: &str) {
    match value {
        Value::String(s) => {
            if s.starts_with("demo-")
                || s == "UNVERIFIED_DEMO"
                || [
                    "version",
                    "publisher",
                    "description",
                    "label",
                    "url",
                    "summary",
                    "locator",
                    "name",
                    "notes",
                    "explanation",
                    "text",
                    "result_template",
                    "units",
                    "edition",
                ]
                .contains(&key)
            {
                *s = format!("{SECRET}-{s}");
            }
        }
        Value::Array(v) => {
            for value in v {
                planted(value, key)
            }
        }
        Value::Object(o) => {
            let before = std::mem::take(o);
            for (key, mut value) in before {
                planted(&mut value, &key);
                let key = if key.starts_with("demo-") {
                    format!("{SECRET}-{key}")
                } else {
                    key
                };
                o.insert(key, value);
            }
        }
        _ => {}
    }
}
fn values() -> (Envelope<CharacterSnapshot>, ContentRevision) {
    let mut s: Value = serde_json::from_str(SNAPSHOT).unwrap();
    let mut c: Value = serde_json::from_str(CONTENT).unwrap();
    planted(&mut s, "");
    planted(&mut c, "");
    s["payload"]["preferences"] = json!({SECRET:{"status":"known","value":false}});
    s["payload"]["objectives"] = json!({SECRET:{"status":"restricted"}});
    s["payload"]["equipment"] = json!({SECRET:{"status":"unsupported"}});
    s["payload"]["role"] = json!({"status":"known","value":SECRET});
    for source in c["sources"].as_array_mut().unwrap() {
        source["provider"] = json!(SECRET);
        for field in [
            "location",
            "license",
            "attribution",
            "retention",
            "update_schedule",
            "failure_behavior",
            "verified_revision",
        ] {
            source[field] = json!({"status":"known","value":SECRET});
        }
        for field in ["allowed_fields", "transformations"] {
            source[field] = json!({"status":"known","value":[SECRET]});
        }
    }

    (
        serde_json::from_value(s).unwrap(),
        serde_json::from_value(c).unwrap(),
    )
}
fn json_report(report: &DiagnosticReport) -> Value {
    serde_json::from_slice(&serialize(report).unwrap()).unwrap()
}
#[test]
fn arbitrary_imported_strings_and_ids_never_enter_report() {
    let (s, c) = values();
    let before = (
        serde_json::to_value(&s).unwrap(),
        serde_json::to_value(&c).unwrap(),
    );
    let report = build(Some(&s), Some(&c), None).unwrap();
    let bytes = serialize(&report).unwrap();
    assert!(!String::from_utf8(bytes.clone()).unwrap().contains(SECRET));
    assert!(bytes.len() < MAX_DIAGNOSTIC_BYTES);
    let j = json_report(&report);
    assert_eq!(j["schema_version"], 1);
    assert_eq!(j["dataset"]["coverage_class"], "demo");
    assert_eq!(j["versions"]["app"], env!("CARGO_PKG_VERSION"));
    assert_eq!(j["versions"]["bundled_addon"], env!("CARGO_PKG_VERSION"));
    assert!(j["versions"]["installed_addon"].is_null());
    assert_eq!(
        j["dataset"]["revision_fingerprint_sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert!(j["game"]["build"].is_null());
    assert_eq!(
        before,
        (
            serde_json::to_value(&s).unwrap(),
            serde_json::to_value(&c).unwrap()
        )
    );
}
#[test]
fn missing_statistics_remain_unknown_observed_zero_and_false_are_distinct() {
    let unknown = json_report(&build(None, None, None).unwrap());
    assert!(unknown["telemetry"]["import_attempts"].is_null());
    let mut telemetry = DiagnosticTelemetry::observed();
    assert_eq!(
        json_report(&build(None, None, Some(&telemetry)).unwrap())["telemetry"]["import_attempts"],
        0
    );
    for _ in 0..100 {
        telemetry
            .record(
                DiagnosticOperation::SnapshotImport,
                Some(0),
                Some(ErrorCode::InvalidInput),
            )
            .unwrap();
    }
    let j = json_report(&build(None, None, Some(&telemetry)).unwrap());
    assert_eq!(j["telemetry"]["events"].as_array().unwrap().len(), 64);
    assert_eq!(j["telemetry"]["import_attempts"], 100);
    assert_eq!(j["telemetry"]["truncated"], true);
    assert_eq!(j["telemetry"]["import_succeeded"], 0);
    assert_eq!(j["telemetry"]["events"][0]["duration_ms"], 0);
    let before = serialize(&build(None, None, Some(&telemetry)).unwrap()).unwrap();
    assert!(
        telemetry
            .record(DiagnosticOperation::Planning, Some(86_400_001), None)
            .is_err()
    );
    assert_eq!(
        before,
        serialize(&build(None, None, Some(&telemetry)).unwrap()).unwrap()
    );
}
#[test]
fn capability_false_restricted_and_stale_survive_without_identifiers() {
    let (s, _) = values();
    let game:GameBuild=serde_json::from_value(json!({"key":s.payload.context.build,"locale":"enGB","capabilities":{"status":"known","observed_at_utc_ms":1,"clock_trust":"trusted","value":{"character_level":{"status":"verified_unavailable","evidence":[SECRET]},"map_position":{"status":"restricted","evidence":[]},SECRET:{"status":"verified_available","evidence":[SECRET]}}}})).unwrap();
    let clock = FreshnessContext {
        now_utc_ms: Some(UtcMillis::new(1000).unwrap()),
        clock_trust: ClockTrust::Trusted,
        session: None,
        max_age_ms: Millis::new(1).unwrap(),
    };
    let report = build_with_context(Some(&s), None, None, Some(&clock), Some(&game)).unwrap();
    let j = json_report(&report);
    assert_eq!(
        j["capabilities"]["character_level"]["verified_available"],
        false
    );
    assert_eq!(
        j["capabilities"]["character_level"]["freshness"]["status"],
        "stale"
    );
    assert_eq!(j["capabilities"]["map_position"]["check"], "restricted");
    assert!(
        !String::from_utf8(serialize(&report).unwrap())
            .unwrap()
            .contains(SECRET)
    );
    assert!(j["historical_user_report"]["installed_revision_match"].is_null());
}
#[test]
fn context_mismatch_does_not_mix_dataset_coverage() {
    let (s, _) = values();
    let c: ContentRevision = serde_json::from_str(CONTENT).unwrap();
    let j = json_report(&build(Some(&s), Some(&c), None).unwrap());
    assert_eq!(j["dataset_context_mismatch"], true);
    assert!(j["dataset"].is_null());
}
#[test]
fn create_new_export_preserves_existing_file_and_stored_state() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "navzeroth-diagnostic-test-{}-{}.json",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let app = SnapshotApplication::in_memory().unwrap();
    app.import_text(&format!("ANAV1:{SNAPSHOT}"), None).unwrap();
    let before = serde_json::to_value(app.profiles().unwrap()).unwrap();
    let report = build(None, None, None).unwrap();
    let receipt = write_new(&report, &path).unwrap();
    assert!(receipt.bytes < MAX_DIAGNOSTIC_BYTES);
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        write_new(&report, &path).unwrap_err().code,
        ErrorCode::StorageError
    );
    assert_eq!(bytes, std::fs::read(&path).unwrap());
    assert_eq!(
        before,
        serde_json::to_value(app.profiles().unwrap()).unwrap()
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn stale_field_age_is_not_replaced_by_fresh_snapshot_age() {
    let mut value: Value = serde_json::from_str(SNAPSHOT).unwrap();
    value["payload"]["capture"] = json!({"clock_trust":"trusted","observed_at_utc_ms":1000});
    value["payload"]["level"] =
        json!({"status":"known","value":12,"clock_trust":"trusted","observed_at_utc_ms":1});
    let snapshot: Envelope<CharacterSnapshot> = serde_json::from_value(value).unwrap();
    let clock = FreshnessContext {
        now_utc_ms: Some(UtcMillis::new(1000).unwrap()),
        clock_trust: ClockTrust::Trusted,
        session: None,
        max_age_ms: Millis::new(1).unwrap(),
    };
    let j =
        json_report(&build_with_context(Some(&snapshot), None, None, Some(&clock), None).unwrap());
    assert_eq!(j["snapshot_freshness"]["status"], "fresh");
    assert_eq!(j["field_freshness"]["level"]["status"], "stale");
    assert_eq!(j["field_freshness"]["level"]["age_ms"], 999);
}
