//! Schema 3 release manifests (ADR 0007, task V2.8e): the target comes from the
//! world-data target file, the LIVE-* rows and the live slice matrix are gone,
//! and evidence binds the accepted-origin policy.
use azeroth_core::{origin_policy::OriginPolicy, release::*, types::Sha256Hex};
use serde_json::{Value, json};

const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const POLICY_HASH: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const COMMIT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
fn repository(path: &str) -> Vec<u8> {
    std::fs::read(format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}
fn target() -> Target {
    decode_world_data_target(&repository("release/v1/world-data-target.json")).unwrap()
}
fn origins() -> OriginPolicy {
    OriginPolicy::decode(&repository("release/v1/accepted-origins.json")).unwrap()
}
fn file() -> Value {
    json!({"path":"evidence/test.txt","bytes":1,"sha256":HASH})
}
fn target_value() -> Value {
    serde_json::to_value(target()).unwrap()
}
fn inventory_value() -> Value {
    json!({"schema_version":3,"inventory_id":"synthetic-v3-test-only","target":target_value(),
        "slice":{"id":"human-1-20","races":["human"],"zones":["elwynn_forest"],"dungeons":[],
            "level_min":1,"level_max":20,"classes":["warrior"],"party_sizes":[1,2]},
        "domains": DOMAINS.map(|domain| json!({"domain":domain,"completeness":"complete",
            "enumeration_evidence":[file()],"units":[{"id":domain,"scope":"Synthetic scenario",
                "required_record_ids":["synthetic:1"]}],
            "out_of_scope":[{"id":format!("{domain}-outside-slice"),"scope":"Everything outside",
                "reason":"Synthetic boundary"}]})),
        "sources":[{"id":"synthetic","revision":COMMIT}]})
}
fn evidence_value() -> Value {
    let checks: Vec<_> = WINDOWS_CHECKS
        .into_iter()
        .map(|id| {
            json!({"id":id,"status":"pass","method":"interactive_windows",
            "operator":"synthetic-test","recorded_at":"2026-10-19T12:00:00Z",
            "environment":"synthetic unit-test record, not actual acceptance","artifacts":[file()]})
        })
        .collect();
    json!({"schema_version":3,"target":target_value(),
        "inventory_sha256":HASH,"admission_registry":file(),"tested_source_commit":COMMIT,"production_sha256":HASH,
        "checks":checks,"coverage": DOMAINS.map(|unit_id| json!({"unit_id":unit_id,
            "status":"pass","included_record_ids":["synthetic:1"],"source_ids":["synthetic"],
            "unresolved_conflicts":0,"unsupported_corrections":0,"validation_evidence":[file()]})),
        "sources":[{"id":"synthetic","revision":COMMIT,"origin_edition":"vanilla_1_12",
            "origin_build":"1.12.1.5875","redistribution":"permitted","files":[file()],
            "license_files":[file()],"attribution_files":[file()],"permission_evidence":[file()],
            "target_validation":"unverified","target_evidence":[]}],
        "packs":[{"manifest":file(),"dataset_revisions":["synthetic-r1"]}],"optional_deferrals":[],
        "origin_policy":{"path":"release/v1/accepted-origins.json","bytes":1,"sha256":POLICY_HASH}})
}
fn decode(inventory: &Value, evidence: &Value) -> (Inventory, Evidence) {
    (
        decode_inventory(&serde_json::to_vec(inventory).unwrap()).unwrap(),
        decode_evidence(&serde_json::to_vec(evidence).unwrap()).unwrap(),
    )
}
fn bindings() -> Bindings<'static> {
    Bindings {
        inventory_sha256: HASH,
        production_sha256: HASH,
        tested_source_commit: COMMIT,
    }
}
fn check_with(i: &Inventory, e: &Evidence, policy: &ReleasePolicy<'_>) -> Readiness {
    assess_with("1.0.0", i, Some(e), &bindings(), policy).unwrap()
}
fn check(i: &Inventory, e: &Evidence) -> Readiness {
    let (target, origins) = (target(), origins());
    check_with(
        i,
        e,
        &ReleasePolicy {
            target: Some(&target),
            origins: Some(&origins),
            origin_policy_sha256: Some(POLICY_HASH),
        },
    )
}
fn has(report: &Readiness, text: &str) -> bool {
    report.blockers.iter().any(|b| b.contains(text))
}

