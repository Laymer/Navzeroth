use azeroth_core::{
    content_v2::decode,
    contract::{ReferenceDataset, decode_envelope},
};
use serde_json::{Value, json};
fn reference() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/reference-dataset.json"
    ))
    .unwrap()
}
fn content(reference: Value) -> Value {
    json!({"format":"navzeroth-reference-content","schema_version":2,"reference":reference,"models":[],"equipment":[],"quest_facts":[],"acquisition_facts":[],"dungeons":[]})
}
#[test]
fn v2_supports_large_catalog_and_v1_still_refuses_it() {
    let mut base = reference();
    let item = base["payload"]["items"][0].clone();
    for n in 0..2048 {
        let mut next = item.clone();
        next["key"]["item_id"] = format!("large-item-{n}").into();
        base["payload"]["items"].as_array_mut().unwrap().push(next);
    }
    assert!(decode_envelope::<ReferenceDataset>(&serde_json::to_string(&base).unwrap()).is_err());
    let parsed = decode(&serde_json::to_vec(&content(base)).unwrap()).unwrap();
    assert!(parsed.reference.payload.items.len() > 2048);
}
#[test]
fn catalog_keeps_cross_region_graph_references_and_refuses_missing_ones() {
    let mut base = reference();
    let map = base["payload"]["map_spaces"][0].clone();
    let mut other = map.clone();
    other["key"]["map_id"] = "region-two".into();
    base["payload"]["map_spaces"]
        .as_array_mut()
        .unwrap()
        .push(other);
    let parsed = decode(&serde_json::to_vec(&content(base.clone())).unwrap()).unwrap();
    assert_eq!(parsed.reference.payload.map_spaces.len(), 2);
    base["payload"]["map_spaces"] = json!([]);
    assert!(decode(&serde_json::to_vec(&content(base)).unwrap()).is_err());
}
