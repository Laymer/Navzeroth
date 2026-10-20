//! Explicit local preparation references; no built-in production dungeon facts.
//! Capture immutable inputs under the store lock, evaluate outside it, and verify
//! current identities before returning a bounded explanatory report.
use crate::{
    party::QuestAssessment,
    snapshots::{SnapshotApplication, context, profile_id},
};
use azeroth_core::{
    contract::Envelope,
    eligibility::{EligibilityInputs, evaluate_participant},
    entities::{
        CharacterSnapshot, Confidence, DurationRange, NodeRef, PartySnapshot, PartySnapshotData,
    },
    error::{AppError, ErrorCode, Result},
    evaluation::{EvaluationContext, EvaluationResult, StalePolicy, Truth, evaluate},
    observation::{Freshness, Observation, ObservationStatus, freshness},
    predicate::{Predicate, RuleLimits},
    travel::{RouteOutcome, RouteSearch, shortest_route},
    types::*,
};
use azeroth_storage::content::ActiveContentRevision;
use azeroth_storage::{ProfileKey, SnapshotStore, SqliteStore, content::ContentScope};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

pub(crate) mod json;
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
pub const MAX_REPORT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_SECTION_RECORDS: usize = 32;
/// Schema2 reference lists accommodate admitted source catalogs. Notes, checks
/// and each declaration's evidence retain their schema1 limit.
pub const MAX_REFERENCE_RECORDS: usize = 4096;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Declared<T> {
    pub observation: Observation<T>,
    pub evidence: Vec<EvidenceId>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckCategory {
    Role,
    Consumables,
    Equipment,
    BeforeDeparture,
    BeforeEntry,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreparationCheck {
    pub id: SourceId,
    pub label: Text,
    pub category: CheckCategory,
    pub requirement: Declared<Predicate>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RewardReference {
    pub item: ItemKey,
    pub source: AcquisitionId,
    /// A local label, not an inferred role benefit or recommendation.
    pub declared_role: Observation<RoleId>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalPreparationPackage {
    pub schema_version: u32,
    pub id: SourceId,
    pub title: Text,
    pub context: DataContext,
    pub fixture: bool,
    pub revision: DatasetRevision,
    pub entrance: Declared<NodeRef>,
    pub access: Declared<Predicate>,
    pub quests: Declared<Vec<QuestId>>,
    pub rewards: Declared<Vec<RewardReference>>,
    pub checks: Declared<Vec<PreparationCheck>>,
    pub formation_ms: Declared<DurationRange>,
    pub run_ms: Declared<DurationRange>,
    pub before_departure: Declared<Vec<Text>>,
    pub before_entry: Declared<Vec<Text>>,
    pub encounter_notes: Declared<Vec<Text>>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreparationSubject {
    Character { profile_id: String },
    Party { party_id: PartyId },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreparationRequest {
    pub schema_version: u32,
    pub subject: PreparationSubject,
    pub package: Option<LocalPreparationPackage>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authored: Option<crate::authored_content::AuthoredSelection>,
    /// Manually declared shared starting node; never inferred from labels.
    pub origin: Option<NodeId>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ConditionAssessment {
    pub availability: ObservationStatus,
    pub evaluation: Option<EvaluationResult>,
}
#[derive(Clone, Debug, Serialize)]
pub struct QuestPreparation {
    pub quest: QuestId,
    pub assessment: Option<QuestAssessment>,
}
#[derive(Clone, Debug, Serialize)]
pub struct CheckPreparation {
    pub check: SourceId,
    pub condition: ConditionAssessment,
}
#[derive(Clone, Debug, Serialize)]
pub struct RewardPreparation {
    pub item: ItemKey,
    pub source: AcquisitionId,
    pub item_requirement: ConditionAssessment,
    pub acquisition_requirement: ConditionAssessment,
    pub probability: Observation<Normalized>,
    pub cost: Observation<Copper>,
}
#[derive(Clone, Debug, Serialize)]
pub struct MemberPreparation {
    pub profile_id: String,
    pub character: CharacterId,
    pub snapshot_id: Option<SnapshotId>,
    pub freshness: Option<Freshness>,
    pub access: Option<ConditionAssessment>,
    pub quests: Vec<QuestPreparation>,
    pub checks: Vec<CheckPreparation>,
    pub rewards: Vec<RewardPreparation>,
}
#[derive(Clone, Debug, Serialize)]
pub struct PreparationEvidence {
    pub id: EvidenceId,
    pub source: SourceId,
    pub confidence: Confidence,
    pub license: Observation<Text>,
    pub redistribution: Observation<azeroth_core::sources::Redistribution>,
    pub offline_usable: Observation<bool>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GapCode {
    PackageMissing,
    LocalPackageUnverified,
    ActiveContentMissing,
    SectionUnavailable,
    EvidenceMissing,
    SnapshotMissing,
    SnapshotNotFresh,
    OriginMissing,
    EntranceUnavailable,
    RouteNeedsFreshMembers,
    NoSupportedRoute,
    ConsumableInventoryUnavailable,
}
#[derive(Clone, Debug, Serialize)]
pub struct PreparationGap {
    pub code: GapCode,
    pub section: &'static str,
    pub availability: Option<ObservationStatus>,
}
#[derive(Clone, Debug, Serialize)]
pub struct AuthoredTravel {
    pub declaration: Declared<Vec<EdgeId>>,
    // Directed reference dependencies, not an ordered route or inferred geometry.
    pub edges: Vec<azeroth_core::entities::TravelEdge>,
}
#[derive(Clone, Debug, Serialize)]
pub struct PreparationReport {
    pub schema_version: u16,
    pub context: DataContext,
    pub fixture: bool,
    pub revision: Option<DatasetRevision>,
    pub package: Option<LocalPreparationPackage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authored: Option<crate::authored_content::AuthoredSelection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authored_travel: Option<AuthoredTravel>,
    pub members: Vec<MemberPreparation>,
    pub evidence: Vec<PreparationEvidence>,
    pub gaps: Vec<PreparationGap>,
    pub route: Option<RouteSearch>,
    pub route_exclusions: Vec<PreparationEdgeExclusion>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PreparationEdgeExclusion {
    pub edge: EdgeId,
    pub character: Option<CharacterId>,
    pub condition: Option<ConditionAssessment>,
    pub reason: &'static str,
}

fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
fn outdated() -> AppError {
    AppError::new(
        ErrorCode::SnapshotOutdated,
        "Preparation inputs changed; refresh before inspection",
        true,
    )
}
fn count(size: usize) -> Result<()> {
    if size > MAX_SECTION_RECORDS {
        Err(invalid("Preparation section exceeds 32 records"))
    } else {
        Ok(())
    }
}
fn reference_count(size: usize, limit: usize) -> Result<()> {
    if size > limit {
        Err(invalid(
            "Preparation reference section exceeds schema bounds",
        ))
    } else {
        Ok(())
    }
}
fn unique<T: Ord>(values: impl IntoIterator<Item = T>) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(invalid("Preparation references repeat an identity"));
        }
    }
    Ok(())
}
fn declaration<T>(
    value: &Declared<T>,
    section: &'static str,
    gaps: &mut Vec<PreparationGap>,
    ids: &mut BTreeSet<EvidenceId>,
) -> Result<()> {
    count(value.evidence.len())?;
    unique(value.evidence.iter())?;
    ids.extend(value.evidence.iter().cloned());
    if value.observation.status() != ObservationStatus::Known {
        gaps.push(PreparationGap {
            code: GapCode::SectionUnavailable,
            section,
            availability: Some(value.observation.status()),
        });
    } else if value.evidence.is_empty() {
        gaps.push(PreparationGap {
            code: GapCode::EvidenceMissing,
            section,
            availability: None,
        });
    }
    Ok(())
}
fn notes(section: &Declared<Vec<Text>>) -> Result<()> {
    if let Some(values) = section.observation.value() {
        count(values.len())?;
        if values
            .iter()
            .any(|v| v.as_str().len() > 512 || v.as_str().trim().is_empty())
        {
            return Err(invalid("Preparation note must contain at most 512 bytes"));
        }
    }
    Ok(())
}
fn validate_package(
    package: &LocalPreparationPackage,
    gaps: &mut Vec<PreparationGap>,
) -> Result<BTreeSet<EvidenceId>> {
    if !matches!(package.schema_version, 1 | 2) {
        return Err(AppError::new(
            ErrorCode::UnsupportedSchema,
            "Unsupported preparation package schema",
            false,
        ));
    }
    if package.title.as_str().trim().is_empty() || package.title.as_str().len() > 160 {
        return Err(invalid("Preparation title must contain at most 160 bytes"));
    }
    let mut ids = BTreeSet::new();
    declaration(&package.entrance, "entrance", gaps, &mut ids)?;
    declaration(&package.access, "access", gaps, &mut ids)?;
    declaration(&package.quests, "quests", gaps, &mut ids)?;
    declaration(&package.rewards, "rewards", gaps, &mut ids)?;
    declaration(&package.checks, "checks", gaps, &mut ids)?;
    declaration(&package.formation_ms, "formation_time", gaps, &mut ids)?;
    declaration(&package.run_ms, "run_time", gaps, &mut ids)?;
    declaration(
        &package.before_departure,
        "before_departure",
        gaps,
        &mut ids,
    )?;
    declaration(&package.before_entry, "before_entry", gaps, &mut ids)?;
    declaration(&package.encounter_notes, "encounter_notes", gaps, &mut ids)?;
    for section in [
        &package.before_departure,
        &package.before_entry,
        &package.encounter_notes,
    ] {
        notes(section)?;
    }
    if let Some(entrance) = package.entrance.observation.value() {
        package.context.ensure_compatible(&entrance.map.context)?;
    }
    if let Some(predicate) = package.access.observation.value() {
        predicate.validate_in_context(&package.context)?;
    }
    let reference_limit = if package.schema_version == 2 {
        MAX_REFERENCE_RECORDS
    } else {
        MAX_SECTION_RECORDS
    };
    if let Some(quests) = package.quests.observation.value() {
        reference_count(quests.len(), reference_limit)?;
        unique(quests.iter())?;
    }
    if let Some(rewards) = package.rewards.observation.value() {
        reference_count(rewards.len(), reference_limit)?;
        unique(rewards.iter().map(|r| (&r.item, &r.source)))?;
        for reward in rewards {
            package.context.ensure_compatible(&reward.item.context)?;
        }
    }
    if let Some(checks) = package.checks.observation.value() {
        count(checks.len())?;
        unique(checks.iter().map(|c| &c.id))?;
        for check in checks {
            if check.label.as_str().trim().is_empty() || check.label.as_str().len() > 160 {
                return Err(invalid(
                    "Preparation check label must contain at most 160 bytes",
                ));
            }
            declaration(&check.requirement, "check", gaps, &mut ids)?;
            if let Some(predicate) = check.requirement.observation.value() {
                predicate.validate_in_context(&package.context)?;
            }
        }
    }
    if ids.len() > 1024 {
        return Err(invalid("Preparation evidence count exceeds bounds"));
    }
    Ok(ids)
}
struct Captured {
    keys: Vec<ProfileKey>,
    snapshots: Vec<Option<Envelope<CharacterSnapshot>>>,
    content: Option<ActiveContentRevision>,
    disposition: Option<azeroth_storage::content_dispositions::DispositionGuard>,
    party: Option<PartyId>,
    /// The activation accepts inferred travel for exactly this content.
    accept_inferred: bool,
}
fn capture(store: &SqliteStore, keys: Vec<ProfileKey>, party: Option<PartyId>) -> Result<Captured> {
    if keys.is_empty() || keys.len() > 40 {
        return Err(invalid("Preparation requires 1..=40 local members"));
    }
    unique(keys.iter())?;
    for key in &keys {
        keys[0].context.ensure_compatible(&key.context)?;
        if key.fixture != keys[0].fixture {
            return Err(invalid("Preparation members mix synthetic status"));
        }
    }
    let content = store.active_catalog_revision(&ContentScope {
        context: keys[0].context.clone(),
        fixture: keys[0].fixture,
    })?;
    let snapshots = keys
        .iter()
        .map(|key| store.current_snapshot(key))
        .collect::<Result<Vec<_>>>()?;
    let disposition = content
        .as_ref()
        .map(|c| {
            store.usable_catalog_revision(&c.key()).and_then(|v| {
                v.map(|(_, d)| d.guard()).ok_or_else(|| {
                    AppError::new(
                        ErrorCode::SnapshotOutdated,
                        "Preparation content changed",
                        false,
                    )
                })
            })
        })
        .transpose()?;
    Ok(Captured {
        disposition,
        keys,
        snapshots,
        content,
        party,
        accept_inferred: false,
    })
}
fn snapshot_hash(snapshot: &Option<Envelope<CharacterSnapshot>>) -> Result<Option<Sha256Hex>> {
    snapshot
        .as_ref()
        .map(azeroth_storage::snapshot_content_hash)
        .transpose()
}
fn content_hash(content: &Option<ActiveContentRevision>) -> Result<Option<String>> {
    use sha2::{Digest, Sha256};
    content
        .as_ref()
        .map(|c| {
            serde_json::to_vec(c)
                .map(|v| format!("{:x}", Sha256::digest(v)))
                .map_err(|_| invalid("Preparation content cannot be hashed"))
        })
        .transpose()
}
#[derive(PartialEq, Eq)]
enum CaptureClass {
    Missing,
    Fresh,
    Stale,
    Uncertain(azeroth_core::observation::ClockUncertainty),
}
fn freshness_class(value: Option<&Freshness>) -> CaptureClass {
    match value {
        None => CaptureClass::Missing,
        Some(Freshness::Fresh { .. }) => CaptureClass::Fresh,
        Some(Freshness::Stale { .. }) => CaptureClass::Stale,
        Some(Freshness::Uncertain { reason }) => CaptureClass::Uncertain(*reason),
    }
}

fn bounded_report(report: &PreparationReport) -> Result<()> {
    struct Limit {
        bytes: usize,
    }
    impl std::io::Write for Limit {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > MAX_REPORT_BYTES - self.bytes {
                return Err(std::io::Error::other("Preparation report limit"));
            }
            self.bytes += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Limit { bytes: 0 }, report).map_err(|_| {
        AppError::new(
            ErrorCode::ImportTooLarge,
            "Preparation report exceeds 4 MiB",
            false,
        )
    })
}
fn condition(
    requirement: &Observation<Predicate>,
    context: Option<&EvaluationContext<'_>>,
) -> Result<ConditionAssessment> {
    Ok(ConditionAssessment {
        availability: requirement.status(),
        evaluation: match (requirement.value(), context) {
            (Some(predicate), Some(context)) => Some(evaluate(predicate, context)?),
            _ => None,
        },
    })
}
fn gap(gaps: &mut Vec<PreparationGap>, code: GapCode, section: &'static str) {
    gaps.push(PreparationGap {
        code,
        section,
        availability: None,
    });
}

impl SnapshotApplication {
    pub fn dungeon_preparation(
        &self,
        text: &str,
        now: Option<UtcMillis>,
    ) -> Result<PreparationReport> {
        self.dungeon_preparation_using(text, now, || Ok(()))
    }
    fn dungeon_preparation_using(
        &self,
        text: &str,
        now: Option<UtcMillis>,
        before_publication: impl FnOnce() -> Result<()>,
    ) -> Result<PreparationReport> {
        let started = Instant::now();
        let mut request = json::request(text)?;
        let mut gaps = Vec::new();
        let mut captured = match &request.subject {
            PreparationSubject::Character { profile_id } => {
                self.with_profile_store_mut(profile_id, |store, key| {
                    store.with_immediate_read(|store| capture(store, vec![key.clone()], None))
                })?
            }
            PreparationSubject::Party { party_id } => self.with_store_mut(|store| {
                store.with_immediate_read(|store| {
                    let party = store
                        .party(party_id)?
                        .ok_or_else(|| invalid("No such preparation party"))?;
                    capture(store, party.participants, Some(party_id.clone()))
                })
            })?,
        };
        if let Some(content) = &captured.content {
            captured.accept_inferred = self.accepts_inferred_travel(&content.key())?;
        }
        if let Some(selection) = &request.authored {
            if request.package.is_some() {
                return Err(invalid(
                    "Choose either authored dungeon content or a local package",
                ));
            }
            let content = captured
                .content
                .as_ref()
                .ok_or_else(|| invalid("No active authored dungeon content"))?;
            request.package = Some(crate::authored_content::dungeon_package(
                selection, content,
            )?);
        }
        let evidence_ids = request
            .package
            .as_ref()
            .map(|p| validate_package(p, &mut gaps))
            .transpose()?
            .unwrap_or_default();
        let hashes = captured
            .snapshots
            .iter()
            .map(snapshot_hash)
            .collect::<Result<Vec<_>>>()?;
        let original_content_hash = content_hash(&captured.content)?;
        let clock = context(now);
        let capture_freshness = captured
            .snapshots
            .iter()
            .map(|s| s.as_ref().map(|s| freshness(&s.payload.capture, &clock)))
            .collect::<Vec<_>>();
        // Independently guard fresh members even when another member is stale or
        // missing. Full cohort membership is checked in the same final transaction.
        let fresh_snapshots = captured
            .snapshots
            .iter()
            .zip(&capture_freshness)
            .filter(|(_, f)| matches!(f, Some(Freshness::Fresh { .. })))
            .filter_map(|(s, _)| s.clone())
            .collect::<Vec<_>>();
        let guard = if fresh_snapshots.is_empty() {
            None
        } else {
            captured
                .content
                .as_ref()
                .map(|content| {
                    crate::planning_job_service::PublicationGuard::new(
                        None,
                        &fresh_snapshots,
                        content,
                        &clock,
                    )?
                    .bind_disposition(
                        captured.disposition.clone().ok_or_else(|| {
                            AppError::new(
                                ErrorCode::SnapshotOutdated,
                                "Preparation content changed",
                                false,
                            )
                        })?,
                    )
                })
                .transpose()?
        };
        let report = evaluate_inputs(
            request,
            &captured,
            capture_freshness.clone(),
            &evidence_ids,
            gaps,
            now,
        )?;
        bounded_report(&report)?;
        before_publication()?;
        self.with_store_mut(|store| {
            store.with_immediate_read(|store| {
                if let Some(id) = &captured.party
                    && store
                        .party(id)?
                        .is_none_or(|p| p.participants != captured.keys)
                {
                    return Err(outdated());
                }
                let mut current_snapshots = Vec::new();
                for (key, hash) in captured.keys.iter().zip(&hashes) {
                    let current = store.current_snapshot(key)?;
                    if snapshot_hash(&current)? != *hash {
                        return Err(outdated());
                    }
                    current_snapshots.push(current);
                }
                let current = store.active_catalog_revision(&ContentScope {
                    context: captured.keys[0].context.clone(),
                    fixture: captured.keys[0].fixture,
                })?;
                if content_hash(&current)? != original_content_hash {
                    return Err(outdated());
                }
                if let Some(disposition) = &captured.disposition
                    && let Err(e) = store.validate_usable_catalog_guard(disposition)
                {
                    if e.code == ErrorCode::StorageError {
                        return Err(e);
                    }
                    return Err(outdated());
                }
                if let Some(guard) = &guard {
                    guard.validate(store, || {
                        crate::planning_job_service::advanced_now(now, started.elapsed())
                    })?;
                }
                // Sample after all hashes and field comparisons, including captures
                // omitted from the fresh-member guard. No second store read follows.
                let final_clock = context(crate::planning_job_service::advanced_now(
                    now,
                    started.elapsed(),
                ));
                for (current, original) in current_snapshots.iter().zip(&capture_freshness) {
                    let new = current
                        .as_ref()
                        .map(|s| freshness(&s.payload.capture, &final_clock));
                    if freshness_class(new.as_ref()) != freshness_class(original.as_ref()) {
                        return Err(outdated());
                    }
                }
                Ok(())
            })
        })?;
        Ok(report)
    }
}

fn evaluate_inputs(
    request: PreparationRequest,
    captured: &Captured,
    capture_freshness: Vec<Option<Freshness>>,
    evidence_ids: &BTreeSet<EvidenceId>,
    mut gaps: Vec<PreparationGap>,
    now: Option<UtcMillis>,
) -> Result<PreparationReport> {
    let key = &captured.keys[0];
    let mut evidence_ids = evidence_ids.clone();
    let authored_travel = if let Some(selection) = &request.authored {
        let content = captured
            .content
            .as_ref()
            .ok_or_else(|| invalid("No active authored travel content"))?;
        let authored = content
            .content_v2()
            .ok_or_else(|| invalid("Authored travel requires v2 content"))?;
        let dungeon = authored
            .dungeons
            .iter()
            .find(|d| d.id == selection.id)
            .ok_or_else(|| invalid("Authored dungeon travel does not resolve"))?;
        let declaration_value = Declared {
            observation: dungeon.travel_edges.clone(),
            evidence: dungeon.evidence.clone(),
        };
        declaration(
            &declaration_value,
            "authored_travel",
            &mut gaps,
            &mut evidence_ids,
        )?;
        let selected = dungeon
            .travel_edges
            .value()
            .map(|ids| ids.iter().collect::<BTreeSet<_>>())
            .unwrap_or_default();
        let edges = content
            .reference()
            .edges
            .iter()
            .filter(|edge| selected.contains(&edge.id))
            .cloned()
            .collect::<Vec<_>>();
        if edges.len() != selected.len() {
            return Err(invalid("Authored travel edge does not resolve"));
        }
        for edge in &edges {
            evidence_ids.extend(edge.evidence.iter().cloned());
        }
        Some(AuthoredTravel {
            declaration: declaration_value,
            edges,
        })
    } else {
        None
    };
    if request.package.is_none() {
        gap(&mut gaps, GapCode::PackageMissing, "package");
    }
    if captured.content.is_none() {
        gap(&mut gaps, GapCode::ActiveContentMissing, "content");
    }
    let mut evidence = Vec::new();
    if let Some(package) = &request.package {
        key.context.ensure_compatible(&package.context)?;
        if key.fixture != package.fixture {
            return Err(invalid(
                "Preparation package synthetic status differs from selected members",
            ));
        }
        if request.authored.is_none() {
            gap(&mut gaps, GapCode::LocalPackageUnverified, "package");
        }
        gap(
            &mut gaps,
            GapCode::ConsumableInventoryUnavailable,
            "consumables",
        );
        if let Some(content) = &captured.content {
            if content.reference().revision != package.revision {
                return Err(outdated());
            }
            let dataset = &content.reference();
            if let Some(entrance) = package.entrance.observation.value()
                && !dataset
                    .nodes
                    .iter()
                    .any(|n| n.id == entrance.id && n.position.map == entrance.map)
            {
                return Err(invalid("Preparation entrance is absent from exact content"));
            }
            if let Some(quests) = package.quests.observation.value()
                && quests
                    .iter()
                    .any(|q| !dataset.quests.iter().any(|d| &d.id == q))
            {
                return Err(invalid("Preparation quest is absent from exact content"));
            }
            if let Some(rewards) = package.rewards.observation.value() {
                for reward in rewards {
                    if !dataset.items.iter().any(|i| i.key == reward.item)
                        || !dataset
                            .acquisitions
                            .iter()
                            .any(|a| a.id == reward.source && a.items.contains(&reward.item))
                    {
                        return Err(invalid(
                            "Preparation reward source does not reference the declared item",
                        ));
                    }
                }
            }
            for id in &evidence_ids {
                let record = dataset
                    .evidence
                    .iter()
                    .find(|e| &e.id == id)
                    .ok_or_else(|| invalid("Preparation evidence is absent from exact content"))?;
                let source = content
                    .sources()
                    .iter()
                    .find(|s| s.id == record.source)
                    .ok_or_else(|| invalid("Preparation evidence source is unregistered"))?;
                evidence.push(PreparationEvidence {
                    id: id.clone(),
                    source: record.source.clone(),
                    confidence: record.confidence,
                    license: record.license.clone(),
                    redistribution: source.redistribution.clone(),
                    offline_usable: source.offline_usable.clone(),
                });
            }
        }
    }
    let available = captured
        .snapshots
        .iter()
        .zip(&capture_freshness)
        .filter(|(_, freshness)| matches!(freshness, Some(Freshness::Fresh { .. })))
        .filter_map(|(snapshot, _)| snapshot.as_ref().map(|s| s.payload.clone()))
        .collect::<Vec<_>>();
    // ParticipantState can read remote typed fields. Only fresh captures belong
    // in this evaluator party, matching the fresh-subset publication guard.
    // The explanatory report below still retains the entire requested cohort.
    let party = captured
        .party
        .as_ref()
        .filter(|_| !available.is_empty())
        .map(|id| {
            PartySnapshot::try_new(PartySnapshotData {
                id: id.clone(),
                context: key.context.clone(),
                participants: available,
            })
        })
        .transpose()?;
    let clock = context(now);
    let facts = BTreeMap::new();
    let mut members = Vec::new();
    for ((key, snapshot), freshness) in captured
        .keys
        .iter()
        .zip(&captured.snapshots)
        .zip(&capture_freshness)
    {
        let usable = matches!(freshness, Some(Freshness::Fresh { .. }));
        if snapshot.is_none() {
            gap(&mut gaps, GapCode::SnapshotMissing, "member");
        } else if !usable {
            gap(&mut gaps, GapCode::SnapshotNotFresh, "member");
        }
        let evaluation = snapshot
            .as_ref()
            .filter(|_| usable)
            .map(|s| EvaluationContext {
                snapshot: &s.payload,
                party: party.as_ref(),
                freshness: &clock,
                stale_policy: StalePolicy::Reject,
                limits: RuleLimits::default(),
            });
        let mut member = MemberPreparation {
            profile_id: profile_id(key)?,
            character: key.character.clone(),
            snapshot_id: snapshot.as_ref().map(|s| s.payload.id.clone()),
            freshness: *freshness,
            access: None,
            quests: vec![],
            checks: vec![],
            rewards: vec![],
        };
        if let (Some(package), Some(content)) = (&request.package, &captured.content) {
            member.access = Some(condition(&package.access.observation, evaluation.as_ref())?);
            let dataset = &content.reference();
            for quest in package.quests.observation.value().into_iter().flatten() {
                let definition = dataset
                    .quests
                    .iter()
                    .find(|q| &q.id == quest)
                    .expect("validated quest");
                let authored = content
                    .content_v2()
                    .map(|c| crate::authored_content::quest_facts(definition, c))
                    .transpose()?;
                let definition = authored
                    .as_ref()
                    .map(|v| &v.definition)
                    .unwrap_or(definition);
                let assessment = evaluation
                    .as_ref()
                    .map(|context| {
                        evaluate_participant(
                            &EligibilityInputs {
                                definition,
                                access: authored.as_ref().and_then(|v| v.access.as_ref()),
                                exclusive_groups: authored
                                    .as_ref()
                                    .map(|v| v.exclusive_groups.as_slice())
                                    .unwrap_or(&[]),
                                facts: &facts,
                                freshness: &clock,
                                stale_policy: StalePolicy::Reject,
                                limits: RuleLimits::default(),
                            },
                            context.snapshot,
                            party.as_ref(),
                        )
                        .map(crate::party::assessment)
                    })
                    .transpose()?;
                member.quests.push(QuestPreparation {
                    quest: quest.clone(),
                    assessment,
                });
            }
            for check in package.checks.observation.value().into_iter().flatten() {
                member.checks.push(CheckPreparation {
                    check: check.id.clone(),
                    condition: condition(&check.requirement.observation, evaluation.as_ref())?,
                });
            }
            for reward in package.rewards.observation.value().into_iter().flatten() {
                let item = dataset
                    .items
                    .iter()
                    .find(|i| i.key == reward.item)
                    .expect("validated item");
                let source = dataset
                    .acquisitions
                    .iter()
                    .find(|a| a.id == reward.source)
                    .expect("validated acquisition");
                let effective_item = content
                    .content_v2()
                    .map(|c| crate::authored_content::item_facts(item, c))
                    .transpose()?;
                let item = effective_item.as_ref().map(|v| &v.0).unwrap_or(item);
                let effective_source = content
                    .content_v2()
                    .map(|c| crate::authored_content::acquisition_facts(source, &reward.item, c))
                    .transpose()?;
                let source = effective_source.as_ref().unwrap_or(source);
                member.rewards.push(RewardPreparation {
                    item: reward.item.clone(),
                    source: reward.source.clone(),
                    item_requirement: condition(&item.requirements, evaluation.as_ref())?,
                    acquisition_requirement: condition(&source.conditions, evaluation.as_ref())?,
                    probability: source.probability.clone(),
                    cost: source.cost.clone(),
                });
            }
        }
        members.push(member);
    }
    let mut route = None;
    let mut route_exclusions = Vec::new();
    if let (Some(package), Some(content)) = (&request.package, &captured.content) {
        match (&request.origin, package.entrance.observation.value()) {
            (None, _) => gap(&mut gaps, GapCode::OriginMissing, "route"),
            (_, None) => gap(&mut gaps, GapCode::EntranceUnavailable, "route"),
            (Some(origin), Some(entrance)) => {
                if !content.reference().nodes.iter().any(|n| &n.id == origin) {
                    return Err(invalid("Preparation origin is absent from exact content"));
                }
                if !capture_freshness
                    .iter()
                    .all(|f| matches!(f, Some(Freshness::Fresh { .. })))
                {
                    gap(&mut gaps, GapCode::RouteNeedsFreshMembers, "route");
                } else {
                    let contexts = captured
                        .snapshots
                        .iter()
                        .flatten()
                        .map(|s| EvaluationContext {
                            snapshot: &s.payload,
                            party: party.as_ref(),
                            freshness: &clock,
                            stale_policy: StalePolicy::Reject,
                            limits: RuleLimits::default(),
                        })
                        .collect::<Vec<_>>();
                    let view =
                        crate::catalog::TravelPolicyView::new(content, captured.accept_inferred);
                    let travel = crate::catalog::travel_inputs(&view);
                    let mut edges = Vec::new();
                    for edge in travel.edges.iter() {
                        let mut usable = true;
                        // An accepted inferred edge's unknown requirement is
                        // assumed (the hop says so), never called satisfied.
                        let assumed = travel.options.accept_unknown_requirements
                            && edge.requirements.value().is_none()
                            && travel.options.inferred_edges.contains(&edge.id);
                        for context in &contexts {
                            let assessment = condition(&edge.requirements, Some(context))?;
                            let member_usable = assessment
                                .evaluation
                                .as_ref()
                                .is_some_and(|r| r.truth == Truth::True)
                                || assessment.availability == ObservationStatus::NotApplicable
                                || assumed;
                            usable &= member_usable;
                            if !member_usable {
                                route_exclusions.push(PreparationEdgeExclusion {
                                    edge: edge.id.clone(),
                                    character: Some(context.snapshot.character.clone()),
                                    condition: Some(assessment),
                                    reason: "member_requirement_unavailable_or_unmet",
                                });
                            }
                        }
                        // Shared entrance travel never spends a stateful return action.
                        let stateful = matches!(
                            edge.mode,
                            azeroth_core::entities::TravelMode::Return
                                | azeroth_core::entities::TravelMode::Teleport
                        );
                        if stateful {
                            route_exclusions.push(PreparationEdgeExclusion {
                                edge: edge.id.clone(),
                                character: None,
                                condition: None,
                                reason: "stateful_action_not_used_for_shared_entrance",
                            });
                        }
                        if usable && !stateful {
                            edges.push(edge.clone());
                        }
                    }
                    // Options may only name edges of the searched graph.
                    let kept: BTreeSet<&azeroth_core::types::EdgeId> =
                        edges.iter().map(|edge| &edge.id).collect();
                    let mut options = travel.options.clone();
                    options.inferred_edges.retain(|id| kept.contains(id));
                    let search = shortest_route(
                        &content.reference().nodes,
                        &edges,
                        origin,
                        &entrance.id,
                        &contexts[0],
                        &options,
                    )?;
                    if !matches!(search.outcome, RouteOutcome::Complete { .. }) {
                        gap(&mut gaps, GapCode::NoSupportedRoute, "route");
                    }
                    route = Some(search);
                }
            }
        }
    }
    Ok(PreparationReport {
        schema_version: 1,
        context: key.context.clone(),
        fixture: key.fixture,
        revision: captured
            .content
            .as_ref()
            .map(|c| c.reference().revision.clone()),
        package: request.package,
        authored: request.authored,
        authored_travel,
        members,
        evidence,
        gaps,
        route,
        route_exclusions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    const NOW: i64 = 1791792000000;
    fn setup() -> (SnapshotApplication, Value, String, String) {
        let app = SnapshotApplication::in_memory().unwrap();
        let mut ids = Vec::new();
        for index in 0..2 {
            let mut s: Value = serde_json::from_str(include_str!(
                "../../../fixtures/contracts/v1/character-snapshot.json"
            ))
            .unwrap();
            s["payload"]["character"] = json!(format!("member-{index}"));
            s["payload"]["id"] = json!(format!("snapshot-{index}"));
            s["payload"]["capture"]["observed_at_utc_ms"] = json!(if index == 0 {
                NOW - 100
            } else {
                NOW - 1_800_100
            });
            s["payload"]["level"]["clock_trust"] = json!("trusted");
            s["payload"]["level"]["observed_at_utc_ms"] = json!(NOW - 100);
            ids.push(
                app.import_text(&format!("ANAV1:{s}"), Some(UtcMillis::new(NOW).unwrap()))
                    .unwrap()
                    .profile_id,
            );
        }
        app.stage_content(include_str!(
            "../../../fixtures/desktop/content/demo-revision.json"
        ))
        .unwrap();
        app.activate_content(&ids[0], &DatasetRevision::new("demo-r1").unwrap())
            .unwrap();
        app.save_party(&PartyId::new("prep-test").unwrap(), &ids)
            .unwrap();
        let request = json!({"schema_version":1,"subject":{"kind":"party","party_id":"prep-test"},"origin":"demo-node-1","package":serde_json::from_str::<Value>(include_str!("../../../fixtures/desktop/preparation/synthetic-package.json")).unwrap()});
        (app, request, ids[0].clone(), ids[1].clone())
    }
    #[test]
    fn versioned_reference_limits_preserve_all_ids_and_other_section_limits() {
        let mut package: LocalPreparationPackage = serde_json::from_str(include_str!(
            "../../../fixtures/desktop/preparation/synthetic-package.json"
        ))
        .unwrap();
        let quests: Vec<_> = (0..MAX_REFERENCE_RECORDS)
            .map(|i| QuestId::new(format!("quest-{i}")).unwrap())
            .collect();
        let template = package.rewards.observation.value().unwrap()[0].clone();
        let rewards: Vec<_> = (0..MAX_REFERENCE_RECORDS)
            .map(|i| {
                let mut reward = template.clone();
                reward.source = AcquisitionId::new(format!("source-{i}")).unwrap();
                reward
            })
            .collect();
        package.quests.observation = Observation::known(quests, Default::default()).unwrap();
        package.rewards.observation = Observation::known(rewards, Default::default()).unwrap();
        assert!(validate_package(&package, &mut vec![]).is_err());
        package.schema_version = 2;
        validate_package(&package, &mut vec![]).unwrap();
        package.quests.observation = Observation::known(
            (0..=MAX_REFERENCE_RECORDS)
                .map(|i| QuestId::new(format!("quest-{i}")).unwrap())
                .collect(),
            Default::default(),
        )
        .unwrap();
        assert!(validate_package(&package, &mut vec![]).is_err());
        package.quests.observation = Observation::unknown();
        let mut rewards = package.rewards.observation.value().unwrap().clone();
        let mut extra = template;
        extra.source = AcquisitionId::new("extra").unwrap();
        rewards.push(extra);
        package.rewards.observation = Observation::known(rewards, Default::default()).unwrap();
        assert!(validate_package(&package, &mut vec![]).is_err());
        package.rewards.observation = Observation::unknown();
        package.before_entry.observation = Observation::known(
            vec![Text::new("manual note").unwrap(); 33],
            Default::default(),
        )
        .unwrap();
        assert!(validate_package(&package, &mut vec![]).is_err());
    }
    #[test]
    fn changed_snapshot_membership_or_content_refuses_prepared_report_after_unlocked_work() {
        for kind in ["snapshot", "membership", "content"] {
            let (app, request, first, second) = setup();
            let result = app.dungeon_preparation_using(
                &request.to_string(),
                Some(UtcMillis::new(NOW).unwrap()),
                || {
                    match kind {
                        "snapshot" => {
                            let mut s = app
                                .current(&first, Some(UtcMillis::new(NOW).unwrap()))?
                                .unwrap()
                                .envelope;
                            let mut raw = serde_json::to_value(&s).unwrap();
                            raw["payload"]["id"] = json!("changed-snapshot");
                            raw["payload"]["capture"]["sequence"] = json!(2000);
                            raw["producer"]["sequence"] = json!(2000);
                            raw["payload"]["capture"]["session_elapsed_ms"] = json!(500000);
                            raw["payload"]["capture"]["observed_at_utc_ms"] = json!(NOW);
                            s = serde_json::from_value(raw).unwrap();
                            app.import_text(
                                &format!("ANAV1:{}", serde_json::to_string(&s).unwrap()),
                                Some(UtcMillis::new(NOW).unwrap()),
                            )?;
                        }
                        "membership" => {
                            app.save_party(&PartyId::new("prep-test").unwrap(), &[second, first])?;
                        }
                        _ => {
                            let mut content: Value = serde_json::from_str(include_str!(
                                "../../../fixtures/desktop/content/demo-revision.json"
                            ))
                            .unwrap();
                            content["envelope"]["payload"]["revision"] =
                                json!("new-preparation-revision");
                            app.stage_content(&content.to_string())?;
                            app.activate_content(
                                &first,
                                &DatasetRevision::new("new-preparation-revision").unwrap(),
                            )?;
                        }
                    }
                    Ok(())
                },
            );
            let Err(error) = result else {
                panic!("Changed captured inputs must fail")
            };
            assert_eq!(error.code, ErrorCode::SnapshotOutdated);
        }
    }
    #[test]
    fn missing_member_stays_missing_with_independent_fresh_member_assessment() {
        let (app, request, _, second) = setup();
        app.with_profile_store_mut(&second, |store, key| store.delete_profile(key))
            .unwrap();
        let report = app
            .dungeon_preparation(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
            .unwrap();
        assert!(report.members[1].snapshot_id.is_none());
        assert!(report.members[1].freshness.is_none());
        assert!(report.members[1].quests[0].assessment.is_none());
        assert!(report.route.is_none());
        assert!(
            report.members[0]
                .access
                .as_ref()
                .unwrap()
                .evaluation
                .is_some()
        );
    }
    #[test]
    fn fresh_member_field_expiry_is_guarded_even_when_other_member_is_stale() {
        let (app, request, first, _) = setup();
        let mut snapshot = serde_json::to_value(
            app.current(&first, Some(UtcMillis::new(NOW).unwrap()))
                .unwrap()
                .unwrap()
                .envelope,
        )
        .unwrap();
        snapshot["payload"]["id"] = json!("field-expiry-snapshot");
        snapshot["payload"]["capture"]["sequence"] = json!(2000);
        snapshot["producer"]["sequence"] = json!(2000);
        snapshot["payload"]["capture"]["session_elapsed_ms"] = json!(500000);
        snapshot["payload"]["capture"]["observed_at_utc_ms"] = json!(NOW);
        snapshot["payload"]["level"]["observed_at_utc_ms"] = json!(NOW - 1_799_950);
        app.import_text(
            &format!("ANAV1:{snapshot}"),
            Some(UtcMillis::new(NOW).unwrap()),
        )
        .unwrap();
        let result = app.dungeon_preparation_using(
            &request.to_string(),
            Some(UtcMillis::new(NOW).unwrap()),
            || {
                std::thread::sleep(std::time::Duration::from_millis(100));
                Ok(())
            },
        );
        let Err(error) = result else {
            panic!("Independently expired field must fail")
        };
        assert_eq!(error.code, ErrorCode::SnapshotOutdated);
    }
    #[test]
    fn stale_remote_capture_cannot_supply_independently_fresh_fields_to_party_predicates() {
        let (app, mut request, _, second) = setup();
        let now = Some(UtcMillis::new(NOW).unwrap());
        let mut snapshot =
            serde_json::to_value(app.current(&second, now).unwrap().unwrap().envelope).unwrap();
        snapshot["payload"]["id"] = json!("remote-expiring-snapshot");
        snapshot["payload"]["capture"]["sequence"] = json!(2000);
        snapshot["producer"]["sequence"] = json!(2000);
        snapshot["payload"]["capture"]["session_elapsed_ms"] = json!(500000);
        snapshot["payload"]["level"]["observed_at_utc_ms"] = json!(NOW - 1_799_950);
        app.import_text(&format!("ANAV1:{snapshot}"), now).unwrap();
        assert_eq!(
            app.current(&second, now)
                .unwrap()
                .unwrap()
                .envelope
                .payload
                .id
                .as_str(),
            "remote-expiring-snapshot"
        );
        request["package"]["access"]["observation"] = json!({"status":"known","value":{"op":"participant_state","participant":"member-1","predicate":{"op":"level_at_least","level":22}}});
        // Supplied clocks span the remote field's expiry without sleeps. Its
        // already stale capture makes that field unavailable at both clocks.
        for at in [NOW, NOW + 100] {
            let report = app
                .dungeon_preparation(&request.to_string(), Some(UtcMillis::new(at).unwrap()))
                .unwrap();
            assert_eq!(
                report.members[0]
                    .access
                    .as_ref()
                    .unwrap()
                    .evaluation
                    .as_ref()
                    .unwrap()
                    .truth,
                Truth::Unknown
            );
            assert!(matches!(
                report.members[1].freshness,
                Some(Freshness::Stale { .. })
            ));
            assert!(
                report.members[1]
                    .access
                    .as_ref()
                    .unwrap()
                    .evaluation
                    .is_none()
            );
        }
    }
}
