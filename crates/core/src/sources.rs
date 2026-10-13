//! Provider/source registry, normalized evidence records and the conflict model
//! (plan sections 8.3-8.5). Pure: no I/O, no clock reads, no persistence.
//!
//! Three rules shape this module:
//! - A source is registered before any evidence citing it is ingested.
//! - Source precedence ranks evidence; it never discards it. Records that lose
//!   stay in the output and every disagreement becomes a [`SourceConflict`].
//! - Records are keyed by full `DataContext`-qualified keys. The same item ID in
//!   another edition, build or namespace is a different record, never a conflict
//!   and never merged. A record imported from another edition cannot become
//!   verified for this edition by changing an edition string.
//!
//! Evidence precedence is owned here and nowhere else. There is no numeric
//! confidence: the six [`Confidence`] categories are ranked ordinally.
use crate::{
    entities::{
        Confidence, EvidenceRecord, ItemDefinition, ItemDefinitionData, QuestDefinition,
        QuestDefinitionData, bounded, unique,
    },
    error::{AppError, ErrorCode, Result, Validate, Validated, invalid},
    observation::Observation,
    types::{
        DataContext, DatasetRevision, EditionId, EvidenceId, ItemKey, ProviderId, QuestId,
        SourceId, Text, UtcMillis,
    },
};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::BTreeMap};

/// Plan 8.3: primary observations, publisher documentation or secondary inference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordClass {
    PrimaryObservation,
    PublisherDocumentation,
    SecondaryInference,
}
/// How a source's data is obtained. Closed set; an unrecorded method is an
/// `Observation` that is not known, never a default variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessMethod {
    PublicApi,
    FileDownload,
    ManualEntry,
    AddonExport,
    Authored,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthRequirement {
    None,
    ApiKey,
    UserAccount,
}
/// Redistribution permission. There is deliberately no default: an unrecorded
/// permission is an unknown `Observation`, which callers must treat as not permitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Redistribution {
    Permitted,
    NotPermitted,
    Conditional,
}

/// One registered provider (plan 8.3). Every policy field is an `Observation`,
/// so an unknown licence or schedule stays distinct from a recorded negative.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceRecordData {
    pub id: SourceId,
    /// Capability and status stay on `ProviderCapability`; this is only a link.
    pub provider: Option<ProviderId>,
    pub publisher: Text,
    pub location: Observation<Text>,
    pub editions: Observation<Vec<EditionId>>,
    pub access: Observation<AccessMethod>,
    pub authentication: Observation<AuthRequirement>,
    pub license: Observation<Text>,
    pub attribution: Observation<Text>,
    pub redistribution: Observation<Redistribution>,
    pub retention: Observation<Text>,
    pub allowed_fields: Observation<Vec<Text>>,
    pub transformations: Observation<Vec<Text>>,
    pub update_schedule: Observation<Text>,
    pub conditional_requests: Observation<bool>,
    pub failure_behavior: Observation<Text>,
    pub last_retrieval_utc_ms: Observation<UtcMillis>,
    pub verified_revision: Observation<DatasetRevision>,
    pub offline_usable: Observation<bool>,
    pub record_class: RecordClass,
}
pub type SourceRecord = Validated<SourceRecordData>;

fn nonblank(text: &Text) -> Result<()> {
    if text.as_str().trim().is_empty() {
        Err(invalid("Source text fields must not be blank"))
    } else {
        Ok(())
    }
}
fn known_text(observation: &Observation<Text>) -> Result<()> {
    observation.value().map_or(Ok(()), nonblank)
}
fn known_texts(observation: &Observation<Vec<Text>>) -> Result<()> {
    if let Some(values) = observation.value() {
        bounded(values.len())?;
        if values.is_empty() {
            return Err(invalid("A known source list must not be empty"));
        }
        values.iter().try_for_each(nonblank)?;
        unique(values.iter().map(Text::as_str))?;
    }
    Ok(())
}
impl Validate for SourceRecordData {
    fn validate(&self) -> Result<()> {
        nonblank(&self.publisher)?;
        for text in [
            &self.location,
            &self.license,
            &self.attribution,
            &self.retention,
            &self.update_schedule,
            &self.failure_behavior,
        ] {
            known_text(text)?;
        }
        if let Some(editions) = self.editions.value() {
            bounded(editions.len())?;
            if editions.is_empty() {
                return Err(invalid("Known edition coverage must not be empty"));
            }
            unique(editions.iter())?;
        }
        known_texts(&self.allowed_fields)?;
        known_texts(&self.transformations)
    }
}

