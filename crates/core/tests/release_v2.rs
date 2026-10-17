//! Schema 2 release manifests (task V1.1, ADR 0006): declared slice, recorded
//! out-of-scope boundaries and a target that stays pending until launch.
use azeroth_core::release::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const COMMIT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
fn file() -> Value {
    json!({"path":"evidence/test.txt","bytes":1,"sha256":HASH})
}
fn slice() -> Value {
    json!({"id":"human-1-20","races":["human"],
        "zones":["northshire_valley","elwynn_forest","westfall","redridge_mountains"],
        "dungeons":["the_deadmines"],"level_min":1,"level_max":20,
        "classes":["warrior","paladin","rogue","priest","mage","warlock"],"party_sizes":[1,2]})
}
fn inventory_value() -> Value {
    json!({"schema_version":2,"inventory_id":"synthetic-v2-test-only","target":null,"slice":slice(),
        "domains": DOMAINS.map(|domain| json!({"domain":domain,"completeness":"complete",
            "enumeration_evidence":[file()],"units":[{"id":domain,"scope":"Synthetic scenario",
                "required_record_ids":["synthetic:1"]}],
            "out_of_scope":[{"id":format!("{domain}-outside-slice"),"scope":"Everything outside the slice",
                "reason":"Later 1.x releases widen the slice"}]})),
        "sources":[{"id":"synthetic","revision":COMMIT}]})
}
fn evidence_value() -> Value {
    let checks: Vec<_> = LIVE_CHECKS
        .into_iter()
        .map(|id| (id, "live_game"))
        .chain(
            WINDOWS_CHECKS
                .into_iter()
                .map(|id| (id, "interactive_windows")),
        )
        .map(|(id, method)| {
            json!({"id":id,"status":"pass","method":method,
            "operator":"synthetic-test","recorded_at":"2026-10-17T12:00:00Z",
            "environment":"synthetic unit-test record, not actual acceptance","artifacts":[file()]})
        })
        .collect();
    json!({"schema_version":2,"target":null,
        "inventory_sha256":HASH,"admission_registry":file(),"tested_source_commit":COMMIT,"production_sha256":HASH,
        "checks":checks,"coverage": DOMAINS.map(|unit_id| json!({"unit_id":unit_id,
            "status":"pass","included_record_ids":["synthetic:1"],"source_ids":["synthetic"],
            "unresolved_conflicts":0,"unsupported_corrections":0,"validation_evidence":[file()]})),
        "sources":[{"id":"synthetic","revision":COMMIT,"origin_edition":"synthetic",
            "origin_build":null,"redistribution":"permitted","files":[file()],
            "license_files":[file()],"attribution_files":[file()],"permission_evidence":[file()],
            "target_validation":"pass","target_evidence":[file()]}],
        "packs":[{"manifest":file(),"dataset_revisions":["synthetic-r1"]}],"optional_deferrals":[]})
}
fn decode(value: &Value) -> Result<Inventory, &'static str> {
    decode_inventory(&serde_json::to_vec(value).unwrap())
}
fn check(inventory: &Inventory, evidence: &Evidence) -> Readiness {
    assess(
        "1.0.0",
        inventory,
        Some(evidence),
        &Bindings {
            inventory_sha256: HASH,
            production_sha256: HASH,
            tested_source_commit: COMMIT,
        },
    )
    .unwrap()
}
fn complete() -> (Inventory, Evidence) {
    (
        decode(&inventory_value()).unwrap(),
        decode_evidence(&serde_json::to_vec(&evidence_value()).unwrap()).unwrap(),
    )
}
fn has(report: &Readiness, text: &str) -> bool {
    report.blockers.iter().any(|b| b.contains(text))
}

#[test]
fn pending_target_blocks_an_otherwise_complete_v2_release() {
    assert_eq!(frozen_release_target(), None);
    let (i, e) = complete();
    let report = check(&i, &e);
    assert!(!report.stable_ready && !report.candidate_ready && !report.publication_allowed);
    assert_eq!(
        report.blockers,
        ["Release target is pending the first stable live build after launch (ADR 0006)"]
    );
}

#[test]
fn a_manifest_cannot_choose_its_own_target() {
    let (mut i, mut e) = complete();
    i.target = Some(Target::forever_70374());
    e.target = i.target.clone();
    assert!(has(&check(&i, &e), "frozen target does not match"));
}

#[test]
fn evidence_must_match_inventory_schema_and_pending_target() {
    let (i, mut e) = complete();
    e.target = Some(Target::forever_70374());
    assert!(has(&check(&i, &e), "Acceptance version or target differs"));
    let (i, mut e) = complete();
    e.schema_version = 1;
    assert!(has(&check(&i, &e), "Acceptance version or target differs"));
}

#[test]
fn target_key_is_required_even_when_null() {
    let mut value = inventory_value();
    value.as_object_mut().unwrap().remove("target");
    assert!(decode(&value).is_err());
    let mut value = evidence_value();
    value.as_object_mut().unwrap().remove("target");
    assert!(decode_evidence(&serde_json::to_vec(&value).unwrap()).is_err());
}

