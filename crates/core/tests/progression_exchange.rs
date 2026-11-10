use azeroth_core::{
    contract::{AnyExchange, ContractKind, MAX_ENVELOPE_BYTES, decode_any, decode_envelope},
    entities::CharacterSnapshot,
    error::ErrorCode,
    progression::ProgressionSnapshot,
};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/progression-snapshot.json"
    ))
    .unwrap()
}
fn decode(
    value: &Value,
) -> azeroth_core::error::Result<azeroth_core::contract::Envelope<ProgressionSnapshot>> {
    decode_envelope(&value.to_string())
}
fn item() -> Value {
    json!({"context": fixture()["payload"]["binding"]["context"], "item_id":"synthetic-item","variant":"base"})
}
fn known(value: Value) -> Value {
    json!({"status":"known","value":value})
}
fn entries(value: Value) -> Value {
    known(json!({"completeness":"complete","entries":value}))
}

#[test]
fn companion_roundtrip_and_old_typed_decoder_refusal() {
    let original = fixture();
    let parsed = decode(&original).unwrap();
    let encoded = serde_json::to_string(&parsed).unwrap();
    let any = decode_any(&encoded).unwrap();
    assert_eq!(any.kind(), ContractKind::ProgressionSnapshot);
    assert!(matches!(any, AnyExchange::ProgressionSnapshot(_)));
    assert_eq!(
        decode_envelope::<ProgressionSnapshot>(&encoded).unwrap(),
        parsed
    );
    assert!(decode_envelope::<CharacterSnapshot>(&encoded).is_err());
    assert_eq!(parsed.payload.money.value().unwrap().get(), 0);
    assert_eq!(parsed.payload.available_talent_points.value(), Some(&0));
    assert_eq!(
        parsed
            .payload
            .recipes
            .value()
            .unwrap()
            .entries
            .values()
            .next()
            .unwrap()
            .value(),
        Some(&false)
    );
}

#[test]
fn dependencies_and_capture_must_match_exactly() {
    for (pointer, value) in [
        ("/dependencies/dataset_revision", json!("other")),
        ("/dependencies/snapshots/0/id", json!("other")),
        ("/dependencies/snapshots/0/character", json!("other")),
        (
            "/dependencies/snapshots/0/context/build/interface",
            json!("other"),
        ),
        ("/dependencies/snapshots", json!([])),
        ("/game/build", json!("other")),
        ("/producer/session_id", json!("other")),
        ("/producer/sequence", json!(999)),
        ("/payload/payload_version", json!(2)),
        ("/payload/binding/snapshot_sha256", json!("bad")),
    ] {
        let mut altered = fixture();
        *altered.pointer_mut(pointer).unwrap() = value;
        assert!(decode(&altered).is_err(), "{pointer}");
    }
    for field in ["ruleset_version", "score_model_version"] {
        let mut altered = fixture();
        altered["dependencies"][field] = json!("unexpected");
        assert!(decode(&altered).is_err());
    }
    for pointer in ["/payload/capture", "/payload/binding/capture"] {
        let mut altered = fixture();
        *altered.pointer_mut(pointer).unwrap() = json!({"sequence":1});
        assert!(decode(&altered).is_err());
    }
    let mut changed = fixture();
    changed["payload"]["unexpected"] = json!(true);
    assert!(decode(&changed).is_err());
    let mut missing = fixture();
    missing["payload"].as_object_mut().unwrap().remove("spells");
    assert!(decode(&missing).is_err());
}

#[test]
fn scoped_inventory_professions_and_effects_preserve_uncertainty() {
    let mut value = fixture();
    value["payload"]["professions"] = entries(
        json!({"profession": {"skill":known(json!(0)),"maximum_skill":known(json!(0)),"rank":{"status":"unknown"}}}),
    );
    let entry = json!({"item":item(),"quantity":known(json!(0))});
    value["payload"]["bags"] = entries(json!({"item":entry}));
    value["payload"]["bank"] = value["payload"]["bags"].clone();
    value["payload"]["equipment_enhancements"] = json!({"main_hand": known(json!({"item":item(),"enhancements":entries(json!({"effect":{"duration":known(json!("temporary")),"expires_at_utc_ms":known(json!(0))}}))}))});
    assert!(decode(&value).is_ok());
    let base = value.clone();
    for (pointer, changed) in [
        (
            "/payload/professions/value/entries/profession/skill/value",
            json!(1),
        ),
        ("/payload/bags/value/entries/item/quantity/value", json!(-1)),
        (
            "/payload/bags/value/entries/item/quantity/value",
            json!(4_294_967_296_u64),
        ),
        (
            "/payload/bags/value/entries/item/item/context/build/build",
            json!("other"),
        ),
        (
            "/payload/equipment_enhancements/main_hand/value/item/context/namespace",
            json!("reference"),
        ),
        (
            "/payload/equipment_enhancements/main_hand/value/enhancements/value/entries/effect/duration/value",
            json!("permanent"),
        ),
    ] {
        let mut value = base.clone();
        *value.pointer_mut(pointer).unwrap() = changed;
        assert!(decode(&value).is_err(), "{pointer}");
    }
    value["payload"]["bags"]["value"]["entries"]["duplicate"] = entry;
    assert!(decode(&value).is_err());
    let mut permanent = base.clone();
    let effect = &mut permanent["payload"]["equipment_enhancements"]["main_hand"]["value"]["enhancements"]
        ["value"]["entries"]["effect"];
    effect["duration"] = known(json!("permanent"));
    effect["expires_at_utc_ms"] = json!({"status":"not_applicable"});
    assert!(decode(&permanent).is_ok());
    let mut unknown = base;
    unknown["payload"]["professions"]["value"]["entries"]["profession"]["maximum_skill"] =
        json!({"status":"unknown"});
    unknown["payload"]["equipment_enhancements"]["main_hand"]["value"]["enhancements"] =
        json!({"status":"restricted"});
    unknown["payload"]["equipment_enhancements"]["off_hand"] = json!({"status":"unsupported"});
    assert!(decode(&unknown).is_ok());
}

#[test]
fn companion_decoding_keeps_byte_depth_and_collection_limits() {
    let mut value = fixture();
    value["padding"] = json!("x".repeat(MAX_ENVELOPE_BYTES));
    assert_eq!(
        decode_any(&value.to_string()).unwrap_err().code,
        ErrorCode::ImportTooLarge
    );
    let mut value = fixture();
    let mut nested = json!(0);
    for _ in 0..33 {
        nested = json!([nested]);
    }
    value["padding"] = nested;
    assert!(decode(&value).is_err());
    let mut value = fixture();
    value["payload"]["spells"] = entries(Value::Object(
        (0..1025)
            .map(|i| (i.to_string(), known(json!(0))))
            .collect(),
    ));
    assert!(decode(&value).is_err());
    let text = serde_json::to_string(&fixture()).unwrap().replace(
        "\"payload_version\":1",
        "\"payload_version\":1,\"payload_version\":1",
    );
    assert!(decode_envelope::<ProgressionSnapshot>(&text).is_err());
}