#[test]
fn world_data_target_file_supplies_the_release_target() {
    assert_eq!(
        target(),
        Target {
            edition: "wow_forever".into(),
            version: "1.60.1".into(),
            build: "70450".into(),
            interface: 16001,
        }
    );
    let original: Value =
        serde_json::from_slice(&repository("release/v1/world-data-target.json")).unwrap();
    for (field, replacement) in [
        ("build", json!("1.60.70450")),
        ("build", json!("1.60.1.x")),
        ("interface", json!("16001a")),
        ("interface", json!(16001)),
        ("format", json!("other")),
        ("schema_version", json!(2)),
        ("edition", json!(" ")),
        ("unexpected", json!(true)),
    ] {
        let mut value = original.clone();
        value[field] = replacement;
        assert!(
            decode_world_data_target(&serde_json::to_vec(&value).unwrap()).is_err(),
            "{field}"
        );
    }
    assert!(decode_world_data_target(b"{}").is_err());
}

#[test]
fn a_complete_v3_release_passes_without_live_checks_or_a_frozen_target() {
    let (i, e) = decode(&inventory_value(), &evidence_value());
    let report = check(&i, &e);
    assert!(report.blockers.is_empty(), "{:?}", report.blockers);
    assert!(report.stable_ready && report.publication_allowed);
    assert_eq!(report.report_version, 3);
    assert_eq!(required_checks(3).len(), WINDOWS_CHECKS.len());
    assert_eq!(
        required_checks(2).len(),
        LIVE_CHECKS.len() + WINDOWS_CHECKS.len()
    );
    assert!(
        required_checks(3)
            .iter()
            .all(|(id, method)| id.starts_with("WIN-") && *method == Method::InteractiveWindows)
    );
}

#[test]
fn live_rows_are_no_longer_accepted_in_schema_3_evidence() {
    let mut evidence = evidence_value();
    evidence["checks"].as_array_mut().unwrap().push(
        json!({"id":"LIVE-BUILD","status":"unverified","method":"live_game",
            "operator":null,"recorded_at":null,"environment":null,"artifacts":[]}),
    );
    let (i, e) = decode(&inventory_value(), &evidence);
    assert!(has(&check(&i, &e), "Mandatory acceptance rows"));
    // The WIN-* rows stay mandatory.
    let mut evidence = evidence_value();
    evidence["checks"].as_array_mut().unwrap().pop();
    let (i, e) = decode(&inventory_value(), &evidence);
    assert!(has(&check(&i, &e), "Mandatory acceptance rows"));
}

#[test]
fn the_target_must_equal_the_world_data_target_file() {
    let (i, e) = decode(&inventory_value(), &evidence_value());
    let origins = origins();
    let missing = check_with(
        &i,
        &e,
        &ReleasePolicy {
            target: None,
            origins: Some(&origins),
            origin_policy_sha256: Some(POLICY_HASH),
        },
    );
    assert!(has(&missing, "World-data target file is missing"));
    let mut other = inventory_value();
    other["target"]["build"] = json!("70374");
    let mut evidence = evidence_value();
    evidence["target"] = other["target"].clone();
    let (i, e) = decode(&other, &evidence);
    assert!(has(&check(&i, &e), "frozen target does not match"));
    let mut evidence = evidence_value();
    evidence["target"] = Value::Null;
    let (i, e) = decode(&inventory_value(), &evidence);
    assert!(has(&check(&i, &e), "Acceptance version or target differs"));
}

#[test]
fn evidence_must_bind_the_applied_policy() {
    let (i, e) = decode(&inventory_value(), &evidence_value());
    let target = target();
    let none = check_with(
        &i,
        &e,
        &ReleasePolicy {
            target: Some(&target),
            origins: None,
            origin_policy_sha256: None,
        },
    );
    assert!(has(&none, "Accepted-origin policy is missing"));
    assert!(has(
        &none,
        "does not bind the applied accepted-origin policy"
    ));
    assert!(has(
        &none,
        "Source permission, provenance or target validation"
    ));
    let origins = origins();
    let changed = check_with(
        &i,
        &e,
        &ReleasePolicy {
            target: Some(&target),
            origins: Some(&origins),
            origin_policy_sha256: Some(HASH),
        },
    );
    assert!(has(
        &changed,
        "does not bind the applied accepted-origin policy"
    ));
    let mut other: Value =
        serde_json::from_slice(&repository("release/v1/accepted-origins.json")).unwrap();
    other["target_edition"] = json!("wow_classic_tbc");
    other.as_object_mut().unwrap().remove("historical_builds");
    let other = OriginPolicy::decode(&serde_json::to_vec(&other).unwrap()).unwrap();
    let report = check_with(
        &i,
        &e,
        &ReleasePolicy {
            target: Some(&target),
            origins: Some(&other),
            origin_policy_sha256: Some(POLICY_HASH),
        },
    );
    assert!(has(&report, "different target edition"));
    assert!(origin_accepted_sources(&i, &e, &ReleasePolicy::default()).is_empty());
    // The policy file is a hashed release input like every other evidence file.
    assert!(
        evidence_files(&i, &e)
            .iter()
            .any(|f| f.sha256.as_str() == POLICY_HASH)
    );
}

