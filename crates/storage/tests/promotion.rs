mod common;
use azeroth_core::types::UtcMillis;
use azeroth_storage::{ImportSource, ProfileKey, SnapshotStore, SqliteStore};
use common::{Doc, TempDb, raw};
#[test]
fn promotion_is_scoped_audited_atomic_and_guards_concurrent_imports() {
    let db = TempDb::new("promotion");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let first = Doc::base().decode();
    let selected = Doc::base()
        .snapshot_id("selected")
        .session("other", 1)
        .observed(1, false)
        .decode();
    let key = ProfileKey::of(&first);
    store
        .import_character_snapshot(&first, ImportSource::Fixture, None)
        .unwrap();
    store
        .import_character_snapshot(&selected, ImportSource::Fixture, None)
        .unwrap();
    assert_eq!(store.current_snapshot(&key).unwrap().unwrap(), first);
    assert_eq!(
        store
            .historical_snapshot(&key, &selected.payload.id)
            .unwrap()
            .unwrap(),
        selected
    );
    let now = UtcMillis::new(100).unwrap();
    store
        .promote_snapshot(
            &key,
            &selected.payload.id,
            &first.payload.id,
            "Selected new session manually",
            now,
        )
        .unwrap();
    assert_eq!(store.current_snapshot(&key).unwrap().unwrap(), selected);
    assert_eq!(
        common::count(&db.path, "SELECT COUNT(*) FROM snapshot_promotions"),
        1
    );
    assert!(
        store
            .promote_snapshot(
                &key,
                &first.payload.id,
                &first.payload.id,
                "stale preview",
                now
            )
            .is_err()
    );
    let other = Doc::base()
        .character("other")
        .snapshot_id("foreign")
        .decode();
    store
        .import_character_snapshot(&other, ImportSource::Fixture, None)
        .unwrap();
    assert!(
        store
            .historical_snapshot(&key, &other.payload.id)
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .promote_snapshot(
                &key,
                &other.payload.id,
                &selected.payload.id,
                "wrong profile",
                now
            )
            .is_err()
    );
    let newer = Doc::base()
        .snapshot_id("watcher")
        .session("other", 2)
        .observed(1, false)
        .decode();
    let mut watcher = SqliteStore::open(&db.path).unwrap();
    watcher
        .import_character_snapshot(&newer, ImportSource::Fixture, None)
        .unwrap();
    assert!(
        store
            .promote_snapshot(
                &key,
                &first.payload.id,
                &selected.payload.id,
                "raced watcher",
                now
            )
            .is_err()
    );
    assert!(
        store
            .promote_snapshot(&key, &selected.payload.id, &newer.payload.id, "older", now)
            .is_err()
    );
    raw(&db.path).execute_batch("CREATE TRIGGER fail_promotion BEFORE INSERT ON snapshot_promotions BEGIN SELECT RAISE(ABORT, 'test'); END;").unwrap();
    assert!(
        store
            .promote_snapshot(
                &key,
                &first.payload.id,
                &newer.payload.id,
                "audit failure",
                now
            )
            .is_err()
    );
    assert_eq!(store.current_snapshot(&key).unwrap().unwrap(), newer);
    assert_eq!(
        common::count(&db.path, "SELECT COUNT(*) FROM snapshot_promotions"),
        1
    );
}

#[test]
fn same_position_promotion_keeps_history_and_audit_bounded() {
    let db = TempDb::new("bounded-promotion");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let first = Doc::base().decode();
    let key = ProfileKey::of(&first);
    store
        .import_character_snapshot(&first, ImportSource::Fixture, None)
        .unwrap();
    let mut current = first.payload.id.clone();
    for i in 0..260 {
        let next = Doc::base().snapshot_id(&format!("same-{i}")).decode();
        store
            .import_character_snapshot(&next, ImportSource::Fixture, None)
            .unwrap();
        assert_eq!(
            store.current_snapshot(&key).unwrap().unwrap().payload.id,
            current
        );
        store
            .promote_snapshot(
                &key,
                &next.payload.id,
                &current,
                "same-position selection",
                UtcMillis::new(i).unwrap(),
            )
            .unwrap();
        current = next.payload.id.clone();
    }
    assert_eq!(store.snapshot_history(&key, 256).unwrap().len(), 256);
    assert_eq!(
        common::count(&db.path, "SELECT COUNT(*) FROM snapshot_promotions"),
        256
    );
    assert!(
        store
            .historical_snapshot(&key, &first.payload.id)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        store.current_snapshot(&key).unwrap().unwrap().payload.id,
        current
    );
}
