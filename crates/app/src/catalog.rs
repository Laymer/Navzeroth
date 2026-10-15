//! Read-only catalog access for both immutable storage formats. Consumers never
//! forge a v1 envelope from a larger v2 catalog or silently read an older revision.
use azeroth_core::{
    contract::ReferenceDataset,
    coverage::{ContentReport, content_report},
    error::{AppError, ErrorCode, Result},
    sources::{SourceConflict, SourceRecord},
    types::DataContext,
};
use azeroth_storage::content::{ActiveContentRevision, ContentRevision, ContentRevisionKey};
use sha2::{Digest, Sha256};

pub trait CatalogView {
    fn reference(&self) -> &ReferenceDataset;
    fn context(&self) -> DataContext;
    fn fixture(&self) -> bool;
    fn sources(&self) -> &[SourceRecord];
    fn conflicts(&self) -> &[SourceConflict];
    fn key(&self) -> ContentRevisionKey;
    fn report(&self) -> ContentReport;
    fn content_hash(&self) -> Result<String>;
    fn encoded(&self) -> Result<Vec<u8>>;
    fn content_v2(&self) -> Option<&azeroth_core::content_v2::ReferenceContent> {
        None
    }
}
fn hash(value: &impl serde::Serialize) -> Result<String> {
    serde_json::to_vec(value)
        .map(|v| format!("{:x}", Sha256::digest(v)))
        .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Catalog cannot be hashed", false))
}
impl CatalogView for ContentRevision {
    fn reference(&self) -> &ReferenceDataset {
        &self.envelope.payload
    }
    fn context(&self) -> DataContext {
        self.envelope.data_context()
    }
    fn fixture(&self) -> bool {
        self.envelope.fixture
    }
    fn sources(&self) -> &[SourceRecord] {
        &self.sources
    }
    fn conflicts(&self) -> &[SourceConflict] {
        &self.conflicts
    }
    fn key(&self) -> ContentRevisionKey {
        self.key()
    }
    fn report(&self) -> ContentReport {
        content_report(&self.envelope)
    }
    fn content_hash(&self) -> Result<String> {
        hash(self)
    }
    fn encoded(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self)
            .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Catalog cannot be encoded", false))
    }
}
impl CatalogView for ActiveContentRevision {
    fn encoded(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self)
            .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Catalog cannot be encoded", false))
    }
    fn content_v2(&self) -> Option<&azeroth_core::content_v2::ReferenceContent> {
        self.content_v2()
    }
    fn reference(&self) -> &ReferenceDataset {
        self.reference()
    }
    fn context(&self) -> DataContext {
        self.context()
    }
    fn fixture(&self) -> bool {
        self.fixture()
    }
    fn sources(&self) -> &[SourceRecord] {
        self.sources()
    }
    fn conflicts(&self) -> &[SourceConflict] {
        self.conflicts()
    }
    fn key(&self) -> ContentRevisionKey {
        self.key()
    }
    fn report(&self) -> ContentReport {
        match self {
            Self::V1(value) => value.report(),
            Self::V2(value) => content_report(&value.content.reference),
        }
    }
    fn content_hash(&self) -> Result<String> {
        match self {
            Self::V1(value) => value.content_hash(),
            Self::V2(value) => hash(value.as_ref()),
        }
    }
}
