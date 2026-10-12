use azeroth_core::{
    contract::*,
    entities::{CharacterSnapshot, RoutePlan},
    error::ErrorCode,
};
use serde_json::{Value, json};
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/contracts/v1")
}
fn read(name: &str) -> String {
    std::fs::read_to_string(fixture_dir().join(name)).expect("fixture is readable")
}
fn code_name(code: ErrorCode) -> String {
    serde_json::to_value(code)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}
fn depth(value: &Value) -> usize {
    match value {
        Value::Array(a) => 1 + a.iter().map(depth).max().unwrap_or(0),
        Value::Object(o) => 1 + o.values().map(depth).max().unwrap_or(0),
        _ => 0,
    }
}
/// Every member spelled out in `original` survives in `canonical` with the same
/// value. Canonical output may add documented defaults (observation metadata).
fn assert_subset(original: &Value, canonical: &Value, path: &str) {
    match (original, canonical) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in a {
                let next = format!("{path}/{k}");
                assert_subset(
                    v,
                    b.get(k).unwrap_or_else(|| panic!("missing {next}")),
                    &next,
                );
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{path}");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                assert_subset(x, y, &format!("{path}/{i}"));
            }
        }
        (Value::Number(a), Value::Number(b)) => assert_eq!(a.as_f64(), b.as_f64(), "{path}"),
        (a, b) => assert_eq!(a, b, "{path}"),
    }
}
fn encode(exchange: &AnyExchange) -> String {
    serde_json::to_string(exchange).unwrap()
}

#[test]
fn manifest_fixtures_have_declared_rust_outcomes() {
    let manifest: Value = serde_json::from_str(&read("manifest.json")).unwrap();
    let entries = manifest["fixtures"].as_array().unwrap();
    let listed: Vec<_> = entries
        .iter()
        .map(|e| e["file"].as_str().unwrap())
        .collect();
    let mut on_disk: Vec<_> = std::fs::read_dir(fixture_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n != "manifest.json")
        .collect();
    on_disk.sort();
    let mut sorted = listed.clone();
    sorted.sort();
    assert_eq!(sorted, on_disk, "every fixture file is listed exactly once");

    let mut kinds_seen = std::collections::BTreeSet::new();
    for entry in entries {
        let file = entry["file"].as_str().unwrap();
        let input = read(file);
        let result = decode_any(&input);
        match entry["expect"].as_str().unwrap() {
            "valid" => {
                let exchange = result.unwrap_or_else(|e| panic!("{file}: {e:?}"));
                assert_eq!(exchange.kind().as_str(), entry["kind"], "{file}");
                kinds_seen.insert(exchange.kind());
                // Decode, encode and decode again: the value and encoding are stable.
                let encoded = encode(&exchange);
                let again = decode_any(&encoded).unwrap_or_else(|e| panic!("{file}: {e:?}"));
                assert_eq!(again, exchange, "{file}: round trip");
                assert_eq!(encode(&again), encoded, "{file}: canonical encoding");
                // The canonical form retains every member the fixture spelled out.
                let original: Value = serde_json::from_str(&input).unwrap();
                let canonical: Value = serde_json::from_str(&encoded).unwrap();
                assert_subset(&original, &canonical, file);
            }
            expect @ ("schema_invalid" | "semantic_invalid") => {
                let err = result.expect_err(file);
                assert_eq!(
                    code_name(err.code),
                    entry["error_code"].as_str().unwrap(),
                    "{file} ({expect}): {err}"
                );
            }
            other => panic!("{file}: unknown expectation {other}"),
        }
    }
    assert_eq!(
        kinds_seen.len(),
        ContractKind::ALL.len(),
        "every kind has a valid fixture"
    );
}

