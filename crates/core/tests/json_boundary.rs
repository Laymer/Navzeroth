mod common;
use azeroth_core::{entities::CharacterSnapshot, error::ErrorCode, json::*, predicate::Predicate};
use common::*;
use serde_json::json;
#[test]
fn bounded_decode_checks_schema_without_echoing_input() {
    let mut s = snapshot();
    s["future_metadata"] = json!({"arbitrary":"ignored"});
    let decoded: CharacterSnapshot = decode_json(&s.to_string()).unwrap();
    assert_eq!(decoded.character.as_str(), "char1");
    s["schema_version"] = json!(2);
    assert_eq!(
        decode_json::<CharacterSnapshot>(&s.to_string())
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedSchema
    );
    s["schema_version"] = json!(null);
    assert!(decode_json::<CharacterSnapshot>(&s.to_string()).is_err());
    let oversized = " ".repeat(MAX_DOMAIN_JSON_BYTES + 1);
    assert_eq!(
        decode_json::<CharacterSnapshot>(&oversized)
            .unwrap_err()
            .code,
        ErrorCode::ImportTooLarge
    );
    let err = decode_json::<CharacterSnapshot>("PRIVATE_PAYLOAD_SENTINEL").unwrap_err();
    assert!(!err.to_string().contains("PRIVATE_PAYLOAD_SENTINEL"));
}
#[test]
fn deep_and_executable_rule_data_is_rejected() {
    let mut node = json!({"op":"all","predicates":[]});
    for _ in 0..40 {
        node = json!({"op":"not","predicate":node});
    }
    assert!(decode_json::<Predicate>(&node.to_string()).is_err());
    assert!(decode_json::<Predicate>(r#"{"op":"eval","code":"return 1"}"#).is_err());
}
