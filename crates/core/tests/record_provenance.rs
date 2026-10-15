use azeroth_core::{
    content_v2::records::reference_records,
    contract::{ReferenceDataset, decode_envelope},
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
fn data() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/reference-dataset.json"
    ))
    .unwrap()
}
fn records(value: &Value) -> azeroth_core::release_party::ConcreteCatalog {
    reference_records(
        &decode_envelope::<ReferenceDataset>(&value.to_string())
            .unwrap()
            .payload,
    )
    .unwrap()
}
#[test]
fn records_and_stages_use_only_their_own_evidence_sources() {
    let mut value = data();
    let mut second = value["payload"]["evidence"][0].clone();
    second["id"] = "second-evidence".into();
    second["source"] = "second-source".into();
    value["payload"]["evidence"]
        .as_array_mut()
        .unwrap()
        .push(second);
    value["payload"]["items"][0]["evidence"] = json!(["second-evidence"]);
    value["payload"]["quests"][0]["evidence"] = json!(["demo-evidence-1"]);
    let found = records(&value);
    for sources in found["equipment"].values() {
        assert_eq!(sources, &BTreeSet::from(["second-source".into()]));
    }
    for (id, sources) in &found["quests"] {
        if id.contains("quests") || id.contains("quest_stages") {
            assert_eq!(sources, &BTreeSet::from(["demo-source-1".into()]));
        }
    }
    for (id, sources) in &found["travel"] {
        if id.contains("nodes") {
            assert!(sources.is_empty());
        }
    }
}
#[test]
fn absent_evidence_never_borrows_another_records_source() {
    let mut value = data();
    value["payload"]["items"][0]["evidence"] = json!([]);
    assert!(
        records(&value)["equipment"]
            .values()
            .all(BTreeSet::is_empty)
    );
    value["payload"]["items"][0]["evidence"] = json!(["missing-evidence"]);
    assert!(decode_envelope::<ReferenceDataset>(&value.to_string()).is_err());
}

#[test]
fn image_provenance_includes_own_facts_but_pins_do_not_inherit_it() {
    let mut value = data();
    for (id, source) in [
        ("calibration-proof", "calibration-source"),
        ("applicable-proof", "applicable-source"),
        ("pin-proof", "pin-source"),
    ] {
        let mut next = value["payload"]["evidence"][0].clone();
        next["id"] = id.into();
        next["source"] = source.into();
        value["payload"]["evidence"]
            .as_array_mut()
            .unwrap()
            .push(next);
    }
    let map = value["payload"]["map_spaces"][0]["key"].clone();
    value["payload"]["map_images"] = json!([{"id":"test-image","map":map,"source":"image-source","evidence":["demo-evidence-1"],"applicable_builds":[{"build":map["context"]["build"],"evidence":["applicable-proof"]}],"width_px":2,"height_px":2,"layers":[{"id":"base","kind":"roads_traced","file":"atlas/base.png","sha256":"0".repeat(64),"width_px":2,"height_px":2,"frame_divisor":1,"registration_offset_px":{"x":0,"y":0}}],"pins":[{"id":"pin","label":"Synthetic pin","kind":{"kind":"landmark"},"pixel":{"x":1,"y":1},"measured_on":"base","evidence":["pin-proof"]}],"calibration":{"status":"unknown"},"calibration_evidence":["calibration-proof"],"control_points":[],"gaps":"Synthetic"}]);
    let found = records(&value);
    assert_eq!(
        found["geography"]["[\"demo-r1\",\"map_images\",\"test-image\"]"],
        BTreeSet::from([
            "demo-source-1".into(),
            "image-source".into(),
            "calibration-source".into(),
            "applicable-source".into()
        ])
    );
    assert_eq!(
        found["geography"]["[\"demo-r1\",\"map_pins\",\"test-image\",\"pin\"]"],
        BTreeSet::from(["pin-source".into()])
    );
}
