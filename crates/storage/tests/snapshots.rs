//! Idempotency, ordering, failure and isolation behavior of the snapshot store.
mod common;

use azeroth_core::{
    contract::decode_envelope,
    entities::CharacterSnapshot,
    error::ErrorCode,
    observation::ObservationStatus,
    types::{CharacterId, UtcMillis},
};
use azeroth_storage::{
    HistoryReason, ImportOutcome, ImportSource, MAX_HISTORY_LIMIT, MAX_SNAPSHOTS_PER_PROFILE,
    ProfileKey, SnapshotStore, SqliteStore, snapshot_content_hash,
};
use common::{BASE_CHARACTER, BASE_SNAPSHOT_ID, Doc, TempDb, count, dump_state, raw};

const SESSION: &str = "demo-session-001";

fn import(store: &mut SqliteStore, doc: &Doc) -> ImportOutcome {
    store
        .import_character_snapshot(&doc.decode(), ImportSource::Fixture, None)
        .unwrap()
}
fn current_id(store: &SqliteStore, doc: &Doc) -> Option<String> {
    store
        .current_snapshot(&ProfileKey::of(&doc.decode()))
        .unwrap()
        .map(|e| e.payload.id.as_str().to_owned())
}
fn newer(id: &str, sequence: u64) -> Doc {
    Doc::base().snapshot_id(id).session(SESSION, sequence)
}

#[test]
fn round_trip_returns_the_equal_validated_envelope() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let doc = Doc::base();
    let input = doc.decode();
    import(&mut store, &doc);
    let stored = store
        .current_snapshot(&ProfileKey::of(&input))
        .unwrap()
        .unwrap();
    assert_eq!(stored, input);
    // The retained top-level extension survives the round trip.
    assert!(stored.extensions.get("x_producer_debug").is_some());
}

#[test]
fn first_import_is_current_and_reads_back_none_for_other_profiles() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    assert_eq!(
        import(&mut store, &Doc::base()),
        ImportOutcome::Stored {
            previous_current: None
        }
    );
    let other = ProfileKey::of(&Doc::base().character("demo-other").decode());
    assert!(store.current_snapshot(&other).unwrap().is_none());
    assert!(store.snapshot_history(&other, 10).unwrap().is_empty());
}

#[test]
fn content_hash_is_a_stable_lowercase_digest() {
    let hash = snapshot_content_hash(&Doc::base().decode()).unwrap();
    assert_eq!(hash.as_str().len(), 64);
    // Pins the canonical form: any change to it needs a migration (see hash.rs).
    assert_eq!(
        hash.as_str(),
        "0a54fd805dd58c6ff4b870699e4308a09546e664e04b7aecb599221f7ace2a14"
    );
}

#[test]
fn content_hash_ignores_formatting_and_receipt_time_but_not_content() {
    let base = snapshot_content_hash(&Doc::base().decode()).unwrap();
    let pretty = decode_envelope::<CharacterSnapshot>(&Doc::base().pretty()).unwrap();
    assert_eq!(snapshot_content_hash(&pretty).unwrap(), base);
    let later_receipt = Doc::base().received_at(1_800_000_000_000).decode();
    assert_eq!(snapshot_content_hash(&later_receipt).unwrap(), base);
    let changed = Doc::base().level(23).decode();
    assert_ne!(snapshot_content_hash(&changed).unwrap(), base);
}

