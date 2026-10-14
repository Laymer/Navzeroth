mod common;
use azeroth_storage::{SqliteStore, recovery::reconstruct_backup, today::Settings};
use common::{TempDb, raw};
#[test]
fn extra_future_history_row_refuses_without_destination_or_source_writes() {
    let source = TempDb::new("future-history-local");
    let destination = TempDb::new("future-history-local-output");
    drop(SqliteStore::open(&source.path).unwrap());
    raw(&source.path)
        .execute(
            "INSERT INTO schema_migrations(version,name) VALUES(11,'unsupported future')",
            [],
        )
        .unwrap();
    let bytes = std::fs::read(&source.path).unwrap();
    assert!(reconstruct_backup(&source.path, &destination.path).is_err());
    assert!(!destination.path.exists());
    assert_eq!(std::fs::read(&source.path).unwrap(), bytes);
}
#[test]
fn populated_settings_unknown_duplicate_and_future_fields_fail_closed() {
    let settings = serde_json::to_string(&Settings::default()).unwrap();
    for (label, document) in [
        ("unknown", format!("{{\"hidden\":true,{}", &settings[1..])),
        (
            "duplicate",
            format!("{{\"schema_version\":1,{}", &settings[1..]),
        ),
        (
            "future",
            settings.replace("\"schema_version\":1", "\"schema_version\":2"),
        ),
    ] {
        let source = TempDb::new(label);
        let destination = TempDb::new(&format!("refused-{label}"));
        drop(SqliteStore::open(&source.path).unwrap());
        raw(&source.path)
            .execute("INSERT INTO app_settings VALUES(1,1,7,?1)", [document])
            .unwrap();
        let bytes = std::fs::read(&source.path).unwrap();
        assert!(
            reconstruct_backup(&source.path, &destination.path).is_err(),
            "{label}"
        );
        assert!(!destination.path.exists());
        assert_eq!(std::fs::read(&source.path).unwrap(), bytes);
    }
}
#[test]
fn every_queue_state_and_erased_revocation_restore_without_resurrection() {
    use azeroth_storage::local_submissions::{SubmissionAction as A, SubmissionState as S};
    let source = TempDb::new("all-queue-states");
    let destination = TempDb::new("all-queue-states-output");
    let mut store = SqliteStore::open(&source.path).unwrap();
    let content = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/content/demo-revision.json"
    ))
    .unwrap();
    store.stage_content_revision(&content).unwrap();
    let observation = azeroth_core::submissions::decode_local_observation(include_str!(
        "../../../fixtures/desktop/submissions/synthetic-price.json"
    ))
    .unwrap();
    let mut expected = Vec::new();
    for (index, state) in [
        S::Pending,
        S::Accepted,
        S::Rejected,
        S::Withdrawn,
        S::Revoked,
        S::Revoked,
    ]
    .into_iter()
    .enumerate()
    {
        let mut doc = observation.clone();
        doc.observed_at_utc_ms = Some(azeroth_core::types::UtcMillis::new(index as i64).unwrap());
        let mut record = store
            .submit_local_observation(&doc, true, None)
            .unwrap()
            .record;
        let action = match state {
            S::Pending => None,
            S::Accepted => Some(A::Accept),
            S::Rejected => Some(A::Reject),
            S::Withdrawn => Some(A::Withdraw),
            S::Revoked => Some(A::Revoke),
        };
        if let Some(action) = action {
            record = store
                .change_local_submission(&record.guard, action, None)
                .unwrap();
        }
        if index == 5 {
            record = store
                .change_local_submission(&record.guard, A::EraseRevoked, None)
                .unwrap();
        }
        expected.push((doc, record));
    }
    drop(store);
    let bytes = std::fs::read(&source.path).unwrap();
    reconstruct_backup(&source.path, &destination.path).unwrap();
    assert_eq!(std::fs::read(&source.path).unwrap(), bytes);
    let mut restored = SqliteStore::open(&destination.path).unwrap();
    assert_eq!(restored.local_submission_page(None, 32).unwrap().total, 6);
    for (doc, record) in expected {
        assert_eq!(
            restored
                .local_submission(&record.guard.id)
                .unwrap()
                .unwrap(),
            record
        );
        let duplicate = restored.submit_local_observation(&doc, true, None).unwrap();
        assert!(duplicate.duplicate);
        assert_eq!(duplicate.record, record);
    }
}
