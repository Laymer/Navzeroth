//! Explicit local content inspection DTOs; checksums do not authenticate publishers.
use azeroth_core::{
    coverage::{ContentReport, content_report},
    sources::{SourceConflict, SourceRecord},
};
use azeroth_storage::content::{ContentRevision, ContentRevisionKey};
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
pub(crate) fn inspect(content: ContentRevision) -> ContentInspection {
    ContentInspection {
        key: content.key(),
        report: content_report(&content.envelope),
        sources: content.sources,
        conflicts: content.conflicts,
    }
}
