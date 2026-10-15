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
