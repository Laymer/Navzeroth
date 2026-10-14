mod common;
use azeroth_core::{
    error::ErrorCode,
    types::{MAX_SAFE_INTEGER, UtcMillis},
};
use azeroth_storage::{
    SqliteStore,
    content::{ContentRevision, StageOutcome},
    content_dispositions::*,
};
use common::{TempDb, raw};
use serde_json::{Value, json};
fn revision(name: &str) -> ContentRevision {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/content/demo-revision.json"
    ))
    .unwrap();
    value["envelope"]["payload"]["revision"] = json!(name);
    serde_json::from_value(value).unwrap()
}
fn stage(s: &mut SqliteStore, name: &str) -> ContentRevision {
    let r = revision(name);
    s.stage_content_revision(&r).unwrap();
    r
}
fn change(s: &mut SqliteStore, r: &ContentRevision, state: DispositionState) -> DispositionOutcome {
    let d = s.content_disposition(&r.key()).unwrap().unwrap();
    s.change_content_disposition(
        &DispositionMutation {
            guard: d.guard(),
            state,
            reason_code: DispositionReason::OperatorReview,
        },
        Some(UtcMillis::new(-1).unwrap()),
    )
    .unwrap()
}
#[test]
fn restrict_uses_only_previously_activated_eligible_content_and_preserves_documents() {
    let mut s = SqliteStore::open_in_memory().unwrap();
    let a = stage(&mut s, "a");
    let b = stage(&mut s, "b");
    let c = stage(&mut s, "c");
    let staged = stage(&mut s, "never-active");
    for r in [&a, &b, &c] {
        s.activate_content_revision(&r.key()).unwrap();
    }
    assert_eq!(
        change(&mut s, &c, DispositionState::Quarantined).active,
        Some(b.key())
    );
    assert_eq!(
        change(&mut s, &b, DispositionState::Revoked).active,
        Some(a.key())
    );
    let out = change(&mut s, &a, DispositionState::Revoked);
    assert!(out.active.is_none());
    assert!(!out.fallback_applied);
    assert!(s.active_content_revision(&a.key().scope).unwrap().is_none());
    assert!(s.activate_content_revision(&a.key()).is_err());
    assert!(s.activate_content_revision(&b.key()).is_err());
    assert!(s.activate_content_revision(&c.key()).is_err());
    assert_eq!(
        s.stage_content_revision(&c).unwrap(),
        StageOutcome::Duplicate
    );
    assert_eq!(
        s.content_disposition(&c.key()).unwrap().unwrap().state,
        DispositionState::Quarantined
    );
    assert_eq!(s.content_revision(&c.key()).unwrap().unwrap(), c);
    let released = change(&mut s, &c, DispositionState::Eligible);
    assert!(released.active.is_none());
    assert_eq!(released.disposition.state_revision, 3);
    assert_eq!(released.disposition.changed_at_utc_ms.unwrap().get(), -1);
    assert!(
        s.content_disposition(&staged.key())
            .unwrap()
            .unwrap()
            .last_activated_order
            .is_none()
    );
    assert!(
        s.rollback_content_revision(&a.key().scope)
            .unwrap()
            .is_none()
    );
}
#[test]
fn cross_process_guard_and_restrict_release_aba_remain_invalid_after_restart() {
    let db = TempDb::new("disposition-aba");
    let mut first = SqliteStore::open(&db.path).unwrap();
    let a = stage(&mut first, "a");
    first.activate_content_revision(&a.key()).unwrap();
    let mut second = SqliteStore::open(&db.path).unwrap();
    let old = second
        .content_disposition(&a.key())
        .unwrap()
        .unwrap()
        .guard();
    change(&mut first, &a, DispositionState::Quarantined);
    let stale = DispositionMutation {
        guard: old.clone(),
        state: DispositionState::Revoked,
        reason_code: DispositionReason::OperatorReview,
    };
    assert!(second.change_content_disposition(&stale, None).is_err());
    change(&mut first, &a, DispositionState::Eligible);
    first.activate_content_revision(&a.key()).unwrap();
    assert!(second.validate_usable_content_guard(&old).is_err());
    drop(first);
    drop(second);
    let mut reopened = SqliteStore::open(&db.path).unwrap();
    let (_, d) = reopened.usable_content_revision(&a.key()).unwrap().unwrap();
    assert_eq!(d.state_revision, 3);
    assert_eq!(d.state, DispositionState::Eligible);
    change(&mut reopened, &a, DispositionState::Revoked);
    assert!(
        reopened
            .change_content_disposition(
                &DispositionMutation {
                    guard: reopened
                        .content_disposition(&a.key())
                        .unwrap()
                        .unwrap()
                        .guard(),
                    state: DispositionState::Eligible,
                    reason_code: DispositionReason::OperatorReview
                },
                None
            )
            .is_err()
    );
}
#[test]
fn exact_scope_hash_and_safe_counter_guards_refuse_without_mutation() {
    let db = TempDb::new("disposition-guards");
    let mut s = SqliteStore::open(&db.path).unwrap();
    let a = stage(&mut s, "a");
    s.activate_content_revision(&a.key()).unwrap();
    let mut guard = s.content_disposition(&a.key()).unwrap().unwrap().guard();
    guard.content_sha256 = azeroth_core::types::Sha256Hex::new("f".repeat(64)).unwrap();
    assert!(
        s.change_content_disposition(
            &DispositionMutation {
                guard,
                state: DispositionState::Quarantined,
                reason_code: DispositionReason::OperatorReview
            },
            None
        )
        .is_err()
    );
    raw(&db.path)
        .execute(
            "UPDATE content_dispositions SET state_revision=?1",
            [MAX_SAFE_INTEGER as i64],
        )
        .unwrap();
    let guard = s.content_disposition(&a.key()).unwrap().unwrap().guard();
    assert!(
        s.change_content_disposition(
            &DispositionMutation {
                guard,
                state: DispositionState::Quarantined,
                reason_code: DispositionReason::OperatorReview
            },
            None
        )
        .is_err()
    );
    assert!(s.active_content_revision(&a.key().scope).unwrap().is_some());
}
#[test]
fn malformed_fallback_is_skipped_and_damaged_active_can_be_restricted() {
    let db = TempDb::new("disposition-malformed");
    let mut s = SqliteStore::open(&db.path).unwrap();
    let a = stage(&mut s, "a");
    let b = stage(&mut s, "b");
    let c = stage(&mut s, "c");
    for r in [&a, &b, &c] {
        s.activate_content_revision(&r.key()).unwrap();
    }
    raw(&db.path)
        .execute_batch("DROP TRIGGER dataset_revisions_no_update;")
        .unwrap();
    raw(&db.path)
        .execute(
            "UPDATE dataset_revisions SET document='{}' WHERE revision IN ('b','c')",
            [],
        )
        .unwrap();
    let out = change(&mut s, &c, DispositionState::Quarantined);
    assert_eq!(out.active, Some(a.key()));
    assert!(out.fallback_applied);
    assert!(s.active_content_revision(&a.key().scope).unwrap().is_some());
}
#[test]
fn malformed_only_fallback_clears_active_but_real_sql_failure_rolls_back_restriction() {
    let db = TempDb::new("disposition-sql");
    let mut s = SqliteStore::open(&db.path).unwrap();
    let a = stage(&mut s, "a");
    let b = stage(&mut s, "b");
    s.activate_content_revision(&a.key()).unwrap();
    s.activate_content_revision(&b.key()).unwrap();
    raw(&db.path)
        .execute_batch("DROP TRIGGER dataset_revisions_no_update;")
        .unwrap();
    raw(&db.path)
        .execute(
            "UPDATE dataset_revisions SET document='{}' WHERE revision='a'",
            [],
        )
        .unwrap();
    raw(&db.path).execute_batch("CREATE TRIGGER refuse_clear BEFORE DELETE ON active_datasets BEGIN SELECT RAISE(ABORT,'synthetic denial'); END;").unwrap();
    let before = s.content_disposition(&b.key()).unwrap().unwrap();
    let failure = s
        .change_content_disposition(
            &DispositionMutation {
                guard: before.guard(),
                state: DispositionState::Revoked,
                reason_code: DispositionReason::InvalidContent,
            },
            None,
        )
        .unwrap_err();
    assert_eq!(failure.code, ErrorCode::StorageError);
    assert_eq!(s.content_disposition(&b.key()).unwrap().unwrap(), before);
    assert_eq!(
        s.active_content_revision(&b.key().scope)
            .unwrap()
            .unwrap()
            .key(),
        b.key()
    );
    raw(&db.path)
        .execute_batch("DROP TRIGGER refuse_clear;")
        .unwrap();
    let out = change(&mut s, &b, DispositionState::Revoked);
    assert!(out.active.is_none());
    assert!(s.active_content_revision(&b.key().scope).unwrap().is_none());
}
#[test]
fn activated_fallback_does_not_cross_build_or_fixture_scope_and_rollback_refuses_blocked() {
    let mut s = SqliteStore::open_in_memory().unwrap();
    let a = stage(&mut s, "a");
    let b = stage(&mut s, "b");
    for r in [&a, &b] {
        s.activate_content_revision(&r.key()).unwrap();
    }
    change(&mut s, &a, DispositionState::Quarantined);
    assert!(s.rollback_content_revision(&a.key().scope).is_err());
    let mut value = serde_json::to_value(revision("foreign")).unwrap();
    fn walk(v: &mut Value) {
        match v {
            Value::Object(m) => {
                if m.contains_key("build") && m.contains_key("interface") {
                    m.insert("build".into(), json!("other-synthetic-build"));
                }
                for child in m.values_mut() {
                    walk(child);
                }
            }
            Value::Array(a) => {
                for child in a {
                    walk(child);
                }
            }
            _ => {}
        }
    }
    walk(&mut value);
    let foreign: ContentRevision = serde_json::from_value(value).unwrap();
    s.stage_content_revision(&foreign).unwrap();
    s.activate_content_revision(&foreign.key()).unwrap();
    assert!(
        change(&mut s, &b, DispositionState::Revoked)
            .active
            .is_none()
    );
    assert_eq!(
        s.active_content_revision(&foreign.key().scope)
            .unwrap()
            .unwrap()
            .key(),
        foreign.key()
    );
}
#[test]
fn backup_restore_preserves_restrictions_counters_and_activation_history() {
    let db = TempDb::new("disposition-backup");
    let backup = TempDb::new("disposition-copy");
    let target = TempDb::new("disposition-restored");
    let mut s = SqliteStore::open(&db.path).unwrap();
    let a = stage(&mut s, "a");
    let b = stage(&mut s, "b");
    s.activate_content_revision(&a.key()).unwrap();
    s.activate_content_revision(&b.key()).unwrap();
    change(&mut s, &b, DispositionState::Quarantined);
    let before = s.list_content_dispositions(&a.key().scope).unwrap();
    validate_content_dispositions(&raw(&db.path)).unwrap();
    s.backup_to(&backup.path).unwrap();
    azeroth_storage::recovery::reconstruct_backup(&backup.path, &target.path).unwrap();
    let restored = SqliteStore::open(&target.path).unwrap();
    assert_eq!(
        restored.list_content_dispositions(&a.key().scope).unwrap(),
        before
    );
    validate_content_dispositions(&raw(&target.path)).unwrap();
    assert_eq!(
        restored
            .active_content_revision(&a.key().scope)
            .unwrap()
            .unwrap()
            .key(),
        a.key()
    );
    assert!(restored.usable_content_revision(&b.key()).is_err());
}
#[test]
fn recovery_validator_refuses_blocked_active_and_missing_disposition_rows() {
    let db = TempDb::new("disposition-validator");
    let mut s = SqliteStore::open(&db.path).unwrap();
    let a = stage(&mut s, "a");
    s.activate_content_revision(&a.key()).unwrap();
    let conn = raw(&db.path);
    conn.execute("UPDATE content_dispositions SET state='quarantined',state_revision=2,reason_code='operator_review'",[]).unwrap();
    assert!(validate_content_dispositions(&conn).is_err());
    conn.execute("DELETE FROM content_dispositions", [])
        .unwrap();
    assert!(validate_content_dispositions(&conn).is_err());
}
#[test]
fn ambiguous_activation_history_never_invents_most_recent_fallback() {
    let db = TempDb::new("disposition-order-tie");
    let mut s = SqliteStore::open(&db.path).unwrap();
    let a = stage(&mut s, "a");
    let b = stage(&mut s, "b");
    let c = stage(&mut s, "c");
    for r in [&a, &b, &c] {
        s.activate_content_revision(&r.key()).unwrap();
    }
    let conn = raw(&db.path);
    conn.execute("UPDATE content_dispositions SET last_activated_order=1 WHERE revision_id=(SELECT id FROM dataset_revisions WHERE revision='b')",[]).unwrap();
    assert!(validate_content_dispositions(&conn).is_err());
    let out = change(&mut s, &c, DispositionState::Quarantined);
    assert!(out.active.is_none());
    assert!(!out.fallback_applied);
    assert_eq!(
        s.content_disposition(&c.key()).unwrap().unwrap().state,
        DispositionState::Quarantined
    );
}
#[test]
fn exhausted_activation_order_commits_restriction_and_clears_instead_of_rolling_back() {
    let db = TempDb::new("disposition-order-exhausted");
    let mut s = SqliteStore::open(&db.path).unwrap();
    let a = stage(&mut s, "a");
    let b = stage(&mut s, "b");
    s.activate_content_revision(&a.key()).unwrap();
    s.activate_content_revision(&b.key()).unwrap();
    raw(&db.path).execute("UPDATE content_dispositions SET last_activated_order=9007199254740991 WHERE revision_id=(SELECT id FROM dataset_revisions WHERE revision='b')",[]).unwrap();
    let out = change(&mut s, &b, DispositionState::Revoked);
    assert!(out.active.is_none());
    assert!(!out.fallback_applied);
    assert_eq!(out.disposition.state, DispositionState::Revoked);
    assert!(s.active_content_revision(&b.key().scope).unwrap().is_none());
}
