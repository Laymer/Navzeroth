mod common;
use azeroth_core::types::{Millis, PartyId, Sha256Hex};
use azeroth_storage::{
    SqliteStore,
    migrations::{MIGRATIONS, migrate_with},
    party_proposals::{LocalAcceptance, PartyProposal, ProposalGuard},
};
use std::collections::BTreeMap;
fn proposal() -> PartyProposal {
    PartyProposal {
        schema_version: 1,
        party_id: PartyId::new("duo").unwrap(),
        guard: ProposalGuard {
            proposal_id: Sha256Hex::new("a".repeat(64)).unwrap(),
            revision: 1,
        },
        proposer_profile_id: "one".into(),
        request: "{}".into(),
        result_hash: Sha256Hex::new("b".repeat(64)).unwrap(),
        context: common::Doc::base().decode().data_context(),
        fixture: true,
        created_at_utc_ms: None,
        acceptances: BTreeMap::new(),
        selected_objectives: vec![azeroth_core::types::ObjectiveId::new("selected").unwrap()],
        expected_total_ms: Millis::new(10).unwrap(),
    }
}
#[test]
fn durable_compare_and_swap_refuses_lost_acceptances_and_party_delete_cascades() {
    let db = common::TempDb::new("party-proposal-cas");
    let mut one = SqliteStore::open(&db.path).unwrap();
    common::raw(&db.path)
        .execute("INSERT INTO party_profiles(party_id) VALUES('duo')", [])
        .unwrap();
    let original = proposal();
    one.save_party_proposal(&original, None).unwrap();
    let mut two = SqliteStore::open(&db.path).unwrap();
    let mut first = two.party_proposal(&original.party_id).unwrap().unwrap();
    first.guard.revision = 2;
    first.acceptances.insert(
        "one".into(),
        LocalAcceptance {
            accepted_at_utc_ms: None,
        },
    );
    one.save_party_proposal(&first, Some(&original.guard))
        .unwrap();
    let mut racing = original.clone();
    racing.guard.revision = 2;
    racing.acceptances.insert(
        "two".into(),
        LocalAcceptance {
            accepted_at_utc_ms: None,
        },
    );
    assert!(
        two.save_party_proposal(&racing, Some(&original.guard))
            .is_err()
    );
    let kept = two.party_proposal(&original.party_id).unwrap().unwrap();
    assert!(kept.acceptances.contains_key("one"));
    assert!(!kept.acceptances.contains_key("two"));
    one.delete_party(&original.party_id).unwrap();
    assert!(two.party_proposal(&original.party_id).unwrap().is_none());
}
#[test]
fn version_seven_upgrade_preserves_reminders_calibration_and_party_membership() {
    let db = common::TempDb::new("proposal-v7-upgrade");
    let mut conn = common::raw(&db.path);
    migrate_with(&mut conn, &MIGRATIONS[..7]).unwrap();
    conn.execute("INSERT INTO profiles(character_id,edition,build,interface,namespace,is_fixture) VALUES('kept','wow_forever','UNVERIFIED_DEMO','UNVERIFIED_DEMO','demo',1)",[]).unwrap();
    conn.execute(
        "INSERT INTO reminder_state(profile_id,document) VALUES(1,'{\"disabled\":true}')",
        [],
    )
    .unwrap();
    conn.execute("INSERT INTO sessions(profile_id,session_id,content_sha256,document,summary_document) VALUES(1,'session',?1,'{}','{}')",["0".repeat(64)]).unwrap();
    conn.execute("INSERT INTO session_calibration_annotations(profile_id,session_id,document) VALUES(1,'session','{\"estimated\":0}')",[]).unwrap();
    conn.execute("INSERT INTO party_profiles(party_id) VALUES('duo')", [])
        .unwrap();
    let key = azeroth_storage::ProfileKey::of(&common::Doc::base().character("kept").decode());
    conn.execute("INSERT INTO party_participants(party_id,position,profile_id,profile_key) VALUES('duo',0,1,?1)",[serde_json::to_string(&key).unwrap()]).unwrap();
    drop(conn);
    let store = SqliteStore::open(&db.path).unwrap();
    assert_eq!(store.schema_version().unwrap(), 8);
    assert_eq!(
        store
            .party(&PartyId::new("duo").unwrap())
            .unwrap()
            .unwrap()
            .participants,
        vec![key]
    );
    let conn = common::raw(&db.path);
    assert_eq!(
        conn.query_row("SELECT document FROM reminder_state", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "{\"disabled\":true}"
    );
    assert_eq!(
        conn.query_row(
            "SELECT document FROM session_calibration_annotations",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "{\"estimated\":0}"
    );
    assert_eq!(
        common::count(&db.path, "SELECT COUNT(*) FROM party_profiles"),
        1
    );
    assert_eq!(
        common::count(&db.path, "SELECT COUNT(*) FROM party_plan_proposals"),
        0
    );
}
#[test]
fn corrupt_unknown_schema_or_oversized_document_is_refused_without_changing_good_record() {
    let db = common::TempDb::new("proposal-corrupt");
    let mut store = SqliteStore::open(&db.path).unwrap();
    common::raw(&db.path)
        .execute("INSERT INTO party_profiles(party_id) VALUES('duo')", [])
        .unwrap();
    let good = proposal();
    store.save_party_proposal(&good, None).unwrap();
    let mut malformed = good.clone();
    malformed.schema_version = 2;
    malformed.guard.revision = 2;
    assert!(
        store
            .save_party_proposal(&malformed, Some(&good.guard))
            .is_err()
    );
    let mut oversized = good.clone();
    oversized.request = "x".repeat(1024 * 1024 + 1);
    oversized.guard.revision = 2;
    assert!(
        store
            .save_party_proposal(&oversized, Some(&good.guard))
            .is_err()
    );
    assert_eq!(
        store.party_proposal(&good.party_id).unwrap().unwrap().guard,
        good.guard
    );
    common::raw(&db.path)
        .execute("UPDATE party_plan_proposals SET document='{}'", [])
        .unwrap();
    assert!(store.party_proposal(&good.party_id).is_err());
}
