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
        // The entry stamp prevents caching any transaction-local catalog after
        // content writes/schema changes, even if a callback violates read-only.
        let cache_read = self.catalog_generation.begin_read(&self.conn)?;
        let result = read(self);
        drop(cache_read);
        match result {
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
