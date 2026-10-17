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
const CLASSES: [&str; 6] = ["warrior", "paladin", "rogue", "priest", "mage", "warlock"];
const ZONES: [&str; 4] = [
    "northshire_valley",
    "elwynn_forest",
    "westfall",
    "redridge_mountains",
];
/// 6 classes x 2 bands, cycling zones and party sizes so each appears.
fn matrix() -> Vec<Value> {
    let mut rows = vec![];
    for (i, class) in CLASSES.iter().enumerate() {
        for (j, (min, max)) in [(1, 10), (11, 20)].into_iter().enumerate() {
            let n = i * 2 + j;
            rows.push(json!({"id":format!("SLICE-{class}-{min}"),"class":class,
                "level_min":min,"level_max":max,"zone":ZONES[n % 4],"party_size":n % 2 + 1,
                "status":"pass","method":"live_game","operator":"synthetic-test",
                "recorded_at":"2026-10-17T12:00:00Z",
                "environment":"synthetic unit-test record, not actual acceptance","artifacts":[file()]}));
        }
    }
    rows
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
        "packs":[{"manifest":file(),"dataset_revisions":["synthetic-r1"]}],"optional_deferrals":[],
        "matrix":matrix()})
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
fn every_schema_2_domain_must_declare_its_boundaries() {
    let (_, e) = complete();
    let mut value = inventory_value();
    value["domains"][2]
        .as_object_mut()
        .unwrap()
        .remove("out_of_scope");
    let report = check(&decode(&value).unwrap(), &e);
    assert!(has(
        &report,
        "Out-of-scope boundaries are not declared: acquisition"
    ));
    let mut value = inventory_value();
    value["domains"][4]["out_of_scope"] = json!([]);
    let report = check(&decode(&value).unwrap(), &e);
    assert!(has(
        &report,
        "Out-of-scope boundaries are not declared: travel"
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
    value.as_object_mut().unwrap().remove("matrix");
    value["target"] = serde_json::to_value(Target::forever_70374()).unwrap();
    let v1 = decode_evidence(&serde_json::to_vec(&value).unwrap()).unwrap();
    let hash: azeroth_core::types::Sha256Hex = "c".repeat(64).try_into().unwrap();
    let v2 = upgrade_evidence(v1, hash.clone()).unwrap();
    assert_eq!((v2.schema_version, v2.target.clone()), (2, None));
    assert_eq!(v2.inventory_sha256, hash);
    assert!(upgrade_evidence(v2, hash).is_err());
}

fn evidence(value: &Value) -> Evidence {
    decode_evidence(&serde_json::to_vec(value).unwrap()).unwrap()
}

#[test]
fn level_bands_split_the_slice_at_ten_level_boundaries() {
    let mut slice: Slice = serde_json::from_value(slice()).unwrap();
    assert_eq!(level_bands(&slice), [(1, 10), (11, 20)]);
    (slice.level_min, slice.level_max) = (5, 23);
    assert_eq!(level_bands(&slice), [(5, 10), (11, 20), (21, 23)]);
    (slice.level_min, slice.level_max) = (60, 60);
    assert_eq!(level_bands(&slice), [(60, 60)]);
}

#[test]
fn a_full_passing_matrix_adds_no_blocker() {
    let (i, e) = complete();
    assert_eq!(e.matrix.len(), 12);
    assert!(
        !check(&i, &e)
            .blockers
            .iter()
            .any(|b| b.contains("Slice acceptance"))
    );
}

#[test]
fn every_class_band_zone_and_party_size_must_be_covered() {
    let (i, _) = complete();
    let mut value = evidence_value();
    value["matrix"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["id"] != "SLICE-mage-11");
    let report = check(&i, &evidence(&value));
    assert!(has(
        &report,
        "Slice acceptance matrix lacks mage at levels 11-20"
    ));

    let mut value = evidence_value();
    for row in value["matrix"].as_array_mut().unwrap() {
        row["zone"] = json!("elwynn_forest");
        row["party_size"] = json!(1);
    }
    let report = check(&i, &evidence(&value));
    for zone in ["northshire_valley", "westfall", "redridge_mountains"] {
        assert!(has(&report, &format!("lacks zone {zone}")), "{zone}");
    }
    assert!(has(&report, "lacks party size 2"));
    assert!(!has(&report, "lacks zone elwynn_forest"));

    let mut value = evidence_value();
    value["matrix"] = json!([]);
    let report = check(&i, &evidence(&value));
    assert_eq!(
        report
            .blockers
            .iter()
            .filter(|b| b.contains("lacks"))
            .count(),
        12 + 4 + 2
    );
}

#[test]
fn rows_outside_the_slice_or_duplicated_block() {
    let (i, _) = complete();
    for (field, bad) in [
        ("class", json!("druid")),
        ("zone", json!("duskwood")),
        ("party_size", json!(3)),
        ("level_max", json!(9)),
    ] {
        let mut value = evidence_value();
        value["matrix"][0][field] = bad;
        let report = check(&i, &evidence(&value));
        assert!(
            has(
                &report,
                "Slice acceptance row is outside the slice: SLICE-warrior-1"
            ),
            "{field}"
        );
    }
    let mut value = evidence_value();
    let duplicate = value["matrix"][0].clone();
    value["matrix"].as_array_mut().unwrap().push(duplicate);
    assert!(has(
        &check(&i, &evidence(&value)),
        "rows are invalid or duplicated"
    ));
}

#[test]
fn pristine_rows_are_pending_and_partial_or_automated_claims_block() {
    let (i, _) = complete();
    let mut value = evidence_value();
    value["matrix"][0] = json!({"id":"SLICE-warrior-1","class":"warrior","level_min":1,"level_max":10,
        "zone":"northshire_valley","party_size":1,"status":"unverified","method":"live_game",
        "operator":null,"recorded_at":null,"environment":null,"artifacts":[]});
    let report = check(&i, &evidence(&value));
    assert!(has(
        &report,
        "Slice acceptance observation is pending: SLICE-warrior-1"
    ));
    assert!(!report.stable_ready);
    assert!(report.pending.iter().any(|p| matches!(p,
        PendingRequirement::Observation { check_id } if check_id == "SLICE-warrior-1")));
    for (field, bad) in [
        ("method", json!("automated")),
        ("status", json!("fail")),
        ("operator", Value::Null),
        ("artifacts", json!([])),
    ] {
        let mut value = evidence_value();
        value["matrix"][1][field] = bad;
        let report = check(&i, &evidence(&value));
        assert!(
            has(
                &report,
                "Slice acceptance has not passed with recorded environment evidence: SLICE-warrior-11"
            ),
            "{field}"
        );
    }
}

#[test]
fn unknown_classes_block_the_matrix_and_schema_1_cannot_carry_one() {
    let mut inventory = inventory_value();
    inventory["slice"]["classes"] = Value::Null;
    let (_, e) = complete();
    let report = check(&decode(&inventory).unwrap(), &e);
    assert!(has(
        &report,
        "matrix cannot be checked until the slice classes are known"
    ));
    let mut value = evidence_value();
    value["schema_version"] = json!(1);
    assert!(decode_evidence(&serde_json::to_vec(&value).unwrap()).is_err());
}
