use azeroth_app::{party::JointStatus, snapshots::SnapshotApplication};
use azeroth_core::{
    contract::decode_envelope,
    types::{DatasetRevision, PartyId, QuestId, UtcMillis},
};
use azeroth_storage::{SqliteStore, content::ContentRevision};
use serde_json::{Value, json};
const NOW: i64 = 1791792000000;

struct AppDb {
    app: SnapshotApplication,
    path: std::path::PathBuf,
}
impl Drop for AppDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
fn setup(content: bool) -> AppDb {
    let path = std::env::temp_dir().join(format!(
        "anav-eligibility-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut store = SqliteStore::open(&path).unwrap();
    if content {
        let mut revision: ContentRevision = serde_json::from_str(include_str!(
            "../../../fixtures/desktop/content/demo-revision.json"
        ))
        .unwrap();
        let mut value = serde_json::to_value(&revision.envelope).unwrap();
        value["payload"]["quests"][0]["eligibility"] = json!({"status":"known","value":{"op":"level_at_least","level":23},"source":"reference_data"});
        value["payload"]["quests"][0]["prerequisites"] = json!({"status":"known","value":{"op":"all","predicates":[]},"source":"reference_data"});
        revision.envelope = decode_envelope(&value.to_string()).unwrap();
        store.stage_content_revision(&revision).unwrap();
        store.activate_content_revision(&revision.key()).unwrap();
    }
    drop(store);
    AppDb {
        app: SnapshotApplication::open(&path).unwrap(),
        path,
    }
}
fn import(app: &SnapshotApplication, who: &str, level: u16, capture: i64) -> String {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    value["payload"]["id"] = json!(format!("snapshot-{who}"));
    value["payload"]["character"] = json!(who);
    value["payload"]["level"]["value"] = json!(level);
    value["payload"]["capture"]["observed_at_utc_ms"] = json!(capture);
    fn date(value: &mut Value) {
        match value {
            Value::Object(map) => {
                if map.get("status") == Some(&json!("known")) {
                    map.insert("clock_trust".into(), json!("trusted"));
                    map.insert("observed_at_utc_ms".into(), json!(NOW));
                }
                map.values_mut().for_each(date);
            }
            Value::Array(v) => v.iter_mut().for_each(date),
            _ => {}
        }
    }
    date(&mut value["payload"]);
    app.import_text(&format!("ANAV1:{value}"), None)
        .unwrap()
        .profile_id
}
fn id() -> PartyId {
    PartyId::new("duo").unwrap()
}
fn report(app: &SnapshotApplication) -> azeroth_app::party::PartyQuestReport {
    app.party_eligibility(
        &id(),
        &DatasetRevision::new("demo-r1").unwrap(),
        &QuestId::new("demo-quest-1").unwrap(),
        Some(UtcMillis::new(NOW).unwrap()),
    )
    .unwrap()
}

#[test]
fn participants_get_independent_acceptance_and_catch_up_with_exact_inputs() {
    let db = setup(true);
    let one = import(&db.app, "one", 22, NOW);
    let two = import(&db.app, "two", 23, NOW);
    db.app.save_party(&id(), &[one, two]).unwrap();
    let result = report(&db.app);
    assert!(matches!(result.joint_status, JointStatus::EligibleForSome));
    assert!(
        result.participants[0]
            .assessment
            .as_ref()
            .unwrap()
            .catch_up
            .is_some()
    );
    assert!(
        result.participants[1]
            .assessment
            .as_ref()
            .unwrap()
            .catch_up
            .is_none()
    );
    assert_eq!(
        result.participants[0]
            .snapshot_id
            .as_ref()
            .unwrap()
            .as_str(),
        "snapshot-one"
    );
    assert!(
        db.app
            .party_eligibility(
                &id(),
                &DatasetRevision::new("changed").unwrap(),
                &QuestId::new("demo-quest-1").unwrap(),
                None
            )
            .is_err()
    );
    assert!(
        db.app
            .party_eligibility(
                &id(),
                &DatasetRevision::new("demo-r1").unwrap(),
                &QuestId::new("missing").unwrap(),
                None
            )
            .is_err()
    );
}

#[test]
fn stale_capture_and_deleted_participant_never_become_definite_joint_advice() {
    use azeroth_storage::SnapshotStore;
    let db = setup(true);
    let one = import(&db.app, "one", 23, NOW - 3_600_000);
    let two = import(&db.app, "two", 23, NOW);
    db.app.save_party(&id(), &[one, two]).unwrap();
    assert!(matches!(
        report(&db.app).joint_status,
        JointStatus::MissingOrOldSnapshots
    ));
    let mut store = SqliteStore::open(&db.path).unwrap();
    let key = store
        .list_profiles()
        .unwrap()
        .into_iter()
        .find(|p| p.key.character.as_str() == "two")
        .unwrap()
        .key;
    store.delete_profile(&key).unwrap();
    let result = report(&db.app);
    assert!(matches!(
        result.joint_status,
        JointStatus::MissingOrOldSnapshots
    ));
    assert!(result.participants[1].assessment.is_none());
    assert!(result.participants[1].snapshot_id.is_none());
}

#[test]
fn absent_content_is_explicit_and_contexts_are_not_relabelled() {
    let db = setup(false);
    let one = import(&db.app, "one", 23, NOW);
    db.app.save_party(&id(), &[one]).unwrap();
    assert!(db.app.party_quests(&id()).unwrap().revision.is_none());
    assert!(
        db.app
            .party_eligibility(
                &id(),
                &DatasetRevision::new("demo-r1").unwrap(),
                &QuestId::new("demo-quest-1").unwrap(),
                None
            )
            .is_err()
    );
    assert!(
        db.app
            .party_quests(&PartyId::new("absent").unwrap())
            .is_err()
    );
}
