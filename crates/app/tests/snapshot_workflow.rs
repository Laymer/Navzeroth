use azeroth_app::{
    DataMode,
    snapshots::{FileInputFormat, HistoryDisposition, ImportDisposition, SnapshotApplication},
};
use azeroth_core::{error::ErrorCode, observation::Freshness};
use serde_json::{Value, json};
const FIXTURE: &str = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
fn doc(id: &str, seq: u64) -> Value {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["payload"]["id"] = json!(id);
    value["producer"]["sequence"] = json!(seq);
    value["payload"]["capture"]["sequence"] = json!(seq);
    value
}
fn text(doc: &Value) -> String {
    format!("ANAV1:{}", serde_json::to_string(doc).unwrap())
}

#[test]
fn manual_party_keeps_individual_freshness_progress_and_ordering() {
    use azeroth_core::types::{PartyId, UtcMillis};
    let app = SnapshotApplication::in_memory().unwrap();
    let mut one = doc("one", 20);
    one["payload"]["capture"]["observed_at_utc_ms"] = json!(1791790000000i64);
    one["payload"]["level"]["value"] = json!(22);
    let first = app.import_text(&text(&one), None).unwrap();
    let mut two = doc("two", 20);
    two["payload"]["character"] = json!("other-character");
    two["payload"]["level"]["value"] = json!(23);
    let second = app.import_text(&text(&two), None).unwrap();
    let id = PartyId::new("manual-duo").unwrap();
    app.save_party(&id, &[first.profile_id.clone(), second.profile_id.clone()])
        .unwrap();
    let party = app
        .party(&id, Some(UtcMillis::new(1791792000000).unwrap()))
        .unwrap()
        .unwrap();
    assert_eq!(party.data_mode, DataMode::Stored);
    let a = party.participants[0].snapshot.as_ref().unwrap();
    let b = party.participants[1].snapshot.as_ref().unwrap();
    assert!(matches!(a.freshness, Freshness::Stale { .. }));
    assert!(matches!(b.freshness, Freshness::Fresh { .. }));
    assert_ne!(a.envelope.payload.level, b.envelope.payload.level);
    one["payload"]["id"] = json!("older");
    one["payload"]["capture"]["sequence"] = json!(19);
    one["producer"]["sequence"] = json!(19);
    app.import_text(&text(&one), None).unwrap();
    assert_eq!(
        app.party(&id, None).unwrap().unwrap().participants[0]
            .snapshot
            .as_ref()
            .unwrap()
            .envelope
            .payload
            .id
            .as_str(),
        "one"
    );
    assert!(
        app.save_party(&id, &[first.profile_id.clone(), "missing".into()])
            .is_err()
    );
    assert!(
        app.save_party(&id, &[first.profile_id.clone(), first.profile_id])
            .is_err()
    );
    assert_eq!(app.party_ids().unwrap(), vec![id.clone()]);
    assert!(app.delete_party(&id).unwrap());
    assert_eq!(app.profiles().unwrap().len(), 2);
    assert!(app.party(&id, None).unwrap().is_none());
}