#[test]
fn an_accepted_origin_satisfies_target_validation_and_others_do_not() {
    let (i, e) = decode(&inventory_value(), &evidence_value());
    let (target, origins) = (target(), origins());
    let policy = ReleasePolicy {
        target: Some(&target),
        origins: Some(&origins),
        origin_policy_sha256: Some(POLICY_HASH),
    };
    assert_eq!(
        origin_accepted_sources(&i, &e, &policy)
            .into_iter()
            .collect::<Vec<_>>(),
        ["synthetic"]
    );
    for origin in ["synthetic", "wow_classic_tbc", "wow_forever"] {
        let mut evidence = evidence_value();
        evidence["sources"][0]["origin_edition"] = json!(origin);
        let (i, e) = decode(&inventory_value(), &evidence);
        assert!(
            has(&check(&i, &e), "target validation is incomplete"),
            "{origin}"
        );
    }
    // A domain citing the source must accept its origin.
    let mut narrow: Value =
        serde_json::from_slice(&repository("release/v1/accepted-origins.json")).unwrap();
    narrow["domains"].as_object_mut().unwrap().remove("travel");
    let narrow = OriginPolicy::decode(&serde_json::to_vec(&narrow).unwrap()).unwrap();
    let report = check_with(
        &i,
        &e,
        &ReleasePolicy {
            target: Some(&target),
            origins: Some(&narrow),
            origin_policy_sha256: Some(POLICY_HASH),
        },
    );
    assert!(has(&report, "target validation is incomplete"));
    // A source no coverage row cites is never accepted by origin.
    let mut evidence = evidence_value();
    for row in evidence["coverage"].as_array_mut().unwrap() {
        row["source_ids"] = json!(["other"]);
    }
    let (i, e) = decode(&inventory_value(), &evidence);
    assert!(origin_accepted_sources(&i, &e, &policy).is_empty());
    // Other proofs stay mandatory for an accepted origin.
    let mut evidence = evidence_value();
    evidence["sources"][0]["license_files"] = json!([]);
    let (i, e) = decode(&inventory_value(), &evidence);
    assert!(has(&check(&i, &e), "target validation is incomplete"));
    // Explicit target validation still passes without the policy's help.
    let mut evidence = evidence_value();
    evidence["sources"][0]["origin_edition"] = json!("wow_forever");
    evidence["sources"][0]["target_validation"] = json!("pass");
    evidence["sources"][0]["target_evidence"] = json!([file()]);
    let (i, e) = decode(&inventory_value(), &evidence);
    assert!(check(&i, &e).blockers.is_empty());
}

#[test]
fn schema_3_shape_is_strict() {
    let mut evidence = evidence_value();
    evidence.as_object_mut().unwrap().remove("origin_policy");
    assert!(decode_evidence(&serde_json::to_vec(&evidence).unwrap()).is_err());
    let mut evidence = evidence_value();
    evidence["matrix"] = json!([{"id":"SLICE-x","class":"warrior","level_min":1,"level_max":10,
        "zone":"elwynn_forest","party_size":1,"status":"unverified","method":"live_game",
        "operator":null,"recorded_at":null,"environment":null,"artifacts":[]}]);
    assert!(decode_evidence(&serde_json::to_vec(&evidence).unwrap()).is_err());
    let mut evidence = evidence_value();
    evidence["schema_version"] = json!(2);
    assert!(decode_evidence(&serde_json::to_vec(&evidence).unwrap()).is_err());
    let mut inventory = inventory_value();
    inventory.as_object_mut().unwrap().remove("slice");
    assert!(decode_inventory(&serde_json::to_vec(&inventory).unwrap()).is_err());
    // Schema 3 keeps the slice and boundary rules of schema 2.
    let mut inventory = inventory_value();
    inventory["domains"][0]["out_of_scope"] = json!([]);
    let (i, e) = decode(&inventory, &evidence_value());
    assert!(has(
        &check(&i, &e),
        "Out-of-scope boundaries are not declared"
    ));
    let mut inventory = inventory_value();
    inventory["slice"]["classes"] = Value::Null;
    let (i, e) = decode(&inventory, &evidence_value());
    let report = check(&i, &e);
    assert!(has(&report, "Release slice classes are unknown"));
    assert!(!has(&report, "matrix"));
}

