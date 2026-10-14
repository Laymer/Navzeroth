//! Versioned clustering explanations separate from flattened ordinary plan steps.
use azeroth_core::objective_clustering::{
    ClusterChoice, ClusteredResult, ObjectiveAssessment, VariantTrace,
};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct ClusteringMetadata {
    pub schema_version: u16,
    pub assessments: Vec<ObjectiveAssessment>,
    pub selected_clusters: Vec<ClusterChoice>,
    pub cluster_alternatives: Vec<Vec<ClusterChoice>>,
    pub variant_trace: Vec<VariantTrace>,
    pub peak_variant_count: usize,
    pub variants_evaluated: usize,
}
impl From<ClusteredResult> for ClusteringMetadata {
    fn from(value: ClusteredResult) -> Self {
        Self {
            schema_version: value.schema_version,
            assessments: value.assessments,
            selected_clusters: value.selected_clusters,
            cluster_alternatives: value.cluster_alternatives,
            variant_trace: value.variant_trace,
            peak_variant_count: value.peak_variant_count,
            variants_evaluated: value.variants_evaluated,
        }
    }
}
