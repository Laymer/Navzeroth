mod common;
use azeroth_core::{observation::Observation, types::PartyId};
use azeroth_storage::{
    ImportSource, ProfileKey, SnapshotStore, SqliteStore,
    party::{MAX_PARTICIPANTS, MAX_PARTIES, PartyProfile},
};
use common::{Doc, TempDb, raw};

fn seed(store: &mut SqliteStore, character: &str) -> ProfileKey {
    let envelope = Doc::base().character(character).decode();
    store
        .import_character_snapshot(&envelope, ImportSource::Fixture, None)
        .unwrap();
    ProfileKey::of(&envelope)
}
fn party(keys: Vec<ProfileKey>) -> PartyProfile {
    PartyProfile {
        id: PartyId::new("local-duo").unwrap(),
        participants: keys,
    }
}

#[test]
fn membership_survives_reopen_deletion_and_reimport_without_changing_snapshots() {
    let db = TempDb::new("party");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let one = seed(&mut store, "one");
    let two = seed(&mut store, "two");
    let expected = party(vec![two.clone(), one.clone()]);
    store.save_party(&expected).unwrap();
    drop(store);
    let mut store = SqliteStore::open(&db.path).unwrap();
    assert_eq!(store.party(&expected.id).unwrap(), Some(expected.clone()));
    assert_eq!(store.party_ids().unwrap(), vec![expected.id.clone()]);
    assert!(store.delete_profile(&two).unwrap());
    assert_eq!(store.party(&expected.id).unwrap(), Some(expected.clone()));
    assert!(store.current_snapshot(&two).unwrap().is_none());
    assert!(store.current_snapshot(&one).unwrap().is_some());
    seed(&mut store, "two");
    assert!(store.current_snapshot(&two).unwrap().is_some());
    assert!(store.delete_party(&expected.id).unwrap());
    assert!(!store.delete_party(&expected.id).unwrap());
    assert_eq!(store.list_profiles().unwrap().len(), 2);
    assert_eq!(store.snapshot_history(&one, 8).unwrap().len(), 1);
}

#[test]
fn invalid_replacements_roll_back_and_keep_the_original_party() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let one = seed(&mut store, "one");
    let two = seed(&mut store, "two");
    let original = party(vec![one.clone(), two.clone()]);
    store.save_party(&original).unwrap();
    let mut missing = two.clone();
    missing.character = azeroth_core::types::CharacterId::new("missing").unwrap();
    let mut fixture = two.clone();
    fixture.fixture = false;
    let reference = ProfileKey::of(&Doc::reference().character("three").decode());
    let build = ProfileKey::of(&Doc::base().character("three").build("OTHER").decode());
    for keys in [
        vec![],
        vec![one.clone(), one.clone()],
        vec![one.clone(), missing],
        vec![one.clone(), fixture],
        vec![one.clone(), reference],
        vec![one.clone(), build],
        vec![one.clone(); MAX_PARTICIPANTS + 1],
    ] {
        assert!(store.save_party(&party(keys)).is_err());
        assert_eq!(store.party(&original.id).unwrap(), Some(original.clone()));
    }
}

#[test]
fn parties_are_bounded_and_existing_parties_can_be_updated_at_capacity() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let one = seed(&mut store, "one");
    for n in 0..MAX_PARTIES {
        let mut value = party(vec![one.clone()]);
        value.id = PartyId::new(format!("party-{n:03}")).unwrap();
        store.save_party(&value).unwrap();
    }
    assert_eq!(store.party_ids().unwrap().len(), MAX_PARTIES);
    assert!(store.save_party(&party(vec![one.clone()])).is_err());
    let mut existing = party(vec![one]);
    existing.id = PartyId::new("party-000").unwrap();
    store.save_party(&existing).unwrap();
}

#[test]
fn profile_without_snapshot_is_a_valid_member_and_corrupt_membership_is_refused() {
    let db = TempDb::new("party-missing");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let key = ProfileKey::of(&Doc::base().decode());
    store
        .set_preference(
            &key,
            &azeroth_core::types::PreferenceId::new("manual").unwrap(),
            &Observation::known(false, Doc::base().decode().payload.capture.clone()).unwrap(),
        )
        .unwrap();
    let value = party(vec![key.clone()]);
    store.save_party(&value).unwrap();
    assert!(store.current_snapshot(&key).unwrap().is_none());
    raw(&db.path)
        .execute("UPDATE party_participants SET profile_key = '{}'", [])
        .unwrap();
    assert!(store.party(&value.id).is_err());
}