#[test]
fn case_007_duplicate_import_is_idempotent() {
    let db = TempDb::new("case007");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let doc = Doc::base();
    assert_eq!(
        import(&mut store, &doc),
        ImportOutcome::Stored {
            previous_current: None
        }
    );
    let (snapshots, observations) = (
        count(&db.path, "SELECT COUNT(*) FROM snapshots"),
        count(&db.path, "SELECT COUNT(*) FROM snapshot_observations"),
    );
    assert_eq!((snapshots, observations > 0), (1, true));
    let before = dump_state(&db.path);

    // Same content again: reformatted text, a later receipt time, a new source.
    for variant in [
        doc.clone(),
        Doc(serde_json::from_str(&doc.pretty()).unwrap()),
        doc.clone().received_at(1_800_000_000_000),
    ] {
        let outcome = store
            .import_character_snapshot(
                &variant.decode(),
                ImportSource::Clipboard,
                Some(UtcMillis::new(1_800_000_001_000).unwrap()),
            )
            .unwrap();
        assert_eq!(outcome, ImportOutcome::Duplicate);
    }
    assert_eq!(count(&db.path, "SELECT COUNT(*) FROM snapshots"), snapshots);
    assert_eq!(
        count(&db.path, "SELECT COUNT(*) FROM snapshot_observations"),
        observations
    );
    assert_eq!(
        dump_state(&db.path),
        before,
        "duplicates must not change stored state"
    );
    assert_eq!(
        count(
            &db.path,
            "SELECT COUNT(*) FROM imports WHERE status = 'stored'"
        ),
        1
    );
    assert_eq!(
        count(
            &db.path,
            "SELECT COUNT(*) FROM imports WHERE status = 'duplicate'"
        ),
        3
    );
    assert_eq!(
        count(
            &db.path,
            "SELECT COUNT(*) FROM imports WHERE status = 'duplicate' AND snapshot_id IS NULL"
        ),
        0
    );
}

#[test]
fn same_snapshot_id_with_different_content_is_rejected_not_overwritten() {
    let db = TempDb::new("sameid");
    let mut store = SqliteStore::open(&db.path).unwrap();
    import(&mut store, &Doc::base());
    let before = dump_state(&db.path);
    let conflicting = Doc::base().level(30);
    let error = store
        .import_character_snapshot(&conflicting.decode(), ImportSource::File, None)
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(dump_state(&db.path), before);
    assert_eq!(
        count(
            &db.path,
            "SELECT COUNT(*) FROM imports WHERE status = 'failed'"
        ),
        1
    );
}

#[test]
fn case_009_late_old_snapshot_stays_history() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let key = ProfileKey::of(&Doc::base().decode());
    assert_eq!(
        import(&mut store, &newer("demo-snapshot-013", 13)),
        ImportOutcome::Stored {
            previous_current: None
        }
    );
    // The base fixture (sequence 12) arrives late.
    assert_eq!(
        import(&mut store, &Doc::base()),
        ImportOutcome::StoredHistoryOnly {
            reason: HistoryReason::Older
        }
    );
    let current = store.current_snapshot(&key).unwrap().unwrap();
    assert_eq!(current.payload.id.as_str(), "demo-snapshot-013");
    assert_eq!(current.payload.capture.sequence.unwrap().get(), 13);

    let history = store.snapshot_history(&key, 10).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].snapshot.as_str(), BASE_SNAPSHOT_ID);
    assert!(!history[0].is_current);
    assert!(history[1].is_current);
}

#[test]
fn newer_snapshot_replaces_current_and_reports_the_previous_one() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    import(&mut store, &Doc::base());
    let outcome = import(&mut store, &newer("demo-snapshot-013", 13));
    assert_eq!(
        outcome,
        ImportOutcome::Stored {
            previous_current: Some(azeroth_core::types::SnapshotId::new(BASE_SNAPSHOT_ID).unwrap())
        }
    );
    assert_eq!(
        current_id(&store, &Doc::base()).as_deref(),
        Some("demo-snapshot-013")
    );
}