#[test]
fn schema_shapes_are_not_mixed() {
    let mut value = inventory_value();
    value.as_object_mut().unwrap().remove("slice");
    assert!(decode(&value).is_err(), "v2 requires a slice");
    let mut value = inventory_value();
    value["schema_version"] = json!(1);
    assert!(
        decode(&value).is_err(),
        "v1 cannot carry a slice or boundaries"
    );
    let mut value = inventory_value();
    value["slice"]["unexpected"] = json!(true);
    assert!(decode(&value).is_err(), "unknown slice fields are refused");
}

#[test]
fn unknown_slice_classes_block_but_stay_distinct_from_invalid() {
    let mut value = inventory_value();
    value["slice"]["classes"] = Value::Null;
    let (_, e) = complete();
    let report = check(&decode(&value).unwrap(), &e);
    assert!(has(&report, "Release slice classes are unknown"));
    assert!(!has(&report, "Release slice is missing or invalid"));
}

#[test]
fn invalid_slices_block() {
    let (_, e) = complete();
    for (field, bad) in [
        ("races", json!([])),
        ("zones", json!(["westfall", "westfall"])),
        ("level_min", json!(0)),
        ("level_min", json!(21)),
        ("level_max", json!(61)),
        ("party_sizes", json!([])),
        ("party_sizes", json!([2, 2])),
        ("party_sizes", json!([6])),
        ("classes", json!([])),
        ("id", json!(" ")),
    ] {
        let mut value = inventory_value();
        value["slice"][field] = bad;
        let report = check(&decode(&value).unwrap(), &e);
        assert!(
            has(&report, "Release slice is missing or invalid"),
            "{field}"
        );
    }
}

#[test]
fn boundaries_must_be_unique_and_explained() {
    let (_, e) = complete();
    let mut value = inventory_value();
    value["domains"][1]["out_of_scope"][0]["id"] = json!("quests-outside-slice");
    assert!(has(
        &check(&decode(&value).unwrap(), &e),
        "Out-of-scope boundaries"
    ));
    let mut value = inventory_value();
    value["domains"][0]["out_of_scope"][0]["reason"] = json!("");
    assert!(has(
        &check(&decode(&value).unwrap(), &e),
        "Out-of-scope boundaries"
    ));
}

#[test]
fn a_boundary_is_not_a_known_unit_and_unknown_units_still_block() {
    let (_, e) = complete();
    let mut value = inventory_value();
    value["domains"][0]["units"][0]["required_record_ids"] = Value::Null;
    let report = check(&decode(&value).unwrap(), &e);
    assert!(has(&report, "Required records are unknown: quests"));
}

#[test]
fn upgrade_keeps_units_and_sources_and_makes_the_target_pending() {
    let v1 = json!({"schema_version":1,"inventory_id":"synthetic-v1","target":Target::forever_70374(),
        "domains": DOMAINS.map(|domain| json!({"domain":domain,"completeness":"unknown",
            "enumeration_evidence":[],"units":[{"id":format!("{domain}-inventory-pending"),
                "scope":"Pending","required_record_ids":null}]})),
        "sources":[]});
    let v1 = decode(&v1).unwrap();
    let slice: Slice = serde_json::from_value(slice()).unwrap();
    let boundaries = BTreeMap::from([(
        "quests".to_owned(),
        vec![Boundary {
            id: "quests-outside".into(),
            scope: "Other races".into(),
            reason: "ADR 0006".into(),
        }],
    )]);
    let v2 = upgrade_inventory(v1.clone(), slice.clone(), boundaries).unwrap();
    assert_eq!(v2.schema_version, 2);
    assert_eq!(v2.target, None);
    assert_eq!(v2.slice.as_ref(), Some(&slice));
    assert_eq!(v2.domains[0].out_of_scope.len(), 1);
    assert!(v2.domains[1..].iter().all(|d| d.out_of_scope.is_empty()));
    assert_eq!(v2.domains[3].units[0].id, v1.domains[3].units[0].id);
    let round_trip = decode(&serde_json::to_value(&v2).unwrap()).unwrap();
    assert_eq!(round_trip.schema_version, 2);
    assert!(upgrade_inventory(v2, slice.clone(), BTreeMap::new()).is_err());
    let unknown = BTreeMap::from([("weather".to_owned(), vec![])]);
    assert!(upgrade_inventory(v1, slice, unknown).is_err());
}

#[test]
fn evidence_upgrade_rebinds_the_inventory_hash() {
    let mut value = evidence_value();
    value["schema_version"] = json!(1);
    value["target"] = serde_json::to_value(Target::forever_70374()).unwrap();
    let v1 = decode_evidence(&serde_json::to_vec(&value).unwrap()).unwrap();
    let hash: azeroth_core::types::Sha256Hex = "c".repeat(64).try_into().unwrap();
    let v2 = upgrade_evidence(v1, hash.clone()).unwrap();
    assert_eq!((v2.schema_version, v2.target.clone()), (2, None));
    assert_eq!(v2.inventory_sha256, hash);
    assert!(upgrade_evidence(v2, hash).is_err());
}
