use azeroth_core::release::*;
use serde_json::{Value, json};

const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const COMMIT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
fn file() -> Value {
    json!({"path":"evidence/test.txt","bytes":1,"sha256":HASH})
}
fn complete() -> (Inventory, Evidence) {
    let inventory = json!({"schema_version":1,"inventory_id":"synthetic-test-only",
        "target": Target::forever_70374(),
        "domains": DOMAINS.map(|domain| json!({"domain":domain,"completeness":"complete",
            "enumeration_evidence":[file()],"units":[{"id":domain,"scope":"Synthetic scenario",
                "required_record_ids":["synthetic:1"]}]})),
        "sources":[{"id":"synthetic","revision":COMMIT}]});
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
            "operator":"synthetic-test","recorded_at":"2026-10-14T12:00:00Z",
            "environment":"synthetic unit-test record, not actual acceptance","artifacts":[file()]})
        })
        .collect();
    let evidence = json!({"schema_version":1,"target":Target::forever_70374(),
        "inventory_sha256":HASH,"tested_source_commit":COMMIT,"production_sha256":HASH,
        "checks":checks,"coverage": DOMAINS.map(|unit_id| json!({"unit_id":unit_id,
            "status":"pass","included_record_ids":["synthetic:1"],"source_ids":["synthetic"],
            "unresolved_conflicts":0,"unsupported_corrections":0,"validation_evidence":[file()]})),
        "sources":[{"id":"synthetic","revision":COMMIT,"origin_edition":"synthetic",
            "origin_build":null,"redistribution":"permitted","files":[file()],
            "license_files":[file()],"attribution_files":[file()],"permission_evidence":[file()],
            "target_validation":"pass","target_evidence":[file()]}],
        "packs":[{"manifest":file(),"dataset_revisions":["synthetic-r1"]}],"optional_deferrals":[]});
    (
        decode_inventory(&serde_json::to_vec(&inventory).unwrap()).unwrap(),
        decode_evidence(&serde_json::to_vec(&evidence).unwrap()).unwrap(),
    )
}
fn check(version: &str, inventory: &Inventory, evidence: Option<&Evidence>) -> Readiness {
    assess(
        version,
        inventory,
        evidence,
        &Bindings {
            inventory_sha256: HASH,
            production_sha256: HASH,
            tested_source_commit: COMMIT,
        },
    )
    .unwrap()
}