#[test]
fn case_009_incomparable_snapshots_do_not_silently_become_current() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let a = Doc::base().session("session-a", 1).observed(1_000, false);
    // A later import from another session whose clocks cannot be trusted.
    let b = Doc::base()
        .snapshot_id("demo-snapshot-b")
        .session("session-b", 1)
        .observed(2_000, false);
    import(&mut store, &a);
    assert_eq!(
        import(&mut store, &b),
        ImportOutcome::StoredHistoryOnly {
            reason: HistoryReason::Incomparable
        }
    );
    assert_eq!(current_id(&store, &a).as_deref(), Some(BASE_SNAPSHOT_ID));

    // One untrusted clock is enough to make the pair incomparable.
    let c = Doc::base()
        .snapshot_id("demo-snapshot-c")
        .session("session-c", 1)
        .observed(3_000, true);
    assert_eq!(
        import(&mut store, &c),
        ImportOutcome::StoredHistoryOnly {
            reason: HistoryReason::Incomparable
        }
    );
    assert_eq!(current_id(&store, &a).as_deref(), Some(BASE_SNAPSHOT_ID));
}

#[test]
fn different_sessions_with_trusted_clocks_order_by_time() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let a = Doc::base().session("session-a", 50).observed(1_000, true);
    let b = Doc::base()
        .snapshot_id("demo-snapshot-b")
        .session("session-b", 1)
        .observed(2_000, true);
    let late_old = Doc::base()
        .snapshot_id("demo-snapshot-c")
        .session("session-c", 99)
        .observed(500, true);
    import(&mut store, &a);
    assert!(matches!(
        import(&mut store, &b),
        ImportOutcome::Stored { .. }
    ));
    assert_eq!(
        import(&mut store, &late_old),
        ImportOutcome::StoredHistoryOnly {
            reason: HistoryReason::Older
        }
    );
    assert_eq!(current_id(&store, &a).as_deref(), Some("demo-snapshot-b"));
}

#[test]
fn equal_position_with_different_content_does_not_flip_current() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    import(&mut store, &Doc::base());
    let same_position = Doc::base().snapshot_id("demo-snapshot-same").level(30);
    assert_eq!(
        import(&mut store, &same_position),
        ImportOutcome::StoredHistoryOnly {
            reason: HistoryReason::SamePosition
        }
    );
    assert_eq!(
        current_id(&store, &Doc::base()).as_deref(),
        Some(BASE_SNAPSHOT_ID)
    );
}

#[test]
fn case_008_failed_import_keeps_prior_state() {
    let db = TempDb::new("case008");
    let mut store = SqliteStore::open(&db.path).unwrap();
    import(&mut store, &Doc::base());
    let before = dump_state(&db.path);

    // Make the next import fail halfway through its transaction: the snapshot
    // row is inserted first, then an observation insert aborts.
    raw(&db.path)
        .execute_batch(
            "CREATE TRIGGER inject_failure BEFORE INSERT ON snapshot_observations
             BEGIN SELECT RAISE(ABORT, 'injected'); END;",
        )
        .unwrap();
    let marker = "marker-snapshot-7c1f";
    let failing = Doc::base().snapshot_id(marker).session(SESSION, 99);
    let error = store
        .import_character_snapshot(&failing.decode(), ImportSource::File, None)
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::StorageError);
    assert!(!error.to_string().contains(marker));
    assert!(!error.to_string().contains("injected"));

    assert_eq!(
        dump_state(&db.path),
        before,
        "rolled-back import must change nothing"
    );
    assert_eq!(
        current_id(&store, &Doc::base()).as_deref(),
        Some(BASE_SNAPSHOT_ID)
    );

    // The audit row holds a code and a static message only.
    let conn = raw(&db.path);
    let (status, code, message, snapshot): (String, String, String, Option<i64>) = conn
        .query_row(
            "SELECT status, error_code, message, snapshot_id FROM imports WHERE status = 'failed'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        (status.as_str(), code.as_str()),
        ("failed", "STORAGE_ERROR")
    );
    assert_eq!(message, "Database operation failed");
    assert_eq!(snapshot, None);
    let everything: String = conn
        .query_row(
            "SELECT group_concat(source_kind || status || coalesce(error_code, '') || coalesce(message, ''))
             FROM imports",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!everything.contains(marker) && !everything.contains(BASE_CHARACTER));
}