#[test]
fn manual_party_reports_missing_snapshots_after_reopen_and_profile_deletion() {
    use azeroth_core::{
        contract::decode_envelope,
        entities::CharacterSnapshot,
        observation::Observation,
        types::{PartyId, PreferenceId},
    };
    use azeroth_storage::{ProfileKey, SnapshotStore, SqliteStore};
    let path = std::env::temp_dir().join(format!("anav-party-{}.sqlite", std::process::id()));
    let envelope = decode_envelope::<CharacterSnapshot>(FIXTURE).unwrap();
    let key = ProfileKey::of(&envelope);
    {
        let mut store = SqliteStore::open(&path).unwrap();
        store
            .set_preference(
                &key,
                &PreferenceId::new("manual").unwrap(),
                &Observation::known(false, envelope.payload.capture.clone()).unwrap(),
            )
            .unwrap();
    }
    let id = PartyId::new("missing-duo").unwrap();
    {
        let app = SnapshotApplication::open(&path).unwrap();
        let profile = app.profiles().unwrap()[0].id.clone();
        app.save_party(&id, &[profile]).unwrap();
        assert!(
            app.party(&id, None).unwrap().unwrap().participants[0]
                .snapshot
                .is_none()
        );
        app.import_text(&text(&doc("captured", 20)), None).unwrap();
        assert!(
            app.party(&id, None).unwrap().unwrap().participants[0]
                .snapshot
                .is_some()
        );
    }
    {
        let mut store = SqliteStore::open(&path).unwrap();
        store.delete_profile(&key).unwrap();
    }
    let app = SnapshotApplication::open(&path).unwrap();
    let party = app.party(&id, None).unwrap().unwrap();
    assert_eq!(party.participants[0].character, key.character);
    assert!(party.participants[0].snapshot.is_none());
    assert!(serde_json::to_value(&party).unwrap()["participants"][0]["snapshot"].is_null());
    drop(app);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn text_duplicate_older_same_position_and_bad_input_preserve_current() {
    let app = SnapshotApplication::in_memory().unwrap();
    assert!(app.status().unwrap().active_profile.is_none());
    assert!(app.markers(None).unwrap().is_empty());
    let first = app.import_text(&text(&doc("current", 20)), None).unwrap();
    assert!(matches!(
        first.disposition,
        ImportDisposition::Stored {
            previous_current: None
        }
    ));
    assert!(first.fixture);
    assert!(matches!(first.freshness, Freshness::Uncertain { .. }));
    assert_eq!(first.data_mode, DataMode::Stored);
    assert!(matches!(
        app.import_text(&text(&doc("current", 20)), None)
            .unwrap()
            .disposition,
        ImportDisposition::Duplicate
    ));
    assert!(matches!(
        app.import_text(&text(&doc("old", 19)), None)
            .unwrap()
            .disposition,
        ImportDisposition::HistoryOnly {
            reason: HistoryDisposition::Older
        }
    ));
    assert!(matches!(
        app.import_text(&text(&doc("same", 20)), None)
            .unwrap()
            .disposition,
        ImportDisposition::HistoryOnly {
            reason: HistoryDisposition::SamePosition
        }
    ));
    assert!(app.import_text("ANAV1:{", None).is_err());
    assert_eq!(
        app.current(&first.profile_id, None)
            .unwrap()
            .unwrap()
            .envelope
            .payload
            .id
            .as_str(),
        "current"
    );
    assert_eq!(app.history(&first.profile_id, 32, None).unwrap().len(), 3);
    assert_eq!(
        app.history(&first.profile_id, 0, None).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    let newer = app.import_text(&text(&doc("new", 21)), None).unwrap();
    assert!(matches!(
        newer.disposition,
        ImportDisposition::Stored {
            previous_current: Some(_)
        }
    ));
    let marker = &app.markers(None).unwrap()[0];
    assert_eq!(marker.profile_id, first.profile_id);
    assert_eq!(marker.snapshot_id.as_str(), "new");
    assert_eq!(marker.data_mode, DataMode::Stored);
}

#[test]
fn incomparable_import_stays_in_history() {
    let app = SnapshotApplication::in_memory().unwrap();
    let original = app.import_text(&text(&doc("current", 20)), None).unwrap();
    let mut other = doc("other-session", 1);
    other["producer"]["session_id"] = json!("different-session");
    other["payload"]["capture"]["session_id"] = json!("different-session");
    other["payload"]["capture"]["clock_trust"] = json!("uncertain");
    assert!(matches!(
        app.import_text(&text(&other), None).unwrap().disposition,
        ImportDisposition::HistoryOnly {
            reason: HistoryDisposition::Incomparable
        }
    ));
    assert_eq!(
        app.current(&original.profile_id, None)
            .unwrap()
            .unwrap()
            .envelope
            .payload
            .id
            .as_str(),
        "current"
    );
}

#[test]
fn profile_ids_resolve_full_context_and_never_a_character_only() {
    let app = SnapshotApplication::in_memory().unwrap();
    let demo = app.import_text(&text(&doc("demo", 12)), None).unwrap();
    let reference: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/reference-namespace-fixture.json"
    ))
    .unwrap();
    let synthetic = app.import_text(&text(&reference), None).unwrap();
    let mut real = reference.clone();
    fn real_sources(v: &mut Value) {
        match v {
            Value::Object(map) => {
                if map.get("source") == Some(&json!("synthetic_fixture")) {
                    map.insert("source".into(), json!("addon_snapshot"));
                }
                map.values_mut().for_each(real_sources);
            }
            Value::Array(values) => values.iter_mut().for_each(real_sources),
            _ => {}
        }
    }
    real_sources(&mut real);
    real["fixture"] = json!(false);
    real["producer"]["component"] = json!("addon");
    let production = app.import_text(&text(&real), None).unwrap();
    assert_ne!(demo.profile_id, synthetic.profile_id);
    assert_ne!(synthetic.profile_id, production.profile_id);
    let mut another_build = reference.clone();
    fn change_build(v: &mut Value) {
        match v {
            Value::Object(map) => {
                if map.contains_key("edition")
                    && map.contains_key("build")
                    && map.contains_key("interface")
                {
                    map.insert("edition".into(), json!("wow_classic"));
                    map.insert("build".into(), json!("OTHER_BUILD"));
                    map.insert("interface".into(), json!("OTHER_INTERFACE"));
                }
                map.values_mut().for_each(change_build);
            }
            Value::Array(values) => values.iter_mut().for_each(change_build),
            _ => {}
        }
    }
    change_build(&mut another_build);
    let distinct_build = app.import_text(&text(&another_build), None).unwrap();
    assert_ne!(distinct_build.profile_id, synthetic.profile_id);
    assert_eq!(app.profiles().unwrap().len(), 4);
    app.select_profile(&demo.profile_id).unwrap();
    assert_eq!(
        app.status().unwrap().active_profile.as_deref(),
        Some(demo.profile_id.as_str())
    );
    assert!(app.select_profile("demo-character-paladin").is_err());
    assert!(
        app.select_profile("../../private-export")
            .unwrap_err()
            .to_string()
            .contains("No such profile")
    );
    assert_eq!(
        app.status().unwrap().active_profile.as_deref(),
        Some(demo.profile_id.as_str())
    );
}