/// Sources by ID. Registration is the only way in, and a duplicate is rejected,
/// never replaced. The registry holds no built-in rows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Vec<SourceRecord>", into = "Vec<SourceRecord>")]
pub struct SourceRegistry {
    sources: BTreeMap<SourceId, SourceRecord>,
}
impl SourceRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&mut self, record: SourceRecord) -> Result<()> {
        if self.sources.contains_key(&record.id) {
            return Err(invalid("Source is already registered"));
        }
        bounded(self.sources.len() + 1)?;
        self.sources.insert(record.id.clone(), record);
        Ok(())
    }
    pub fn get(&self, id: &SourceId) -> Option<&SourceRecord> {
        self.sources.get(id)
    }
    pub fn len(&self) -> usize {
        self.sources.len()
    }
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
    /// Plan 8.3: a provider is registered before its data is ingested.
    pub fn ensure_registered(&self, id: &SourceId) -> Result<&SourceRecord> {
        self.sources.get(id).ok_or_else(|| {
            AppError::new(
                ErrorCode::DatasetIncomplete,
                "Evidence source is not registered",
                false,
            )
        })
    }
}
impl TryFrom<Vec<SourceRecord>> for SourceRegistry {
    type Error = AppError;
    fn try_from(records: Vec<SourceRecord>) -> Result<Self> {
        bounded(records.len())?;
        let mut registry = Self::new();
        for record in records {
            registry.register(record)?;
        }
        Ok(registry)
    }
}
impl From<SourceRegistry> for Vec<SourceRecord> {
    fn from(registry: SourceRegistry) -> Self {
        registry.sources.into_values().collect()
    }
}

/// Ordinal rank of an evidence category; larger is preferred. The total order is
/// verified on build, publisher documented, limited sample, cross-edition
/// unverified, inferred, then disputed or stale (the declaration order of
/// [`Confidence`]).
///
/// It is a ranking rule, not a probability, and not permission to drop the
/// lower-ranked evidence.
pub const fn precedence_rank(confidence: Confidence) -> u8 {
    match confidence {
        Confidence::VerifiedOnBuild => 5,
        Confidence::PublisherDocumented => 4,
        Confidence::LimitedSample => 3,
        Confidence::CrossEditionUnverified => 2,
        Confidence::Inferred => 1,
        Confidence::DisputedOrStale => 0,
    }
}

/// Key of a normalized record. Both variants carry the full `DataContext`, so
/// equal IDs in different editions, builds or namespaces never share a key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordKey {
    Item(ItemKey),
    Quest {
        context: DataContext,
        quest: QuestId,
    },
}
impl RecordKey {
    pub fn context(&self) -> &DataContext {
        match self {
            Self::Item(key) => &key.context,
            Self::Quest { context, .. } => context,
        }
    }
}

/// The reference record a piece of evidence supports. Items and quests only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordPayload {
    Item(Box<ItemDefinition>),
    Quest(Box<QuestDefinition>),
}
impl RecordPayload {
    pub fn key(&self) -> RecordKey {
        match self {
            Self::Item(item) => RecordKey::Item(item.key.clone()),
            Self::Quest(quest) => RecordKey::Quest {
                context: quest.context.clone(),
                quest: quest.id.clone(),
            },
        }
    }
    fn cited_evidence(&self) -> &[EvidenceId] {
        match self {
            Self::Item(item) => &item.evidence,
            Self::Quest(quest) => &quest.evidence,
        }
    }
    /// Equality of the described record, ignoring which evidence IDs it cites.
    fn same_content(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Item(a), Self::Item(b)) => {
                let strip = |d: &ItemDefinitionData| ItemDefinitionData {
                    evidence: Vec::new(),
                    ..d.clone()
                };
                strip(a) == strip(b)
            }
            (Self::Quest(a), Self::Quest(b)) => {
                let strip = |d: &QuestDefinitionData| QuestDefinitionData {
                    evidence: Vec::new(),
                    ..d.clone()
                };
                strip(a) == strip(b)
            }
            _ => false,
        }
    }
}

