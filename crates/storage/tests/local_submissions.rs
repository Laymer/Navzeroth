mod common;
use azeroth_core::{
    submissions::{LocalObservation, decode_local_observation},
    types::UtcMillis,
};
use azeroth_storage::{
    SqliteStore,
    content::ContentRevision,
    local_submissions::{SubmissionAction, SubmissionState, validate_local_submissions},
};
use serde_json::{Value, json};
fn content() -> ContentRevision {
    serde_json::from_str(include_str!(
        "../../../fixtures/desktop/content/demo-revision.json"
    ))
    .unwrap()
}
fn doc() -> LocalObservation {
    decode_local_observation(include_str!(
        "../../../fixtures/desktop/submissions/synthetic-price.json"
    ))
    .unwrap()
}
fn store() -> SqliteStore {
    let mut s = SqliteStore::open_in_memory().unwrap();
    s.stage_content_revision(&content()).unwrap();
    s
}
#[test]
fn explicit_opt_in_dedup_withdrawal_and_terminal_revocation_preserve_tombstones() {
    let mut s = store();
    let d = doc();
    assert!(s.submit_local_observation(&d, false, None).is_err());
    let first = s
        .submit_local_observation(&d, true, Some(UtcMillis::new(-1).unwrap()))
        .unwrap();
    assert!(!first.duplicate);
    let duplicate = s
        .submit_local_observation(&d, true, Some(UtcMillis::new(0).unwrap()))
        .unwrap();
    assert!(duplicate.duplicate);
    assert_eq!(duplicate.record, first.record);
    let accepted = s
        .change_local_submission(&first.record.guard, SubmissionAction::Accept, None)
        .unwrap();
    assert_eq!(accepted.state, SubmissionState::Accepted);
    assert!(
        s.change_local_submission(&first.record.guard, SubmissionAction::Withdraw, None)
            .is_err()
    );
    let erased = s
        .change_local_submission(&accepted.guard, SubmissionAction::Withdraw, None)
        .unwrap();
    assert_eq!(erased.state, SubmissionState::Withdrawn);
    assert!(erased.document.is_none());
    let duplicate = s.submit_local_observation(&d, true, None).unwrap();
    assert!(duplicate.duplicate);
    assert_eq!(duplicate.record, erased);
    assert!(
        s.change_local_submission(&erased.guard, SubmissionAction::Accept, None)
            .is_err()
    );
    let mut another = d;
    another.observed_at_utc_ms = Some(UtcMillis::new(1).unwrap());
    let p = s.submit_local_observation(&another, true, None).unwrap();
    let revoked = s
        .change_local_submission(&p.record.guard, SubmissionAction::Revoke, None)
        .unwrap();
    assert!(revoked.document.is_some());
    let erased = s
        .change_local_submission(&revoked.guard, SubmissionAction::EraseRevoked, None)
        .unwrap();
    assert_eq!(erased.state, SubmissionState::Revoked);
    assert!(erased.document.is_none());
    assert!(
        s.change_local_submission(&erased.guard, SubmissionAction::Accept, None)
            .is_err()
    );
    validate_local_submissions(&s).unwrap();
}
#[test]
fn exact_build_fixture_source_evidence_and_target_links_are_required() {
    let mut s = store();
    for edit in ["build", "fixture", "source", "evidence", "item"] {
        let mut v = serde_json::to_value(doc()).unwrap();
        match edit {
            "build" => v["context"]["build"]["build"] = json!("different"),
            "fixture" => v["fixture"] = json!(false),
            "source" => v["source"] = json!("private-unregistered-source"),
            "evidence" => v["evidence"] = json!("missing-evidence"),
            _ => v["payload"]["item"] = json!("unregistered-item"),
        };
        let d = decode_local_observation(&v.to_string()).unwrap();
        let e = s.submit_local_observation(&d, true, None).unwrap_err();
        assert!(!e.to_string().contains("private-unregistered"));
    }
    assert_eq!(s.local_submission_page(None, 32).unwrap().total, 0);
}
#[test]
fn queue_capacity_includes_tombstones_and_pages_never_truncate_silently() {
    let mut s = store();
    for i in 0..1024 {
        let mut d = doc();
        d.observed_at_utc_ms = Some(UtcMillis::new(i).unwrap());
        let row = s.submit_local_observation(&d, true, None).unwrap();
        if i == 0 {
            s.change_local_submission(&row.record.guard, SubmissionAction::Withdraw, None)
                .unwrap();
        }
    }
    let mut next = None;
    let mut seen = 0;
    loop {
        let page = s.local_submission_page(next.as_ref(), 32).unwrap();
        assert_eq!(page.total, 1024);
        assert!(page.rows.len() <= 32);
        seen += page.rows.len();
        match page.next {
            Some(cursor) => next = Some(cursor),
            None => break,
        }
    }
    assert_eq!(seen, 1024);
    let mut extra = doc();
    extra.observed_at_utc_ms = Some(UtcMillis::new(1024).unwrap());
    assert!(s.submit_local_observation(&extra, true, None).is_err());
    assert!(s.local_submission_page(None, 33).is_err());
    validate_local_submissions(&s).unwrap();
}
#[test]
fn second_connection_stale_cas_is_rejected_without_overwriting_payload() {
    let temp = common::TempDb::new("submission-cas");
    let mut one = SqliteStore::open(&temp.path).unwrap();
    one.stage_content_revision(&content()).unwrap();
    let mut two = SqliteStore::open(&temp.path).unwrap();
    let first = one.submit_local_observation(&doc(), true, None).unwrap();
    let old = two
        .local_submission(&first.record.guard.id)
        .unwrap()
        .unwrap();
    let accepted = one
        .change_local_submission(&old.guard, SubmissionAction::Accept, None)
        .unwrap();
    assert!(
        two.change_local_submission(&old.guard, SubmissionAction::Revoke, None)
            .is_err()
    );
    assert_eq!(
        two.local_submission(&old.guard.id).unwrap().unwrap(),
        accepted
    );
}
#[test]
fn typed_validator_refuses_corrupt_fingerprint_payload_and_state_even_after_sql_checks_bypass() {
    for mutation in [
        "UPDATE local_submissions SET fingerprint=printf('%064d',0)",
        "UPDATE local_submissions SET document='{}'",
        "UPDATE local_submissions SET state_revision=0",
        "UPDATE local_submissions SET decision_code='accepted_local_review'",
    ] {
        let temp = common::TempDb::new("submission-corrupt");
        let mut s = SqliteStore::open(&temp.path).unwrap();
        s.stage_content_revision(&content()).unwrap();
        s.submit_local_observation(&doc(), true, None).unwrap();
        let raw = rusqlite::Connection::open(&temp.path).unwrap();
        raw.execute_batch("PRAGMA ignore_check_constraints=ON")
            .unwrap();
        raw.execute_batch(mutation).unwrap();
        assert!(validate_local_submissions(&s).is_err(), "{mutation}");
    }
}
#[test]
fn each_supported_payload_remains_a_declaration_without_confidence_promotion() {
    let mut s = store();
    let mut c = content();
    let mut v = serde_json::to_value(&c.envelope).unwrap();
    v["payload"]["revision"] = json!("quest-cited-r1");
    v["payload"]["quests"][0]["evidence"] = json!(["demo-evidence-1"]);
    c.envelope = azeroth_core::contract::decode_envelope(&v.to_string()).unwrap();
    s.stage_content_revision(&c).unwrap();
    let mut d: Value = serde_json::to_value(doc()).unwrap();
    d["base_revision"] = json!("quest-cited-r1");
    d["payload"] =
        json!({"kind":"quest_state","quest":"demo-quest-1","state":{"status":"unsupported"}});
    let q = s
        .submit_local_observation(
            &decode_local_observation(&d.to_string()).unwrap(),
            true,
            None,
        )
        .unwrap();
    assert_eq!(q.record.state, SubmissionState::Pending);
    d["payload"] = json!({"kind":"acquisition_outcome","item":"demo-item-1","acquired":{"status":"known","value":false},"opportunities":{"status":"unknown"},"successes":{"status":"known","value":0},"group_eligible":{"status":"restricted"},"personally_received":{"status":"stale"}});
    s.submit_local_observation(
        &decode_local_observation(&d.to_string()).unwrap(),
        true,
        None,
    )
    .unwrap();
    validate_local_submissions(&s).unwrap();
}
