use azeroth_core::submissions::*;
use serde_json::{Value, json};
fn input() -> Value {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/reference-dataset.json"
    ))
    .unwrap();
    json!({"schema_version":1,"context":fixture["payload"]["evidence"][0]["context"],"fixture":true,"base_revision":fixture["payload"]["revision"],"source":"demo-source-1","evidence":"demo-evidence-1","observed_at_utc_ms":null,"payload":{"kind":"acquisition_outcome","item":"1","acquired":{"status":"known","value":false},"opportunities":{"status":"unknown"},"successes":{"status":"known","value":0},"group_eligible":{"status":"restricted"},"personally_received":{"status":"stale"}}})
}
#[test]
fn false_zero_unknown_and_stale_remain_distinct() {
    let doc = decode_local_observation(&input().to_string()).unwrap();
    assert_eq!(serde_json::to_value(doc).unwrap(), input());
}
#[test]
fn no_personal_extensions_future_versions_or_input_echo() {
    let mut v = input();
    v["context"]["private_name"] = json!("private secret");
    let e = decode_local_observation(&v.to_string()).unwrap_err();
    assert!(!e.to_string().contains("private secret"));
    v = input();
    v["schema_version"] = json!(2);
    assert!(decode_local_observation(&v.to_string()).is_err());
    assert!(decode_local_observation(&"x".repeat(16385)).is_err());
    v = input();
    v["payload"]["opportunities"] = json!({"status":"unknown","value":0});
    assert!(decode_local_observation(&v.to_string()).is_err());
}
#[test]
fn anomaly_checks_do_not_infer_drop_probability() {
    let mut v = input();
    v["payload"]["opportunities"] = json!({"status":"known","value":0});
    v["payload"]["successes"] = json!({"status":"known","value":1});
    assert!(decode_local_observation(&v.to_string()).is_err());
}
#[test]
fn registered_source_identifiers_keep_the_existing_256_byte_contract() {
    for source in ["s".repeat(129), "s".repeat(256), "é".repeat(128)] {
        let mut v = input();
        v["source"] = json!(source);
        assert_eq!(
            decode_local_observation(&v.to_string())
                .unwrap()
                .source
                .as_str()
                .len(),
            v["source"].as_str().unwrap().len()
        );
    }
    for source in ["s".repeat(257), "é".repeat(129)] {
        let mut v = input();
        v["source"] = json!(source);
        assert!(decode_local_observation(&v.to_string()).is_err());
    }
}
