//! Local error constructors. `azeroth_core::error::invalid` is crate private, so
//! ingest builds its own from `AppError::new`. Messages are `'static` on
//! purpose: imported content can never reach them.
use azeroth_core::error::{AppError, ErrorCode};

pub(crate) fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
pub(crate) fn too_large(message: &'static str) -> AppError {
    AppError::new(ErrorCode::ImportTooLarge, message, false)
}
/// A read failure can be transient (the game may hold the file), so it is retryable.
pub(crate) fn unreadable(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, true)
}