#[test]
fn explicit_v2_to_v3_migration_drops_live_rows_and_binds_policy() {
    let v2 = json!({"schema_version":2,"inventory_id":"x","target":null,
        "slice":inventory_value()["slice"],"domains":inventory_value()["domains"],
        "sources":inventory_value()["sources"]});
    let inventory = decode_inventory(&serde_json::to_vec(&v2).unwrap()).unwrap();
    let upgraded = upgrade_inventory_v3(inventory.clone(), target()).unwrap();
    assert_eq!(upgraded.schema_version, 3);
    assert_eq!(upgraded.target, Some(target()));
    assert!(upgrade_inventory_v3(upgraded, target()).is_err());
    let mut evidence = evidence_value();
    evidence["schema_version"] = json!(2);
    evidence["target"] = Value::Null;
    evidence.as_object_mut().unwrap().remove("origin_policy");
    let live = LIVE_CHECKS.map(|id| {
        json!({"id":id,"status":"unverified","method":"live_game","operator":null,
        "recorded_at":null,"environment":null,"artifacts":[]})
    });
    evidence["checks"].as_array_mut().unwrap().extend(live);
    let evidence = decode_evidence(&serde_json::to_vec(&evidence).unwrap()).unwrap();
    let policy: FileRef =
        serde_json::from_value(evidence_value()["origin_policy"].clone()).unwrap();
    let upgraded = upgrade_evidence_v3(
        evidence.clone(),
        Sha256Hex::new(HASH).unwrap(),
        target(),
        policy.clone(),
    )
    .unwrap();
    assert_eq!(upgraded.checks.len(), WINDOWS_CHECKS.len());
    assert_eq!(upgraded.origin_policy, Some(policy.clone()));
    assert!(upgraded.matrix.is_empty());
    assert!(
        upgrade_evidence_v3(upgraded, Sha256Hex::new(HASH).unwrap(), target(), policy).is_err()
    );
    let _ = inventory;
}

#[test]
fn record_limit_admits_a_whole_world_inventory_and_refuses_one_more() {
    assert_eq!(MAX_RECORDS, 2_000_000);
    let (mut i, e) = decode(&inventory_value(), &evidence_value());
    // The other eight domains each require one record.
    let ids: Vec<String> = (0..MAX_RECORDS - (DOMAINS.len() - 1))
        .map(|n| format!("r{n}"))
        .collect();
    i.domains[0].units[0].required_record_ids = Some(ids);
    let report = check(&i, &e);
    assert!(
        !has(&report, "empty or excessive"),
        "{:?}",
        &report.blockers[..1]
    );
    i.domains[0].units[0]
        .required_record_ids
        .as_mut()
        .unwrap()
        .push("one-more".into());
    assert!(
        assess_with(
            "1.0.0",
            &i,
            Some(&e),
            &bindings(),
            &ReleasePolicy::default()
        )
        .is_err()
    );
}

#[test]
fn exact_historical_forever_build_can_satisfy_source_target_validation() {
    for (build, accepted) in [
        (Some("1.60.1.70245"), true),
        (Some("1.60.1.70246"), false),
        (None, false),
    ] {
        let mut evidence = evidence_value();
        evidence["sources"][0]["origin_edition"] = json!("wow_forever");
        evidence["sources"][0]["origin_build"] = json!(build);
        let (i, e) = decode(&inventory_value(), &evidence);
        let (target, origins) = (target(), origins());
        let policy = ReleasePolicy {
            target: Some(&target),
            origins: Some(&origins),
            origin_policy_sha256: Some(POLICY_HASH),
        };
        assert_eq!(
            origin_accepted_sources(&i, &e, &policy).contains("synthetic"),
            accepted
        );
        assert_eq!(
            has(&check(&i, &e), "target validation is incomplete"),
            !accepted
        );
    }
}

