//! Generated world shape: quest stages, evidence, unknown completion/details,
//! and no persisted profile-specific objectives.
use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::types::{DatasetRevision, UtcMillis};
use serde_json::{Value, json};
fn setup() -> (SnapshotApplication, String, Value, UtcMillis) {
    let app = SnapshotApplication::in_memory().unwrap();
    let mut snapshot: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    let now = UtcMillis::new(
        snapshot["payload"]["capture"]["observed_at_utc_ms"]
            .as_i64()
            .unwrap()
            + 100,
    )
    .unwrap();
    let observed = |value| json!({"status":"known","value":value,"observed_at_utc_ms":now.get()-100,"clock_trust":"trusted","source":"synthetic_fixture"});
    for (field, value) in [
        ("accepted", true),
        ("completed", false),
        ("turned_in", false),
    ] {
        snapshot["payload"]["quests"]["demo-quest-1"][field] = observed(value);
    }
    snapshot["payload"]["objectives"]["demo-objective-1"] = observed(false);
    let profile = app
        .import_text(&format!("ANAV1:{snapshot}"), Some(now))
        .unwrap()
        .profile_id;
    let mut content: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v2/reference-content.json"
    ))
    .unwrap();
    content["reference"]["payload"]["objectives"] = json!([]);
    let quest = &mut content["reference"]["payload"]["quests"][0];
    quest["evidence"] = json!(["demo-evidence-1"]);
    quest["eligibility"] = json!({"status":"known","value":{"op":"level_at_least","level":1}});
    quest["prerequisites"] = json!({"status":"known","value":{"op":"all","predicates":[]}});
    quest["completion"] = json!({"status":"unknown"});
    let facts = &mut content["quest_facts"][0];
    facts["access"] = json!({"status":"known","value":{"op":"all","predicates":[]}});
    facts["access_requirements"] =
        json!({"status":"known","value":{"classes":null,"races":null,"factions":null}});
    let old: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    let bundle = json!({"content":content,"sources":old["sources"],"conflicts":[]});
    (app, profile, bundle, now)
}
fn stage(app: &SnapshotApplication, profile: &str, bundle: &Value) {
    app.stage_content(&bundle.to_string()).unwrap();
    app.activate_content(profile, &DatasetRevision::new("demo-r1").unwrap())
        .unwrap();
}
fn request(app: &SnapshotApplication, profile: &str) -> Value {
    let catalog = app.planning_catalog(profile).unwrap();
    assert_eq!(catalog.objectives.len(), 1);
    let objective = &catalog.objectives[0];
    assert!(objective.expected_ms.value().is_none());
    assert_eq!(objective.evidence[0].as_str(), "demo-evidence-1");
    json!({"profile_id":profile,"snapshot_id":catalog.snapshot_id,"revision":catalog.revision,"goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":100000,"stopping_service_ms":0,"candidates":[{"objective":objective.id,"node":"demo-node-1","utility":10,"completion_ms":1000,"service_ms":0}],"excluded":[]})
}
#[test]
fn generated_stage_can_be_planned_and_exported_without_fabricating_details() {
    let (app, profile, mut bundle, now) = setup();
    bundle["sources"][0]["editions"] =
        json!({"status":"known","value":["vanilla_1_12","wow_forever"]});
    stage(&app, &profile, &bundle);
    let request = request(&app, &profile);
    let origins = app.record_origins(&json!({"subject":{"kind":"character","profile_id":profile},"revision":"demo-r1","records":[{"collection":"objectives","id":request["candidates"][0]["objective"]}]}).to_string()).unwrap();
    assert_eq!(
        origins.origins,
        vec![Some(vec![azeroth_app::world::OriginMark::Vanilla112])]
    );
    let result = app.plan_session(&request.to_string(), Some(now)).unwrap();
    assert_eq!(result.result.selected.len(), 1, "{result:?}");
    let export = app.export_plan(&request.to_string(), Some(now)).unwrap();
    assert!(export.text.contains("planner-quest-stage-projection-v2"));
    let catalog = app.planning_catalog(&profile).unwrap();
    let info = &catalog.pages[&azeroth_core::catalog::Collection::Objectives];
    let page = app.catalog_page(&json!({"subject":{"kind":"character","profile_id":profile},"collection":"objectives","revision":info.revision,"catalog_sha256":info.catalog_sha256,"offset":0}).to_string()).unwrap();
    assert_eq!(page.records[0]["id"], request["candidates"][0]["objective"]);
}
#[test]
fn unavailable_eligibility_prerequisites_and_access_never_become_eligible() {
    for (section, field, value) in [
        ("quest", "eligibility", json!({"status":"unknown"})),
        ("quest", "prerequisites", json!({"status":"restricted"})),
        (
            "quest",
            "eligibility",
            json!({"status":"known","value":{"op":"any","predicates":[]}}),
        ),
        ("facts", "access_requirements", json!({"status":"unknown"})),
        (
            "facts",
            "access_requirements",
            json!({"status":"known","value":{"classes":["class:mage"],"races":null,"factions":null}}),
        ),
    ] {
        let (app, profile, mut bundle, now) = setup();
        if section == "quest" {
            bundle["content"]["reference"]["payload"]["quests"][0][field] = value;
        } else {
            bundle["content"]["quest_facts"][0][field] = value;
        }
        stage(&app, &profile, &bundle);
        let request = request(&app, &profile);
        assert!(
            app.plan_session(&request.to_string(), Some(now))
                .unwrap()
                .result
                .selected
                .is_empty(),
            "{section}/{field}"
        );
    }
}