/// One piece of evidence for one record, with its origin. `origin` is the context
/// the data was originally produced for; it is local to this module because
/// `EvidenceRecord` carries no origin edition and must not be extended.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateData {
    pub payload: RecordPayload,
    pub origin: DataContext,
    pub evidence: EvidenceRecord,
}
pub type Candidate = Validated<CandidateData>;
impl CandidateData {
    /// A record filed under another edition than the one it came from, whose
    /// evidence claims more than `CrossEditionUnverified`. Changing the edition
    /// string does not create build-specific evidence (plan 8.5).
    pub fn is_relabelled(&self) -> bool {
        self.origin.build.edition != self.payload.key().context().build.edition
            && precedence_rank(self.evidence.confidence)
                > precedence_rank(Confidence::CrossEditionUnverified)
    }
    pub fn effective_confidence(&self) -> Confidence {
        if self.is_relabelled() {
            Confidence::CrossEditionUnverified
        } else {
            self.evidence.confidence
        }
    }
}
impl Validate for CandidateData {
    fn validate(&self) -> Result<()> {
        let key = self.payload.key();
        if !self.payload.cited_evidence().contains(&self.evidence.id) {
            return Err(invalid("Record does not cite the evidence supporting it"));
        }
        key.context().ensure_compatible(&self.evidence.context)?;
        if self.origin.build.edition == key.context().build.edition {
            // Same edition: the origin must be the exact build and namespace.
            self.origin.ensure_compatible(key.context())?;
        } else if self.origin.namespace != key.context().namespace {
            // Cross-edition is allowed; mixing demo and reference never is.
            return Err(AppError::new(
                ErrorCode::BuildMismatch,
                "Demo and reference data never mix",
                false,
            ));
        }
        Ok(())
    }
}

/// A candidate after ingestion: the evidence, its origin and the category used
/// for precedence. `relabelled` records are demoted and never rank as verified.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NormalizedRecord {
    pub origin: DataContext,
    pub effective_confidence: Confidence,
    pub relabelled: bool,
    pub evidence: EvidenceRecord,
    pub payload: RecordPayload,
}
impl NormalizedRecord {
    fn from_candidate(candidate: Candidate) -> Self {
        let relabelled = candidate.is_relabelled();
        let effective_confidence = candidate.effective_confidence();
        let CandidateData {
            payload,
            origin,
            evidence,
        } = candidate.into_inner();
        Self {
            origin,
            effective_confidence,
            relabelled,
            evidence,
            payload,
        }
    }
}
/// `Less` means `a` takes precedence: higher effective category first, then the
/// lower `EvidenceId`, then the lower `SourceId`. Total and input-order independent.
pub fn compare_precedence(a: &NormalizedRecord, b: &NormalizedRecord) -> Ordering {
    precedence_rank(b.effective_confidence)
        .cmp(&precedence_rank(a.effective_confidence))
        .then_with(|| a.evidence.id.cmp(&b.evidence.id))
        .then_with(|| a.evidence.source.cmp(&b.evidence.source))
}

/// What kind of disagreement was classified. Never resolved by last writer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKind {
    /// Records under one key differ and the preferred one does not outrank all
    /// of them by category, so precedence rests on the identifier tie-break.
    ValueDisagreement,
    /// Records under one key differ and every disagreeing record is in a
    /// strictly lower category than the preferred one.
    CategoryDowngrade,
    /// Evidence from another edition claims more than `CrossEditionUnverified`
    /// for a record filed under this edition.
    CrossEditionRelabel,
}
/// A classified disagreement. For `CrossEditionRelabel`, `preferred` is the best
/// non-relabelled record (or the best record when all are relabelled) and
/// `others` lists the relabelled evidence, which can be empty when the
/// relabelled record is the only one. Plain data so storage can persist it later.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceConflict {
    pub key: RecordKey,
    pub kind: ConflictKind,
    pub preferred: EvidenceId,
    pub others: Vec<EvidenceId>,
}

/// All evidence for one key, best first. `records` always holds every distinct
/// evidence record that was ingested for the key.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NormalizedEntry {
    pub key: RecordKey,
    pub preferred: EvidenceId,
    pub records: Vec<NormalizedRecord>,
}

