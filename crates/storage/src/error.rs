//! Storage failures reach callers only as safe `AppError` values. SQLite error
//! text can contain SQL or file paths, so it is never forwarded; messages are
//! static strings and imported content never appears in them.
use azeroth_core::error::{AppError, ErrorCode};

/// A non-retryable storage failure with a static, content-free message.
pub(crate) fn storage(message: &'static str) -> AppError {
    AppError::new(ErrorCode::StorageError, message, false)
}

/// Rejected caller input (core's `invalid` helper is crate-private).
pub(crate) fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}

/// Maps a SQLite failure to `STORAGE_ERROR`. Only BUSY and LOCKED are
/// retryable; the SQLite message itself is dropped.
pub(crate) fn from_sqlite(error: rusqlite::Error) -> AppError {
    let busy = matches!(
        &error,
        rusqlite::Error::SqliteFailure(failure, _)
            if matches!(
                failure.code,
                rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
            )
    );
    if busy {
        AppError::new(ErrorCode::StorageError, "Database is busy", true)
    } else {
        storage("Database operation failed")
    }
}

/// Lets `?` convert SQLite errors inside functions returning `Result<_, AppError>`.
pub(crate) trait SqliteResult<T> {
    fn db(self) -> Result<T, AppError>;
}
impl<T> SqliteResult<T> for rusqlite::Result<T> {
    fn db(self) -> Result<T, AppError> {
        self.map_err(from_sqlite)
    }
}
