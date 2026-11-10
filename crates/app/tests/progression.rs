use azeroth_app::{progression::*, snapshots::SnapshotApplication};
use azeroth_core::{
    error::ErrorCode,
    observation::{Freshness, StateSource},
    types::UtcMillis,
};
use serde_json::{Value, json};
fn parent(id: &str, sequence: u64) -> Value {
    let mut v: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    v["payload"]["id"] = json!(id);
    v["payload"]["capture"]["sequence"] = json!(sequence);
    v["producer"]["sequence"] = json!(sequence);
    v
}
fn seed(app: &SnapshotApplication) -> String {
    app.import_text(&format!("ANAV1:{}", parent("one", 12)), None)
        .unwrap()
        .profile_id
}
fn request(app: &SnapshotApplication, id: &str) -> Value {
    let view = app.progression_view(id, None).unwrap();
    let u = json!({"status":"unknown"});
    json!({"profile_id":id,"expected":view.binding,"id":"manual-one","spells":u,"money":u,"talents":u,"available_talent_points":u,"professions":u,"recipes":u,"bags":u,"bank":u,"equipment_enhancements":{}})
}
fn known(v: Value) -> Value {
    json!({"status":"known","value":v})
}
fn enumeration(entries: Value) -> Value {
    known(json!({"completeness":"partial","entries":entries}))
}

#[test]
fn manual_full_capture_preserves_zero_false_partial_and_stamps_provenance() {
    let app = SnapshotApplication::in_memory().unwrap();
    let id = seed(&app);
    let mut r = request(&app, &id);
    let item = json!({"context":r["expected"]["context"],"item_id":"item","variant":"base"});
    r["money"] = known(json!("0"));
    r["available_talent_points"] = known(json!(0));
    r["spells"] = enumeration(json!({"spell":0}));
    r["talents"] = enumeration(json!({"talent":0}));
    r["recipes"] = enumeration(json!({"recipe":false}));
    r["professions"] = enumeration(json!({"profession":{"skill":0,"maximum_skill":0,"rank":null}}));
    r["bags"] = enumeration(json!({"stack":{"item":item,"quantity":0}}));
    r["bank"] = known(json!({"completeness":"complete","entries":{}}));
    r["equipment_enhancements"] = json!({"main_hand":known(json!({"item":item,"enhancements":enumeration(json!({"effect":{"duration":"temporary","expires_at_utc_ms":0},"unknown":{"duration":null,"expires_at_utc_ms":null}}))})),"off_hand":{"status":"unknown"}});
    let now = Some(UtcMillis::new(1000).unwrap());
    assert!(
        app.save_manual_progression(&r.to_string(), now)
            .unwrap()
            .stored
    );
    assert!(
        !app.save_manual_progression(&r.to_string(), now)
            .unwrap()
            .stored
    );
    let view = app.progression_view(&id, now).unwrap();
    assert!(view.imported.is_empty());
    assert_eq!(view.manual.len(), 1);
    assert!(matches!(view.manual[0].freshness, Freshness::Fresh { .. }));
    let s = &view.manual[0].snapshot;
    assert_eq!(s.money.value().unwrap().get(), 0);
    assert_eq!(s.money.metadata().source, StateSource::SyntheticFixture);
    assert_eq!(
        s.recipes
            .value()
            .unwrap()
            .entries
            .values()
            .next()
            .unwrap()
            .value(),
        Some(&false)
    );
    assert_eq!(s.bank.value().unwrap().entries.len(), 0);
    assert!(matches!(
        app.progression_view(&id, None).unwrap().manual[0].freshness,
        Freshness::Uncertain { .. }
    ));
    assert!(matches!(
        app.progression_view(&id, Some(UtcMillis::new(2_000_000).unwrap()))
            .unwrap()
            .manual[0]
            .freshness,
        Freshness::Stale { .. }
    ));
}

