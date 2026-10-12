//! Synthetic content persistence, migration and backup regression checks.
mod common;
use azeroth_core::{
    contract::{ReferenceDataset, decode_envelope},
    error::ErrorCode,
    sources::{ConflictKind, RecordKey, SourceConflict, SourceRecord},
};
use azeroth_storage::{
    ImportSource, ProfileKey, SnapshotStore, SqliteStore,
    content::{ContentRevision, StageOutcome},
    migrations::{MIGRATIONS, migrate_with},
    snapshot_content_hash,
};
use common::{Doc, TempDb, count, dump_state, raw};
use serde_json::{Value, json};

fn source(id: &str) -> SourceRecord {
    let mut value = json!({"id":id,"provider":null,"publisher":"Synthetic author","record_class":"secondary_inference"});
    for field in [
        "location",
        "editions",
        "access",
        "authentication",
        "license",
        "attribution",
        "redistribution",
        "retention",
        "allowed_fields",
        "transformations",
        "update_schedule",
        "conditional_requests",
        "failure_behavior",
        "last_retrieval_utc_ms",
        "verified_revision",
        "offline_usable",
    ] {
        value[field] = json!({"status":"unknown"});
    }
    value["offline_usable"] = json!({"status":"known","value":false});
    serde_json::from_value(value).unwrap()
}
fn revision(name: &str) -> ContentRevision {
    let mut document: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/reference-dataset.json"
    ))
    .unwrap();
    document["payload"]["revision"] = json!(name);
    ContentRevision {
        envelope: decode_envelope::<ReferenceDataset>(&document.to_string()).unwrap(),
        sources: vec![source("demo-source-1")],
        conflicts: vec![],
    }
}
fn conflict_revision(name: &str) -> ContentRevision {
    let mut revision = revision(name);
    let mut document = serde_json::to_value(&revision.envelope).unwrap();
    let mut second = document["payload"]["evidence"][0].clone();
    second["id"] = json!("second-evidence");
    document["payload"]["evidence"]
        .as_array_mut()
        .unwrap()
        .push(second);
    revision.envelope = decode_envelope(&document.to_string()).unwrap();
    revision.conflicts.push(SourceConflict {
        key: RecordKey::Item(revision.envelope.payload.items[0].key.clone()),
        kind: ConflictKind::ValueDisagreement,
        preferred: revision.envelope.payload.evidence[0].id.clone(),
        others: vec![revision.envelope.payload.evidence[1].id.clone()],
    });
    revision
}
fn recontext(revision: ContentRevision, field: &str, value: Value) -> ContentRevision {
    fn walk(doc: &mut Value, field: &str, value: &Value) {
        match doc {
            Value::Object(map) => {
                if map.get(field).is_some_and(Value::is_string) {
                    map.insert(field.to_string(), value.clone());
                }
                for child in map.values_mut() {
                    walk(child, field, value);
                }
            }
            Value::Array(items) => {
                for item in items {
                    walk(item, field, value);
                }
            }
            _ => {}
        }
    }
    let mut doc = serde_json::to_value(&revision.envelope).unwrap();
    walk(&mut doc, field, &value);
    ContentRevision {
        envelope: decode_envelope(&doc.to_string()).unwrap(),
        ..revision
    }
}

