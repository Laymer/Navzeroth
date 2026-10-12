//! Profile preferences keep status and value distinct.
mod common;

use azeroth_core::{
    observation::{Observation, ObservationMetadata, ObservationStatus},
    types::PreferenceId,
};
use azeroth_storage::{MAX_PREFERENCES_PER_PROFILE, ProfileKey, SnapshotStore, SqliteStore};
use common::{Doc, TempDb, raw};

fn key() -> ProfileKey {
    ProfileKey::of(&Doc::base().decode())
}
fn known(value: bool) -> Observation<bool> {
    Observation::known(value, ObservationMetadata::default()).unwrap()
}
fn id(name: &str) -> PreferenceId {
    PreferenceId::new(name).unwrap()
}

#[test]
fn false_unknown_and_restricted_are_stored_distinctly() {
    let db = TempDb::new("prefs");
    let mut store = SqliteStore::open(&db.path).unwrap();
    store
        .set_preference(&key(), &id("a_false"), &known(false))
        .unwrap();
    store
        .set_preference(&key(), &id("b_true"), &known(true))
        .unwrap();
    store
        .set_preference(&key(), &id("c_unknown"), &Observation::unknown())
        .unwrap();
    let restricted = Observation::unavailable(
        ObservationStatus::Restricted,
        ObservationMetadata::default(),
    )
    .unwrap();
    store
        .set_preference(&key(), &id("d_restricted"), &restricted)
        .unwrap();

    let all = store.preferences(&key()).unwrap();
    assert_eq!(all.len(), 4);
    assert_eq!(all[&id("a_false")].value(), Some(&false));
    assert_eq!(all[&id("b_true")].value(), Some(&true));
    assert_eq!(all[&id("c_unknown")].status(), ObservationStatus::Unknown);
    assert_eq!(
        all[&id("d_restricted")].status(),
        ObservationStatus::Restricted
    );
    assert_eq!(all[&id("c_unknown")].value(), None);

    let nulls: i64 = raw(&db.path)
        .query_row(
            "SELECT COUNT(*) FROM profile_preferences WHERE value IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(nulls, 2, "only unavailable observations have a NULL value");
}

#[test]
fn upsert_replaces_and_profile_is_created_on_demand() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    assert!(store.preferences(&key()).unwrap().is_empty());
    store
        .set_preference(&key(), &id("x"), &known(true))
        .unwrap();
    store
        .set_preference(&key(), &id("x"), &known(false))
        .unwrap();
    assert_eq!(
        store.preferences(&key()).unwrap()[&id("x")].value(),
        Some(&false)
    );
    let profiles = store.list_profiles().unwrap();
    assert_eq!(profiles.len(), 1);
    assert!(profiles[0].current.is_none());
    assert_eq!(profiles[0].snapshot_count, 0);
}

#[test]
fn preferences_are_scoped_per_profile_and_deleted_with_it() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let other = ProfileKey::of(&Doc::reference().decode());
    store
        .set_preference(&key(), &id("x"), &known(true))
        .unwrap();
    assert!(store.preferences(&other).unwrap().is_empty());
    store
        .set_preference(&other, &id("x"), &known(false))
        .unwrap();
    assert!(store.delete_profile(&key()).unwrap());
    assert!(store.preferences(&key()).unwrap().is_empty());
    assert_eq!(
        store.preferences(&other).unwrap()[&id("x")].value(),
        Some(&false)
    );
}

#[test]
fn preference_count_is_bounded() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    for n in 0..MAX_PREFERENCES_PER_PROFILE {
        store
            .set_preference(&key(), &id(&format!("p{n:04}")), &known(true))
            .unwrap();
    }
    let error = store
        .set_preference(&key(), &id("one-too-many"), &known(true))
        .unwrap_err();
    assert_eq!(error.code, azeroth_core::error::ErrorCode::InvalidInput);
    // Replacing an existing preference is still allowed at the limit.
    store
        .set_preference(&key(), &id("p0000"), &known(false))
        .unwrap();
}
