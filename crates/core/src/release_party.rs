//! Party support is a workflow/evidence matrix, not a gameplay catalog.
//! Callers must supply identities from a verified, non-fixture content closure
//! and verify every returned evidence file before accepting this declaration.
use crate::{
    release::{FileRef, MAX_RELEASE_BYTES, Target},
    types::Sha256Hex,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Workflow {
    JointPlanning,
    UnequalProgress,
    Sharing,
    StaleMembers,
    Fairness,
    Agreement,
    Advancement,
    Review,
}
pub const WORKFLOWS: [Workflow; 8] = [
    Workflow::JointPlanning,
    Workflow::UnequalProgress,
    Workflow::Sharing,
    Workflow::StaleMembers,
    Workflow::Fairness,
    Workflow::Agreement,
    Workflow::Advancement,
    Workflow::Review,
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportMatrix {
    pub schema_version: u32,
    pub target: Target,
    pub catalog_sha256: Sha256Hex,
    pub supported_sizes: Vec<u8>,
    pub rows: Vec<SupportRow>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportRow {
    pub id: String,
    pub size: u8,
    pub workflow: Workflow,
    /// Each dependency is the canonical identity from the content catalog.
    pub dependencies: Vec<Dependency>,
    /// Static scenario specification, not a claim of observed acceptance.
    pub procedure: FileRef,
    /// Evidence supporting the workflow's declared applicability/content facts.
    pub supporting_evidence: Vec<FileRef>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub domain: String,
    pub record_id: String,
}
/// Domain -> canonical identity -> source IDs, supplied by verified pack decoding.
pub type ConcreteCatalog = BTreeMap<String, BTreeMap<String, BTreeSet<String>>>;

fn text(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control)
}
fn files_valid(files: &[FileRef]) -> bool {
    let mut seen = BTreeSet::new();
    !files.is_empty()
        && files.len() <= 64
        && files
            .iter()
            .all(|f| f.bytes > 0 && f.bytes <= 256 * 1024 * 1024 && seen.insert(f.path.as_str()))
}
impl SupportMatrix {
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > MAX_RELEASE_BYTES {
            return Err("Party support exceeds byte limit");
        }
        let value: Self =
            serde_json::from_slice(bytes).map_err(|_| "Invalid party support matrix")?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1 {
            return Err("Unsupported party support version");
        }
        if self.target != Target::forever_70374() {
            return Err("Party support target differs");
        }
        // The implemented local party boundary is at most 40 participants.
        // This bound is not a claim that all sizes have supported game workflows.
        let sizes: BTreeSet<_> = self.supported_sizes.iter().copied().collect();
        if sizes.is_empty()
            || sizes.len() != self.supported_sizes.len()
            || sizes.iter().any(|s| !(2..=40).contains(s))
            || self.rows.len() != sizes.len() * WORKFLOWS.len()
        {
            return Err("Missing or invalid party sizes/workflow rows");
        }
        let mut ids = BTreeSet::new();
        let mut pairs = BTreeSet::new();
        for row in &self.rows {
            if !text(&row.id)
                || !ids.insert(&row.id)
                || !sizes.contains(&row.size)
                || !pairs.insert((row.size, row.workflow))
            {
                return Err("Duplicate or invalid party support row");
            }
            if row.dependencies.is_empty()
                || row.dependencies.len() > 1024
                || row.dependencies.iter().collect::<BTreeSet<_>>().len() != row.dependencies.len()
                || row.dependencies.iter().any(|d| {
                    !text(&d.record_id)
                        || !matches!(
                            d.domain.as_str(),
                            "quests"
                                | "equipment"
                                | "acquisition"
                                | "geography"
                                | "travel"
                                | "dungeons"
                                | "class_roles"
                                | "reminders"
                        )
                })
                || !files_valid(std::slice::from_ref(&row.procedure))
                || !files_valid(&row.supporting_evidence)
            {
                return Err("Missing or invalid party content/evidence dependencies");
            }
        }
        Ok(())
    }
    /// Resolve every dependency to actual content; profiles and arbitrary replay
    /// IDs cannot bind. Returns exact source sets for release inventory matching.
    pub fn bind(
        &self,
        catalog_sha256: &str,
        catalog: &ConcreteCatalog,
    ) -> Result<BTreeMap<String, BTreeSet<String>>, &'static str> {
        self.validate()?;
        if self.catalog_sha256.as_str() != catalog_sha256 {
            return Err("Party support belongs to a different content catalog");
        }
        let mut rows = BTreeMap::new();
        for row in &self.rows {
            let mut sources = BTreeSet::new();
            for dependency in &row.dependencies {
                let record = catalog
                    .get(&dependency.domain)
                    .and_then(|domain| domain.get(&dependency.record_id))
                    .ok_or("Party support requires absent concrete content")?;
                if record.is_empty() || record.iter().any(|s| !text(s)) {
                    return Err("Party dependency has no valid source provenance");
                }
                sources.extend(record.iter().cloned());
            }
            let identity = serde_json::to_string(&(catalog_sha256, "party_support", &row.id))
                .map_err(|_| "Invalid party support identity")?;
            rows.insert(identity, sources);
        }
        Ok(rows)
    }
    pub fn evidence_files(&self) -> impl Iterator<Item = &FileRef> {
        self.rows
            .iter()
            .flat_map(|row| std::iter::once(&row.procedure).chain(&row.supporting_evidence))
    }
}
