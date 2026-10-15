//! Explicit bounded local content governance. These decisions do not authenticate
//! a publisher, change source rights or upgrade gameplay evidence.
use azeroth_core::{
    error::{AppError, ErrorCode, Result},
    types::UtcMillis,
};
use azeroth_storage::{
    SqliteStore,
    content::{ContentRevisionKey, ContentScope},
    content_dispositions::{ContentDisposition, DispositionMutation, DispositionOutcome},
};
use serde::{Deserialize, Serialize};
pub const MAX_DISPOSITION_REQUEST_BYTES: usize = 8192;
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispositionRequest {
    pub schema_version: u32,
    pub profile_id: String,
    pub mutation: DispositionMutation,
}
#[derive(Debug, Serialize)]
pub struct ContentGovernance {
    pub schema_version: u32,
    pub scope: ContentScope,
    pub active: Option<ContentRevisionKey>,
    pub revisions: Vec<ContentDisposition>,
}
#[derive(Debug, Serialize)]
pub struct ContentDecision {
    pub schema_version: u32,
    pub outcome: DispositionOutcome,
}
pub(crate) fn parse(text: &str) -> Result<DispositionRequest> {
    if text.len() > MAX_DISPOSITION_REQUEST_BYTES {
        return Err(AppError::new(
            ErrorCode::ImportTooLarge,
            "Content decision exceeds limit",
            false,
        ));
    }
    let request: DispositionRequest = serde_json::from_str(text).map_err(|_| {
        AppError::new(
            ErrorCode::InvalidInput,
            "Invalid content decision data",
            false,
        )
    })?;
    if request.schema_version != 1
        || request.profile_id.is_empty()
        || request.profile_id.len() > 128
    {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Content decision schema or profile is invalid",
            false,
        ));
    }
    Ok(request)
}
pub(crate) fn read(store: &SqliteStore, scope: ContentScope) -> Result<ContentGovernance> {
    store.with_immediate_read(|store| {
        Ok(ContentGovernance {
            schema_version: 1,
            active: store.active_catalog_revision(&scope)?.map(|r| r.key()),
            revisions: store.list_content_dispositions(&scope)?,
            scope,
        })
    })
}
pub(crate) fn change(
    store: &mut SqliteStore,
    scope: &ContentScope,
    request: &DispositionRequest,
    now: Option<UtcMillis>,
) -> Result<ContentDecision> {
    if request.mutation.guard.key.scope != *scope {
        return Err(AppError::new(
            ErrorCode::BuildMismatch,
            "Content decision belongs to another profile context",
            false,
        ));
    }
    Ok(ContentDecision {
        schema_version: 1,
        outcome: store.change_content_disposition(&request.mutation, now)?,
    })
}
