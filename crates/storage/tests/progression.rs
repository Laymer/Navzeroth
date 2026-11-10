mod common;
use azeroth_core::{
    contract::{Envelope, decode_envelope},
    entities::CharacterSnapshot,
    progression::{ProgressionSnapshot, SnapshotBinding},
};
use azeroth_storage::{
    ImportSource, ProfileKey, SnapshotStore, SqliteStore,
    progression::{MAX_PROGRESSION_RECORDS, ProgressionOrigin},
    snapshot_content_hash,
};
use common::{Doc, TempDb, raw};
use serde_json::{Value, json};

fn companion(parent: &Envelope<CharacterSnapshot>, id: &str) -> Envelope<ProgressionSnapshot> {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/progression-snapshot.json"
    ))
    .unwrap();
    value["payload"]["id"] = json!(id);
    value["payload"]["binding"] = serde_json::to_value(SnapshotBinding::from_snapshot(
        &parent.payload,
        snapshot_content_hash(parent).unwrap(),
    ))
    .unwrap();
    value["dependencies"]["snapshots"][0] = json!({"id":parent.payload.id,"character":parent.payload.character,"context":parent.payload.context});
    value["dependencies"]["dataset_revision"] = json!(parent.payload.source_revision);
    value["game"] = serde_json::to_value(&parent.game).unwrap();
    value["namespace"] = json!(parent.namespace);
    value["payload"]["capture"]["source"] = json!("synthetic_fixture");
    fn mark(value: &mut Value) {
        if let Value::Object(map) = value {
            if map.contains_key("status") {
                map.insert("source".into(), json!("synthetic_fixture"));
            }
            for item in map.values_mut() {
                mark(item);
            }
        }
    }
    mark(&mut value["payload"]);
    decode_envelope(&value.to_string()).unwrap()
}
fn seed(store: &mut SqliteStore) -> (Envelope<CharacterSnapshot>, ProfileKey) {
    let parent = Doc::base().decode();
    let key = ProfileKey::of(&parent);
    store
        .import_character_snapshot(&parent, ImportSource::Fixture, None)
        .unwrap();
    (parent, key)
}

#[test]
fn immutable_separate_origins_restart_and_snapshot_invalidation() {
    let db = TempDb::new("progression");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let (parent, key) = seed(&mut store);
    let value = companion(&parent, "one");
    assert!(
        store
            .save_progression(&key, &value, ProgressionOrigin::Imported)
            .unwrap()
    );
    assert!(
        !store
            .save_progression(&key, &value, ProgressionOrigin::Imported)
            .unwrap()
    );
    assert!(
        store
            .current_progression(&key, ProgressionOrigin::Manual)
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .save_progression(&key, &value, ProgressionOrigin::Manual)
            .unwrap()
    );
    let mut changed = serde_json::to_value(&value).unwrap();
    changed["payload"]["money"]["value"] = json!("1");
    let changed = decode_envelope(&changed.to_string()).unwrap();
    assert!(
        store
            .save_progression(&key, &changed, ProgressionOrigin::Imported)
            .is_err()
    );
    drop(store);
    let mut store = SqliteStore::open(&db.path).unwrap();
    assert_eq!(
        store
            .current_progression(&key, ProgressionOrigin::Imported)
            .unwrap(),
        vec![value.clone()]
    );
    assert_eq!(
        store
            .current_progression(&key, ProgressionOrigin::Manual)
            .unwrap(),
        vec![value.clone()]
    );
    let combined = store
        .with_immediate_read(|store| store.current_progression(&key, ProgressionOrigin::Imported))
        .unwrap();
    assert_eq!(combined, vec![value.clone()]);
    let newer = Doc::base()
        .snapshot_id("newer")
        .session("demo-session-001", 13)
        .elapsed(999999)
        .decode();
    store
        .import_character_snapshot(&newer, ImportSource::Fixture, None)
        .unwrap();
    assert!(
        store
            .current_progression(&key, ProgressionOrigin::Imported)
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .current_progression(&key, ProgressionOrigin::Manual)
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .save_progression(&key, &value, ProgressionOrigin::Imported)
            .is_err()
    );
    assert!(
        store
            .save_progression(&key, &companion(&newer, "two"), ProgressionOrigin::Imported)
            .unwrap()
    );
    store.delete_profile(&key).unwrap();
    assert!(
        store
            .current_progression(&key, ProgressionOrigin::Imported)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        common::count(&db.path, "SELECT count(*) FROM progression_snapshots"),
        0
    );
}

