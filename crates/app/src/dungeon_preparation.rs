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
    travel::{RouteOptions, RouteOutcome, RouteSearch, shortest_route},
    types::*,
};
use azeroth_storage::{
    ProfileKey, SnapshotStore, SqliteStore,
    content::{ContentRevision, ContentScope},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};

mod json;
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
pub const MAX_REPORT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_SECTION_RECORDS: usize = 32;

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
pub struct PreparationReport {
    pub schema_version: u16,
    pub context: DataContext,
    pub fixture: bool,
    pub revision: Option<DatasetRevision>,
    pub package: Option<LocalPreparationPackage>,
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
    if package.schema_version != 1 {
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
    if let Some(quests) = package.quests.observation.value() {
        count(quests.len())?;
        unique(quests.iter())?;
    }
    if let Some(rewards) = package.rewards.observation.value() {
        count(rewards.len())?;
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
    content: Option<ContentRevision>,
    party: Option<PartyId>,
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
    let content = store.active_content_revision(&ContentScope {
        context: keys[0].context.clone(),
        fixture: keys[0].fixture,
    })?;
    let snapshots = keys
        .iter()
        .map(|key| store.current_snapshot(key))
        .collect::<Result<Vec<_>>>()?;
    Ok(Captured {
        keys,
        snapshots,
        content,
        party,
    })
}
fn snapshot_hash(snapshot: &Option<Envelope<CharacterSnapshot>>) -> Result<Option<Sha256Hex>> {
    snapshot
        .as_ref()
        .map(azeroth_storage::snapshot_content_hash)
        .transpose()
}
fn content_hash(content: &Option<ContentRevision>) -> Result<Option<String>> {
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
fn freshness_class(value: Option<&Freshness>) -> u8 {
    match value {
        None => 0,
        Some(Freshness::Fresh { .. }) => 1,
        Some(Freshness::Stale { .. }) => 2,
        Some(Freshness::Uncertain { reason }) => 3 + *reason as u8,
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
        let started = Instant::now();
        let request = json::request(text)?;
        let mut gaps = Vec::new();
        let evidence_ids = request
            .package
            .as_ref()
            .map(|p| validate_package(p, &mut gaps))
            .transpose()?
            .unwrap_or_default();
        let captured = match &request.subject {
            PreparationSubject::Character { profile_id } => self
                .with_profile_store_mut(profile_id, |store, key| {
                    capture(store, vec![key.clone()], None)
                })?,
            PreparationSubject::Party { party_id } => self.with_store_mut(|store| {
                let party = store
                    .party(party_id)?
                    .ok_or_else(|| invalid("No such preparation party"))?;
                capture(store, party.participants, Some(party_id.clone()))
            })?,
        };
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
        let all_fresh = capture_freshness
            .iter()
            .all(|f| matches!(f, Some(Freshness::Fresh { .. })));
        let guard = if all_fresh {
            captured
                .content
                .as_ref()
                .map(|content| {
                    crate::planning_job_service::PublicationGuard::new(
                        captured.party.clone(),
                        &captured
                            .snapshots
                            .iter()
                            .flatten()
                            .cloned()
                            .collect::<Vec<_>>(),
                        content,
                        &clock,
                    )
                })
                .transpose()?
        } else {
            None
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
        self.with_store_mut(|store| {
            if let Some(id) = &captured.party {
                if store
                    .party(id)?
                    .is_none_or(|p| p.participants != captured.keys)
                {
                    return Err(outdated());
                }
            }
            for ((key, hash), old_freshness) in
                captured.keys.iter().zip(&hashes).zip(&capture_freshness)
            {
                let current = store.current_snapshot(key)?;
                if snapshot_hash(&current)? != *hash {
                    return Err(outdated());
                }
                let new = current.as_ref().map(|s| {
                    freshness(
                        &s.payload.capture,
                        &context(crate::planning_job_service::advanced_now(
                            now,
                            started.elapsed(),
                        )),
                    )
                });
                if freshness_class(new.as_ref()) != freshness_class(old_freshness.as_ref()) {
                    return Err(outdated());
                }
            }
            let current = store.active_content_revision(&ContentScope {
                context: captured.keys[0].context.clone(),
                fixture: captured.keys[0].fixture,
            })?;
            if content_hash(&current)? != original_content_hash {
                return Err(outdated());
            }
            Ok(())
        })?;
        if let Some(guard) = guard {
            guard.publish(
                self,
                crate::planning_job_service::advanced_now(now, started.elapsed()),
                &mut || Ok(()),
            )?;
        }
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
        gap(&mut gaps, GapCode::LocalPackageUnverified, "package");
        gap(
            &mut gaps,
            GapCode::ConsumableInventoryUnavailable,
            "consumables",
        );
        if let Some(content) = &captured.content {
            if content.envelope.payload.revision != package.revision {
                return Err(outdated());
            }
            let dataset = &content.envelope.payload;
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
            for id in evidence_ids {
                let record = dataset
                    .evidence
                    .iter()
                    .find(|e| &e.id == id)
                    .ok_or_else(|| invalid("Preparation evidence is absent from exact content"))?;
                let source = content
                    .sources
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
        .flatten()
        .map(|s| s.payload.clone())
        .collect::<Vec<_>>();
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
            freshness: freshness.clone(),
            access: None,
            quests: vec![],
            checks: vec![],
            rewards: vec![],
        };
        if let (Some(package), Some(content)) = (&request.package, &captured.content) {
            member.access = Some(condition(&package.access.observation, evaluation.as_ref())?);
            let dataset = &content.envelope.payload;
            for quest in package.quests.observation.value().into_iter().flatten() {
                let definition = dataset
                    .quests
                    .iter()
                    .find(|q| &q.id == quest)
                    .expect("validated quest");
                let assessment = evaluation
                    .as_ref()
                    .map(|context| {
                        evaluate_participant(
                            &EligibilityInputs {
                                definition,
                                access: None,
                                exclusive_groups: &[],
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
                if !content
                    .envelope
                    .payload
                    .nodes
                    .iter()
                    .any(|n| &n.id == origin)
                {
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
                    let mut edges = Vec::new();
                    for edge in &content.envelope.payload.edges {
                        let mut usable = true;
                        for context in &contexts {
                            let assessment = condition(&edge.requirements, Some(context))?;
                            let member_usable = assessment.evaluation.as_ref().is_some_and(|r|r.truth==Truth::True)
                                || assessment.availability==ObservationStatus::NotApplicable;
                            usable &= member_usable;
                            if !member_usable {route_exclusions.push(PreparationEdgeExclusion {
                                edge:edge.id.clone(),character:Some(context.snapshot.character.clone()),
                                condition:Some(assessment),reason:"member_requirement_unavailable_or_unmet",
                            });}
                        }
                        // Shared entrance travel never spends a stateful return action.
                        let stateful=matches!(
                                edge.mode,
                                azeroth_core::entities::TravelMode::Return
                                    | azeroth_core::entities::TravelMode::Teleport
                            );
                        if stateful {route_exclusions.push(PreparationEdgeExclusion {edge:edge.id.clone(),character:None,condition:None,reason:"stateful_action_not_used_for_shared_entrance"});}
                        if usable && !stateful {
                            edges.push(edge.clone());
                        }
                    }
                    let search = shortest_route(
                        &content.envelope.payload.nodes,
                        &edges,
                        origin,
                        &entrance.id,
                        &contexts[0],
                        &RouteOptions::default(),
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
            .map(|c| c.envelope.payload.revision.clone()),
        package: request.package,
        members,
        evidence,
        gaps,
        route,
        route_exclusions,
    })
}
