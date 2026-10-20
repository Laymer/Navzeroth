use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::{
    observation::ObservationStatus,
    types::{DatasetRevision, UtcMillis},
};
use serde_json::{Value, json};
const NOW: i64 = 1791791990100;
fn known(value: Value) -> Value {
    json!({"status":"known","value":value,"source":"synthetic_fixture","clock_trust":"trusted","observed_at_utc_ms":NOW-100})
}
#[test]
fn no_active_authored_catalog_stays_unbound_for_character_and_party() {
    let app = SnapshotApplication::in_memory().unwrap();
    let mut profiles = Vec::new();
    for name in ["first-unbound", "second-unbound"] {
        let mut snapshot: Value = serde_json::from_str(include_str!(
            "../../../fixtures/contracts/v1/character-snapshot.json"
        ))
        .unwrap();
        snapshot["payload"]["character"] = json!(name);
        snapshot["payload"]["id"] = json!(name);
        profiles.push(
            app.import_text(
                &format!("ANAV1:{snapshot}"),
                Some(UtcMillis::new(NOW).unwrap()),
            )
            .unwrap()
            .profile_id,
        );
    }
    let party = azeroth_core::types::PartyId::new("unbound-party").unwrap();
    app.save_party(&party, &profiles).unwrap();
    for subject in [
        json!({"kind":"character","profile_id":profiles[0]}),
        json!({"kind":"party","party_id":party}),
    ] {
        let mut request = json!({"subject":subject,"kind":"models","revision":null,"catalog_sha256":null,"offset":0});
        let empty = app.authored_catalog(&request.to_string()).unwrap();
        assert!(empty.revision.is_none());
        assert!(empty.catalog_sha256.is_none());
        assert!(empty.records.is_empty());
        assert_eq!(empty.total, 0);
        request["revision"] = json!("stale-selection");
        assert!(app.authored_catalog(&request.to_string()).is_err());
    }
    assert!(app.authored_catalog(&" ".repeat(4097)).is_err());
}
fn bundle() -> Value {
    let mut content: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v2/reference-content.json"
    ))
    .unwrap();
    content["models"][0]["applicability"] = known(json!({"op":"all","predicates":[]}));
    content["models"][0]["proficiency"] = known(
        json!({"classes":{"synthetic-class":{"allowed":["synthetic-category"],"disallowed":[]}}}),
    );
    content["models"][0]["layout"] = known(json!({"slots":["main_hand"],"hands":null}));
    content["models"][0]["profile"]["weights"] = json!({"stamina":1000});
    content["equipment"][0]["configuration"] = known(json!({"op":"all","predicates":[]}));
    content["equipment"][0]["category"] = known(json!("synthetic-category"));
    let old: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    json!({"content":content,"sources":old["sources"],"conflicts":[]})
}
fn setup(bundle: &Value) -> (SnapshotApplication, String, Value) {
    let app = SnapshotApplication::in_memory().unwrap();
    let mut snapshot: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    snapshot["payload"]["level"] = known(json!(20));
    snapshot["payload"]["equipment"] = json!({"main_hand":known(json!({"state":"empty"}))});
    let profile = app
        .import_text(
            &format!("ANAV1:{snapshot}"),
            Some(UtcMillis::new(NOW).unwrap()),
        )
        .unwrap()
        .profile_id;
    app.stage_content(&bundle.to_string()).unwrap();
    app.activate_content(&profile, &DatasetRevision::new("demo-r1").unwrap())
        .unwrap();
    let request = json!({"profile_id":profile,"snapshot_id":snapshot["payload"]["id"],"revision":"demo-r1","authored_model":"model-1","model":{"role":"ignored-manual-role","version":"gear-model-v1","weight_source":{"type":"user_preference"},"weights":{},"known_unmodeled":[]},"layout":{"slots":[],"hands":null},"class":known(json!("synthetic-class")),"proficiency":{"classes":{}},"candidates":[{"item":bundle["content"]["reference"]["payload"]["items"][0]["key"],"category":{"status":"unknown"},"acquisition":null}]});
    (app, profile, request)
}
#[test]
fn selected_model_is_consumed_and_manual_override_cannot_replace_authored_weights() {
    let (app, _, request) = setup(&bundle());
    let report = app
        .compare_gear(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap();
    assert_eq!(report.request.model.role.as_str(), "synthetic-role");
    assert_eq!(report.request.model.weights.len(), 1);
    assert_eq!(report.candidates[0].score.total, 0);
    let mut invalid = request.clone();
    invalid["authored_model"] = json!("absent");
    assert!(
        app.compare_gear(&invalid.to_string(), Some(UtcMillis::new(NOW).unwrap()))
            .is_err()
    );
    invalid = request.clone();
    invalid["class"] = json!({"status":"unknown"});
    assert!(
        app.compare_gear(&invalid.to_string(), Some(UtcMillis::new(NOW).unwrap()))
            .is_err()
    );
    assert!(
        app.compare_gear(
            &request.to_string(),
            Some(UtcMillis::new(NOW + 7200000).unwrap())
        )
        .is_err()
    );
}
#[test]
fn authored_dungeon_retains_unknown_sections_and_actual_revision() {
    let (app, profile, _) = setup(&bundle());
    let mut request = json!({"schema_version":1,"subject":{"kind":"character","profile_id":profile},"package":null,"origin":null,"authored":{"revision":"demo-r1","id":"dungeon-1"}});
    let report = app
        .dungeon_preparation(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap();
    assert_eq!(report.authored.unwrap().id.as_str(), "dungeon-1");
    let package = report.package.unwrap();
    assert_eq!(
        package.run_ms.observation.status(),
        ObservationStatus::Unknown
    );
    assert_eq!(
        report.members[0].checks[0].condition.availability,
        ObservationStatus::Unknown
    );
    assert!(report.gaps.iter().all(|g| !matches!(
        g.code,
        azeroth_app::dungeon_preparation::GapCode::LocalPackageUnverified
    )));
    request["authored"]["revision"] = json!("old");
    assert!(
        app.dungeon_preparation(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
            .is_err()
    );
}
#[test]
fn catalog_pages_are_bounded_and_revision_bound() {
    let mut data = bundle();
    let model = data["content"]["models"][0].clone();
    data["content"]["models"] = Value::Array(
        (0..1205)
            .map(|i| {
                let mut v = model.clone();
                v["id"] = json!(format!("model-{i:04}"));
                v
            })
            .collect(),
    );
    let (app, profile, _) = setup(&data);
    let mut request = json!({"subject":{"kind":"character","profile_id":profile},"kind":"models","revision":null,"offset":0});
    let page = app.authored_catalog(&request.to_string()).unwrap();
    assert_eq!(page.records.len(), 200);
    assert_eq!(page.next_offset, Some(200));
    request["offset"] = json!(200);
    assert!(app.authored_catalog(&request.to_string()).is_err());
    request["revision"] = json!("demo-r1");
    request["catalog_sha256"] = json!(page.catalog_sha256);
    assert_eq!(
        app.authored_catalog(&request.to_string())
            .unwrap()
            .records
            .len(),
        200
    );
    request["offset"] = json!(1200);
    assert_eq!(
        app.authored_catalog(&request.to_string())
            .unwrap()
            .records
            .len(),
        5
    );
    request["revision"] = json!("old");
    assert!(app.authored_catalog(&request.to_string()).is_err());
}
#[test]
fn strict_staging_preserves_prior_active_revision_on_malformed_or_duplicate_content() {
    let (app, profile, _) = setup(&bundle());
    let mut bad = bundle();
    bad["content"]["models"][0]["profile"]["extra"] = json!(true);
    assert!(app.stage_content(&bad.to_string()).is_err());
    let duplicate = bundle().to_string().replacen(
        "\"schema_version\":2",
        "\"schema_version\":2,\"schema_version\":2",
        1,
    );
    assert!(app.stage_content(&duplicate).is_err());
    assert_eq!(
        app.active_content(&profile)
            .unwrap()
            .unwrap()
            .key
            .revision
            .as_str(),
        "demo-r1"
    );
}

#[test]
fn generic_catalog_pages_bind_hash_scope_and_reject_unbounded_requests() {
    let (app, profile, _) = setup(&bundle());
    let first = app.gear_catalog(&profile).unwrap();
    let info = &first.pages[&azeroth_core::catalog::Collection::Items];
    let mut request = json!({"subject":{"kind":"character","profile_id":profile},"collection":"items","revision":info.revision,"catalog_sha256":info.catalog_sha256,"offset":0});
    let page = app.catalog_page(&request.to_string()).unwrap();
    assert_eq!(page.records.len(), first.items.len());
    assert_eq!(
        page.records[0],
        serde_json::to_value(&first.items[0]).unwrap()
    );
    request["limit"] = json!(1000);
    assert!(app.catalog_page(&request.to_string()).is_err());
    request.as_object_mut().unwrap().remove("limit");
    request["catalog_sha256"] = json!("0".repeat(64));
    assert!(app.catalog_page(&request.to_string()).is_err());
    request["catalog_sha256"] = json!(info.catalog_sha256);
    request["offset"] = json!(1000);
    assert!(app.catalog_page(&request.to_string()).is_err());
    assert!(app.catalog_page(&" ".repeat(4097)).is_err());
}

#[test]
fn authored_travel_preserves_unavailable_states_and_resolves_known_directed_edges() {
    for status in ["unknown", "restricted", "unsupported", "not_applicable"] {
        let mut data = bundle();
        data["content"]["dungeons"][0]["travel_edges"] = json!({"status":status});
        let (app, profile, _) = setup(&data);
        let request = json!({"schema_version":1,"subject":{"kind":"character","profile_id":profile},"package":null,"origin":null,"authored":{"revision":"demo-r1","id":"dungeon-1"}});
        let report = app
            .dungeon_preparation(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
            .unwrap();
        let travel = report.authored_travel.unwrap();
        assert_eq!(
            serde_json::to_value(travel.declaration.observation).unwrap()["status"],
            status
        );
        assert!(travel.edges.is_empty());
        let gap = report
            .gaps
            .iter()
            .find(|g| g.section == "authored_travel")
            .unwrap();
        assert_eq!(serde_json::to_value(gap.availability).unwrap(), status);
    }
    for empty in [false, true] {
        let mut data = bundle();
        let edge = data["content"]["reference"]["payload"]["edges"][0].clone();
        data["content"]["dungeons"][0]["travel_edges"] = known(if empty {
            json!([])
        } else {
            json!([edge["id"]])
        });
        let (app, profile, _) = setup(&data);
        let request = json!({"schema_version":1,"subject":{"kind":"character","profile_id":profile},"package":null,"origin":null,"authored":{"revision":"demo-r1","id":"dungeon-1"}});
        let report = app
            .dungeon_preparation(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
            .unwrap();
        let travel = report.authored_travel.unwrap();
        assert_eq!(
            travel.declaration.observation.status(),
            ObservationStatus::Known
        );
        assert_eq!(travel.edges.len(), usize::from(!empty));
        assert!(!report.gaps.iter().any(|g| g.section == "authored_travel"));
        if !empty {
            let expected: azeroth_core::entities::TravelEdge =
                serde_json::from_value(edge).unwrap();
            assert_eq!(
                serde_json::to_value(&travel.edges[0]).unwrap(),
                serde_json::to_value(expected).unwrap()
            );
        }
        // Merely referencing directed dependencies does not create a route.
        assert!(report.route.is_none());
    }
}

#[test]
fn authored_dungeon_catalog_uses_exact_declared_identity_title_and_evidence() {
    let data = bundle();
    let (app, profile, _) = setup(&data);
    let request = json!({"subject":{"kind":"character","profile_id":profile},"kind":"dungeons","revision":null,"catalog_sha256":null,"offset":0});
    let page = app.authored_catalog(&request.to_string()).unwrap();
    let declared = &data["content"]["dungeons"][0];
    assert_eq!(page.records.len(), 1);
    assert_eq!(
        page.records[0].id.as_str(),
        declared["id"].as_str().unwrap()
    );
    assert_eq!(page.records[0].label, declared["title"].as_str().unwrap());
    assert_eq!(
        serde_json::to_value(&page.records[0].evidence).unwrap(),
        declared["evidence"]
    );
}

#[test]
fn dungeon_estimate_derivation_remains_visible_and_never_passes_readiness() {
    let mut data = bundle();
    let label = "Entrance estimate: CMaNGOS ghost-entry XY; portal, elevation and walkability unverified. Authored heuristic.";
    data["content"]["dungeons"][0]["checklist"]["value"][0]["label"] = json!(label);
    let (app, profile, _) = setup(&data);
    let request = json!({"schema_version":1,"subject":{"kind":"character","profile_id":profile},
        "package":null,"origin":null,"authored":{"revision":"demo-r1","id":"dungeon-1"}});
    let report = app
        .dungeon_preparation(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap();
    let package = report.package.unwrap();
    assert_eq!(
        package.checks.observation.value().unwrap()[0]
            .label
            .as_str(),
        label
    );
    assert_eq!(
        report.members[0].checks[0].condition.availability,
        ObservationStatus::Unknown
    );
    assert!(report.members[0].checks[0].condition.evaluation.is_none());
}

#[test]
fn full_source_reward_reference_list_survives_preparation_and_keeps_unknown_requirements() {
    let mut data = bundle();
    let acquisition = data["content"]["reference"]["payload"]["acquisitions"][0].clone();
    let mut facts = data["content"]["acquisition_facts"][0].clone();
    facts["rewards"]["value"][0]["quantity"] = json!(1);
    let ids: Vec<_> = (0..1024)
        .map(|i| {
            if i == 0 {
                "demo-acq-1".to_owned()
            } else {
                format!("reward-source-{i}")
            }
        })
        .collect();
    data["content"]["reference"]["payload"]["acquisitions"] = json!(
        ids.iter()
            .map(|id| {
                let mut row = acquisition.clone();
                row["id"] = json!(id);
                row
            })
            .collect::<Vec<_>>()
    );
    data["content"]["acquisition_facts"] = json!(
        ids.iter()
            .map(|id| {
                let mut row = facts.clone();
                row["acquisition"] = json!(id);
                row
            })
            .collect::<Vec<_>>()
    );
    data["content"]["dungeons"][0]["rewards"] = known(json!(ids));
    let (app, profile, _) = setup(&data);
    let request = json!({"schema_version":1,"subject":{"kind":"character","profile_id":profile},"authored":{"revision":"demo-r1","id":"dungeon-1"}});
    let report = app
        .dungeon_preparation(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap();
    assert_eq!(report.package.as_ref().unwrap().schema_version, 3);
    assert_eq!(
        report
            .package
            .as_ref()
            .unwrap()
            .rewards
            .observation
            .value()
            .unwrap()
            .len(),
        1024
    );
    assert_eq!(report.members[0].rewards.len(), 1024);
    assert_eq!(
        report.members[0].rewards.last().unwrap().source.as_str(),
        "reward-source-1023"
    );
    assert_eq!(
        report.members[0].rewards[0]
            .acquisition_requirement
            .availability,
        ObservationStatus::Unknown
    );
    assert!(
        serde_json::to_vec(&report).unwrap().len()
            < azeroth_app::dungeon_preparation::MAX_REPORT_BYTES
    );
    // A catalog source can name multiple items. Bound the expanded references,
    // not just the number of acquisition IDs, before allocating an oversized list.
    let item = data["content"]["reference"]["payload"]["items"][0].clone();
    let items: Vec<_> = (0..5)
        .map(|i| {
            let mut item = item.clone();
            if i > 0 {
                item["key"]["item_id"] = json!(format!("additional-item-{i}"));
            }
            item
        })
        .collect();
    let keys: Vec<_> = items.iter().map(|item| item["key"].clone()).collect();
    data["content"]["reference"]["payload"]["items"] = json!(items);
    for expanded in [4, 5] {
        for source in data["content"]["reference"]["payload"]["acquisitions"]
            .as_array_mut()
            .unwrap()
        {
            source["items"] = json!(&keys[..expanded]);
        }
        let (app, profile, _) = setup(&data);
        let request = json!({"schema_version":1,"subject":{"kind":"character","profile_id":profile},"authored":{"revision":"demo-r1","id":"dungeon-1"}});
        let result =
            app.dungeon_preparation(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()));
        if expanded == 4 {
            let report = result.unwrap();
            assert_eq!(report.members[0].rewards.len(), 4096);
            let bytes = serde_json::to_vec(&report).unwrap().len();
            assert!(bytes < azeroth_app::dungeon_preparation::MAX_REPORT_BYTES);
            eprintln!("Synthetic 4096-reference single-member report: {bytes} bytes");
        } else {
            let error = result.unwrap_err();
            assert_eq!(
                serde_json::to_value(error).unwrap()["message"],
                "Dungeon reward references exceed preparation bounds"
            );
        }
    }
}

#[test]
fn source_guidance_labels_preserve_full_utf8_text_under_new_schema() {
    let mut data = bundle();
    let label = "é".repeat(256);
    data["content"]["dungeons"][0]["checklist"]["value"][0]["label"] = json!(label);
    let (app, profile, _) = setup(&data);
    let request = json!({"schema_version":1,"subject":{"kind":"character","profile_id":profile},"authored":{"revision":"demo-r1","id":"dungeon-1"}});
    let report = app
        .dungeon_preparation(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap();
    let package = report.package.unwrap();
    assert_eq!(package.schema_version, 3);
    assert_eq!(
        package.checks.observation.value().unwrap()[0]
            .label
            .as_str(),
        label
    );
    assert_eq!(
        report.members[0].checks[0].condition.availability,
        ObservationStatus::Unknown
    );
}