#[test]
fn complete_synthetic_acceptance_exercises_the_success_path_only() {
    let (i, e) = complete();
    let result = check("1.0.0", &i, Some(&e));
    assert!(result.stable_ready && result.publication_allowed);
    assert!(result.blockers.is_empty());
}
#[test]
fn absent_evidence_blocks_stable_but_explicit_preview_keeps_blockers() {
    let (i, _) = complete();
    assert!(!check("1.0.0", &i, None).publication_allowed);
    for version in ["0.8.1", "1.0.0-rc.1"] {
        let result = check(version, &i, None);
        assert!(result.publication_allowed && !result.stable_ready);
        assert!(!result.blockers.is_empty());
    }
}
#[test]
fn invalid_version_cannot_bypass_stable_gate() {
    for version in [
        "v1.0.0",
        "1.0",
        "01.0.0",
        "1.0.0-",
        "1.0.0-rc..1",
        "1.0.0-01",
        "1.0.0+metadata",
        "1.0.0\n",
    ] {
        assert!(is_stable(version).is_err(), "{version}");
    }
    assert!(is_stable("2.0.0").unwrap());
}
#[test]
fn unknown_coverage_differs_from_known_zero_and_missing_domains() {
    let (mut i, mut e) = complete();
    i.domains[0].units[0].required_record_ids = None;
    assert!(!check("1.0.0", &i, Some(&e)).stable_ready);
    i.domains[0].units[0].required_record_ids = Some(vec![]);
    e.coverage[0].included_record_ids.clear();
    assert!(!check("1.0.0", &i, Some(&e)).stable_ready);
    // A real empty subregion stays distinct from an absent inventory; another
    // unit must still enumerate the mandatory domain's available content.
    let (full_i, full_e) = complete();
    let mut nonempty = full_i.domains[0].units[0].clone();
    nonempty.id = "another-region".into();
    i.domains[0].units.push(nonempty);
    let mut nonempty = full_e.coverage[0].clone();
    nonempty.unit_id = "another-region".into();
    e.coverage.push(nonempty);
    assert!(check("1.0.0", &i, Some(&e)).stable_ready);
    i.domains[0].completeness = Completeness::Unknown;
    assert!(!check("1.0.0", &i, Some(&e)).stable_ready);
    i.domains.remove(0);
    assert!(!check("1.0.0", &i, Some(&e)).stable_ready);
}
#[test]
fn all_empty_inventory_and_duplicate_units_fail() {
    let (mut i, mut e) = complete();
    for d in &mut i.domains {
        d.units[0].required_record_ids = Some(vec![]);
    }
    for u in &mut e.coverage {
        u.included_record_ids.clear();
    }
    assert!(!check("1.0.0", &i, Some(&e)).stable_ready);
    let (mut i, e) = complete();
    i.domains[1].units[0].id = i.domains[0].units[0].id.clone();
    assert!(!check("1.0.0", &i, Some(&e)).stable_ready);
}
#[test]
fn every_mandatory_row_rejects_missing_failed_or_automated_live_results() {
    let (i, e) = complete();
    for index in 0..e.checks.len() {
        let mut bad = e.clone();
        bad.checks.remove(index);
        assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
        for status in [Status::Fail, Status::Unverified] {
            let mut bad = e.clone();
            bad.checks[index].status = status;
            assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
        }
        let mut bad = e.clone();
        bad.checks[index].method = Method::Automated;
        assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
        let mut bad = e.clone();
        bad.checks[index].artifacts.clear();
        assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
    }
}
#[test]
fn evidence_cannot_reuse_another_build_inventory_or_tested_revision() {
    let (i, e) = complete();
    let mut bad = e.clone();
    bad.target.build = "70338".into();
    assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
    let mut bad = e.clone();
    bad.inventory_sha256 = "c".repeat(64).try_into().unwrap();
    assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
    let mut bad = e.clone();
    bad.tested_source_commit = Some("c".repeat(40));
    assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
    let mut bad = e;
    bad.production_sha256 = None;
    assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
}
#[test]
fn conflicts_missing_corrections_and_incomplete_records_block_full_content() {
    let (i, e) = complete();
    let mut bad = e.clone();
    bad.coverage[0].unresolved_conflicts = 1;
    assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
    let mut bad = e.clone();
    bad.coverage[0].unsupported_corrections = 1;
    assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
    let mut bad = e.clone();
    bad.coverage[0].included_record_ids.clear();
    assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
    let mut bad = e;
    bad.coverage[0]
        .included_record_ids
        .push("invented:2".into());
    assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
}
#[test]
fn source_revision_permissions_and_required_materials_are_enforced() {
    let (i, e) = complete();
    for permission in [
        Permission::Unknown,
        Permission::Conditional,
        Permission::Prohibited,
    ] {
        let mut bad = e.clone();
        bad.sources[0].redistribution = permission;
        assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
    }
    let mut bad = e.clone();
    bad.sources[0].revision = "c".repeat(40);
    assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
    let mut bad = e.clone();
    bad.sources[0].license_files.clear();
    assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
    let mut bad = e;
    bad.sources[0].target_validation = Status::Unverified;
    assert!(!check("1.0.0", &i, Some(&bad)).stable_ready);
}
#[test]
fn optional_deferrals_cannot_remove_mandatory_checks_or_repeat() {
    let (i, mut e) = complete();
    e.optional_deferrals.push(Deferral {
        feature: OptionalFeature::PartyMessaging,
        reason: "API not verified".into(),
    });
    assert!(check("1.0.0", &i, Some(&e)).stable_ready);
    e.optional_deferrals.push(e.optional_deferrals[0].clone());
    assert!(!check("1.0.0", &i, Some(&e)).stable_ready);
}
#[test]
fn parsing_rejects_unknown_duplicate_oversized_and_executable_inputs() {
    let (i, e) = complete();
    let mut value = serde_json::to_value(&i).unwrap();
    value["optional"] = json!(true);
    assert!(decode_inventory(&serde_json::to_vec(&value).unwrap()).is_err());
    let text = serde_json::to_string(&e).unwrap().replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"schema_version\":1",
        1,
    );
    assert!(decode_evidence(text.as_bytes()).is_err());
    assert!(decode_evidence(&vec![b' '; MAX_RELEASE_BYTES + 1]).is_err());
    assert!(decode_evidence(b"return os.execute('anything')").is_err());
    let mut value = serde_json::to_value(&e).unwrap();
    value["checks"][0]["mandatory"] = json!(false);
    assert!(decode_evidence(&serde_json::to_vec(&value).unwrap()).is_err());
    let mut value = serde_json::to_value(&i).unwrap();
    value["schema_version"] = json!(2);
    assert!(decode_inventory(&serde_json::to_vec(&value).unwrap()).is_err());
}

#[test]
fn future_schema_and_excessive_collections_fail_even_for_prereleases() {
    let (mut i, e) = complete();
    let bindings = Bindings {
        inventory_sha256: HASH,
        production_sha256: HASH,
        tested_source_commit: COMMIT,
    };
    i.schema_version = 2;
    assert!(assess("1.0.0-rc.1", &i, Some(&e), &bindings).is_err());
    i.schema_version = 1;
    let mut excessive = e.clone();
    excessive.coverage = vec![e.coverage[0].clone(); MAX_UNITS + 1];
    assert!(assess("1.0.0-rc.1", &i, Some(&excessive), &bindings).is_err());
}
