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
    /// Travel options the active content policy allows. The default excludes
    /// every unverified edge.
    fn route_options(&self) -> azeroth_core::travel::RouteOptions {
        azeroth_core::travel::RouteOptions::default()
    }
}
/// A catalog whose planning accepts inferred travel edges (ADR 0007 §8) when
/// the activation policy says so; otherwise it is the plain catalog.
pub(crate) struct TravelPolicyView<'a> {
    inner: &'a dyn CatalogView,
    accept_inferred: bool,
}
impl<'a> TravelPolicyView<'a> {
    pub(crate) fn new(inner: &'a dyn CatalogView, accept_inferred: bool) -> Self {
        Self {
            inner,
            accept_inferred,
        }
    }
}
impl CatalogView for TravelPolicyView<'_> {
    fn reference(&self) -> &ReferenceDataset {
        self.inner.reference()
    }
    fn context(&self) -> DataContext {
        self.inner.context()
    }
    fn fixture(&self) -> bool {
        self.inner.fixture()
    }
    fn sources(&self) -> &[SourceRecord] {
        self.inner.sources()
    }
    fn conflicts(&self) -> &[SourceConflict] {
        self.inner.conflicts()
    }
    fn key(&self) -> ContentRevisionKey {
        self.inner.key()
    }
    fn report(&self) -> ContentReport {
        self.inner.report()
    }
    fn content_hash(&self) -> Result<String> {
        self.inner.content_hash()
    }
    fn encoded(&self) -> Result<Vec<u8>> {
        self.inner.encoded()
    }
    fn content_v2(&self) -> Option<&azeroth_core::content_v2::ReferenceContent> {
        self.inner.content_v2()
    }
    fn route_options(&self) -> azeroth_core::travel::RouteOptions {
        let mut options = self.inner.route_options();
        if self.accept_inferred {
            let data = self.inner.reference();
            options.inferred_edges =
                azeroth_core::travel::inferred_edges(&data.edges, &data.evidence);
        }
        options
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn revision(confidence: &str, status: &str) -> ActiveContentRevision {
        let mut content: Value = serde_json::from_str(include_str!(
            "../../../fixtures/contracts/v2/reference-content.json"
        ))
        .unwrap();
        content["reference"]["payload"]["evidence"][0]["confidence"] = json!(confidence);
        content["reference"]["payload"]["edges"][0]["status"] = json!(status);
        let old: Value = serde_json::from_str(include_str!(
            "../../../fixtures/desktop/planning/synthetic-revision.json"
        ))
        .unwrap();
        let revision: azeroth_storage::content::ContentRevisionV2 = serde_json::from_value(
            json!({"content":content,"sources":old["sources"],"conflicts":[]}),
        )
        .unwrap();
        ActiveContentRevision::V2(std::sync::Arc::new(revision))
    }
    #[test]
    fn travel_policy_view_delegates_and_adds_only_accepted_inferred_edges() {
        let inner = revision("inferred", "unverified");
        for accept in [false, true] {
            let view = TravelPolicyView::new(&inner, accept);
            assert_eq!(view.key(), inner.key());
            assert_eq!(view.context(), inner.context());
            assert_eq!(view.fixture(), inner.fixture());
            assert_eq!(view.sources().len(), inner.sources().len());
            assert_eq!(view.conflicts().len(), inner.conflicts().len());
            assert_eq!(view.report(), CatalogView::report(&inner));
            assert_eq!(view.content_hash().unwrap(), inner.content_hash().unwrap());
            assert_eq!(view.encoded().unwrap(), inner.encoded().unwrap());
            assert!(view.content_v2().is_some());
            assert_eq!(view.reference().edges.len(), 1);
            assert_eq!(
                view.route_options().inferred_edges.len(),
                usize::from(accept)
            );
        }
        assert!(inner.route_options().inferred_edges.is_empty());
        let estimated = revision("limited_sample", "unverified");
        assert!(
            TravelPolicyView::new(&estimated, true)
                .route_options()
                .inferred_edges
                .is_empty()
        );
    }
}
