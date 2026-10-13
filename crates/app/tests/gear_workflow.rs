use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::types::{DatasetRevision, UtcMillis};
use serde_json::{Value, json};
const NOW: i64 = 1791791990100;
fn known(value: Value) -> Value {
    json!({"status":"known","value":value,"clock_trust":"trusted","observed_at_utc_ms":NOW - 100,"source":"synthetic_fixture"})
}
fn setup() -> (SnapshotApplication, Value) {
    let app = SnapshotApplication::in_memory().unwrap();
    let mut snapshot: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    snapshot["payload"]["equipment"] = json!({"head":known(json!({"state":"empty"}))});
    let receipt = app
        .import_text(
            &format!("ANAV1:{snapshot}"),
            Some(UtcMillis::new(NOW).unwrap()),
        )
        .unwrap();
    let mut bundle: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    let context = snapshot["payload"]["context"].clone();
    let key = |id: &str| json!({"context":context,"item_id":id,"variant":"base"});
    let item = |id: &str, power: i64| json!({"key":key(id),"slots":["head"],"two_handed":{"status":"not_applicable"},"unique_group":{"status":"not_applicable"},"attributes":known(json!({"synthetic-power":power})),"requirements":known(json!({"op":"all","predicates":[]})),"effects":known(json!([])),"evidence":[]});
    bundle["envelope"]["payload"]["items"] = json!([
        item("synthetic-a", 10),
        item("synthetic-b", 20),
        item("synthetic-c", 30)
    ]);
    bundle["envelope"]["payload"]["acquisitions"] = json!([
        {"id":"synthetic-source-a","context":context,"kind":"other","items":[key("synthetic-a")],"conditions":{"status":"not_applicable"},"cost":{"status":"not_applicable"},"probability":known(json!(1)),"evidence":[]},
        {"id":"synthetic-source-b","context":context,"kind":"other","items":[key("synthetic-b")],"conditions":{"status":"not_applicable"},"cost":{"status":"not_applicable"},"probability":known(json!(0.5)),"evidence":[]}
    ]);
    app.stage_content(&bundle.to_string()).unwrap();
    app.activate_content(
        &receipt.profile_id,
        &DatasetRevision::new("synthetic-planning-v1").unwrap(),
    )
    .unwrap();
    let effort = |ms: i64| json!({"incremental_travel_ms":known(json!(0)),"formation_ms":known(json!(0)),"attempt_ms":known(json!(ms)),"stopping_ms":known(json!(0)),"available_copper":{"status":"unknown"},"session_budget_ms":100,"max_attempts":1,"independent_player_attempts":true,"cost_per_attempt":false});
    let catalog = app.gear_catalog(&receipt.profile_id).unwrap();
    let request = json!({"profile_id":receipt.profile_id,"snapshot_id":catalog.snapshot_id,"revision":catalog.revision,"model":{"role":"synthetic-role","version":"gear-model-v1","weight_source":{"type":"synthetic_test"},"weights":{"synthetic-power":1000},"known_unmodeled":[]},"layout":{"slots":["head"],"hands":null},"class":{"status":"unknown"},"proficiency":{"classes":{}},"candidates":[
        {"item":key("synthetic-a"),"category":{"status":"not_applicable"},"acquisition":{"source":"synthetic-source-a","effort":effort(0)}},
        {"item":key("synthetic-b"),"category":{"status":"not_applicable"},"acquisition":{"source":"synthetic-source-b","effort":effort(10)}},
        {"item":key("synthetic-c"),"category":{"status":"not_applicable"},"acquisition":null}
    ]});
    (app, request)
}
fn now() -> Option<UtcMillis> {
    Some(UtcMillis::new(NOW).unwrap())
}
#[test]
fn equipment_eligibility_is_separate_from_acquisition_and_practical_is_conditional() {
    let (app, request) = setup();
    let result = app.compare_gear(&request.to_string(), now()).unwrap();
    assert_eq!(result.equip_eligible.valid[0].score.total, 30000);
    assert_eq!(result.obtainable.valid[0].score.total, 10000);
    assert_eq!(result.practical.len(), 2);
    assert_eq!(result.practical[0].item.item_id.as_str(), "synthetic-a");
    assert_eq!(result.practical[0].reserved_effort_ms.get(), 0);
    assert!(!result.practical[0].conditional);
    assert!(result.practical[1].conditional);
    assert_eq!(result.practical[1].conditional_expected_gain_milli, 10000);
    assert!(!result.candidates[2].practical_reasons.is_empty());
    let mut reversed = request.clone();
    reversed["candidates"].as_array_mut().unwrap().reverse();
    assert_eq!(
        serde_json::to_value(result.practical).unwrap(),
        serde_json::to_value(
            app.compare_gear(&reversed.to_string(), now())
                .unwrap()
                .practical
        )
        .unwrap()
    );
}
#[test]
fn unknown_probability_zero_false_and_stale_are_preserved() {
    let (app, mut request) = setup();
    request["candidates"][0]["acquisition"]["effort"]["independent_player_attempts"] = json!(false);
    request["candidates"][1]["acquisition"]["effort"]["attempt_ms"] =
        json!({"status":"restricted"});
    let result = app.compare_gear(&request.to_string(), now()).unwrap();
    assert!(result.practical.is_empty());
    assert!(
        result.candidates[0]
            .acquisition
            .as_ref()
            .unwrap()
            .success_probability
            .is_none()
    );
    assert!(
        result.candidates[1]
            .acquisition
            .as_ref()
            .unwrap()
            .budgeted_effort_ms
            .is_none()
    );
    assert_eq!(
        result.candidates[0]
            .acquisition
            .as_ref()
            .unwrap()
            .effort
            .attempt_ms
            .value()
            .unwrap()
            .get(),
        0
    );
    let result = app.compare_gear(&request.to_string(), None).unwrap();
    assert!(!result.baseline_gaps.is_empty());
    assert!(result.practical.is_empty());
}
#[test]
fn identity_replacements_duplicates_and_bounds_fail_safely() {
    let (app, request) = setup();
    for field in ["snapshot_id", "revision"] {
        let mut bad = request.clone();
        bad[field] = json!("replaced");
        assert!(app.compare_gear(&bad.to_string(), now()).is_err());
    }
    let mut bad = request.clone();
    bad["candidates"]
        .as_array_mut()
        .unwrap()
        .push(request["candidates"][0].clone());
    assert!(app.compare_gear(&bad.to_string(), now()).is_err());
    bad["candidates"] = json!(vec![request["candidates"][0].clone(); 129]);
    assert!(app.compare_gear(&bad.to_string(), now()).is_err());
    assert_eq!(
        app.compare_gear(&" ".repeat(1024 * 1024 + 1), now())
            .unwrap_err()
            .code,
        azeroth_core::error::ErrorCode::ImportTooLarge
    );
    bad = request.clone();
    bad["candidates"][0]["item"]["variant"] = json!("unknown-variant");
    assert!(app.compare_gear(&bad.to_string(), now()).is_err());
}
#[test]
fn captured_baseline_modifiers_unknown_variants_and_illegal_replacements_block_advice() {
    let (app, mut request) = setup();
    let mut snapshot = serde_json::to_value(
        app.current(request["profile_id"].as_str().unwrap(), now())
            .unwrap()
            .unwrap()
            .envelope,
    )
    .unwrap();
    snapshot["payload"]["id"] = json!("synthetic-equipped");
    snapshot["payload"]["capture"]["sequence"] = json!(13);
    snapshot["producer"]["sequence"] = json!(13);
    let context = snapshot["payload"]["context"].clone();
    snapshot["payload"]["equipment"] = json!({"head":known(json!({"state":"item","item":{"context":context,"item_id":"synthetic-a","variant":known(json!("base")),"instance":{"status":"unknown"},"modifiers":known(json!({}))}}))});
    let receipt = app
        .import_text(&format!("ANAV1:{snapshot}"), now())
        .unwrap();
    request["snapshot_id"] = json!(receipt.snapshot_id);
    let result = app.compare_gear(&request.to_string(), now()).unwrap();
    assert_eq!(result.current_score.total, 10000);
    assert_eq!(result.practical.len(), 1);
    assert_eq!(result.practical[0].conditional_expected_gain_milli, 5000);
    let mut unknown_eligibility = request.clone();
    unknown_eligibility["candidates"][0]["category"] = known(json!("synthetic-armor"));
    let result = app
        .compare_gear(&unknown_eligibility.to_string(), now())
        .unwrap();
    assert!(result.practical.is_empty());
    assert!(
        result
            .baseline_gaps
            .iter()
            .any(|gap| gap.contains("eligibility"))
    );

    snapshot["payload"]["id"] = json!("synthetic-unknown-modifiers");
    snapshot["payload"]["capture"]["sequence"] = json!(14);
    snapshot["producer"]["sequence"] = json!(14);
    snapshot["payload"]["equipment"]["head"]["value"]["item"]["modifiers"] =
        json!({"status":"unknown"});
    let receipt = app
        .import_text(&format!("ANAV1:{snapshot}"), now())
        .unwrap();
    request["snapshot_id"] = json!(receipt.snapshot_id);
    let result = app.compare_gear(&request.to_string(), now()).unwrap();
    assert!(result.practical.is_empty());
    assert!(
        result
            .baseline_gaps
            .iter()
            .any(|gap| gap.contains("modifiers"))
    );
    snapshot["payload"]["id"] = json!("synthetic-unknown-variant");
    snapshot["payload"]["capture"]["sequence"] = json!(15);
    snapshot["producer"]["sequence"] = json!(15);
    snapshot["payload"]["equipment"]["head"]["value"]["item"]["variant"] =
        json!({"status":"unknown"});
    let receipt = app
        .import_text(&format!("ANAV1:{snapshot}"), now())
        .unwrap();
    request["snapshot_id"] = json!(receipt.snapshot_id);
    let result = app.compare_gear(&request.to_string(), now()).unwrap();
    assert!(!result.current.is_complete());
    assert!(result.practical.is_empty());
}
#[test]
fn unknown_scores_false_requirements_zero_probability_and_configuration_gaps_remain_visible() {
    let (app, request) = setup();
    let profile = request["profile_id"].as_str().unwrap();
    let snapshot = app.current(profile, now()).unwrap().unwrap().envelope;
    let catalog = app.gear_catalog(profile).unwrap();
    let mut bundle: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    bundle["envelope"]["payload"]["items"] = serde_json::to_value(catalog.items).unwrap();
    bundle["envelope"]["payload"]["acquisitions"] =
        serde_json::to_value(catalog.acquisitions).unwrap();
    bundle["envelope"]["payload"]["items"][0]["attributes"] = json!({"status":"unknown"});
    bundle["envelope"]["payload"]["items"][1]["requirements"] =
        known(json!({"op":"level_at_least","level":60}));
    bundle["envelope"]["payload"]["items"][2]["two_handed"] = known(json!(true));
    bundle["envelope"]["payload"]["acquisitions"][0]["probability"] = known(json!(0));
    let content = serde_json::from_value(bundle).unwrap();
    let clock = azeroth_core::observation::FreshnessContext {
        now_utc_ms: now(),
        clock_trust: azeroth_core::observation::ClockTrust::Trusted,
        session: None,
        max_age_ms: azeroth_core::types::Millis::new(10000).unwrap(),
    };
    let result = azeroth_app::gear::calculate(
        serde_json::from_value(request).unwrap(),
        &snapshot,
        &content,
        &clock,
    )
    .unwrap();
    assert!(result.practical.is_empty());
    assert!(result.candidates[0].score.certainty != azeroth_core::gear::Certainty::Complete);
    assert_eq!(
        result.candidates[0]
            .acquisition
            .as_ref()
            .unwrap()
            .success_probability,
        Some(0.0)
    );
    assert!(matches!(
        result.candidates[1].eligibility,
        azeroth_core::gear::GearEligibility::FutureProgression { .. }
    ));
    // A two-handed item without a supplied hand relationship has uncertain legality.
    assert!(
        result.candidates[2]
            .practical_reasons
            .iter()
            .any(|reason| reason.contains("hand-slot"))
    );
}
