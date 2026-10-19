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

fn overlapping_dungeons() -> (Value, Value) {
    let mut low = world();
    low["sources"][0]["editions"] =
        json!({"status":"known","value":["vanilla_1_12","wow_forever"]});
    low["content"]["reference"]["payload"]["evidence"][0]["confidence"] =
        json!("cross_edition_unverified");
    low["content"]["reference"]["payload"]["evidence"][0]["origin"] =
        json!({"edition":"vanilla_1_12","build":{"status":"known","value":"1.12.1"}});
    low["content"]["dungeons"][0]["id"] = json!("dungeon:the-deadmines");
    low["content"]["dungeons"][0]["checklist"]["value"][0]["id"] = json!("estimated-entry");
    low["content"]["dungeons"][0]["checklist"]["value"][0]["label"] =
        json!("Entrance estimate: ghost-entry XY; portal, access and elevation unverified");
    let mut high = authored();
    high["sources"][0]["id"] = json!("authored-source");
    high["sources"][0]["access"] = json!({"status":"known","value":"authored"});
    high["content"]["reference"]["payload"]["evidence"][0]["source"] = json!("authored-source");
    let mut map = low["content"]["reference"]["payload"]["map_spaces"][0].clone();
    map["key"]["map_id"] = json!("authored:the-deadmines");
    map["key"]["floor"] = json!("authored:pending");
    map["evidence"] = json!(["authored-evidence-1"]);
    high["content"]["reference"]["payload"]["map_spaces"] = json!([map]);
    let mut dungeon = low["content"]["dungeons"][0].clone();
    dungeon["title"] = json!("Authored dungeon title");
    dungeon["map"] = high["content"]["reference"]["payload"]["map_spaces"][0]["key"].clone();
    dungeon["evidence"] = json!(["authored-evidence-1"]);
    for name in [
        "entrance",
        "access",
        "preparation_quests",
        "rewards",
        "travel_edges",
    ] {
        dungeon[name] = json!({"status":"unknown"});
    }
    dungeon["checklist"]["value"][0]["id"] = json!("authored-check");
    dungeon["checklist"]["value"][0]["label"] = json!("Authored readiness declaration");
    dungeon["checklist"]["value"][0]["evidence"] = json!(["authored-evidence-1"]);
    high["content"]["dungeons"] = json!([dungeon]);
    (high, low)
}

#[test]
fn dungeon_enrichment_activates_one_identity_with_visible_estimates_and_conflicts() {
    let (high, low) = overlapping_dungeons();
    let app = SnapshotApplication::in_memory().unwrap();
    let (profile, _, now) = profile(&app);
    app.stage_content(&high.to_string()).unwrap();
    app.stage_content(&low.to_string()).unwrap();
    let set = [revision("authored-r1"), revision("demo-r1")];
    let active = app.activate_content_set(&profile, &set, true).unwrap();
    assert!(active.conflicts.iter().any(|conflict| matches!(
        conflict.key,
        azeroth_core::sources::RecordKey::Dungeon {
            field: azeroth_core::dungeon_composition::DungeonField::Map,
            ..
        }
    )));
    let request = json!({"schema_version":1,"subject":{"kind":"character","profile_id":profile},
        "package":null,"origin":null,"authored":{"revision":active.key.revision,"id":"dungeon:the-deadmines"}});
    let report = app
        .dungeon_preparation(&request.to_string(), Some(now))
        .unwrap();
    let package = report.package.unwrap();
    assert_eq!(package.title.as_str(), "Authored dungeon title");
    assert!(package.entrance.observation.value().is_some());
    let checks = package.checks.observation.value().unwrap();
    assert_eq!(checks.len(), 2);
    assert!(
        checks
            .iter()
            .any(|check| check.label.as_str().contains("elevation unverified"))
    );
    let reversed = [revision("demo-r1"), revision("authored-r1")];
    assert_eq!(
        app.activate_content_set(&profile, &reversed, true)
            .unwrap()
            .key,
        active.key
    );
}