#[test]
fn mismatch_missing_profile_and_provenance_fail_atomically() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let parent = Doc::base().decode();
    let key = ProfileKey::of(&parent);
    let value = companion(&parent, "one");
    assert!(
        store
            .save_progression(&key, &value, ProgressionOrigin::Imported)
            .is_err()
    );
    seed(&mut store);
    for pointer in [
        "/payload/binding/snapshot_sha256",
        "/payload/capture/source",
        "/provenance/source",
        "/payload/money/source",
    ] {
        let mut bad = serde_json::to_value(&value).unwrap();
        *bad.pointer_mut(pointer).unwrap() = if pointer.ends_with("sha256") {
            json!("a".repeat(64))
        } else {
            json!("unrecorded")
        };
        if let Ok(bad) = decode_envelope::<ProgressionSnapshot>(&bad.to_string()) {
            assert!(
                store
                    .save_progression(&key, &bad, ProgressionOrigin::Imported)
                    .is_err(),
                "{pointer}"
            );
        }
    }
    let foreign = Doc::base().character("other").decode();
    assert!(
        store
            .save_progression(
                &key,
                &companion(&foreign, "two"),
                ProgressionOrigin::Imported
            )
            .is_err()
    );
    assert!(
        store
            .current_progression(&key, ProgressionOrigin::Imported)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn capacity_is_bounded_and_duplicate_still_succeeds_at_limit() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let (parent, key) = seed(&mut store);
    for id in 0..MAX_PROGRESSION_RECORDS {
        store
            .save_progression(
                &key,
                &companion(&parent, &id.to_string()),
                ProgressionOrigin::Imported,
            )
            .unwrap();
    }
    assert!(
        !store
            .save_progression(&key, &companion(&parent, "0"), ProgressionOrigin::Imported)
            .unwrap()
    );
    assert!(
        store
            .save_progression(
                &key,
                &companion(&parent, "overflow"),
                ProgressionOrigin::Manual
            )
            .is_err()
    );
    assert_eq!(
        store
            .current_progression(&key, ProgressionOrigin::Imported)
            .unwrap()
            .len(),
        MAX_PROGRESSION_RECORDS
    );
}

#[test]
fn migration_backup_restore_and_corruption_checks() {
    let db = TempDb::new("progression-migrate");
    let mut conn = raw(&db.path);
    azeroth_storage::migrations::migrate_with(
        &mut conn,
        &azeroth_storage::migrations::MIGRATIONS[..13],
    )
    .unwrap();
    conn.execute("INSERT INTO profiles(character_id,edition,build,interface,namespace,is_fixture) VALUES('kept','wow_forever','SYNTHETIC','SYNTHETIC','demo',1)",[]).unwrap();
    drop(conn);
    let mut store = SqliteStore::open(&db.path).unwrap();
    assert_eq!(store.schema_version().unwrap(), 14);
    assert_eq!(store.list_profiles().unwrap().len(), 1);
    let (parent, key) = seed(&mut store);
    let value = companion(&parent, "one");
    store
        .save_progression(&key, &value, ProgressionOrigin::Imported)
        .unwrap();
    let backup = TempDb::new("progression-backup");
    store.backup_to(&backup.path).unwrap();
    let restored = TempDb::new("progression-restore");
    azeroth_storage::recovery::reconstruct_backup(&backup.path, &restored.path).unwrap();
    let restored = SqliteStore::open(&restored.path).unwrap();
    assert_eq!(
        restored
            .current_progression(&key, ProgressionOrigin::Imported)
            .unwrap(),
        vec![value]
    );
    raw(&db.path)
        .execute(
            "UPDATE progression_snapshots SET content_sha256=?1",
            ["a".repeat(64)],
        )
        .unwrap();
    assert!(
        store
            .current_progression(&key, ProgressionOrigin::Imported)
            .is_err()
    );
    let refused = TempDb::new("progression-bad-restore");
    assert!(azeroth_storage::recovery::reconstruct_backup(&db.path, &refused.path).is_err());
}

