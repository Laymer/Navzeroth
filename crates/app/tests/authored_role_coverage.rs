//! Actual original authored weights, with clearly synthetic items/character facts.
//! No live class, proficiency, item-stat or observed performance claim.
use azeroth_app::{authored_scores::GearComparison, snapshots::SnapshotApplication};
use azeroth_core::types::{DatasetRevision, UtcMillis};
use serde_json::{Value, json};
const NOW: i64 = 1791791990100;
fn known(value: Value) -> Value {
    json!({"status":"known","value":value,"clock_trust":"trusted","observed_at_utc_ms":NOW-100,"source":"synthetic_fixture"})
}
fn models() -> Vec<Value> {
    let data: Value = serde_json::from_str(include_str!(
        "../../../content/authored/slice-v1/catalog.json"
    ))
    .unwrap();
    data["models"].as_array().unwrap().clone()
}
fn setup(model: &Value, level: Value, role: Value) -> (SnapshotApplication, Value) {
    setup_edit(model, level, role, |_| {})
}
fn setup_edit(
    model: &Value,
    level: Value,
    role: Value,
    edit: impl FnOnce(&mut Value),
) -> (SnapshotApplication, Value) {
    let app = SnapshotApplication::in_memory().unwrap();
    let mut snapshot: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    snapshot["payload"]["level"] = level;
    snapshot["payload"]["role"] = role;
    let profile = app
        .import_text(
            &format!("ANAV1:{snapshot}"),
            Some(UtcMillis::new(NOW).unwrap()),
        )
        .unwrap()
        .profile_id;
    let mut content: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v2/reference-content.json"
    ))
    .unwrap();
    let mut model = model.clone();
    model["evidence"] = json!(["demo-evidence-1"]);
    model["profile"]["weight_source"]["source"] = json!("demo-source-1");
    content["models"] = json!([model]);
    content["reference"]["payload"]["items"][0]["attributes"] = known(json!({"stat:stamina":10}));
    edit(&mut content);
    let old: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    let bundle = json!({"content":content,"sources":old["sources"],"conflicts":[]});
    app.stage_content(&bundle.to_string()).unwrap();
    app.activate_content(&profile, &DatasetRevision::new("demo-r1").unwrap())
        .unwrap();
    let request = json!({"profile_id":profile,"snapshot_id":snapshot["payload"]["id"],"revision":"demo-r1","authored_model":model["id"],"model":model["profile"],"layout":{"slots":["main_hand"],"hands":null},"class":known(model["class"].clone()),"proficiency":{"classes":{"class:warrior":{"allowed":["invented"],"disallowed":[]}}},"candidates":[{"item":bundle["content"]["reference"]["payload"]["items"][0]["key"],"category":{"status":"not_applicable"},"acquisition":null}]});
    (app, request)
}
#[test]
fn every_authored_class_role_and_level_band_scores_without_implying_equipment_eligibility() {
    for model in models() {
        for boundary in ["minimum_level", "maximum_level"] {
            let (app, request) = setup(
                &model,
                known(model[boundary].clone()),
                known(model["profile"]["role"].clone()),
            );
            let output = app
                .compare_gear_or_score(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                .unwrap();
            let GearComparison::ScoreOnly(scores) = output else {
                panic!("unavailable authored equipment facts must yield score-only")
            };
            assert_eq!(
                scores.candidates[0].score.total,
                model["profile"]["weights"]["stat:stamina"]
                    .as_i64()
                    .unwrap()
                    * 10
            );
            assert_eq!(
                scores.source_layout.status(),
                azeroth_core::observation::ObservationStatus::Unknown
            );
            assert_eq!(
                scores.source_proficiency.status(),
                azeroth_core::observation::ObservationStatus::Unknown
            );
            assert!(
                scores.equipment_eligibility.value().is_none()
                    && scores.acquisition_availability.value().is_none()
            );
            let json = serde_json::to_value(scores).unwrap();
            assert!(
                json.get("practical").is_none()
                    && json.get("theoretical").is_none()
                    && json.get("current_score").is_none()
            );
            // Existing route/practical API remains strict instead of promoting scores.
            assert!(
                app.compare_gear(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                    .is_err()
            );
        }
        if model["minimum_level"].as_u64().unwrap() > 1 {
            let (app, request) = setup(
                &model,
                known(json!(model["minimum_level"].as_u64().unwrap() - 1)),
                known(model["profile"]["role"].clone()),
            );
            assert!(
                app.compare_gear_or_score(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                    .is_err()
            );
        }
        let (app, request) = setup(
            &model,
            known(json!(model["maximum_level"].as_u64().unwrap() + 1)),
            known(model["profile"]["role"].clone()),
        );
        assert!(
            app.compare_gear_or_score(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                .is_err()
        );
    }
}
#[test]
fn score_only_refuses_unknown_wrong_stale_or_restricted_class_level_and_role_inputs() {
    let model = &models()[0];
    let role = known(model["profile"]["role"].clone());
    let (app, request) = setup(model, known(model["minimum_level"].clone()), role.clone());
    for class in [
        json!({"status":"unknown"}),
        json!({"status":"restricted"}),
        json!({"status":"unsupported"}),
        known(json!("class:other")),
    ] {
        let mut changed = request.clone();
        changed["class"] = class;
        assert!(
            app.compare_gear_or_score(&changed.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                .is_err()
        );
    }
    for (level, role) in [
        (json!({"status":"unknown"}), role.clone()),
        (json!({"status":"restricted"}), role.clone()),
        (
            known(model["minimum_level"].clone()),
            json!({"status":"unknown"}),
        ),
        (
            known(model["minimum_level"].clone()),
            known(json!("other-role")),
        ),
    ] {
        let (app, request) = setup(model, level, role);
        assert!(
            app.compare_gear_or_score(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                .is_err()
        );
    }
    assert!(
        app.compare_gear_or_score(
            &request.to_string(),
            Some(UtcMillis::new(NOW + 7_200_000).unwrap())
        )
        .is_err()
    );
    for (field, value) in [
        ("authored_model", json!("missing")),
        (
            "candidates",
            json!(vec![request["candidates"][0].clone(); 129]),
        ),
        (
            "candidates",
            json!([request["candidates"][0], request["candidates"][0]]),
        ),
    ] {
        let mut changed = request.clone();
        changed[field] = value;
        assert!(
            app.compare_gear_or_score(&changed.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                .is_err()
        );
    }
}

#[test]
fn score_only_preserves_unavailable_source_fields_and_incomplete_subtotals() {
    let model = &models()[0];
    for status in ["unknown", "restricted", "unsupported", "not_applicable"] {
        let (app, request) = setup_edit(
            model,
            known(model["minimum_level"].clone()),
            known(model["profile"]["role"].clone()),
            |content| {
                content["models"][0]["layout"] = json!({"status":status});
                content["models"][0]["proficiency"] = json!({"status":status});
                content["reference"]["payload"]["items"][0]["attributes"] =
                    json!({"status":"unknown"});
            },
        );
        let output = app
            .compare_gear_or_score(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
            .unwrap();
        let json = serde_json::to_value(output).unwrap();
        assert_eq!(json["source_layout"]["status"], status);
        assert_eq!(json["source_proficiency"]["status"], status);
        assert_eq!(json["candidates"][0]["score"]["total"], 0);
        assert_ne!(json["candidates"][0]["score"]["certainty"], "complete");
        assert_eq!(json["equipment_eligibility"]["status"], "unknown");
    }
    for (field, value) in [
        ("applicability", json!({"status":"unknown"})),
        ("exclusions", json!({"status":"unknown"})),
        (
            "exclusions",
            known(json!([{"op":"role_is","role":model["profile"]["role"]}])),
        ),
    ] {
        let (app, request) = setup_edit(
            model,
            known(model["minimum_level"].clone()),
            known(model["profile"]["role"].clone()),
            |content| content["models"][0][field] = value,
        );
        assert!(
            app.compare_gear_or_score(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                .is_err()
        );
    }
}