/// Deterministic normalized state: entries sorted by key, conflicts sorted.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct NormalizedState {
    entries: Vec<NormalizedEntry>,
    conflicts: Vec<SourceConflict>,
}
impl NormalizedState {
    pub fn entries(&self) -> &[NormalizedEntry] {
        &self.entries
    }
    pub fn conflicts(&self) -> &[SourceConflict] {
        &self.conflicts
    }
    pub fn entry(&self, key: &RecordKey) -> Option<&NormalizedEntry> {
        self.entries
            .binary_search_by(|entry| entry.key.cmp(key))
            .ok()
            .map(|index| &self.entries[index])
    }
    /// Entries of exactly one context; other contexts are never included.
    pub fn entries_in_context<'a>(
        &'a self,
        context: &'a DataContext,
    ) -> impl Iterator<Item = &'a NormalizedEntry> + 'a {
        self.entries
            .iter()
            .filter(move |entry| entry.key.context() == context)
    }
    pub fn conflicts_for<'a>(
        &'a self,
        key: &'a RecordKey,
    ) -> impl Iterator<Item = &'a SourceConflict> + 'a {
        self.conflicts
            .iter()
            .filter(move |conflict| &conflict.key == key)
    }

    /// Adds candidates to this state and returns the new state; `self` is
    /// unchanged and an error leaves nothing half-applied. Every source must be
    /// registered and, when its edition coverage is known, cover the origin
    /// edition. An identical evidence record is a no-op (idempotent); an evidence
    /// ID reused with different content is an error, never an overwrite. More
    /// than 1024 candidates or total evidence records is rejected.
    pub fn ingest(
        &self,
        registry: &SourceRegistry,
        candidates: impl IntoIterator<Item = Candidate>,
    ) -> Result<Self> {
        let mut records: BTreeMap<(RecordKey, EvidenceId), NormalizedRecord> = self
            .entries
            .iter()
            .flat_map(|entry| entry.records.iter().cloned())
            .map(|record| ((record.payload.key(), record.evidence.id.clone()), record))
            .collect();
        let mut seen = 0usize;
        for candidate in candidates {
            seen += 1;
            bounded(seen)?;
            let source = registry.ensure_registered(&candidate.evidence.source)?;
            if let Some(editions) = source.editions.value()
                && !editions.contains(&candidate.origin.build.edition)
            {
                return Err(AppError::new(
                    ErrorCode::BuildMismatch,
                    "Source does not cover the origin edition of its evidence",
                    false,
                ));
            }
            let record = NormalizedRecord::from_candidate(candidate);
            if records.values().any(|existing| {
                existing.evidence.id == record.evidence.id && existing.evidence != record.evidence
            }) {
                return Err(invalid(
                    "Evidence identifier reused with different evidence content",
                ));
            }
            let key = (record.payload.key(), record.evidence.id.clone());
            match records.get(&key) {
                Some(existing) if *existing == record => {}
                Some(_) => {
                    return Err(invalid("Evidence identifier reused with different content"));
                }
                None => {
                    records.insert(key, record);
                    bounded(records.len())?;
                }
            }
        }
        for record in records.values() {
            registry.ensure_registered(&record.evidence.source)?;
        }
        Ok(Self::build(records.into_values()))
    }

    fn build(records: impl IntoIterator<Item = NormalizedRecord>) -> Self {
        let mut grouped: BTreeMap<RecordKey, Vec<NormalizedRecord>> = BTreeMap::new();
        for record in records {
            grouped
                .entry(record.payload.key())
                .or_default()
                .push(record);
        }
        let mut entries = Vec::with_capacity(grouped.len());
        let mut conflicts = Vec::new();
        for (key, mut group) in grouped {
            group.sort_by(compare_precedence);
            conflicts.extend(classify(&key, &group));
            entries.push(NormalizedEntry {
                key,
                preferred: group[0].evidence.id.clone(),
                records: group,
            });
        }
        conflicts.sort();
        Self { entries, conflicts }
    }
}

/// `group` is non-empty and sorted by `compare_precedence`.
fn classify(key: &RecordKey, group: &[NormalizedRecord]) -> Vec<SourceConflict> {
    let ids = |records: &[&NormalizedRecord]| -> Vec<EvidenceId> {
        records.iter().map(|r| r.evidence.id.clone()).collect()
    };
    let mut found = Vec::new();
    let relabelled: Vec<&NormalizedRecord> = group.iter().filter(|r| r.relabelled).collect();
    if !relabelled.is_empty() {
        let best = group.iter().find(|r| !r.relabelled).unwrap_or(&group[0]);
        let others: Vec<&NormalizedRecord> = relabelled
            .into_iter()
            .filter(|r| r.evidence.id != best.evidence.id)
            .collect();
        found.push(SourceConflict {
            key: key.clone(),
            kind: ConflictKind::CrossEditionRelabel,
            preferred: best.evidence.id.clone(),
            others: ids(&others),
        });
    }
    let preferred = &group[0];
    let differing: Vec<&NormalizedRecord> = group[1..]
        .iter()
        .filter(|r| !r.payload.same_content(&preferred.payload))
        .collect();
    if !differing.is_empty() {
        let kind = if differing.iter().all(|r| {
            precedence_rank(r.effective_confidence)
                < precedence_rank(preferred.effective_confidence)
        }) {
            ConflictKind::CategoryDowngrade
        } else {
            ConflictKind::ValueDisagreement
        };
        found.push(SourceConflict {
            key: key.clone(),
            kind,
            preferred: preferred.evidence.id.clone(),
            others: ids(&differing),
        });
    }
    found
}
