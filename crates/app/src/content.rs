//! Explicit local content inspection DTOs; checksums do not authenticate publishers.
use crate::catalog::CatalogView;
use azeroth_core::{
    coverage::ContentReport,
    sources::{SourceConflict, SourceRecord},
};
use azeroth_storage::content::ContentRevisionKey;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct ContentInspection {
    pub key: ContentRevisionKey,
    pub report: ContentReport,
    pub sources: Vec<SourceRecord>,
    pub conflicts: Vec<SourceConflict>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ContentRollback {
    pub changed: bool,
    pub active: Option<ContentInspection>,
}
pub(crate) fn inspect(content: impl CatalogView) -> ContentInspection {
    ContentInspection {
        key: content.key(),
        report: content.report(),
        sources: content.sources().to_vec(),
        conflicts: content.conflicts().to_vec(),
    }
}
