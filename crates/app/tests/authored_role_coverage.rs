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

#[test]
fn explicit_declared_role_enables_only_individual_scores_without_changing_imported_role() {
    for model in models() {
        let (app, mut request) = setup(
            &model,
            known(model["minimum_level"].clone()),
            json!({"status":"unknown"}),
        );
        let profile = request["profile_id"].as_str().unwrap().to_owned();
        let before = serde_json::to_value(
            &app.current(&profile, Some(UtcMillis::new(NOW).unwrap()))
                .unwrap()
                .unwrap()
                .envelope
                .payload
                .role,
        )
        .unwrap();
        assert!(
            app.compare_gear_or_score(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                .is_err()
        );
        request["score_role_assumption"] = model["profile"]["role"].clone();
        let GearComparison::ScoreOnly(result) = app
            .compare_gear_or_score(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
            .unwrap()
        else {
            panic!("declared role may produce only arithmetic")
        };
        assert_eq!(serde_json::to_value(&result.imported_role).unwrap(), before);
        assert_eq!(
            serde_json::to_value(&result.role_assumption).unwrap(),
            model["profile"]["role"]
        );
        assert_eq!(
            serde_json::to_value(
                &app.current(&profile, Some(UtcMillis::new(NOW).unwrap()))
                    .unwrap()
                    .unwrap()
                    .envelope
                    .payload
                    .role
            )
            .unwrap(),
            before
        );
        assert!(
            app.compare_gear(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                .is_err()
        );
    }
    let model = &models()[0];
    for imported in [
        json!({"status":"restricted"}),
        json!({"status":"unsupported"}),
        known(json!("different-imported-role")),
    ] {
        let imported_status = imported["status"].clone();
        let (app, mut request) = setup(model, known(model["minimum_level"].clone()), imported);
        assert!(
            app.compare_gear_or_score(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                .is_err()
        );
        request["score_role_assumption"] = model["profile"]["role"].clone();
        let result = serde_json::to_value(
            app.compare_gear_or_score(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(result["mode"], "authored_score_only_v1");
        assert_eq!(result["imported_role"]["status"], imported_status);
        assert!(result.get("practical").is_none());
    }
}

#[test]
fn declared_roles_do_not_override_class_level_exclusions_or_strict_equipment_paths() {
    let model = &models()[0];
    let positive = known(
        json!({"op":"all","predicates":[{"op":"role_is","role":model["profile"]["role"]},{"op":"any","predicates":[{"op":"level_at_least","level":1}]},{"op":"not","predicate":{"op":"role_is","role":"different"}}]}),
    );
    for (field, value, passes) in [
        ("applicability", positive, true),
        (
            "applicability",
            known(json!({"op":"role_is","role":"different"})),
            false,
        ),
        ("applicability", json!({"status":"unknown"}), false),
        ("exclusions", json!({"status":"unknown"}), false),
        (
            "exclusions",
            known(json!([{"op":"role_is","role":model["profile"]["role"]}])),
            false,
        ),
        (
            "exclusions",
            known(json!([{"op":"role_is","role":"different"}])),
            true,
        ),
        (
            "applicability",
            known(json!({"op":"known","field":{"field":"role"}})),
            false,
        ),
    ] {
        let (app, mut request) = setup_edit(
            model,
            known(model["minimum_level"].clone()),
            json!({"status":"unknown"}),
            |content| content["models"][0][field] = value,
        );
        request["score_role_assumption"] = model["profile"]["role"].clone();
        assert_eq!(
            app.compare_gear_or_score(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                .is_ok(),
            passes,
            "{field}"
        );
    }
    let (app, mut request) = setup(
        model,
        known(model["minimum_level"].clone()),
        json!({"status":"unknown"}),
    );
    request["score_role_assumption"] = json!("wrong-role");
    assert!(
        app.compare_gear_or_score(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
            .is_err()
    );
    request["score_role_assumption"] = model["profile"]["role"].clone();
    request["class"] = json!({"status":"unknown"});
    assert!(
        app.compare_gear_or_score(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
            .is_err()
    );
    for level in [
        json!({"status":"unknown"}),
        json!({"status":"restricted"}),
        known(json!(model["maximum_level"].as_u64().unwrap() + 1)),
    ] {
        let (app, mut request) = setup(model, level, json!({"status":"unknown"}));
        request["score_role_assumption"] = model["profile"]["role"].clone();
        assert!(
            app.compare_gear_or_score(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                .is_err()
        );
    }
    let (app, mut request) = setup_edit(
        model,
        known(model["minimum_level"].clone()),
        json!({"status":"unknown"}),
        |content| {
            content["models"][0]["layout"] = known(json!({"slots":["main_hand"],"hands":null}));
            content["models"][0]["proficiency"] = known(
                json!({"classes":std::collections::BTreeMap::from([(model["class"].as_str().unwrap().to_owned(),json!({"allowed":[],"disallowed":[]}))])}),
            );
        },
    );
    request["score_role_assumption"] = model["profile"]["role"].clone();
    let error = app
        .compare_gear_or_score(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("only to individual heuristic scores")
    );
}
