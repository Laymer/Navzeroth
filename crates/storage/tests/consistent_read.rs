mod common;
use azeroth_core::{
    error::{AppError, ErrorCode},
    types::PartyId,
};
use azeroth_storage::SqliteStore;
use std::time::Duration;
#[test]
fn validation_reservation_excludes_another_connection_writer_until_handoff() {
    let db = common::TempDb::new("consistent-read");
    let store = SqliteStore::open(&db.path).unwrap();
    common::raw(&db.path)
        .execute("CREATE TABLE transaction_probe(value INTEGER)", [])
        .unwrap();
    store
        .with_immediate_read(|view| {
            assert!(view.party(&PartyId::new("missing").unwrap())?.is_none());
            let writer = common::raw(&db.path);
            writer.busy_timeout(Duration::from_millis(40)).unwrap();
            let error = writer
                .execute("INSERT INTO transaction_probe(value) VALUES(1)", [])
                .unwrap_err();
            assert_eq!(
                error.sqlite_error_code(),
                Some(rusqlite::ErrorCode::DatabaseBusy)
            );
            assert!(view.party(&PartyId::new("after").unwrap())?.is_none());
            Ok(())
        })
        .unwrap();
    common::raw(&db.path)
        .execute("INSERT INTO transaction_probe(value) VALUES(1)", [])
        .unwrap();
    let count: i64 = common::raw(&db.path)
        .query_row("SELECT COUNT(*) FROM transaction_probe", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}
#[test]
fn rejected_or_nested_validation_rolls_back_and_releases_the_writer_reservation() {
    let db = common::TempDb::new("consistent-read-error");
    let store = SqliteStore::open(&db.path).unwrap();
    common::raw(&db.path)
        .execute("CREATE TABLE transaction_probe(value INTEGER)", [])
        .unwrap();
    let error = store
        .with_immediate_read::<()>(|_| {
            Err(AppError::new(
                ErrorCode::InvalidInput,
                "Rejected validation",
                false,
            ))
        })
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    let error = store
        .with_immediate_read(|view| view.with_immediate_read(|_| Ok(())))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::StorageError);
    common::raw(&db.path)
        .execute("INSERT INTO transaction_probe(value) VALUES(1)", [])
        .unwrap();
    assert!(
        store
            .with_immediate_read(|view| view.party(&PartyId::new("missing").unwrap()))
            .unwrap()
            .is_none()
    );
}