#[test]
fn file_import_converges_with_text_and_watcher_can_stop_when_file_missing() {
    let path = std::env::temp_dir().join(format!("anav-app-{}.txt", std::process::id()));
    let contents = text(&doc("file-test", 12));
    std::fs::write(&path, &contents).unwrap();
    let app = SnapshotApplication::in_memory().unwrap();
    let imported = app
        .import_file(&path, &FileInputFormat::Anav1Text, None)
        .unwrap();
    let second = app.import_text(&contents, None).unwrap();
    assert_eq!(imported.profile_id, second.profile_id);
    assert!(matches!(second.disposition, ImportDisposition::Duplicate));
    std::fs::remove_file(&path).unwrap();
    assert!(
        app.start_watch(&path, &FileInputFormat::Anav1Text)
            .unwrap()
            .active
    );
    assert!(!app.stop_watch().unwrap().active);
    assert!(app.watch_status().unwrap().last_import.is_none());
    assert!(app.current(&imported.profile_id, None).unwrap().is_some());
}

#[test]
fn savedvariables_and_anav1_share_storage_and_survive_reopen() {
    let path = std::env::temp_dir().join(format!("anav-app-{}.sqlite", std::process::id()));
    let export = std::env::temp_dir().join(format!("anav-app-{}.lua", std::process::id()));
    let contents = text(&doc("saved-vars", 12));
    std::fs::write(
        &export,
        format!(
            "TestDB = {{ export = {} }}",
            serde_json::to_string(&contents).unwrap()
        ),
    )
    .unwrap();
    let id = {
        let app = SnapshotApplication::open(&path).unwrap();
        let imported = app
            .import_file(
                &export,
                &FileInputFormat::SavedVariables {
                    global: "TestDB".into(),
                    key: "export".into(),
                },
                None,
            )
            .unwrap();
        assert!(matches!(
            app.import_text(&contents, None).unwrap().disposition,
            ImportDisposition::Duplicate
        ));
        imported.profile_id
    };
    let reopened = SnapshotApplication::open(&path).unwrap();
    assert!(reopened.current(&id, None).unwrap().is_some());
    drop(reopened);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(export).unwrap();
}

