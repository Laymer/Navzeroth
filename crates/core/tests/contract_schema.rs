use azeroth_core::contract::{ContractKind, schema::*};
use serde_json::Value;

const REGENERATE: &str =
    "regenerate with: cargo run --locked -p azeroth-cli -- export-schemas --output schemas/v1";

#[test]
fn committed_schema_matches_rust_definitions() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/v1")
        .join(SCHEMA_FILE);
    let committed = std::fs::read_to_string(path).expect(REGENERATE);
    assert!(
        committed == exchange_schema_text(),
        "schemas/v1/{SCHEMA_FILE} is out of date; {REGENERATE}"
    );
}

#[test]
fn generation_is_deterministic() {
    assert_eq!(exchange_schema_text(), exchange_schema_text());
}

#[test]
fn schema_encodes_key_contract_constraints() {
    let schema = exchange_schema();
    let defs = &schema["$defs"];
    assert_eq!(
        schema["oneOf"].as_array().unwrap().len(),
        ContractKind::ALL.len()
    );
    for (kind, name) in [
        (ContractKind::CharacterSnapshot, "CharacterSnapshotEnvelope"),
        (ContractKind::RoutePlan, "RoutePlanEnvelope"),
        (
            ContractKind::GearRecommendations,
            "GearRecommendationSetEnvelope",
        ),
    ] {
        let envelope = &defs[name];
        assert_eq!(envelope["properties"]["kind"]["const"], kind.as_str());
        assert_eq!(
            envelope["properties"]["format"]["const"],
            "azeroth-navigator"
        );
        let required: Vec<_> = envelope["required"].as_array().unwrap().iter().collect();
        for member in [
            "format",
            "kind",
            "schema_version",
            "fixture",
            "game",
            "namespace",
            "provenance",
            "payload",
        ] {
            assert!(
                required.contains(&&Value::from(member)),
                "{name} requires {member}"
            );
        }
    }
    assert_eq!(defs["SchemaVersion"]["const"], 1);
    assert_eq!(defs["Copper"]["type"], "string");
    assert_eq!(defs["Millis"]["maximum"], 9_007_199_254_740_991_u64);
    let branches = defs["Observation_Level"]["oneOf"].as_array().unwrap();
    assert_eq!(
        branches[0]["required"],
        serde_json::json!(["status", "value"])
    );
    assert_eq!(branches[1]["properties"]["value"], Value::Bool(false));
}
