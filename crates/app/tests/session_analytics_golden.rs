//! Rust producer for `fixtures/desktop/sessions/analytics.synthetic.json`, which the
//! vitest and Playwright suites consume. The only hand-written field is the
//! methodology sentence; every other field must equal real analytics output.
use azeroth_core::{
    analytics::analyze_session,
    sessions::{SessionBatch, SessionBuildConfig, build_sessions},
    types::{Millis, SessionId},
};
use serde_json::{Value, json};

const GOLDEN: &str = "../../fixtures/desktop/sessions/analytics.synthetic.json";

fn produce() -> Value {
    let batch: SessionBatch = serde_json::from_value(json!({
        "character": "demo-character-paladin",
        "context": {"namespace": "demo", "build": {"edition": "wow_forever",
            "build": "UNVERIFIED_DEMO", "interface": "UNVERIFIED_DEMO"}},
        "events": [
            {"session": "synthetic-session", "sequence": 1, "elapsed_ms": 0,
             "kind": {"type": "session_start"}},
            {"session": "synthetic-session", "sequence": 2, "elapsed_ms": 0,
             "kind": {"type": "level", "level": {"status": "known", "value": 12}}},
            {"session": "synthetic-session", "sequence": 3, "elapsed_ms": 10000,
             "kind": {"type": "activity", "label": "manual_sample", "quality": "unclassified"}},
            {"session": "synthetic-session", "sequence": 4, "elapsed_ms": 10000,
             "kind": {"type": "recording_paused"}},
            {"session": "synthetic-session", "sequence": 5, "elapsed_ms": 15000,
             "kind": {"type": "level", "level": {"status": "known", "value": 13}}}
        ]
    }))
    .unwrap();
    let config = SessionBuildConfig {
        max_observed_span_ms: Millis::new(60_000).unwrap(),
        clock_tolerance_ms: Millis::new(1_000).unwrap(),
    };
    let builds = build_sessions(&batch, &config).unwrap();
    let build = &builds[&SessionId::new("synthetic-session").unwrap()];
    serde_json::to_value(analyze_session(build).unwrap()).unwrap()
}

#[test]
fn analytics_fixture_matches_real_analytics_output() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN);
    let committed: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let mut produced = produce();
    // The committed methodology sentence is a labelled mock; it is the sole
    // permitted difference and must still say so.
    let mock = committed["summary"]["methodology"].as_str().unwrap();
    assert!(mock.starts_with("Synthetic"), "{mock}");
    if std::env::var("UPDATE_DESKTOP_FIXTURES").as_deref() == Ok("1") {
        produced["summary"]["methodology"] = json!(mock);
        std::fs::write(
            &path,
            format!("{}\n", serde_json::to_string_pretty(&produced).unwrap()),
        )
        .unwrap();
        return;
    }
    produced["summary"]["methodology"] = json!(mock);
    assert_eq!(
        produced, committed,
        "analytics.synthetic.json drifted; rerun with UPDATE_DESKTOP_FIXTURES=1 and review"
    );
}