#[test]
fn direct_forever_and_ambiguous_authored_overlaps_refuse_without_changing_active_content() {
    for both_authored in [false, true] {
        let (high, mut low) = overlapping_dungeons();
        if both_authored {
            low["sources"][0]["access"] = json!({"status":"known","value":"authored"});
        } else {
            low["sources"][0]["editions"] = json!({"status":"known","value":["wow_forever"]});
            low["content"]["reference"]["payload"]["evidence"][0]["confidence"] =
                json!("publisher_documented");
            low["content"]["reference"]["payload"]["evidence"][0]
                .as_object_mut()
                .unwrap()
                .remove("origin");
        }
        let app = SnapshotApplication::in_memory().unwrap();
        let (profile, _, _) = profile(&app);
        app.stage_content(&high.to_string()).unwrap();
        app.stage_content(&low.to_string()).unwrap();
        app.activate_content(&profile, &revision("demo-r1"))
            .unwrap();
        assert!(
            app.activate_content_set(
                &profile,
                &[revision("authored-r1"), revision("demo-r1")],
                true
            )
            .is_err()
        );
        assert_eq!(
            app.active_content(&profile).unwrap().unwrap().key.revision,
            revision("demo-r1")
        );
    }
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

#[test]
fn bundled_defaults_apply_before_profile_import_and_upgrade_without_repeating_within_version() {
    let app = SnapshotApplication::in_memory().unwrap();
    let first = app.stage_content(&world().to_string()).unwrap().key;
    let second = app.stage_content(&authored().to_string()).unwrap().key;
    let composed = app
        .stage_content_set(&[first.clone(), second.clone()])
        .unwrap();
    assert!(app.stage_content_set(&[]).is_err());
    assert!(app.configure_bundled_defaults("1.0.0", &[]).is_err());
    assert!(
        app.configure_bundled_defaults("1.0.0", &[first.clone(), second])
            .is_err()
    );
    assert_eq!(
        app.configure_bundled_defaults("1.0.0-rc.1", std::slice::from_ref(&composed))
            .unwrap(),
        [true]
    );
    let (id, _, _) = profile(&app);
    let active = app.active_content(&id).unwrap().unwrap();
    assert_eq!(active.key, composed);
    assert_eq!(active.accepts_inferred_travel, Some(true));
    assert_eq!(active.bundle_defaults.unwrap().app_version, "1.0.0-rc.1");
    app.activate_content(&id, &first.revision).unwrap();
    assert_eq!(
        app.configure_bundled_defaults("1.0.0-rc.1", std::slice::from_ref(&composed))
            .unwrap(),
        [false]
    );
    assert_eq!(app.active_content(&id).unwrap().unwrap().key, first);
    assert_eq!(
        app.configure_bundled_defaults("1.0.0", std::slice::from_ref(&composed))
            .unwrap(),
        [true]
    );
    let active = app.active_content(&id).unwrap().unwrap();
    assert_eq!(active.key, composed);
    assert_eq!(
        active
            .bundle_defaults
            .unwrap()
            .previous_app_version
            .as_deref(),
        Some("1.0.0-rc.1")
    );
    assert_eq!(active.accepts_inferred_travel, Some(true));
    app.rollback_content(&id).unwrap();
    let rolled = app.active_content(&id).unwrap().unwrap();
    assert_eq!(rolled.key, first);
    assert_eq!(rolled.accepts_inferred_travel, Some(false));
}

#[test]
fn inferred_choice_and_default_marker_survive_application_restart() {
    let root = std::env::temp_dir().join(format!(
        "anav-defaults-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("app.sqlite");
    let (id, key) = {
        let app = SnapshotApplication::open(&path).unwrap();
        let key = app.stage_content(&world().to_string()).unwrap().key;
        app.configure_bundled_defaults("1.0.0", std::slice::from_ref(&key))
            .unwrap();
        let (id, _, _) = profile(&app);
        (id, key)
    };
    {
        let app = SnapshotApplication::open(&path).unwrap();
        let active = app.active_content(&id).unwrap().unwrap();
        assert_eq!(active.key, key);
        assert_eq!(active.accepts_inferred_travel, Some(true));
        app.activate_content_set(&id, std::slice::from_ref(&key.revision), false)
            .unwrap();
        assert_eq!(
            app.configure_bundled_defaults("1.0.0", std::slice::from_ref(&key))
                .unwrap(),
            [false]
        );
    }
    {
        let app = SnapshotApplication::open(&path).unwrap();
        assert_eq!(
            app.active_content(&id)
                .unwrap()
                .unwrap()
                .accepts_inferred_travel,
            Some(false)
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_historical_origin_metadata_is_accepted_only_for_unambiguous_admitted_editions() {
    for mixed_foreign in [false, true] {
        let (high, mut low) = overlapping_dungeons();
        low["content"]["reference"]["payload"]["evidence"][0]
            .as_object_mut()
            .unwrap()
            .remove("origin");
        if mixed_foreign {
            low["sources"][0]["editions"] =
                json!({"status":"known","value":["vanilla_1_12","wow_forever","wow_classic_tbc"]});
        }
        let app = SnapshotApplication::in_memory().unwrap();
        let (profile, _, _) = profile(&app);
        app.stage_content(&high.to_string()).unwrap();
        app.stage_content(&low.to_string()).unwrap();
        assert_eq!(
            app.activate_content_set(
                &profile,
                &[revision("authored-r1"), revision("demo-r1")],
                true
            )
            .is_ok(),
            !mixed_foreign
        );
    }
}

#[test]
fn immutable_set_identity_changes_with_content_even_if_external_labels_are_reused() {
    let (high, low) = overlapping_dungeons();
    let app = SnapshotApplication::in_memory().unwrap();
    let (first_profile, _, _) = profile(&app);
    app.stage_content(&high.to_string()).unwrap();
    app.stage_content(&low.to_string()).unwrap();
    let first = app
        .activate_content_set(
            &first_profile,
            &[revision("authored-r1"), revision("demo-r1")],
            false,
        )
        .unwrap();
    let (mut changed, low) = overlapping_dungeons();
    changed["content"]["dungeons"][0]["title"] = json!("Reviewed changed declaration");
    let other = SnapshotApplication::in_memory().unwrap();
    let (profile, _, _) = profile(&other);
    other.stage_content(&changed.to_string()).unwrap();
    other.stage_content(&low.to_string()).unwrap();
    let second = other
        .activate_content_set(
            &profile,
            &[revision("authored-r1"), revision("demo-r1")],
            false,
        )
        .unwrap();
    assert_ne!(first.key.revision, second.key.revision);
}
