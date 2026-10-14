use azeroth_app::{planning_jobs::PlanningKind, snapshots::SnapshotApplication};
use azeroth_core::{
    error::ErrorCode,
    search_control::NoopSearchControl,
    types::{DatasetRevision, UtcMillis},
};
use serde_json::{Value, json};
const SNAPSHOT: &str = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
const BUNDLE: &str = include_str!("../../../fixtures/desktop/planning/synthetic-revision.json");
const REFERENCE: &str = include_str!("../../../fixtures/contracts/v1/reference-dataset.json");
const NOW: i64 = 1791792000000;
fn stamp(observation: &mut Value, at: i64) {
    observation["source"] = json!("synthetic_fixture");
    observation["clock_trust"] = json!("trusted");
    observation["observed_at_utc_ms"] = json!(at);
    observation
        .as_object_mut()
        .unwrap()
        .remove("received_at_utc_ms");
}
fn setup(
    change_snapshot: impl FnOnce(&mut Value),
    change_bundle: impl FnOnce(&mut Value),
) -> (SnapshotApplication, Value, UtcMillis) {
    let app = SnapshotApplication::in_memory().unwrap();
    let now = UtcMillis::new(NOW).unwrap();
    let mut snapshot: Value = serde_json::from_str(SNAPSHOT).unwrap();
    snapshot["payload"]["capture"]["observed_at_utc_ms"] = json!(NOW - 100);
    stamp(&mut snapshot["payload"]["level"], NOW - 100);
    stamp(&mut snapshot["payload"]["location"], NOW - 100);
    change_snapshot(&mut snapshot);
    let profile = app
        .import_text(&format!("ANAV1:{snapshot}"), Some(now))
        .unwrap()
        .profile_id;
    let mut bundle: Value = serde_json::from_str(BUNDLE).unwrap();
    change_bundle(&mut bundle);
    app.stage_content(&bundle.to_string()).unwrap();
    app.activate_content(
        &profile,
        &DatasetRevision::new("synthetic-planning-v1").unwrap(),
    )
    .unwrap();
    let catalog = app.planning_catalog(&profile).unwrap();
    let request = json!({"profile_id":profile,"snapshot_id":catalog.snapshot_id,"revision":catalog.revision,"goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":100000,"stopping_service_ms":0,"candidates":[{"objective":"synthetic-service","node":"demo-node-2","utility":10,"completion_ms":1000,"service_ms":0}],"excluded":[]});
    (app, request, now)
}
fn requirement(bundle: &mut Value, predicate: Value) {
    bundle["envelope"]["payload"]["objectives"][0]["requirements"] =
        json!({"status":"known","value":predicate});
}
fn assert_expiry(app: &SnapshotApplication, request: &Value, now: UtcMillis) {
    let (report, guard) = app
        .calculate_planning_job(
            PlanningKind::Solo,
            &request.to_string(),
            Some(now),
            &mut NoopSearchControl,
        )
        .unwrap();
    let report = serde_json::to_value(report).unwrap();
    assert_eq!(report["result"]["selected"].as_array().unwrap().len(), 1);
    let mut published = false;
    let later = UtcMillis::new(now.get() + 100).unwrap();
    let result = guard.publish(app, Some(later), &mut || {
        published = true;
        Ok(())
    });
    assert_eq!(result.unwrap_err().code, ErrorCode::SnapshotOutdated);
    assert!(!published);
    assert!(
        app.plan_session(&request.to_string(), Some(later))
            .unwrap()
            .result
            .selected
            .is_empty()
    );
}
#[test]
fn fresh_capture_cannot_publish_level_eligibility_after_independent_field_expiry() {
    let (app, request, now) = setup(
        |s| stamp(&mut s["payload"]["level"], NOW - 1_799_950),
        |b| requirement(b, json!({"op":"level_at_least","level":22})),
    );
    assert_expiry(&app, &request, now);
}
#[test]
fn nested_equipped_variant_expiry_rejects_eligible_item_comparison() {
    let (app, request, now) = setup(
        |s| {
            stamp(&mut s["payload"]["equipment"]["main_hand"], NOW - 100);
            stamp(
                &mut s["payload"]["equipment"]["main_hand"]["value"]["item"]["variant"],
                NOW - 1_799_950,
            );
        },
        |b| {
            let reference: Value = serde_json::from_str(REFERENCE).unwrap();
            b["envelope"]["payload"]["items"] = reference["payload"]["items"].clone();
            let item = b["envelope"]["payload"]["items"][0]["key"].clone();
            requirement(
                b,
                json!({"op":"equipment_matches","slot":"main_hand","item":item,"modifiers":{}}),
            );
        },
    );
    assert_expiry(&app, &request, now);
}
#[test]
fn nested_quest_progress_expiry_cannot_publish_accepted_quest_requirement() {
    let (app, request, now) = setup(
        |s| {
            stamp(
                &mut s["payload"]["quests"]["demo-quest-1"]["accepted"],
                NOW - 1_799_950,
            )
        },
        |b| {
            let reference: Value = serde_json::from_str(REFERENCE).unwrap();
            let mut quest = reference["payload"]["quests"][0].clone();
            quest["locations"] = json!({"status":"unknown"});
            quest["rewards"] = json!({"status":"unknown"});
            b["envelope"]["payload"]["quests"] = json!([quest]);
            requirement(
                b,
                json!({"op":"quest_state","quest":"demo-quest-1","state":"accepted","value":true}),
            );
        },
    );
    assert_expiry(&app, &request, now);
}
#[test]
fn declared_travel_preference_expiry_rejects_a_previously_usable_route() {
    let (app, request, now) = setup(
        |s| {
            s["payload"]["preferences"]["synthetic-travel-unlock"] =
                json!({"status":"known","value":true});
            stamp(
                &mut s["payload"]["preferences"]["synthetic-travel-unlock"],
                NOW - 1_799_950,
            );
        },
        |b| {
            b["envelope"]["payload"]["edges"][0]["requirements"] = json!({"status":"known","value":{"op":"user_preference","key":"synthetic-travel-unlock","value":true}});
        },
    );
    assert_expiry(&app, &request, now);
}
#[test]
fn unchanged_fresh_classes_preserve_false_zero_and_ignore_extension_metadata() {
    let (app, request, now) = setup(
        |s| {
            s["payload"]["preferences"]["synthetic-false"] =
                json!({"status":"known","value":false});
            stamp(
                &mut s["payload"]["preferences"]["synthetic-false"],
                NOW - 100,
            );
            stamp(&mut s["payload"]["equipment"]["main_hand"], NOW - 100);
            stamp(
                &mut s["payload"]["equipment"]["main_hand"]["value"]["item"]["variant"],
                NOW - 100,
            );
            s["payload"]["equipment"]["main_hand"]["value"]["item"]["modifiers"] =
                json!({"status":"known","value":{"synthetic-stat":0}});
            stamp(
                &mut s["payload"]["equipment"]["main_hand"]["value"]["item"]["modifiers"],
                NOW - 100,
            );
            s["x_metadata_lookalike"] = json!({"status":"known","value":true,"clock_trust":"trusted","observed_at_utc_ms":NOW - 1_799_950});
        },
        |b| {
            let reference: Value = serde_json::from_str(REFERENCE).unwrap();
            b["envelope"]["payload"]["items"] = reference["payload"]["items"].clone();
            let item = b["envelope"]["payload"]["items"][0]["key"].clone();
            requirement(
                b,
                json!({"op":"all","predicates":[{"op":"user_preference","key":"synthetic-false","value":false},{"op":"equipment_matches","slot":"main_hand","item":item,"modifiers":{"synthetic-stat":0}}]}),
            );
        },
    );
    let (report, guard) = app
        .calculate_planning_job(
            PlanningKind::Solo,
            &request.to_string(),
            Some(now),
            &mut NoopSearchControl,
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(report).unwrap()["result"]["selected"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let mut published = false;
    guard
        .publish(&app, Some(UtcMillis::new(NOW + 100).unwrap()), &mut || {
            published = true;
            Ok(())
        })
        .unwrap();
    assert!(published);
    assert_eq!(
        app.plan_session(
            &request.to_string(),
            Some(UtcMillis::new(NOW + 100).unwrap())
        )
        .unwrap()
        .result
        .selected
        .len(),
        1
    );
}
#[test]
fn already_stale_or_unavailable_observations_keep_unchanged_filtered_results() {
    for status in [
        "stale_known",
        "unknown",
        "unsupported",
        "restricted",
        "not_applicable",
        "uncertain_known",
    ] {
        let (app, request, now) = setup(
            |s| {
                if status == "stale_known" {
                    stamp(&mut s["payload"]["level"], NOW - 1_800_100);
                } else if status == "uncertain_known" {
                    s["payload"]["level"]["clock_trust"] = json!("uncertain");
                } else {
                    s["payload"]["level"] = json!({"status":status,"clock_trust":"trusted","observed_at_utc_ms":NOW - 1_799_950});
                }
            },
            |b| requirement(b, json!({"op":"level_at_least","level":22})),
        );
        let (report, guard) = app
            .calculate_planning_job(
                PlanningKind::Solo,
                &request.to_string(),
                Some(now),
                &mut NoopSearchControl,
            )
            .unwrap();
        assert!(
            serde_json::to_value(report).unwrap()["result"]["selected"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let mut published = false;
        guard
            .publish(&app, Some(UtcMillis::new(NOW + 100).unwrap()), &mut || {
                published = true;
                Ok(())
            })
            .unwrap();
        assert!(published);
    }
}
#[test]
fn newly_usable_previously_uncertain_field_requires_recalculation() {
    let (app, request, now) = setup(
        |s| stamp(&mut s["payload"]["level"], NOW + 50),
        |b| requirement(b, json!({"op":"level_at_least","level":22})),
    );
    let (_, guard) = app
        .calculate_planning_job(
            PlanningKind::Solo,
            &request.to_string(),
            Some(now),
            &mut NoopSearchControl,
        )
        .unwrap();
    assert_eq!(
        guard
            .publish(&app, Some(UtcMillis::new(NOW + 100).unwrap()), &mut || Ok(
                ()
            ))
            .unwrap_err()
            .code,
        ErrorCode::SnapshotOutdated
    );
}