#[test]
fn watcher_imports_on_its_worker_and_preserves_a_different_selected_profile() {
    let app = SnapshotApplication::in_memory().unwrap();
    let one = app
        .import_text(&text(&doc("one-current", 20)), None)
        .unwrap();
    let mut two = doc("two-current", 20);
    two["payload"]["character"] = json!("other-character");
    let two = app.import_text(&text(&two), None).unwrap();
    let path = std::env::temp_dir().join(format!("anav-app-{}.watch", std::process::id()));
    std::fs::write(&path, text(&doc("watched-new", 21))).unwrap();
    app.start_watch(&path, &FileInputFormat::Anav1Text).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if app.watch_status().unwrap().last_import.is_some() {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "watcher did not deliver a stable import"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let state = app.stop_watch().unwrap();
    assert_eq!(
        state.last_import.unwrap().snapshot_id.as_str(),
        "watched-new"
    );
    assert_eq!(
        app.status().unwrap().active_profile.as_deref(),
        Some(two.profile_id.as_str())
    );
    assert_eq!(
        app.current(&one.profile_id, None)
            .unwrap()
            .unwrap()
            .envelope
            .payload
            .id
            .as_str(),
        "watched-new"
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn historical_lookup_and_guarded_promotion_keep_current_until_explicit_success() {
    use azeroth_core::types::{SnapshotId, UtcMillis};
    let app = SnapshotApplication::in_memory().unwrap();
    let first = app
        .import_text(&text(&doc("history-first", 10)), None)
        .unwrap();
    app.import_text(&text(&doc("history-second", 11)), None)
        .unwrap();
    let old = SnapshotId::new("history-first").unwrap();
    let current = SnapshotId::new("history-second").unwrap();
    let historical = app
        .historical(&first.profile_id, &old, None)
        .unwrap()
        .unwrap();
    assert_eq!(historical.envelope.payload.id, old);
    assert_eq!(historical.data_mode, DataMode::Stored);
    assert!(
        app.historical(
            &first.profile_id,
            &SnapshotId::new("missing").unwrap(),
            None
        )
        .unwrap()
        .is_none()
    );
    assert!(
        app.promote(&first.profile_id, &old, &current, "operator-review", None)
            .is_err()
    );
    let now = Some(UtcMillis::new(1791792000000).unwrap());
    assert!(
        app.promote(&first.profile_id, &old, &old, "operator-review", now)
            .is_err()
    );
    assert_eq!(
        app.current(&first.profile_id, None)
            .unwrap()
            .unwrap()
            .envelope
            .payload
            .id,
        current
    );
    assert!(
        app.promote(&first.profile_id, &old, &current, "operator-review", now)
            .is_err()
    );
    let mut incomparable = doc("history-incomparable", 1);
    incomparable["producer"]["session_id"] = json!("different-session");
    incomparable["payload"]["capture"]["session_id"] = json!("different-session");
    incomparable["payload"]["capture"]["clock_trust"] = json!("uncertain");
    app.import_text(&text(&incomparable), None).unwrap();
    let selected = SnapshotId::new("history-incomparable").unwrap();
    app.promote(
        &first.profile_id,
        &selected,
        &current,
        "operator-review",
        now,
    )
    .unwrap();
    assert_eq!(
        app.current(&first.profile_id, None)
            .unwrap()
            .unwrap()
            .envelope
            .payload
            .id,
        selected
    );
    assert_eq!(
        app.historical(&first.profile_id, &current, None)
            .unwrap()
            .unwrap()
            .envelope
            .payload
            .id,
        current
    );
}