#[test]
fn production_manual_provenance_cannot_be_imported_or_mix_known_sources() {
    let reference = Doc::reference().decode();
    let mut parent = serde_json::to_value(&reference).unwrap();
    parent["fixture"] = json!(false);
    parent["producer"]["component"] = json!("addon");
    let parent: Envelope<CharacterSnapshot> = decode_envelope(
        &parent
            .to_string()
            .replace("synthetic_fixture", "addon_snapshot"),
    )
    .unwrap();
    let mut store = SqliteStore::open_in_memory().unwrap();
    let key = ProfileKey::of(&parent);
    store
        .import_character_snapshot(&parent, ImportSource::Clipboard, None)
        .unwrap();
    // Build from a synthetic reference, then explicitly switch only companion
    // observations to manual; predecessor metadata remains original addon data.
    let fixture = companion(&reference, "manual");
    let mut value = serde_json::to_value(&fixture).unwrap();
    value["fixture"] = json!(false);
    value["producer"]["component"] = json!("desktop");
    let mut value: Value = serde_json::from_str(
        &value
            .to_string()
            .replace("synthetic_fixture", "manual_entry"),
    )
    .unwrap();
    value["payload"]["binding"] = serde_json::to_value(SnapshotBinding::from_snapshot(
        &parent.payload,
        snapshot_content_hash(&parent).unwrap(),
    ))
    .unwrap();
    let manual: Envelope<ProgressionSnapshot> = decode_envelope(&value.to_string()).unwrap();
    assert!(
        store
            .save_progression(&key, &manual, ProgressionOrigin::Imported)
            .is_err()
    );
    assert!(
        store
            .save_progression(&key, &manual, ProgressionOrigin::Manual)
            .unwrap()
    );
    value["payload"]["id"] = json!("mixed");
    value["payload"]["money"]["source"] = json!("addon_snapshot");
    let mixed = decode_envelope(&value.to_string()).unwrap();
    assert!(
        store
            .save_progression(&key, &mixed, ProgressionOrigin::Manual)
            .is_err()
    );
    value["payload"]["money"]["source"] = json!("unrecorded");
    let unrecorded = decode_envelope(&value.to_string()).unwrap();
    assert!(
        store
            .save_progression(&key, &unrecorded, ProgressionOrigin::Manual)
            .is_err()
    );
    assert_eq!(
        store
            .current_progression(&key, ProgressionOrigin::Manual)
            .unwrap(),
        vec![manual]
    );
}

#[test]
fn recovery_validates_corrupt_noncurrent_companions_and_parent_profile() {
    for corruption in ["companion_id", "parent_profile"] {
        let db = TempDb::new(corruption);
        let mut store = SqliteStore::open(&db.path).unwrap();
        let (parent, key) = seed(&mut store);
        store
            .save_progression(
                &key,
                &companion(&parent, "old"),
                ProgressionOrigin::Imported,
            )
            .unwrap();
        let newer = Doc::base()
            .snapshot_id("newer")
            .session("demo-session-001", 13)
            .elapsed(999999)
            .decode();
        store
            .import_character_snapshot(&newer, ImportSource::Fixture, None)
            .unwrap();
        let other = Doc::base().character("other").decode();
        store
            .import_character_snapshot(&other, ImportSource::Fixture, None)
            .unwrap();
        if corruption == "companion_id" {
            raw(&db.path)
                .execute(
                    "UPDATE progression_snapshots SET companion_id='altered'",
                    [],
                )
                .unwrap();
        } else {
            raw(&db.path).execute("UPDATE progression_snapshots SET snapshot_row_id=(SELECT s.id FROM snapshots s JOIN profiles p ON p.id=s.profile_id WHERE p.character_id='other')",[]).unwrap();
        }
        // The active predecessor has no companion. Restore must still validate
        // the entire retained history rather than checking only this view.
        assert!(
            store
                .current_progression(&key, ProgressionOrigin::Imported)
                .unwrap()
                .is_empty()
        );
        let dest = TempDb::new("refused-history");
        assert!(azeroth_storage::recovery::reconstruct_backup(&db.path, &dest.path).is_err());
    }
}