#[test]
fn immutable_revisions_round_trip_sources_conflicts_and_unknown_policies() {
    let db = TempDb::new("content-roundtrip");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let revision = conflict_revision("r1");
    assert_eq!(
        store.stage_content_revision(&revision).unwrap(),
        StageOutcome::Inserted
    );
    assert_eq!(
        store.stage_content_revision(&revision).unwrap(),
        StageOutcome::Duplicate
    );
    let stored = store.content_revision(&revision.key()).unwrap().unwrap();
    assert_eq!(stored, revision);
    assert_eq!(stored.sources[0].offline_usable.value(), Some(&false));
    assert_eq!(
        stored.sources[0].redistribution.status(),
        azeroth_core::observation::ObservationStatus::Unknown
    );
    assert_eq!(count(&db.path, "SELECT COUNT(*) FROM source_records"), 1);
    assert_eq!(count(&db.path, "SELECT COUNT(*) FROM source_conflicts"), 1);
    assert!(
        !azeroth_core::coverage::content_report(&stored.envelope)
            .gaps
            .is_empty()
    );
    let conn = raw(&db.path);
    for sql in [
        "UPDATE dataset_revisions SET document='changed'",
        "DELETE FROM dataset_revisions",
        "UPDATE dataset_manifests SET content_sha256='changed'",
        "DELETE FROM source_records",
        "DELETE FROM source_conflicts",
        "INSERT INTO source_records (revision_id,source_id,document) VALUES (1,'late-source','{}')",
        "INSERT INTO source_conflicts (revision_id,conflict_index,document) VALUES (1,99,'{}')",
    ] {
        assert!(conn.execute(sql, []).is_err(), "immutable: {sql}");
    }
    let mut changed = revision.clone();
    changed.sources[0] = source("other-source");
    changed.sources.push(source("demo-source-1"));
    assert_eq!(
        store.stage_content_revision(&changed).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        store.content_revision(&revision.key()).unwrap(),
        Some(revision)
    );
}

#[test]
fn activation_rollback_and_failed_update_preserve_last_good_revision() {
    let db = TempDb::new("content-activate");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let first = revision("r1");
    let second = revision("r2");
    store.stage_content_revision(&first).unwrap();
    store.stage_content_revision(&second).unwrap();
    assert_eq!(
        store.active_content_revision(&first.key().scope).unwrap(),
        None
    );
    assert!(
        store
            .activate_content_revision(&first.key())
            .unwrap()
            .changed
    );
    assert!(
        !store
            .activate_content_revision(&first.key())
            .unwrap()
            .changed
    );
    raw(&db.path).execute_batch("CREATE TRIGGER reject_activation AFTER UPDATE ON active_datasets BEGIN SELECT RAISE(ABORT, 'synthetic failure'); END;").unwrap();
    assert_eq!(
        store
            .activate_content_revision(&second.key())
            .unwrap_err()
            .code,
        ErrorCode::StorageError
    );
    assert_eq!(
        store.active_content_revision(&first.key().scope).unwrap(),
        Some(first.clone())
    );
    raw(&db.path)
        .execute_batch("DROP TRIGGER reject_activation;")
        .unwrap();
    assert_eq!(
        store
            .activate_content_revision(&second.key())
            .unwrap()
            .previous,
        Some(first.key().revision.clone())
    );
    let outcome = store
        .rollback_content_revision(&first.key().scope)
        .unwrap()
        .unwrap();
    assert_eq!(outcome.active, first.key());
    assert_eq!(
        store.active_content_revision(&first.key().scope).unwrap(),
        Some(first)
    );
}

