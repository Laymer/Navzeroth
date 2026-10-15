//! Bounded local inspection DTOs. Checksums do not authenticate publishers.
use crate::catalog::CatalogView;
use azeroth_core::{
    coverage::ContentReport,
    error::{AppError, ErrorCode, Result},
    sources::{SourceConflict, SourceRecord},
    types::Sha256Hex,
};
use azeroth_storage::content::ContentRevisionKey;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize)]
pub struct ContentInspection {
    pub key: ContentRevisionKey,
    pub report: ContentReport,
    pub sources: Vec<SourceRecord>,
    pub conflicts: Vec<SourceConflict>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<InspectionPages>,
}
#[derive(Clone, Debug, Serialize)]
pub struct InspectionPages {
    pub content_sha256: Sha256Hex,
    pub sources: usize,
    pub conflicts: usize,
    pub gaps: usize,
    pub errors: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct ContentRollback {
    pub changed: bool,
    pub active: Option<ContentInspection>,
}
pub(crate) fn inspect(content: impl CatalogView) -> Result<ContentInspection> {
    let mut report = content.report();
    let counts = (
        content.sources().len(),
        content.conflicts().len(),
        report.gaps.len(),
        report.errors.len(),
    );
    let pages = if content.content_v2().is_some()
        || [counts.0, counts.1, counts.2, counts.3]
            .into_iter()
            .any(|n| n > 200)
    {
        Some(InspectionPages {
            content_sha256: Sha256Hex::new(content.content_hash()?)?,
            sources: counts.0,
            conflicts: counts.1,
            gaps: counts.2,
            errors: counts.3,
        })
    } else {
        None
    };
    report.gaps.truncate(200);
    report.errors.truncate(200);
    Ok(ContentInspection {
        key: content.key(),
        report,
        sources: content.sources().iter().take(200).cloned().collect(),
        conflicts: content.conflicts().iter().take(200).cloned().collect(),
        pages,
    })
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectionCollection {
    Sources,
    Conflicts,
    Gaps,
    Errors,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectionRequest {
    key: ContentRevisionKey,
    content_sha256: Sha256Hex,
    collection: InspectionCollection,
    offset: usize,
}
#[derive(Serialize)]
pub struct InspectionPage {
    pub key: ContentRevisionKey,
    pub content_sha256: Sha256Hex,
    pub collection: InspectionCollection,
    pub offset: usize,
    pub total: usize,
    pub next_offset: Option<usize>,
    pub records: Vec<serde_json::Value>,
}
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
impl crate::snapshots::SnapshotApplication {
    pub fn content_inspection_page(&self, text: &str) -> Result<InspectionPage> {
        if text.len() > 4096 {
            return Err(invalid("Inspection request exceeds bounds"));
        }
        let request: InspectionRequest =
            serde_json::from_str(text).map_err(|_| invalid("Invalid inspection page request"))?;
        self.with_store_mut(|store| {
            store.with_immediate_read(|store| {
                let content = store
                    .catalog_revision(&request.key)?
                    .ok_or_else(|| invalid("Inspected revision is not stored"))?;
                if content.content_hash()? != request.content_sha256.as_str() {
                    return Err(invalid("Inspected revision hash changed"));
                }
                fn rows<T: Serialize>(
                    values: &[T],
                    offset: usize,
                ) -> Result<(usize, Vec<serde_json::Value>)> {
                    if offset > values.len() {
                        return Err(invalid("Inspection offset outside revision"));
                    }
                    Ok((
                        values.len(),
                        values
                            .iter()
                            .skip(offset)
                            .take(200)
                            .map(serde_json::to_value)
                            .collect::<std::result::Result<Vec<_>, _>>()
                            .map_err(|_| invalid("Inspection record cannot serialize"))?,
                    ))
                }
                let (total, records) = match request.collection {
                    InspectionCollection::Sources => rows(content.sources(), request.offset)?,
                    InspectionCollection::Conflicts => rows(content.conflicts(), request.offset)?,
                    InspectionCollection::Gaps => rows(&content.report().gaps, request.offset)?,
                    InspectionCollection::Errors => rows(&content.report().errors, request.offset)?,
                };
                let end = request.offset + records.len();
                Ok(InspectionPage {
                    key: request.key,
                    content_sha256: request.content_sha256,
                    collection: request.collection,
                    offset: request.offset,
                    total,
                    next_offset: (end < total).then_some(end),
                    records,
                })
            })
        })
    }
    pub fn stage_verified_content_bundle(
        &self,
        root: &std::path::Path,
    ) -> Result<ContentInspection> {
        let bundle = azeroth_ingest::pack::v2::read_verified(root).map_err(invalid)?;
        let content = azeroth_storage::content::ContentRevisionV2 {
            content: bundle.content,
            sources: bundle.sources,
            conflicts: vec![],
        };
        let assets = bundle
            .assets
            .into_iter()
            .map(|asset| azeroth_storage::catalog_assets::CatalogAsset {
                file: asset.path,
                bytes: asset.bytes,
            })
            .collect::<Vec<_>>();
        self.with_store_mut(|store| {
            store.stage_content_v2_bundle(&content, &assets)?;
            Ok(())
        })?;
        inspect(azeroth_storage::content::ActiveContentRevision::V2(
            std::sync::Arc::new(content),
        ))
    }
}
