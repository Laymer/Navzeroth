//! Finite compatibility/evidence inventory, never a global support assertion.
use crate::snapshots::{SnapshotApplication, resolve};
use azeroth_core::{
    coverage::{DatasetCoverage, StatusCounts, content_report},
    error::{AppError, ErrorCode, Result},
    observation::ObservationStatus,
    sources::Redistribution,
    types::{BuildId, BuildKey, DataContext, DatasetRevision, EditionId, InterfaceId},
};
use azeroth_storage::{
    SnapshotStore,
    content::{ContentRevision, ContentScope},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const MAX_COMPATIBILITY_BYTES: usize = 64 * 1024;
const DIAGNOSTIC: &str =
    include_str!("../../../fixtures/compatibility/forever-70338-user-diagnostic.json");
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    ReportedFlow,
    FunctionPresentOnly,
    ReportedMissing,
    Unverified,
    AssumedReference,
}
#[derive(Clone, Debug, Serialize)]
pub struct CapabilityEvidence {
    pub capability: String,
    pub state: EvidenceState,
    pub statement: String,
    pub source: &'static str,
}
#[derive(Clone, Debug, Serialize)]
pub struct BuildEvidence {
    pub build: BuildKey,
    pub version: String,
    pub basis: EvidenceState,
    pub reported_on: String,
    pub addon_version: Option<String>,
    pub matches_selected_build: Option<bool>,
    pub limitations: &'static str,
    pub capabilities: Vec<CapabilityEvidence>,
}
#[derive(Clone, Debug, Serialize)]
pub struct SelectedContext {
    pub context: DataContext,
    pub fixture: bool,
    pub snapshot_present: bool,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct PolicyValues {
    pub permitted: usize,
    pub not_permitted: usize,
    pub conditional: usize,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct BoolValues {
    pub known_true: usize,
    pub known_false: usize,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct SourceCoverage {
    pub registered: usize,
    pub referenced_distinct: usize,
    pub unregistered_distinct: usize,
    pub evidence_references: usize,
    pub image_references: usize,
    pub policy_statuses: BTreeMap<&'static str, StatusCounts>,
    pub redistribution_values: PolicyValues,
    pub offline_values: BoolValues,
    pub editions_include_selected: usize,
    pub editions_exclude_selected: usize,
    pub verified_revision_matches: usize,
    pub verified_revision_differs: usize,
    pub declared_conflicts: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct ActiveCoverage {
    pub context: DataContext,
    pub fixture: bool,
    pub revision: DatasetRevision,
    pub structurally_valid: bool,
    pub finite_scope: &'static str,
    pub counts: DatasetCoverage,
    pub errors_by_code: BTreeMap<&'static str, usize>,
    pub gaps_by_code: BTreeMap<&'static str, usize>,
    pub sources: SourceCoverage,
}
#[derive(Clone, Debug, Serialize)]
pub struct UnverifiedCheck {
    pub check: &'static str,
    pub required_evidence: &'static str,
}
#[derive(Clone, Debug, Serialize)]
pub struct CompatibilityReport {
    pub report_version: u32,
    pub app_version: &'static str,
    pub selected: Option<SelectedContext>,
    pub active: Option<ActiveCoverage>,
    pub recorded_builds: Vec<BuildEvidence>,
    pub unverified_checks: Vec<UnverifiedCheck>,
    pub official_api: &'static str,
    pub limitations: &'static str,
}
fn invalid() -> AppError {
    AppError::new(
        ErrorCode::InvalidInput,
        "Invalid compatibility report request",
        false,
    )
}
fn add(count: &mut StatusCounts, status: ObservationStatus) {
    match status {
        ObservationStatus::Known => count.known += 1,
        ObservationStatus::Unknown => count.unknown += 1,
        ObservationStatus::Unsupported => count.unsupported += 1,
        ObservationStatus::Restricted => count.restricted += 1,
        ObservationStatus::NotApplicable => count.not_applicable += 1,
    }
}
fn active(content: &ContentRevision) -> Result<ActiveCoverage> {
    let report = content_report(&content.envelope);
    let dataset = &content.envelope.payload;
    let context = content.key().scope.context;
    let registered = content
        .sources
        .iter()
        .map(|s| &s.id)
        .collect::<BTreeSet<_>>();
    let referenced = dataset
        .evidence
        .iter()
        .map(|e| &e.source)
        .chain(dataset.map_images.iter().map(|i| &i.source))
        .collect::<BTreeSet<_>>();
    let mut sources = SourceCoverage {
        registered: registered.len(),
        referenced_distinct: referenced.len(),
        unregistered_distinct: referenced.difference(&registered).count(),
        evidence_references: dataset.evidence.len(),
        image_references: dataset.map_images.len(),
        declared_conflicts: content.conflicts.len(),
        ..Default::default()
    };
    for source in &content.sources {
        for (field, status) in [
            ("location", source.location.status()),
            ("editions", source.editions.status()),
            ("access", source.access.status()),
            ("authentication", source.authentication.status()),
            ("license", source.license.status()),
            ("attribution", source.attribution.status()),
            ("redistribution", source.redistribution.status()),
            ("retention", source.retention.status()),
            ("allowed_fields", source.allowed_fields.status()),
            ("transformations", source.transformations.status()),
            ("update_schedule", source.update_schedule.status()),
            ("conditional_requests", source.conditional_requests.status()),
            ("failure_behavior", source.failure_behavior.status()),
            ("last_retrieval", source.last_retrieval_utc_ms.status()),
            ("verified_revision", source.verified_revision.status()),
            ("offline_usable", source.offline_usable.status()),
        ] {
            add(sources.policy_statuses.entry(field).or_default(), status);
        }
        match source.redistribution.value() {
            Some(Redistribution::Permitted) => sources.redistribution_values.permitted += 1,
            Some(Redistribution::NotPermitted) => sources.redistribution_values.not_permitted += 1,
            Some(Redistribution::Conditional) => sources.redistribution_values.conditional += 1,
            None => {}
        }
        match source.offline_usable.value() {
            Some(true) => sources.offline_values.known_true += 1,
            Some(false) => sources.offline_values.known_false += 1,
            None => {}
        }
        if let Some(editions) = source.editions.value() {
            if editions.contains(&context.build.edition) {
                sources.editions_include_selected += 1;
            } else {
                sources.editions_exclude_selected += 1;
            }
        }
        if let Some(revision) = source.verified_revision.value() {
            if revision == &dataset.revision {
                sources.verified_revision_matches += 1;
            } else {
                sources.verified_revision_differs += 1;
            }
        }
    }
    Ok(ActiveCoverage {
        context,
        fixture: content.envelope.fixture,
        revision: dataset.revision.clone(),
        structurally_valid: report.passes,
        finite_scope: "Counts cover only records in this exact active revision. Zero is an observed empty collection; these counts do not establish geographic, gameplay or live-client completeness.",
        counts: report.coverage.ok_or_else(invalid)?,
        errors_by_code: report.errors.iter().fold(BTreeMap::new(), |mut m, f| {
            *m.entry(f.code()).or_default() += 1;
            m
        }),
        gaps_by_code: report.gaps.iter().fold(BTreeMap::new(), |mut m, f| {
            *m.entry(f.code()).or_default() += 1;
            m
        }),
        sources,
    })
}
#[derive(Deserialize)]
struct Diagnostic {
    reported_on: String,
    addon_version: String,
    client: Client,
    observations: BTreeMap<String, String>,
}
#[derive(Deserialize)]
struct Client {
    version: String,
    edition: EditionId,
    build: BuildId,
    interface: InterfaceId,
}
fn recorded(selected: Option<&SelectedContext>) -> Result<Vec<BuildEvidence>> {
    if DIAGNOSTIC.len() > 16384 {
        return Err(invalid());
    }
    let diagnostic: Diagnostic = serde_json::from_str(DIAGNOSTIC).map_err(|_| invalid())?;
    if diagnostic.observations.len() > 32
        || diagnostic
            .observations
            .iter()
            .any(|(id, text)| id.len() > 128 || text.len() > 4096)
    {
        return Err(invalid());
    }
    let build = BuildKey {
        edition: diagnostic.client.edition,
        build: diagnostic.client.build,
        interface: diagnostic.client.interface,
    };
    let mut capabilities = diagnostic
        .observations
        .into_iter()
        .map(|(capability, statement)| {
            let state = match capability.as_str() {
                "GetBuildInfo" | "SavedVariables" | "UnitLevel" => EvidenceState::ReportedFlow,
                "CombatLogGetCurrentEventInfo" => EvidenceState::ReportedMissing,
                _ if statement.starts_with("function present;") => {
                    EvidenceState::FunctionPresentOnly
                }
                _ => EvidenceState::Unverified,
            };
            CapabilityEvidence {
                capability,
                state,
                statement,
                source: "fixtures/compatibility/forever-70338-user-diagnostic.json",
            }
        })
        .collect::<Vec<_>>();
    capabilities.push(CapabilityEvidence{capability:"trusted_capture_age_flow".into(),state:EvidenceState::ReportedFlow,statement:"Two narrowly user-reported flows: trusted age after clock confirmation and uncertain age after reload. Follow-up date/addon revision not separately recorded. No raw exports, inspected addon revision, exact values or combat/zone conditions supplied.".into(),source:"docs/compatibility/BUILD_CAPABILITIES.md#Wave-3-reported-clock-follow-up"});
    let observed = BuildEvidence {
        matches_selected_build: selected.map(|s| s.context.build == build),
        build,
        version: diagnostic.client.version,
        basis: EvidenceState::ReportedFlow,
        reported_on: diagnostic.reported_on,
        addon_version: Some(diagnostic.addon_version),
        limitations: "Historical sanitized user report: one level-12 manual flow; function presence is not usability. Edition was the addon target, locale/restrictions and installed current revision are unknown. Clock follow-up has no independently inspected addon revision.",
        capabilities,
    };
    let assumed_build = BuildKey {
        edition: EditionId::new("wow_forever")?,
        build: BuildId::new("1.60.1.70205")?,
        interface: InterfaceId::new("16001")?,
    };
    let assumed=BuildEvidence{matches_selected_build:selected.map(|s|s.context.build==assumed_build),build:assumed_build,version:"1.60.1".into(),basis:EvidenceState::AssumedReference,reported_on:"2026-10-12".into(),addon_version:None,limitations:"Poster footer/community interface assumption only, not a client session. Never inherit 70338 observations for this reference tuple.",capabilities:vec![CapabilityEvidence{capability:"reference_map_and_position".into(),state:EvidenceState::Unverified,statement:"Map identifiers, coordinates, axis calibration, zone embeddings and real travel connections remain unverified for this reference dataset.".into(),source:"docs/data/atlas-eastern-kingdoms.md"}]};
    Ok(vec![observed, assumed])
}
impl SnapshotApplication {
    pub fn compatibility_report(&self, profile_id: Option<&str>) -> Result<CompatibilityReport> {
        if profile_id.is_some_and(|id| id.is_empty() || id.len() > 128) {
            return Err(invalid());
        }
        let (selected, content) = self.with_store_mut(|store| {
            let Some(id) = profile_id else {
                return Ok((None, None));
            };
            let key = resolve(store, id)?;
            let selected = SelectedContext {
                context: key.context.clone(),
                fixture: key.fixture,
                snapshot_present: store.current_snapshot(&key)?.is_some(),
            };
            let content = store.active_content_revision(&ContentScope {
                context: key.context,
                fixture: key.fixture,
            })?;
            Ok((Some(selected), content))
        })?;
        let report = CompatibilityReport {
            report_version: 1,
            app_version: env!("CARGO_PKG_VERSION"),
            recorded_builds: recorded(selected.as_ref())?,
            active: content.as_ref().map(active).transpose()?,
            selected,
            official_api: "Enablement unverified: requires recorded exact-edition endpoint, authentication, retention and permitted fields/terms evidence. Lack of a verified provider is not evidence that an official API does not exist.",
            unverified_checks: vec![
                UnverifiedCheck {
                    check: "native_interaction",
                    required_evidence: "Recorded Windows build, focus/DPI/window workflows and actual installation/removal conditions.",
                },
                UnverifiedCheck {
                    check: "live_plan_and_party",
                    required_evidence: "Sanitized load/advance/divergence results on the recorded client build; no inferred party bridge.",
                },
                UnverifiedCheck {
                    check: "restricted_contexts",
                    required_evidence: "Permitted out-of-combat/combat read and serialization checks with exact client/addon revision and conditions.",
                },
                UnverifiedCheck {
                    check: "real_playable_content",
                    required_evidence: "Finite supported region/quest/travel/acquisition package with exact-build source coverage and explicit gaps.",
                },
            ],
            limitations: "Recorded evidence is historical and finite. Synthetic fixtures never verify live capability. No network requests, account/character data, personal filesystem paths, source URLs or personal exports are included.",
        };
        if serde_json::to_vec(&report).map_err(|_| invalid())?.len() > MAX_COMPATIBILITY_BYTES {
            return Err(AppError::new(
                ErrorCode::ImportTooLarge,
                "Compatibility report exceeds limit",
                false,
            ));
        }
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_registry_gaps_remain_separate_from_structural_dataset_validity() {
        let mut content: ContentRevision = serde_json::from_str(include_str!(
            "../../../fixtures/desktop/content/demo-revision.json"
        ))
        .unwrap();
        content.sources.clear();
        let coverage = active(&content).unwrap();
        assert!(coverage.structurally_valid);
        assert_eq!(coverage.sources.registered, 0);
        assert_eq!(coverage.sources.unregistered_distinct, 1);
        assert_eq!(coverage.sources.evidence_references, 1);
        assert!(!coverage.gaps_by_code.is_empty());
    }
}