#[test]
fn bad_content_references_and_missing_sources_fail_without_affecting_active_content() {
    let db = TempDb::new("content-validation");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let first = revision("r1");
    store.stage_content_revision(&first).unwrap();
    store.activate_content_revision(&first.key()).unwrap();
    let mut missing = revision("bad-source");
    missing.sources.clear();
    assert_eq!(
        store.stage_content_revision(&missing).unwrap_err().code,
        ErrorCode::DatasetIncomplete
    );
    let mut bad = conflict_revision("bad-conflict");
    bad.conflicts[0].others = vec![azeroth_core::types::EvidenceId::new("absent").unwrap()];
    assert_eq!(
        store.stage_content_revision(&bad).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    let mut doc = serde_json::to_value(&revision("bad-predicate").envelope).unwrap();
    doc["payload"]["quests"][0]["prerequisites"] = json!({"status":"known","value":{"op":"quest_state","quest":"absent-quest","state":"completed","value":true}});
    let mut bad = revision("bad-predicate");
    bad.envelope = decode_envelope(&doc.to_string()).unwrap();
    assert_eq!(
        store.stage_content_revision(&bad).unwrap_err().code,
        ErrorCode::DatasetIncomplete
    );
    assert_eq!(
        store.active_content_revision(&first.key().scope).unwrap(),
        Some(first)
    );
    assert_eq!(count(&db.path, "SELECT COUNT(*) FROM dataset_revisions"), 1);
}

#[test]
fn staging_failure_rolls_back_revision_manifest_sources_and_conflicts() {
    let db = TempDb::new("content-stage-failure");
    let mut store = SqliteStore::open(&db.path).unwrap();
    raw(&db.path).execute_batch("CREATE TRIGGER reject_source BEFORE INSERT ON source_records BEGIN SELECT RAISE(ABORT, 'synthetic failure'); END;").unwrap();
    assert_eq!(
        store
            .stage_content_revision(&conflict_revision("r1"))
            .unwrap_err()
            .code,
        ErrorCode::StorageError
    );
    for table in [
        "dataset_revisions",
        "dataset_manifests",
        "source_records",
        "source_conflicts",
    ] {
        assert_eq!(count(&db.path, &format!("SELECT COUNT(*) FROM {table}")), 0);
    }
}

#[test]
fn context_and_fixture_scopes_have_independent_activation_and_foreign_keys() {
    let db = TempDb::new("content-context");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let base = revision("shared-r1");
    let other_build = recontext(base.clone(), "build", json!("OTHER_BUILD"));
    let other_edition = recontext(base.clone(), "edition", json!("wow_classic"));
    let reference = recontext(base.clone(), "namespace", json!("reference"));
    let mut doc = serde_json::to_value(&reference.envelope).unwrap();
    doc["fixture"] = json!(false);
    doc["producer"]["component"] = json!("desktop");
    doc["provenance"]["source"] = json!("manual_entry");
    let real = ContentRevision {
        envelope: decode_envelope(&doc.to_string()).unwrap(),
        ..reference.clone()
    };
    for revision in [&base, &other_build, &other_edition, &reference, &real] {
        store.stage_content_revision(revision).unwrap();
        store.activate_content_revision(&revision.key()).unwrap();
    }
    for revision in [&base, &other_build, &other_edition, &reference, &real] {
        assert_eq!(
            store
                .active_content_revision(&revision.key().scope)
                .unwrap(),
            Some(revision.clone())
        );
    }
    assert_eq!(count(&db.path, "SELECT COUNT(*) FROM active_datasets"), 5);
    let conn = raw(&db.path);
    conn.pragma_update(None, "foreign_keys", true).unwrap();
    assert!(conn.execute("UPDATE active_datasets SET revision_id=(SELECT id FROM dataset_revisions WHERE build='OTHER_BUILD') WHERE build='UNVERIFIED_DEMO'", []).is_err());
}

#[test]
fn activation_revalidates_tampered_staging_and_preserves_active_pointer() {
    let db = TempDb::new("content-corruption");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let first = revision("r1");
    let second = revision("r2");
    store.stage_content_revision(&first).unwrap();
    store.stage_content_revision(&second).unwrap();
    store.activate_content_revision(&first.key()).unwrap();
    raw(&db.path).execute_batch("DROP TRIGGER dataset_revisions_no_update; UPDATE dataset_revisions SET document='{}' WHERE revision='r2';").unwrap();
    assert_eq!(
        store
            .activate_content_revision(&second.key())
            .unwrap_err()
            .code,
        ErrorCode::StorageError
    );
    assert_eq!(
        store.active_content_revision(&first.key().scope).unwrap(),
        Some(first)
    );
}

#[test]
fn v1_snapshot_database_upgrades_without_changing_snapshot_hash_or_state() {
    let db = TempDb::new("content-v1-migration");
    let mut conn = raw(&db.path);
    conn.pragma_update(None, "foreign_keys", true).unwrap();
    migrate_with(&mut conn, &MIGRATIONS[..1]).unwrap();
    assert_eq!(
        azeroth_storage::migrations::current_version(&conn).unwrap(),
        1
    );
    let snapshot = Doc::base().decode();
    let hash = snapshot_content_hash(&snapshot).unwrap();
    conn.execute("INSERT INTO profiles (id,character_id,edition,build,interface,namespace,is_fixture) VALUES (1,'demo-character-paladin','wow_forever','UNVERIFIED_DEMO','UNVERIFIED_DEMO','demo',1)",[]).unwrap();
    conn.execute("INSERT INTO snapshots (id,profile_id,snapshot_id,content_sha256,producer_component,producer_version,game_locale,source_revision,capture_source,capture_clock_trust,document) VALUES (1,1,'demo-snapshot-001',?1,'test_fixture','0.1.0','enGB','demo-r1','synthetic_fixture','trusted',?2)", rusqlite::params![hash.as_str(),serde_json::to_string(&snapshot).unwrap()]).unwrap();
    conn.execute("UPDATE profiles SET current_snapshot_id=1 WHERE id=1", [])
        .unwrap();
    conn.execute("INSERT INTO snapshot_observations (snapshot_id,field,status,clock_trust) VALUES (1,'level','known','trusted')",[]).unwrap();
    drop(conn);
    let before = dump_state(&db.path);
    let store = SqliteStore::open(&db.path).unwrap();
    assert_eq!(
        store.schema_version().unwrap(),
        azeroth_storage::migrations::latest_version()
    );
    assert_eq!(dump_state(&db.path), before);
    let key = ProfileKey::of(&snapshot);
    assert_eq!(
        snapshot_content_hash(&store.current_snapshot(&key).unwrap().unwrap()).unwrap(),
        hash
    );
}

#[test]
fn sqlite_backup_restores_committed_wal_snapshots_content_and_sources_consistently() {
    let db = TempDb::new("content-backup");
    let backup = TempDb::new("content-backup-target");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let writer = raw(&db.path);
    writer.pragma_update(None, "journal_mode", "WAL").unwrap();
    writer.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
    let snapshot = Doc::base().decode();
    store
        .import_character_snapshot(&snapshot, ImportSource::File, None)
        .unwrap();
    let revision = conflict_revision("r1");
    store.stage_content_revision(&revision).unwrap();
    store.activate_content_revision(&revision.key()).unwrap();
    let state = dump_state(&db.path);
    writer.execute_batch("BEGIN IMMEDIATE; INSERT INTO profiles (character_id,edition,build,interface,namespace,is_fixture) VALUES ('uncommitted','wow_forever','SYNTHETIC','SYNTHETIC','demo',1);").unwrap();
    store.backup_to(&backup.path).unwrap();
    writer.execute_batch("ROLLBACK").unwrap();
    let restored = SqliteStore::open(&backup.path).unwrap();
    assert_eq!(dump_state(&backup.path), state);
    assert_eq!(
        restored
            .active_content_revision(&revision.key().scope)
            .unwrap(),
        Some(revision)
    );
    assert_eq!(
        count(&backup.path, "SELECT COUNT(*) FROM source_conflicts"),
        1
    );
    assert_eq!(
        count(&backup.path, "SELECT COUNT(*) FROM source_records"),
        1
    );
    assert!(restored.foreign_keys_enabled().unwrap());
    assert_eq!(
        count(
            &backup.path,
            "SELECT COUNT(*) FROM pragma_foreign_key_check"
        ),
        0
    );
    let integrity: String = raw(&backup.path)
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    let bytes = std::fs::read(&backup.path).unwrap();
    assert_eq!(
        store.backup_to(&backup.path).unwrap_err().code,
        ErrorCode::StorageError
    );
    assert_eq!(std::fs::read(&backup.path).unwrap(), bytes);
}

#[test]
fn content_bounds_and_duplicate_source_ids_fail_before_staging() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let mut duplicate = revision("duplicate-sources");
    duplicate.sources.push(duplicate.sources[0].clone());
    assert_eq!(
        store.stage_content_revision(&duplicate).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    let mut oversized = revision("oversized-sources");
    oversized.sources =
        vec![source("demo-source-1"); azeroth_storage::content::MAX_CONTENT_RECORDS + 1];
    assert_eq!(
        store.stage_content_revision(&oversized).unwrap_err().code,
        ErrorCode::ImportTooLarge
    );
    assert_eq!(store.content_revision(&duplicate.key()).unwrap(), None);
    assert_eq!(
        store
            .rollback_content_revision(&duplicate.key().scope)
            .unwrap(),
        None
    );
}

#[test]
fn activation_rejects_manifest_and_child_record_corruption() {
    for tamper in [
        "DROP TRIGGER source_records_no_update; UPDATE source_records SET document='{}' WHERE revision_id=2;",
        "DROP TRIGGER source_conflicts_no_delete; DELETE FROM source_conflicts WHERE revision_id=2;",
        "DROP TRIGGER dataset_manifests_no_update; UPDATE dataset_manifests SET coverage_document='{}' WHERE revision_id=2;",
    ] {
        let db = TempDb::new("content-record-corruption");
        let mut store = SqliteStore::open(&db.path).unwrap();
        let first = conflict_revision("r1");
        let second = conflict_revision("r2");
        store.stage_content_revision(&first).unwrap();
        store.stage_content_revision(&second).unwrap();
        store.activate_content_revision(&first.key()).unwrap();
        raw(&db.path).execute_batch(tamper).unwrap();
        assert_eq!(
            store
                .activate_content_revision(&second.key())
                .unwrap_err()
                .code,
            ErrorCode::StorageError
        );
        assert_eq!(
            store.active_content_revision(&first.key().scope).unwrap(),
            Some(first)
        );
    }
}

#[test]
fn source_order_is_canonical_and_declared_edition_exclusions_are_preserved() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let mut input = revision("canonical-sources");
    input.sources.push(source("unused-source"));
    assert_eq!(
        store.stage_content_revision(&input).unwrap(),
        StageOutcome::Inserted
    );
    input.sources.reverse();
    assert_eq!(
        store.stage_content_revision(&input).unwrap(),
        StageOutcome::Duplicate
    );
    let mut bad = revision("excluded-edition");
    let mut src = serde_json::to_value(&bad.sources[0]).unwrap();
    src["editions"] = json!({"status":"known","value":["wow_classic"]});
    bad.sources[0] = serde_json::from_value(src).unwrap();
    assert_eq!(
        store.stage_content_revision(&bad).unwrap_err().code,
        ErrorCode::BuildMismatch
    );
}