#[test]
fn typed_decoding_checks_the_declared_kind() {
    let snapshot = read("character-snapshot.json");
    assert!(decode_envelope::<CharacterSnapshot>(&snapshot).is_ok());
    let err = decode_envelope::<RoutePlan>(&snapshot).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn header_is_checked_before_the_payload() {
    let input = json!({"format":"azeroth-navigator","kind":"route_plan","schema_version":2,"payload":"not a plan"});
    let err = decode_any(&input.to_string()).unwrap_err();
    assert_eq!(err.code, ErrorCode::UnsupportedSchema);
    let input = json!({"format":"azeroth-navigator","kind":"route_plan","schema_version":-1});
    assert_eq!(
        decode_any(&input.to_string()).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    let input = json!({"format":"azeroth-navigator","kind":"lua_chunk","schema_version":1});
    assert_eq!(
        decode_any(&input.to_string()).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    let err = decode_any("NaN").unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn extensions_are_retained_but_nested_unknown_members_are_dropped() {
    let mut doc: Value = serde_json::from_str(&read("character-snapshot.json")).unwrap();
    doc["payload"]["future_member"] = json!("dropped");
    let exchange = decode_envelope::<CharacterSnapshot>(&doc.to_string()).unwrap();
    assert_eq!(
        exchange.extensions.get("x_producer_debug").unwrap()["note"],
        "retained extension"
    );
    let encoded: Value = serde_json::to_value(&exchange).unwrap();
    assert_eq!(encoded["x_producer_debug"]["values"], json!([1, 2]));
    assert!(encoded["payload"].get("future_member").is_none());

    for i in 0..MAX_EXTENSIONS {
        doc[format!("x_{i:03}")] = json!(i);
    }
    let err = decode_envelope::<CharacterSnapshot>(&doc.to_string()).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn addon_facing_depth_limit_is_exact() {
    let base: Value = serde_json::from_str(&read("character-snapshot.json")).unwrap();
    let with_depth = |total: usize| {
        let mut doc = base.clone();
        let mut nested = json!(0);
        for _ in 0..total - 1 {
            nested = json!([nested]);
        }
        doc["x_deep"] = nested;
        assert_eq!(depth(&doc), total);
        doc.to_string()
    };
    assert!(decode_envelope::<CharacterSnapshot>(&with_depth(MAX_ADDON_JSON_DEPTH)).is_ok());
    assert!(decode_envelope::<CharacterSnapshot>(&with_depth(MAX_ADDON_JSON_DEPTH + 1)).is_err());
    // Brackets inside strings do not count toward nesting.
    let mut doc = base;
    doc["x_text"] = json!("[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[\\\"{{{{");
    assert!(decode_envelope::<CharacterSnapshot>(&doc.to_string()).is_ok());
    // The deepest permitted plan predicate fits the addon nesting limit exactly.
    let plan: Value = serde_json::from_str(&read("route-plan-max-depth.json")).unwrap();
    assert_eq!(depth(&plan), MAX_ADDON_JSON_DEPTH);
}

#[test]
fn oversized_documents_are_rejected_before_parsing() {
    let snapshot = format!(
        "{}{}",
        read("character-snapshot.json"),
        " ".repeat(MAX_ENVELOPE_BYTES)
    );
    assert_eq!(
        decode_any(&snapshot).unwrap_err().code,
        ErrorCode::ImportTooLarge
    );
    assert_eq!(
        decode_envelope::<CharacterSnapshot>(&snapshot)
            .unwrap_err()
            .code,
        ErrorCode::ImportTooLarge
    );
    // Datasets have a larger limit, but it is still enforced.
    let dataset = format!(
        "{}{}",
        read("reference-dataset.json"),
        " ".repeat(MAX_ENVELOPE_BYTES)
    );
    assert!(decode_any(&dataset).is_ok());
    let dataset = format!(
        "{}{}",
        read("reference-dataset.json"),
        " ".repeat(MAX_DATASET_BYTES)
    );
    assert_eq!(
        decode_any(&dataset).unwrap_err().code,
        ErrorCode::ImportTooLarge
    );
}

#[test]
fn errors_do_not_echo_imported_content() {
    let mut doc: Value = serde_json::from_str(&read("character-snapshot.json")).unwrap();
    doc["payload"]["character"] = json!(" PRIVATE_PAYLOAD_SENTINEL");
    let err = decode_any(&doc.to_string()).unwrap_err();
    assert!(!format!("{err} {err:?}").contains("PRIVATE_PAYLOAD_SENTINEL"));
}

#[test]
fn large_copper_and_observation_states_survive_round_trips() {
    let exchange = decode_any(&read("reference-dataset.json")).unwrap();
    let encoded: Value = serde_json::to_value(&exchange).unwrap();
    assert_eq!(
        encoded["payload"]["acquisitions"][0]["cost"]["value"],
        "9007199254740993"
    );
    let snapshot = decode_envelope::<CharacterSnapshot>(&read("character-snapshot.json")).unwrap();
    let encoded: Value = serde_json::to_value(&snapshot).unwrap();
    let p = &encoded["payload"];
    assert_eq!(
        p["preferences"]["show_tips"],
        json!({"status":"known","value":false,"source":"unrecorded","clock_trust":"uncertain"})
    );
    assert_eq!(p["equipment"]["head"]["value"], json!({"state":"empty"}));
    assert_eq!(p["equipment"]["off_hand"]["status"], "restricted");
    assert!(p["equipment"]["off_hand"].get("value").is_none());
    assert_eq!(p["level"]["session_id"], "demo-session-001");
}

#[test]
fn extensions_cannot_shadow_envelope_members() {
    for key in RESERVED_MEMBERS {
        assert!(
            Extensions::new([(key.to_owned(), json!({}))]).is_err(),
            "{key}"
        );
    }
    assert!(Extensions::new([("x_ok".to_owned(), json!({"n": 1}))]).is_ok());
    assert!(Extensions::new([("x_n".to_owned(), json!(1e300))]).is_err());
    let big = "x".repeat(MAX_EXTENSION_VALUE_BYTES / 2);
    let many = (0..9).map(|i| (format!("x_{i}"), json!(big.clone())));
    assert!(
        Extensions::new(many).is_err(),
        "total extension size is bounded"
    );
}

#[test]
fn non_fixture_documents_require_non_synthetic_sources() {
    // In-memory test of the production path: the same synthetic structure,
    // relabeled as an addon export in the reference namespace, is accepted.
    let text =
        read("reference-namespace-fixture.json").replace("synthetic_fixture", "addon_snapshot");
    let mut doc: Value = serde_json::from_str(&text).unwrap();
    doc["fixture"] = json!(false);
    doc["producer"]["component"] = json!("addon");
    let exchange = decode_envelope::<CharacterSnapshot>(&doc.to_string()).unwrap();
    assert!(!exchange.fixture);
    // A single synthetic observation is enough to reject it.
    doc["payload"]["role"]["source"] = json!("synthetic_fixture");
    assert!(decode_envelope::<CharacterSnapshot>(&doc.to_string()).is_err());
}