#[test]
fn request_validation_and_stale_guards_leave_prior_records_unchanged() {
    let app = SnapshotApplication::in_memory().unwrap();
    let id = seed(&app);
    let r = request(&app, &id);
    app.save_manual_progression(&r.to_string(), None).unwrap();
    for (field, value) in [
        ("profile_id", json!("missing")),
        (
            "money",
            json!({"status":"known","value":"1","source":"addon_snapshot"}),
        ),
        ("spells", enumeration(json!({"spell":-1}))),
    ] {
        let mut bad = r.clone();
        bad[field] = value;
        assert!(app.save_manual_progression(&bad.to_string(), None).is_err());
    }
    assert!(app.save_manual_progression("{}", None).is_err());
    assert!(
        app.save_manual_progression("{\"id\":1,\"id\":2}", None)
            .is_err()
    );
    assert!(app.save_manual_progression(&"[".repeat(33), None).is_err());
    assert_eq!(
        app.save_manual_progression(&"x".repeat(MAX_PROGRESSION_REQUEST + 1), None)
            .unwrap_err()
            .code,
        ErrorCode::ImportTooLarge
    );
    let mut bad = r.clone();
    bad["professions"] = enumeration(json!({"p":{"skill":2,"maximum_skill":1,"rank":null}}));
    assert!(app.save_manual_progression(&bad.to_string(), None).is_err());
    assert_eq!(app.progression_view(&id, None).unwrap().manual.len(), 1);
    app.import_text(&format!("ANAV1:{}", parent("two", 13)), None)
        .unwrap();
    assert_eq!(
        app.save_manual_progression(&r.to_string(), None)
            .unwrap_err()
            .code,
        ErrorCode::SnapshotOutdated
    );
    assert!(app.progression_view(&id, None).unwrap().manual.is_empty());
}

#[test]
fn imported_companion_requires_opt_in_identity_and_stays_separate() {
    let app = SnapshotApplication::in_memory().unwrap();
    let id = seed(&app);
    let r = request(&app, &id);
    app.save_manual_progression(&r.to_string(), None).unwrap();
    let view = app.progression_view(&id, None).unwrap();
    let mut envelope: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/progression-snapshot.json"
    ))
    .unwrap();
    envelope["payload"] = serde_json::to_value(&view.manual[0].snapshot).unwrap();
    envelope["producer"]
        .as_object_mut()
        .unwrap()
        .remove("session_id");
    envelope["producer"]
        .as_object_mut()
        .unwrap()
        .remove("sequence");
    envelope["dependencies"]["snapshots"][0]["id"] = json!("one");
    let input = json!({"profile_id":id,"expected":view.binding,"text":format!("ANAV1:{envelope}")});
    let first = app
        .import_progression_text(&input.to_string(), None)
        .unwrap();
    assert!(first.stored);
    assert!(
        !app.import_progression_text(&input.to_string(), None)
            .unwrap()
            .stored
    );
    let view = app.progression_view(&id, None).unwrap();
    assert_eq!(view.manual.len(), 1);
    assert_eq!(view.imported.len(), 1);
    let mut bad = input.clone();
    bad["text"] = json!(format!("ANAV1:{}", parent("one", 12)));
    assert!(app.import_progression_text(&bad.to_string(), None).is_err());
    let mut bad = input.clone();
    bad["expected"]["snapshot_sha256"] = json!("a".repeat(64));
    assert_eq!(
        app.import_progression_text(&bad.to_string(), None)
            .unwrap_err()
            .code,
        ErrorCode::SnapshotOutdated
    );
    envelope["payload"]["binding"]["snapshot_sha256"] = json!("b".repeat(64));
    bad = input;
    bad["text"] = json!(format!("ANAV1:{envelope}"));
    assert_eq!(
        app.import_progression_text(&bad.to_string(), None)
            .unwrap_err()
            .code,
        ErrorCode::SnapshotOutdated
    );
}

#[test]
fn production_manual_source_is_assigned_by_service_and_survives_restart() {
    let path = std::env::temp_dir().join(format!(
        "navzeroth-progression-app-{}.sqlite",
        std::process::id()
    ));
    let app = SnapshotApplication::open(&path).unwrap();
    let mut p = parent("production", 12);
    p["fixture"] = json!(false);
    p["namespace"] = json!("reference");
    p["payload"]["context"]["namespace"] = json!("reference");
    p["producer"]["component"] = json!("addon");
    let text = format!("ANAV1:{p}")
        .replace("synthetic_fixture", "addon_snapshot")
        .replace("\"namespace\":\"demo\"", "\"namespace\":\"reference\"");
    let id = app.import_text(&text, None).unwrap().profile_id;
    let mut r = request(&app, &id);
    r["money"] = known(json!("0"));
    app.save_manual_progression(&r.to_string(), None).unwrap();
    drop(app);
    let app = SnapshotApplication::open(&path).unwrap();
    let view = app.progression_view(&id, None).unwrap();
    assert!(!view.fixture);
    assert_eq!(
        view.manual[0].snapshot.money.metadata().source,
        StateSource::ManualEntry
    );
    assert_eq!(
        view.manual[0].snapshot.capture.source,
        StateSource::ManualEntry
    );
    drop(app);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn generated_progression_api_schema_matches_committed() {
    assert_eq!(
        api_schema_text(),
        include_str!("../../../schemas/v1/progression-api.schema.json")
    );
}