#[test]
fn case_008_decode_failure_is_recorded_without_content() {
    let db = TempDb::new("case008-decode");
    let mut store = SqliteStore::open(&db.path).unwrap();
    import(&mut store, &Doc::base());
    let before = dump_state(&db.path);

    let secret = "secret-character-name-4d2a";
    let malformed = format!("{{\"format\": \"azeroth-navigator\", \"note\": \"{secret}\"");
    let error = decode_envelope::<CharacterSnapshot>(&malformed).unwrap_err();
    store
        .record_failed_import(ImportSource::Clipboard, &error, None)
        .unwrap();

    assert_eq!(dump_state(&db.path), before);
    let (code, message): (String, String) = raw(&db.path)
        .query_row(
            "SELECT error_code, message FROM imports WHERE status = 'failed'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(code, "INVALID_INPUT");
    assert!(!message.contains(secret));
    assert_eq!(
        current_id(&store, &Doc::base()).as_deref(),
        Some(BASE_SNAPSHOT_ID)
    );
}

#[test]
fn case_015_demo_and_reference_isolated() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let demo = Doc::base();
    let reference = Doc::reference();
    assert_eq!(
        import(&mut store, &demo),
        ImportOutcome::Stored {
            previous_current: None
        }
    );
    // Same character id and snapshot id, other namespace: a separate profile,
    // current immediately, never history-only or a duplicate.
    assert_eq!(
        import(&mut store, &reference),
        ImportOutcome::Stored {
            previous_current: None
        }
    );

    let demo_key = ProfileKey::of(&demo.decode());
    let reference_key = ProfileKey::of(&reference.decode());
    assert_ne!(demo_key, reference_key);
    for (key, doc) in [(&demo_key, &demo), (&reference_key, &reference)] {
        assert_eq!(store.snapshot_history(key, 10).unwrap().len(), 1);
        assert_eq!(store.current_snapshot(key).unwrap().unwrap(), doc.decode());
    }
    // A newer demo snapshot leaves the reference profile alone.
    import(&mut store, &newer("demo-snapshot-013", 13));
    assert_eq!(
        store.current_snapshot(&reference_key).unwrap().unwrap(),
        reference.decode()
    );
    assert_eq!(store.list_profiles().unwrap().len(), 2);
}

#[test]
fn case_015_other_build_never_rewrites_or_replaces_history() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let original = Doc::base();
    import(&mut store, &original);
    let other_build = Doc::base()
        .build("SYNTHETIC_OTHER_BUILD")
        .snapshot_id("demo-snapshot-other")
        .session(SESSION, 99);
    assert_eq!(
        import(&mut store, &other_build),
        ImportOutcome::Stored {
            previous_current: None
        }
    );

    let original_key = ProfileKey::of(&original.decode());
    let other_key = ProfileKey::of(&other_build.decode());
    assert_ne!(original_key.context, other_key.context);
    // The historical snapshot still claims its own build.
    let kept = store.current_snapshot(&original_key).unwrap().unwrap();
    assert_eq!(kept.game.build.as_str(), "UNVERIFIED_DEMO");
    assert_eq!(kept, original.decode());
    assert_eq!(store.snapshot_history(&original_key, 10).unwrap().len(), 1);
    let other = store.current_snapshot(&other_key).unwrap().unwrap();
    assert_eq!(other.game.build.as_str(), "SYNTHETIC_OTHER_BUILD");
}

#[test]
fn fixture_and_real_data_in_one_context_keep_separate_current_pointers() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let fixture = Doc::reference();
    let real = Doc::reference().real().snapshot_id("real-snapshot-001");
    import(&mut store, &fixture);
    assert_eq!(
        import(&mut store, &real),
        ImportOutcome::Stored {
            previous_current: None
        }
    );
    let (fixture_key, real_key) = (
        ProfileKey::of(&fixture.decode()),
        ProfileKey::of(&real.decode()),
    );
    assert!(fixture_key.fixture && !real_key.fixture);
    assert_eq!(
        store
            .current_snapshot(&fixture_key)
            .unwrap()
            .unwrap()
            .payload
            .id
            .as_str(),
        BASE_SNAPSHOT_ID
    );
    assert_eq!(
        store
            .current_snapshot(&real_key)
            .unwrap()
            .unwrap()
            .payload
            .id
            .as_str(),
        "real-snapshot-001"
    );
}