#[test]
fn self_conflicts_and_empty_disagreements_fail_but_single_relabel_is_preserved() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    for kind in [
        ConflictKind::ValueDisagreement,
        ConflictKind::CategoryDowngrade,
        ConflictKind::CrossEditionRelabel,
    ] {
        let mut input = conflict_revision("self-conflict");
        input.conflicts[0].kind = kind;
        input.conflicts[0].others = vec![input.conflicts[0].preferred.clone()];
        assert_eq!(
            store.stage_content_revision(&input).unwrap_err().code,
            ErrorCode::InvalidInput
        );
        input.conflicts[0].others.clear();
        if kind == ConflictKind::CrossEditionRelabel {
            assert_eq!(
                store.stage_content_revision(&input).unwrap(),
                StageOutcome::Inserted
            );
            assert!(
                store
                    .content_revision(&input.key())
                    .unwrap()
                    .unwrap()
                    .conflicts[0]
                    .others
                    .is_empty()
            );
        } else {
            assert_eq!(
                store.stage_content_revision(&input).unwrap_err().code,
                ErrorCode::InvalidInput
            );
        }
    }
}

#[test]
fn exact_revision_reads_reject_mismatched_index_identity() {
    let db = TempDb::new("content-index-identity");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let input = revision("r1");
    store.stage_content_revision(&input).unwrap();
    raw(&db.path).execute_batch("DROP TRIGGER dataset_revisions_no_update; UPDATE dataset_revisions SET revision='impostor';").unwrap();
    let mut key = input.key();
    key.revision = azeroth_core::types::DatasetRevision::new("impostor").unwrap();
    assert_eq!(
        store.content_revision(&key).unwrap_err().code,
        ErrorCode::StorageError
    );
}
