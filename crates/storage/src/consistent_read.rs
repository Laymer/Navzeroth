//! Consistent read/validation with exclusion of concurrent SQLite writers.
use crate::{
    SqliteStore,
    error::{SqliteResult, storage},
};
use azeroth_core::error::Result;
use rusqlite::{Transaction, TransactionBehavior};
impl SqliteStore {
    /// Hold a consistent view and a SQLite writer reservation until validation
    /// and its bounded handoff finish. The callback must not start transactions
    /// or perform writes. This does not authenticate other local processes.
    pub fn with_immediate_read<T>(&self, read: impl FnOnce(&Self) -> Result<T>) -> Result<T> {
        if !self.conn.is_autocommit() {
            return Err(storage("Consistent validation cannot nest a transaction"));
        }
        let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate).db()?;
        match read(self) {
            Ok(value) => {
                tx.commit().db()?;
                Ok(value)
            }
            Err(error) => {
                tx.rollback().db()?;
                Err(error)
            }
        }
    }
}