#[test]
fn observation_statuses_stay_distinct() {
    let db = TempDb::new("statuses");
    let mut store = SqliteStore::open(&db.path).unwrap();
    import(&mut store, &Doc::base());
    let status = |field: &str, key: &str| -> String {
        raw(&db.path)
            .query_row(
                "SELECT status FROM snapshot_observations WHERE field = ?1 AND entry_key = ?2",
                [field, key],
                |r| r.get(0),
            )
            .unwrap()
    };
    // A known false is not unknown; unknown, unsupported and restricted differ.
    assert_eq!(status("preferences", "show_tips"), "known");
    assert_eq!(status("preferences", "zero_like"), "not_applicable");
    assert_eq!(status("role", ""), "unknown");
    assert_eq!(status("equipment", "off_hand"), "restricted");
    let all: i64 = count(
        &db.path,
        "SELECT COUNT(DISTINCT status) FROM snapshot_observations",
    );
    assert!(all >= 4);
    assert_eq!(
        serde_json::to_value(ObservationStatus::NotApplicable).unwrap(),
        "not_applicable"
    );
}

#[test]
fn history_limit_is_validated_and_history_is_bounded() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let key = ProfileKey::of(&Doc::base().decode());
    assert_eq!(
        store.snapshot_history(&key, 0).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        store
            .snapshot_history(&key, MAX_HISTORY_LIMIT + 1)
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    for sequence in 1..=(MAX_SNAPSHOTS_PER_PROFILE as u64 + 4) {
        import(
            &mut store,
            &newer(&format!("demo-snapshot-{sequence:04}"), sequence),
        );
    }
    let history = store.snapshot_history(&key, MAX_HISTORY_LIMIT).unwrap();
    assert_eq!(history.len(), MAX_SNAPSHOTS_PER_PROFILE);
    assert!(
        history[0].is_current,
        "the newest import is current and retained"
    );
    assert_eq!(
        store.list_profiles().unwrap()[0].snapshot_count as usize,
        MAX_SNAPSHOTS_PER_PROFILE
    );
}

#[test]
fn delete_profile_is_scoped_to_one_profile() {
    let db = TempDb::new("delete");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let keep = Doc::base();
    let drop = Doc::base().character("demo-character-mage");
    import(&mut store, &keep);
    import(&mut store, &drop);
    let drop_key = ProfileKey::of(&drop.decode());
    let observations_per_snapshot =
        count(&db.path, "SELECT COUNT(*) FROM snapshot_observations") / 2;

    assert!(store.delete_profile(&drop_key).unwrap());
    assert!(!store.delete_profile(&drop_key).unwrap());
    assert!(store.current_snapshot(&drop_key).unwrap().is_none());
    assert_eq!(
        store
            .current_snapshot(&ProfileKey::of(&keep.decode()))
            .unwrap()
            .unwrap(),
        keep.decode()
    );
    assert_eq!(count(&db.path, "SELECT COUNT(*) FROM snapshots"), 1);
    assert_eq!(
        count(&db.path, "SELECT COUNT(*) FROM snapshot_observations"),
        observations_per_snapshot
    );
    // Audit rows survive with their snapshot reference cleared.
    assert_eq!(count(&db.path, "SELECT COUNT(*) FROM imports"), 2);
    assert_eq!(
        count(
            &db.path,
            "SELECT COUNT(*) FROM imports WHERE snapshot_id IS NULL"
        ),
        1
    );
    let _ = CharacterId::new(BASE_CHARACTER).unwrap();
}