fn stable_values() -> (Value, Value) {
    let mut inventory = inventory_value();
    let mut evidence = evidence_value();
    inventory["schema_version"] = json!(SCHEMA_STABLE);
    evidence["schema_version"] = json!(SCHEMA_STABLE);
    evidence["candidate_installer_sha256"] = json!(HASH);
    evidence["checks"] =
        json!(required_checks(SCHEMA_STABLE).into_iter().map(|(id, method)| {
        json!({"id":id,"method":method,"status":"pass","operator":"synthetic owner/runner",
            "recorded_at":"2026-10-19T12:00:00Z","environment":"synthetic only",
            "artifacts":[file()],"installer_sha256":HASH})
    }).collect::<Vec<_>>());
    (inventory, evidence)
}

#[test]
fn schema4_requires_hosted_windows_and_separate_exact_installer_owner_smoke() {
    let (inventory, evidence) = stable_values();
    let (i, e) = decode(&inventory, &evidence);
    let report = check(&i, &e);
    assert!(report.stable_ready, "{:?}", report.blockers);
    assert!(report.publication_allowed);
    assert_eq!(report.report_version, 4);
    assert_eq!(required_checks(4).len(), 9);
    assert_eq!(
        required_checks(4).last(),
        Some(&(OWNER_INSTALLER_SMOKE, Method::InteractiveWindows))
    );
    assert!(
        required_checks(4)[..8]
            .iter()
            .all(|(_, m)| *m == Method::HostedWindows)
    );
    assert!(is_world_schema(3) && is_world_schema(4));
    assert!(!is_world_schema(2) && !is_world_schema(5));
}

#[test]
fn schema4_missing_failed_wrong_method_and_mismatched_installer_hold_publication() {
    let (inventory, evidence) = stable_values();
    let mutations = [
        ("status", json!("fail")),
        ("method", json!("automated")),
        ("installer_sha256", json!(POLICY_HASH)),
        ("installer_sha256", Value::Null),
    ];
    for index in [0, 8] {
        for (field, value) in &mutations {
            let mut changed = evidence.clone();
            changed["checks"][index][field] = value.clone();
            let (i, e) = decode(&inventory, &changed);
            let report = check(&i, &e);
            assert!(
                !report.stable_ready && !report.publication_allowed,
                "{index} {field}"
            );
            assert!(has(&report, "Required acceptance has not passed"));
        }
    }
    for field in [
        "candidate_installer_sha256",
        "tested_source_commit",
        "production_sha256",
    ] {
        let mut changed = evidence.clone();
        changed[field] = Value::Null;
        let (i, e) = decode(&inventory, &changed);
        assert!(!check(&i, &e).publication_allowed, "{field}");
    }
    let mut missing = evidence.clone();
    missing["checks"].as_array_mut().unwrap().pop();
    let (i, e) = decode(&inventory, &missing);
    assert!(has(&check(&i, &e), "Mandatory acceptance rows"));
}

#[test]
fn schema4_pending_smoke_is_unverified_and_historical_rows_are_not_promoted() {
    let (inventory, mut evidence) = stable_values();
    evidence["checks"][8] = json!({"id":OWNER_INSTALLER_SMOKE,"method":"interactive_windows",
        "status":"unverified","operator":null,"recorded_at":null,"environment":null,"artifacts":[]});
    let (i, e) = decode(&inventory, &evidence);
    let report = check(&i, &e);
    assert!(!report.publication_allowed);
    assert!(
        report
            .blockers
            .iter()
            .any(|b| b.contains(OWNER_INSTALLER_SMOKE))
    );
    let historical = evidence_value();
    evidence["checks"] = historical["checks"].clone();
    let (i, e) = decode(&inventory, &evidence);
    assert!(has(&check(&i, &e), "Mandatory acceptance rows"));
    // The new method and installer bindings cannot silently enter old contracts.
    for schema in [1, 2, 3] {
        let (_, mut old) = stable_values();
        old["schema_version"] = json!(schema);
        assert!(decode_evidence(&serde_json::to_vec(&old).unwrap()).is_err());
    }
}
