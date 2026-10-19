//! Content sets (ADR 0007, task V2.8e): the authored slice pack activates
//! beside a generated world pack, and inferred travel is opt-in per activation.
use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::types::{DatasetRevision, UtcMillis};
use serde_json::{Value, json};

const PLANNING: &str = include_str!("../../../fixtures/desktop/planning/synthetic-revision.json");
fn world() -> Value {
    let content: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v2/reference-content.json"
    ))
    .unwrap();
    let old: Value = serde_json::from_str(PLANNING).unwrap();
    json!({"content":content,"sources":old["sources"],"conflicts":[]})
}
/// An authored-style member: its own evidence and models, no game facts.
fn authored() -> Value {
    let mut bundle = world();
    let content = &mut bundle["content"];
    let payload = &mut content["reference"]["payload"];
    let mut evidence = payload["evidence"][0].clone();
    evidence["id"] = json!("authored-evidence-1");
    for (name, values) in payload.as_object_mut().unwrap() {
        if name != "revision" {
            *values = json!([]);
        }
    }
    payload["revision"] = json!("authored-r1");
    payload["evidence"] = json!([evidence]);
    let mut model = content["models"][0].clone();
    model["id"] = json!("authored-model-1");
    model["evidence"] = json!(["authored-evidence-1"]);
    for name in ["equipment", "quest_facts", "acquisition_facts", "dungeons"] {
        content[name] = json!([]);
    }
    content["models"] = json!([model]);
    bundle
}
fn profile(app: &SnapshotApplication) -> (String, Value, UtcMillis) {
    let snapshot: Value = serde_json::from_str(include_str!(
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
    let id = app
        .import_text(&format!("ANAV1:{snapshot}"), Some(now))
        .unwrap()
        .profile_id;
    (id, snapshot, now)
}
fn revision(id: &str) -> DatasetRevision {
    DatasetRevision::new(id).unwrap()
}

#[test]
fn authored_and_world_revisions_activate_as_one_composed_revision() {
    let app = SnapshotApplication::in_memory().unwrap();
    let (profile, _, _) = profile(&app);
    app.stage_content(&world().to_string()).unwrap();
    app.stage_content(&authored().to_string()).unwrap();
    let set = [revision("demo-r1"), revision("authored-r1")];
    let active = app.activate_content_set(&profile, &set, false).unwrap();
    let composed = active.key.revision.as_str().to_owned();
    assert!(composed.starts_with("set-"), "{composed}");
    // Deterministic: the member order does not change the composed identity.
    let reversed = [revision("authored-r1"), revision("demo-r1")];
    let again = app
        .activate_content_set(&profile, &reversed, false)
        .unwrap();
    assert_eq!(again.key.revision.as_str(), composed);
    let current = app.active_content(&profile).unwrap().unwrap();
    assert_eq!(current.key.revision.as_str(), composed);
    assert_eq!(current.report.dataset.unwrap().revision.as_str(), composed);
    let request = json!({"subject":{"kind":"character","profile_id":profile},"kind":"models",
        "revision":null,"offset":0});
    let models = app.authored_catalog(&request.to_string()).unwrap();
    assert_eq!(models.total, 2);
    // A single member activates unchanged; rollback returns to the composed set.
    let single = app
        .activate_content_set(&profile, &[revision("demo-r1")], false)
        .unwrap();
    assert_eq!(single.key.revision.as_str(), "demo-r1");
    assert!(app.rollback_content(&profile).unwrap().changed);
}

#[test]
fn content_sets_refuse_overlaps_missing_members_and_bad_counts() {
    let app = SnapshotApplication::in_memory().unwrap();
    let (profile, _, _) = profile(&app);
    app.stage_content(&world().to_string()).unwrap();
    let mut clash = authored();
    // Same model identity with different content cannot be merged.
    clash["content"]["models"][0]["id"] = json!("model-1");
    clash["content"]["reference"]["payload"]["revision"] = json!("clash-r1");
    app.stage_content(&clash.to_string()).unwrap();
    assert!(
        app.activate_content_set(
            &profile,
            &[revision("demo-r1"), revision("clash-r1")],
            false
        )
        .is_err()
    );
    assert!(
        app.activate_content_set(&profile, &[revision("demo-r1"), revision("absent")], false)
            .is_err()
    );
    assert!(
        app.activate_content_set(&profile, &[revision("demo-r1"), revision("demo-r1")], false)
            .is_err()
    );
    assert!(app.activate_content_set(&profile, &[], false).is_err());
    let many: Vec<_> = (0..9).map(|n| revision(&format!("r{n}"))).collect();
    assert!(app.activate_content_set(&profile, &many, false).is_err());
    let mut source = authored();
    source["sources"][0]["publisher"] = json!("A different registration of the same source");
    source["content"]["reference"]["payload"]["revision"] = json!("resourced-r1");
    app.stage_content(&source.to_string()).unwrap();
    assert!(
        app.activate_content_set(
            &profile,
            &[revision("demo-r1"), revision("resourced-r1")],
            false
        )
        .is_err()
    );
    // A v1 revision cannot join a set, but still activates alone.
    app.stage_content(PLANNING).unwrap();
    let v1 = revision("synthetic-planning-v1");
    assert!(
        app.activate_content_set(&profile, &[revision("demo-r1"), v1.clone()], false)
            .is_err()
    );
    assert_eq!(
        app.activate_content_set(&profile, &[v1], false)
            .unwrap()
            .key
            .revision
            .as_str(),
        "synthetic-planning-v1"
    );
    // Nothing failed above changed the active revision away from a stored one.
    assert!(app.active_content(&profile).unwrap().is_some());
}

#[test]
fn members_must_share_one_game_context() {
    let app = SnapshotApplication::in_memory().unwrap();
    let (profile, _, _) = profile(&app);
    app.stage_content(&world().to_string()).unwrap();
    let mut other = authored();
    other["content"]["reference"]["game"]["locale"] = json!("enUS");
    other["content"]["reference"]["payload"]["revision"] = json!("locale-r1");
    app.stage_content(&other.to_string()).unwrap();
    assert!(
        app.activate_content_set(
            &profile,
            &[revision("demo-r1"), revision("locale-r1")],
            false
        )
        .is_err()
    );
}

/// Both planning edges become generated walk edges: unverified, with
/// `inferred` evidence (ADR 0007 §8).
fn inferred_planning() -> String {
    let mut bundle: Value = serde_json::from_str(PLANNING).unwrap();
    let payload = &mut bundle["envelope"]["payload"];
    payload["evidence"][0]["confidence"] = json!("inferred");
    for edge in payload["edges"].as_array_mut().unwrap() {
        edge["status"] = json!("unverified");
    }
    bundle.to_string()
}

#[test]
fn inferred_travel_is_used_only_after_an_accepting_activation() {
    let app = SnapshotApplication::in_memory().unwrap();
    let (id, _, now) = profile(&app);
    app.stage_content(&inferred_planning()).unwrap();
    let set = [revision("synthetic-planning-v1")];
    let plan = |app: &SnapshotApplication| {
        let catalog = app.planning_catalog(&id).unwrap();
        let request = json!({"profile_id":id,"snapshot_id":catalog.snapshot_id,"revision":catalog.revision,
            "goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1",
            "budget_ms":100000,"stopping_service_ms":0,"candidates":[{"objective":"synthetic-service",
            "node":"demo-node-2","utility":10,"completion_ms":1000,"service_ms":0}],"excluded":[]});
        app.plan_session(&request.to_string(), Some(now))
            .unwrap()
            .result
            .selected
            .len()
    };
    app.activate_content(&id, &set[0]).unwrap();
    assert_eq!(plan(&app), 0, "strict default excludes unverified edges");
    app.activate_content_set(&id, &set, true).unwrap();
    assert_eq!(plan(&app), 1, "accepted inferred edges are routable");
    app.activate_content_set(&id, &set, false).unwrap();
    assert_eq!(plan(&app), 0);
    app.activate_content_set(&id, &set, true).unwrap();
    app.activate_content(&id, &set[0]).unwrap();
    assert_eq!(
        plan(&app),
        0,
        "a plain activation restores the strict default"
    );
    app.activate_content_set(&id, &set, true).unwrap();
    app.rollback_content(&id).unwrap();
    app.activate_content(&id, &set[0]).unwrap();
    assert_eq!(plan(&app), 0);
}

#[test]
fn unknown_travel_requirements_are_assumed_only_under_the_accepting_activation() {
    let app = SnapshotApplication::in_memory().unwrap();
    let (id, _, now) = profile(&app);
    let mut bundle: Value = serde_json::from_str(&inferred_planning()).unwrap();
    for edge in bundle["envelope"]["payload"]["edges"]
        .as_array_mut()
        .unwrap()
    {
        edge["requirements"] = json!({"status":"unknown"});
    }
    app.stage_content(&bundle.to_string()).unwrap();
    let set = [revision("synthetic-planning-v1")];
    let plan = |app: &SnapshotApplication| {
        let catalog = app.planning_catalog(&id).unwrap();
        let request = json!({"profile_id":id,"snapshot_id":catalog.snapshot_id,"revision":catalog.revision,
            "goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1",
            "budget_ms":100000,"stopping_service_ms":0,"candidates":[{"objective":"synthetic-service",
            "node":"demo-node-2","utility":10,"completion_ms":1000,"service_ms":0}],"excluded":[]});
        app.plan_session(&request.to_string(), Some(now)).unwrap()
    };
    app.activate_content(&id, &set[0]).unwrap();
    let strict = plan(&app);
    assert!(strict.result.selected.is_empty());
    assert!(
        strict
            .assumptions
            .iter()
            .any(|a| a.contains("not accept inferred travel"))
    );
    app.activate_content_set(&id, &set, true).unwrap();
    let accepted = plan(&app);
    assert_eq!(accepted.result.selected.len(), 1);
    assert!(accepted.result.selected[0].travel.assumes_requirements());
    assert!(accepted.result.selected[0].travel.is_estimate());
    assert!(
        accepted
            .assumptions
            .iter()
            .any(|a| a.contains("assumed, not checked"))
    );
}