#[test]
fn projection_pages_are_complete_revision_bound_and_do_not_fill_missing_details() {
    let (app, profile, mut bundle, _) = setup();
    bundle["content"]["reference"]["payload"]["quests"][0]["stages"] =
        json!((0..205).map(|i| format!("stage-{i}")).collect::<Vec<_>>());
    stage(&app, &profile, &bundle);
    let catalog = app.planning_catalog(&profile).unwrap();
    assert_eq!(catalog.objectives.len(), 200);
    let info = &catalog.pages[&azeroth_core::catalog::Collection::Objectives];
    assert_eq!(info.total, 205);
    assert_eq!(info.next_offset, Some(200));
    let mut request = json!({"subject":{"kind":"character","profile_id":profile},"collection":"objectives","revision":info.revision,"catalog_sha256":info.catalog_sha256,"offset":200});
    let last = app.catalog_page(&request.to_string()).unwrap();
    assert_eq!(last.records.len(), 5);
    assert_eq!(last.page.next_offset, None);
    for (field, value) in [
        ("offset", json!(206)),
        ("revision", json!("old")),
        ("catalog_sha256", json!("0".repeat(64))),
    ] {
        let old = request[field].clone();
        request[field] = value;
        assert!(app.catalog_page(&request.to_string()).is_err());
        request[field] = old;
    }
}

#[test]
fn source_free_quests_are_not_promoted_and_declared_objectives_take_precedence() {
    let (app, profile, mut bundle, _) = setup();
    bundle["content"]["reference"]["payload"]["quests"][0]["evidence"] = json!([]);
    stage(&app, &profile, &bundle);
    assert!(
        app.planning_catalog(&profile)
            .unwrap()
            .objectives
            .is_empty()
    );
    let (app, profile, mut bundle, _) = setup();
    let old: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v2/reference-content.json"
    ))
    .unwrap();
    bundle["content"]["reference"]["payload"]["objectives"] =
        old["reference"]["payload"]["objectives"].clone();
    stage(&app, &profile, &bundle);
    let catalog = app.planning_catalog(&profile).unwrap();
    assert_eq!(
        catalog.objectives.len(),
        old["reference"]["payload"]["objectives"]
            .as_array()
            .unwrap()
            .len()
    );
}

#[test]
fn exported_partial_plan_retains_unresolved_stage_readiness() {
    let (app, profile, mut bundle, now) = setup();
    bundle["content"]["reference"]["payload"]["quests"][0]["stages"] =
        json!(["demo-objective-1", "missing-observation"]);
    stage(&app, &profile, &bundle);
    let catalog = app.planning_catalog(&profile).unwrap();
    let candidates: Vec<_> = catalog.objectives.iter().map(|o|json!({"objective":o.id,"node":"demo-node-1","utility":10,"completion_ms":1000,"service_ms":0})).collect();
    let request = json!({"profile_id":profile,"snapshot_id":catalog.snapshot_id,"revision":catalog.revision,"goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":100000,"stopping_service_ms":0,"candidates":candidates,"excluded":[]});
    let result = app.plan_session(&request.to_string(), Some(now)).unwrap();
    assert_eq!(result.result.selected.len(), 1);
    assert_eq!(result.filtered.len(), 1);
    let export = app.export_plan(&request.to_string(), Some(now)).unwrap();
    assert!(export.text.contains("readiness is unknown"));
}
