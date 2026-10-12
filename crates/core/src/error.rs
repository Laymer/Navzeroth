//! Safe application errors and reusable validated records.
use serde::{Deserialize, Deserializer, Serialize};
use std::ops::Deref;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidInput,
    UnsupportedSchema,
    BuildMismatch,
    SnapshotOutdated,
    MissingRequiredObservation,
    DatasetIncomplete,
    ProviderUnavailable,
    ImportTooLarge,
    OperationCanceled,
    StorageError,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, thiserror::Error)]
#[error("{message}")]
pub struct AppError {
    pub code: ErrorCode,
    message: &'static str,
    pub retryable: bool,
}
impl AppError {
    pub fn new(code: ErrorCode, message: &'static str, retryable: bool) -> Self {
        Self {
            code,
            message,
            retryable,
        }
    }
}
pub(crate) fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
pub type Result<T> = std::result::Result<T, AppError>;

pub trait Validate {
    fn validate(&self) -> Result<()>;
}

/// Draft data can be edited; a validated record provides only shared access.
/// Deserialize performs exactly the same checks as `try_new`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Validated<T>(T);
impl<T: Validate> Validated<T> {
    pub fn try_new(data: T) -> Result<Self> {
        data.validate()?;
        Ok(Self(data))
    }
    pub fn into_inner(self) -> T {
        self.0
    }
}
impl<T> Deref for Validated<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}
impl<'de, T: Deserialize<'de> + Validate> Deserialize<'de> for Validated<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        Self::try_new(T::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
impl<T: Validate> Validate for Validated<T> {
    fn validate(&self) -> Result<()> {
        self.0.validate()
    }
}
