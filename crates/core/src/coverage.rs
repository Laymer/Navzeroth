//! Coverage and content validation report for one reference dataset (M3.5).
//! Counts records, evidence links and observation statuses per collection,
//! resolves predicate references and finds quest prerequisite cycles. It never
//! ranks evidence, computes a probability, reads a clock or does I/O, and the
//! report carries identifiers only, never imported free text.
//!
//! Quest cycles use positive references in the `prerequisites` and `eligibility`
//! predicates: a `quest_state` (or `equals` on a quest field) that must hold,
//! counted after cancelling `not` pairs. A positive objective reference adds an
//! edge to each quest whose stages own that objective (unconditional only when
//! exactly one quest owns it). A reference that is optional (inside `any`, or inside
//! `all` under an odd number of `not`) is conditional. Cycles made of
//! unconditional edges are errors. A cycle that needs a conditional edge is
//! reported as a possible cycle and is a gap. A predicate objective reference
//! resolves against quest stages and `Objective.id` alike. A predicate quest
//! reference to a quest outside the dataset is an error: cross-dataset
//! references are not modelled.
use crate::{
    atlas::{ProjectionGap, ProjectionIndex},
    contract::{Envelope, Payload, ReferenceDataset, decode_envelope},
    entities::{Confidence, EdgeStatus},
    error::ErrorCode,
    observation::{Observation, ObservationStatus},
    predicate::{Field, Predicate, PredicateNode, Scalar},
    types::*,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Version of the report shape; bumped when a field changes meaning.
pub const REPORT_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ReportOutcome {
    /// The dataset decoded; coverage and findings follow.
    Decoded,
    /// The dataset did not decode. Only the code and the static message are
    /// kept; imported content is never echoed.
    DecodeFailed { code: ErrorCode, message: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageClass {
    /// Demo namespace: synthetic fixture coverage, never Forever coverage.
    Demo,
    /// Reference namespace flagged as a fixture.
    ReferenceFixture,
    Reference,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DatasetHeader {
    pub revision: DatasetRevision,
    pub namespace: DatasetNamespace,
    pub fixture: bool,
    pub edition: EditionId,
    pub build: BuildId,
    pub interface: InterfaceId,
    pub coverage_class: CoverageClass,
    /// True only for a non-fixture reference-namespace dataset of edition
    /// `wow_forever`. It says nothing about verification; see the confidence
    /// counts for that.
    pub forever_reference_content: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Collection {
    Evidence,
    Items,
    Acquisitions,
    Quests,
    MapSpaces,
    Nodes,
    Edges,
    Objectives,
    Rules,
    MapImages,
    MapEmbeddings,
}

/// Counts by observation status. Unknown, unsupported, restricted and
/// not-applicable stay separate from each other and from known values.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct StatusCounts {
    pub known: usize,
    pub unknown: usize,
    pub unsupported: usize,
    pub restricted: usize,
    pub not_applicable: usize,
}
impl StatusCounts {
    fn add(&mut self, status: ObservationStatus) {
        match status {
            ObservationStatus::Known => self.known += 1,
            ObservationStatus::Unknown => self.unknown += 1,
            ObservationStatus::Unsupported => self.unsupported += 1,
            ObservationStatus::Restricted => self.restricted += 1,
            ObservationStatus::NotApplicable => self.not_applicable += 1,
        }
    }
}

/// Counts over the six descriptive categories, zeros included. No order or
/// precedence between categories is implied.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ConfidenceCounts {
    pub verified_on_build: usize,
    pub publisher_documented: usize,
    pub limited_sample: usize,
    pub cross_edition_unverified: usize,
    pub inferred: usize,
    pub disputed_or_stale: usize,
}
impl ConfidenceCounts {
    fn add(&mut self, confidence: Confidence) {
        match confidence {
            Confidence::VerifiedOnBuild => self.verified_on_build += 1,
            Confidence::PublisherDocumented => self.publisher_documented += 1,
            Confidence::LimitedSample => self.limited_sample += 1,
            Confidence::CrossEditionUnverified => self.cross_edition_unverified += 1,
            Confidence::Inferred => self.inferred += 1,
            Confidence::DisputedOrStale => self.disputed_or_stale += 1,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct EdgeStatusCounts {
    pub verified: usize,
    pub estimated: usize,
    pub temporarily_unavailable: usize,
    pub unverified: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct EvidenceLinkCoverage {
    pub records_with_evidence: usize,
    pub records_without_evidence: usize,
    /// Each evidence reference of each record counts once, by the confidence of
    /// the referenced evidence record.
    pub links_by_confidence: ConfidenceCounts,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CollectionCoverage {
    pub total: usize,
    /// Absent for collections whose records carry no evidence list.
    pub evidence: Option<EvidenceLinkCoverage>,
    /// Observation-typed fields by status.
    pub fields: BTreeMap<&'static str, StatusCounts>,
}
impl CollectionCoverage {
    fn new(has_evidence: bool, fields: &[&'static str]) -> Self {
        Self {
            total: 0,
            evidence: has_evidence.then(EvidenceLinkCoverage::default),
            fields: fields
                .iter()
                .map(|name| (*name, StatusCounts::default()))
                .collect(),
        }
    }
    fn field<T>(&mut self, name: &'static str, value: &Observation<T>) {
        self.fields.entry(name).or_default().add(value.status());
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct EvidenceRecordCoverage {
    pub by_confidence: ConfidenceCounts,
    pub records_with_conflicts: usize,
    /// Recorded conflict references; no conflict model is applied here.
    pub conflict_links: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct QuestGraphCoverage {
    pub quests: usize,
    pub unconditional_edges: usize,
    pub conditional_edges: usize,
}

/// Projection gap variants, for tallies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionGapKind {
    NamespaceMismatch,
    NoImageForMap,
    InInstance,
    DifferentFloor,
    BuildMismatch,
    EmbeddingChainTooDeep,
    UncalibratedImage,
    OutOfBounds,
}
impl From<&ProjectionGap> for ProjectionGapKind {
    fn from(gap: &ProjectionGap) -> Self {
        match gap {
            ProjectionGap::NamespaceMismatch => Self::NamespaceMismatch,
            ProjectionGap::NoImageForMap { .. } => Self::NoImageForMap,
            ProjectionGap::InInstance { .. } => Self::InInstance,
            ProjectionGap::DifferentFloor { .. } => Self::DifferentFloor,
            ProjectionGap::BuildMismatch { .. } => Self::BuildMismatch,
            ProjectionGap::EmbeddingChainTooDeep { .. } => Self::EmbeddingChainTooDeep,
            ProjectionGap::UncalibratedImage { .. } => Self::UncalibratedImage,
            ProjectionGap::OutOfBounds { .. } => Self::OutOfBounds,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct AtlasCoverage {
    /// Travel node positions and known quest locations tried on the imagery.
    pub positions_checked: usize,
    pub projected: usize,
    pub gaps: BTreeMap<ProjectionGapKind, usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DatasetCoverage {
    pub collections: BTreeMap<Collection, CollectionCoverage>,
    pub evidence_records: EvidenceRecordCoverage,
    pub edge_status: EdgeStatusCounts,
    pub quest_graph: QuestGraphCoverage,
    pub atlas: AtlasCoverage,
}

/// The record a finding refers to, qualified by the key the dataset uses.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecordRef {
    Evidence { id: EvidenceId },
    Item { key: ItemKey },
    Acquisition { id: AcquisitionId },
    Quest { id: QuestId },
    MapSpace { key: MapKey },
    Node { id: NodeId },
    Edge { id: EdgeId },
    Objective { id: ObjectiveId },
    Rule { id: RuleId },
    MapImage { id: ImageId },
    MapEmbedding { child: MapKey },
}

/// The predicate-bearing member of a record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PredicateSite {
    Eligibility,
    Prerequisites,
    Requirements,
    Conditions,
    Predicate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldStatus {
    Known,
    Unknown,
    Unsupported,
    Restricted,
    NotApplicable,
}
impl From<ObservationStatus> for FieldStatus {
    fn from(status: ObservationStatus) -> Self {
        match status {
            ObservationStatus::Known => Self::Known,
            ObservationStatus::Unknown => Self::Unknown,
            ObservationStatus::Unsupported => Self::Unsupported,
            ObservationStatus::Restricted => Self::Restricted,
            ObservationStatus::NotApplicable => Self::NotApplicable,
        }
    }
}

/// A closed finding. Errors fail the report; gaps make missing or unverified
/// content visible without failing it. Codes carry identifiers, never text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum CoverageFinding {
    /// Error: quests that require each other through unconditional edges.
    QuestPrerequisiteCycle {
        quests: Vec<QuestId>,
    },
    DanglingPredicateQuest {
        referrer: RecordRef,
        site: PredicateSite,
        quest: QuestId,
    },
    DanglingPredicateObjective {
        referrer: RecordRef,
        site: PredicateSite,
        objective: ObjectiveId,
    },
    DanglingPredicateItem {
        referrer: RecordRef,
        site: PredicateSite,
        item: ItemKey,
    },
    UndeclaredPredicateAreaMap {
        referrer: RecordRef,
        site: PredicateSite,
        map: MapKey,
    },
    /// Gap: a cycle that exists only if a conditional (`any`) branch is taken.
    PossibleQuestPrerequisiteCycle {
        quests: Vec<QuestId>,
    },
    RecordWithoutEvidence {
        record: RecordRef,
    },
    /// Gap: the record has evidence, but none of it is `verified_on_build` or
    /// `publisher_documented`.
    UnverifiedEvidence {
        record: RecordRef,
        evidence: Vec<EvidenceId>,
    },
    UnknownMapTransform {
        map: MapKey,
        status: FieldStatus,
    },
    UncalibratedImage {
        image: ImageId,
        status: FieldStatus,
    },
    UnprojectablePosition {
        record: RecordRef,
        gap: ProjectionGapKind,
    },
}
impl CoverageFinding {
    pub fn code(&self) -> &'static str {
        match self {
            Self::QuestPrerequisiteCycle { .. } => "quest_prerequisite_cycle",
            Self::DanglingPredicateQuest { .. } => "dangling_predicate_quest",
            Self::DanglingPredicateObjective { .. } => "dangling_predicate_objective",
            Self::DanglingPredicateItem { .. } => "dangling_predicate_item",
            Self::UndeclaredPredicateAreaMap { .. } => "undeclared_predicate_area_map",
            Self::PossibleQuestPrerequisiteCycle { .. } => "possible_quest_prerequisite_cycle",
            Self::RecordWithoutEvidence { .. } => "record_without_evidence",
            Self::UnverifiedEvidence { .. } => "unverified_evidence",
            Self::UnknownMapTransform { .. } => "unknown_map_transform",
            Self::UncalibratedImage { .. } => "uncalibrated_image",
            Self::UnprojectablePosition { .. } => "unprojectable_position",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContentReport {
    pub report_version: u32,
    /// True only for a decoded dataset with no errors. Gaps never fail it.
    pub passes: bool,
    pub outcome: ReportOutcome,
    pub dataset: Option<DatasetHeader>,
    pub coverage: Option<DatasetCoverage>,
    pub errors: Vec<CoverageFinding>,
    pub gaps: Vec<CoverageFinding>,
}

/// Decode a reference dataset envelope and report on it. A decode failure
/// (invalid coordinates and structural breakage included) is a failing report
/// that keeps only the error code and its static message.
pub fn content_report_from_json(input: &str) -> ContentReport {
    match decode_envelope::<ReferenceDataset>(input) {
        Ok(envelope) => content_report(&envelope),
        Err(error) => ContentReport {
            report_version: REPORT_VERSION,
            passes: false,
            outcome: ReportOutcome::DecodeFailed {
                code: error.code,
                message: error.to_string(),
            },
            dataset: None,
            coverage: None,
            errors: Vec::new(),
            gaps: Vec::new(),
        },
    }
}

fn is_strong(confidence: Confidence) -> bool {
    matches!(
        confidence,
        Confidence::VerifiedOnBuild | Confidence::PublisherDocumented
    )
}

struct Builder<'a> {
    confidence: BTreeMap<&'a EvidenceId, Confidence>,
    errors: BTreeSet<CoverageFinding>,
    gaps: BTreeSet<CoverageFinding>,
}
impl Builder<'_> {
    /// Count one record and its evidence links; list it when it has none or
    /// none of its evidence is verified or documented.
    fn track(&mut self, c: &mut CollectionCoverage, record: RecordRef, ids: &[EvidenceId]) {
        c.total += 1;
        let Some(link) = c.evidence.as_mut() else {
            return;
        };
        if ids.is_empty() {
            link.records_without_evidence += 1;
            self.gaps
                .insert(CoverageFinding::RecordWithoutEvidence { record });
            return;
        }
        link.records_with_evidence += 1;
        let mut strong = false;
        for id in ids {
            if let Some(confidence) = self.confidence.get(id) {
                link.links_by_confidence.add(*confidence);
                strong |= is_strong(*confidence);
            }
        }
        if !strong {
            let mut evidence = ids.to_vec();
            evidence.sort();
            self.gaps
                .insert(CoverageFinding::UnverifiedEvidence { record, evidence });
        }
    }
}

enum Found<'a> {
    Quest {
        quest: &'a QuestId,
        /// The quest must reach the state for the predicate to hold.
        positive: bool,
        /// Only some branches need it.
        conditional: bool,
    },
    Objective {
        objective: &'a ObjectiveId,
        positive: bool,
        conditional: bool,
    },
    Item(&'a ItemKey),
    Area(&'a MapKey),
}

fn field_found<'a>(
    field: &'a Field,
    positive_if: Option<bool>,
    conditional: bool,
) -> Vec<Found<'a>> {
    match field {
        Field::Quest { quest, .. } => vec![Found::Quest {
            quest,
            positive: positive_if == Some(true),
            conditional,
        }],
        Field::Objective { objective } => vec![Found::Objective {
            objective,
            positive: positive_if == Some(true),
            conditional,
        }],
        _ => Vec::new(),
    }
}

/// Visit every reference in a predicate with an explicit stack. `negated` is
/// the parity of enclosing `not` nodes. De Morgan makes `all` optional under
/// negation and `any` optional otherwise.
fn references(root: &PredicateNode) -> Vec<Found<'_>> {
    let mut found = Vec::new();
    let mut stack = vec![(root, false, false)];
    while let Some((node, negated, conditional)) = stack.pop() {
        match node {
            PredicateNode::All { predicates } => {
                let conditional = conditional || negated;
                stack.extend(predicates.iter().map(|p| (p, negated, conditional)));
            }
            PredicateNode::Any { predicates } => {
                let conditional = conditional || !negated;
                stack.extend(predicates.iter().map(|p| (p, negated, conditional)));
            }
            PredicateNode::Not { predicate } => stack.push((predicate, !negated, conditional)),
            PredicateNode::ParticipantState { predicate, .. } => {
                stack.push((predicate, negated, conditional))
            }
            PredicateNode::Known { field } => found.extend(field_found(field, None, conditional)),
            PredicateNode::Equals { field, value } => {
                let wanted = match value {
                    Scalar::Bool(b) => Some(*b != negated),
                    _ => None,
                };
                found.extend(field_found(field, wanted, conditional));
            }
            PredicateNode::GreaterThanOrEqual { field, .. } => {
                found.extend(field_found(field, None, conditional))
            }
            PredicateNode::QuestState { quest, value, .. } => found.push(Found::Quest {
                quest,
                positive: *value != negated,
                conditional,
            }),
            PredicateNode::ObjectiveState {
                objective,
                completed,
            } => found.push(Found::Objective {
                objective,
                positive: *completed != negated,
                conditional,
            }),
            PredicateNode::EquipmentMatches { item, .. } => found.push(Found::Item(item)),
            PredicateNode::LocationInSupportedArea { area } => found.push(Found::Area(&area.map)),
            PredicateNode::LevelAtLeast { .. }
            | PredicateNode::RoleIs { .. }
            | PredicateNode::UserPreference { .. } => {}
        }
    }
    found
}

/// Strongly connected components that contain a cycle (more than one member,
/// or a self-loop), each sorted. Tarjan with an explicit stack, no recursion.
fn cyclic_components(adjacency: &[BTreeSet<usize>]) -> Vec<Vec<usize>> {
    let n = adjacency.len();
    let edges: Vec<Vec<usize>> = adjacency
        .iter()
        .map(|set| set.iter().copied().collect())
        .collect();
    let mut index: Vec<Option<usize>> = vec![None; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut stack = Vec::new();
    let mut counter = 0;
    let mut components = Vec::new();
    for root in 0..n {
        if index[root].is_some() {
            continue;
        }
        index[root] = Some(counter);
        low[root] = counter;
        counter += 1;
        stack.push(root);
        on_stack[root] = true;
        let mut work = vec![(root, 0usize)];
        while let Some(&(v, next)) = work.last() {
            if next < edges[v].len() {
                if let Some(top) = work.last_mut() {
                    top.1 += 1;
                }
                let w = edges[v][next];
                match index[w] {
                    None => {
                        index[w] = Some(counter);
                        low[w] = counter;
                        counter += 1;
                        stack.push(w);
                        on_stack[w] = true;
                        work.push((w, 0));
                    }
                    Some(order) if on_stack[w] => low[v] = low[v].min(order),
                    Some(_) => {}
                }
            } else {
                work.pop();
                if let Some(&(parent, _)) = work.last() {
                    low[parent] = low[parent].min(low[v]);
                }
                if Some(low[v]) == index[v] {
                    let mut component = Vec::new();
                    while let Some(w) = stack.pop() {
                        on_stack[w] = false;
                        component.push(w);
                        if w == v {
                            break;
                        }
                    }
                    component.sort_unstable();
                    if component.len() > 1 || edges[v].contains(&v) {
                        components.push(component);
                    }
                }
            }
        }
    }
    components
}

/// Build the report for a decoded dataset. Deterministic: every collection is
/// ordered and nothing reads a clock or random source.
pub fn content_report<P: Payload + AsRef<ReferenceDataset>>(
    envelope: &Envelope<P>,
) -> ContentReport {
    let context = envelope.data_context();
    let dataset = envelope.payload.as_ref();
    let header = DatasetHeader {
        revision: dataset.revision.clone(),
        namespace: envelope.namespace,
        fixture: envelope.fixture,
        edition: envelope.game.edition.clone(),
        build: envelope.game.build.clone(),
        interface: envelope.game.interface.clone(),
        coverage_class: match (envelope.namespace, envelope.fixture) {
            (DatasetNamespace::Demo, _) => CoverageClass::Demo,
            (DatasetNamespace::Reference, true) => CoverageClass::ReferenceFixture,
            (DatasetNamespace::Reference, false) => CoverageClass::Reference,
        },
        forever_reference_content: envelope.namespace == DatasetNamespace::Reference
            && !envelope.fixture
            && envelope.game.edition.as_str() == "wow_forever",
    };
    let mut b = Builder {
        confidence: dataset
            .evidence
            .iter()
            .map(|e| (&e.id, e.confidence))
            .collect(),
        errors: BTreeSet::new(),
        gaps: BTreeSet::new(),
    };

    let mut evidence_records = EvidenceRecordCoverage::default();
    let mut evidence = CollectionCoverage::new(false, &["license", "observed_at_utc_ms"]);
    for e in &dataset.evidence {
        evidence.total += 1;
        evidence.field("license", &e.license);
        evidence.field("observed_at_utc_ms", &e.observed_at_utc_ms);
        evidence_records.by_confidence.add(e.confidence);
        if !e.conflicts.is_empty() {
            evidence_records.records_with_conflicts += 1;
            evidence_records.conflict_links += e.conflicts.len();
        }
    }

    // Predicates to check: (referrer, site, predicate).
    let mut predicates: Vec<(RecordRef, PredicateSite, &Predicate)> = Vec::new();

    let mut items = CollectionCoverage::new(
        true,
        &[
            "two_handed",
            "unique_group",
            "attributes",
            "requirements",
            "effects",
        ],
    );
    for r in &dataset.items {
        let record = RecordRef::Item { key: r.key.clone() };
        b.track(&mut items, record.clone(), &r.evidence);
        items.field("two_handed", &r.two_handed);
        items.field("unique_group", &r.unique_group);
        items.field("attributes", &r.attributes);
        items.field("requirements", &r.requirements);
        items.field("effects", &r.effects);
        if let Some(p) = r.requirements.value() {
            predicates.push((record, PredicateSite::Requirements, p));
        }
    }

    let mut acquisitions = CollectionCoverage::new(true, &["conditions", "cost", "probability"]);
    for r in &dataset.acquisitions {
        let record = RecordRef::Acquisition { id: r.id.clone() };
        b.track(&mut acquisitions, record.clone(), &r.evidence);
        acquisitions.field("conditions", &r.conditions);
        acquisitions.field("cost", &r.cost);
        acquisitions.field("probability", &r.probability);
        if let Some(p) = r.conditions.value() {
            predicates.push((record, PredicateSite::Conditions, p));
        }
    }

    let mut quests = CollectionCoverage::new(
        true,
        &[
            "eligibility",
            "prerequisites",
            "locations",
            "rewards",
            "completion",
            "shared_progress",
        ],
    );
    for r in &dataset.quests {
        let record = RecordRef::Quest { id: r.id.clone() };
        b.track(&mut quests, record.clone(), &r.evidence);
        quests.field("eligibility", &r.eligibility);
        quests.field("prerequisites", &r.prerequisites);
        quests.field("locations", &r.locations);
        quests.field("rewards", &r.rewards);
        quests.field("completion", &r.completion);
        quests.field("shared_progress", &r.shared_progress);
        if let Some(p) = r.eligibility.value() {
            predicates.push((record.clone(), PredicateSite::Eligibility, p));
        }
        if let Some(p) = r.prerequisites.value() {
            predicates.push((record, PredicateSite::Prerequisites, p));
        }
    }

    let mut map_spaces = CollectionCoverage::new(true, &["transform"]);
    for r in &dataset.map_spaces {
        b.track(
            &mut map_spaces,
            RecordRef::MapSpace { key: r.key.clone() },
            &r.evidence,
        );
        map_spaces.field("transform", &r.transform);
        if r.transform.status() != ObservationStatus::Known {
            b.gaps.insert(CoverageFinding::UnknownMapTransform {
                map: r.key.clone(),
                status: r.transform.status().into(),
            });
        }
    }

    let mut nodes = CollectionCoverage::new(false, &["service"]);
    for r in &dataset.nodes {
        nodes.total += 1;
        nodes.field("service", &r.service);
    }

    let mut edges = CollectionCoverage::new(
        true,
        &[
            "requirements",
            "expected_ms",
            "wait_ms",
            "uncertainty",
            "consumes_resource",
            "cooldown_ms",
        ],
    );
    let mut edge_status = EdgeStatusCounts::default();
    for r in &dataset.edges {
        let record = RecordRef::Edge { id: r.id.clone() };
        b.track(&mut edges, record.clone(), &r.evidence);
        edges.field("requirements", &r.requirements);
        edges.field("expected_ms", &r.expected_ms);
        edges.field("wait_ms", &r.wait_ms);
        edges.field("uncertainty", &r.uncertainty);
        edges.field("consumes_resource", &r.consumes_resource);
        edges.field("cooldown_ms", &r.cooldown_ms);
        match r.status {
            EdgeStatus::Verified => edge_status.verified += 1,
            EdgeStatus::Estimated => edge_status.estimated += 1,
            EdgeStatus::TemporarilyUnavailable => edge_status.temporarily_unavailable += 1,
            EdgeStatus::Unverified => edge_status.unverified += 1,
        }
        if let Some(p) = r.requirements.value() {
            predicates.push((record, PredicateSite::Requirements, p));
        }
    }

    let mut objectives = CollectionCoverage::new(true, &["requirements", "expected_ms"]);
    for r in &dataset.objectives {
        let record = RecordRef::Objective { id: r.id.clone() };
        b.track(&mut objectives, record.clone(), &r.evidence);
        objectives.field("requirements", &r.requirements);
        objectives.field("expected_ms", &r.expected_ms);
        if let Some(p) = r.requirements.value() {
            predicates.push((record, PredicateSite::Requirements, p));
        }
    }

    let mut rules = CollectionCoverage::new(true, &[]);
    for r in &dataset.rules {
        let record = RecordRef::Rule { id: r.id.clone() };
        b.track(&mut rules, record.clone(), &r.evidence);
        predicates.push((record, PredicateSite::Predicate, &r.predicate));
    }

    let mut map_images = CollectionCoverage::new(true, &["calibration"]);
    for r in &dataset.map_images {
        b.track(
            &mut map_images,
            RecordRef::MapImage { id: r.id.clone() },
            &r.evidence,
        );
        map_images.field("calibration", &r.calibration);
        if r.calibration.status() != ObservationStatus::Known {
            b.gaps.insert(CoverageFinding::UncalibratedImage {
                image: r.id.clone(),
                status: r.calibration.status().into(),
            });
        }
    }

    let mut map_embeddings = CollectionCoverage::new(true, &[]);
    for r in &dataset.map_embeddings {
        b.track(
            &mut map_embeddings,
            RecordRef::MapEmbedding {
                child: r.child.clone(),
            },
            &r.evidence,
        );
    }

    // Reference resolution. Objective IDs name both quest stages and
    // `Objective` records, so either resolves a predicate reference.
    let quest_ids: BTreeMap<&QuestId, usize> = dataset
        .quests
        .iter()
        .map(|q| &q.id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();
    let objective_ids: BTreeSet<&ObjectiveId> = dataset
        .quests
        .iter()
        .flat_map(|q| &q.stages)
        .chain(dataset.objectives.iter().map(|o| &o.id))
        .collect();
    let item_keys: BTreeSet<&ItemKey> = dataset.items.iter().map(|i| &i.key).collect();
    let map_keys: BTreeSet<&MapKey> = dataset.map_spaces.iter().map(|m| &m.key).collect();

    let mut objective_owners: BTreeMap<&ObjectiveId, Vec<usize>> = BTreeMap::new();
    for quest in &dataset.quests {
        for objective in &quest.stages {
            objective_owners
                .entry(objective)
                .or_default()
                .push(quest_ids[&quest.id]);
        }
    }
    let mut unconditional = vec![BTreeSet::new(); quest_ids.len()];
    let mut all_edges = vec![BTreeSet::new(); quest_ids.len()];
    for (referrer, site, predicate) in predicates {
        for found in references(predicate) {
            match found {
                Found::Quest {
                    quest,
                    positive,
                    conditional,
                } => match quest_ids.get(quest) {
                    None => {
                        b.errors.insert(CoverageFinding::DanglingPredicateQuest {
                            referrer: referrer.clone(),
                            site,
                            quest: quest.clone(),
                        });
                    }
                    Some(&to) => {
                        if positive
                            && matches!(
                                site,
                                PredicateSite::Prerequisites | PredicateSite::Eligibility
                            )
                            && let RecordRef::Quest { id } = &referrer
                            && let Some(&from) = quest_ids.get(id)
                        {
                            all_edges[from].insert(to);
                            if !conditional {
                                unconditional[from].insert(to);
                            }
                        }
                    }
                },
                Found::Objective {
                    objective,
                    positive,
                    conditional,
                } => {
                    if positive
                        && matches!(
                            site,
                            PredicateSite::Prerequisites | PredicateSite::Eligibility
                        )
                        && let RecordRef::Quest { id } = &referrer
                        && let Some(&from) = quest_ids.get(id)
                        && let Some(owners) = objective_owners.get(objective)
                    {
                        for &to in owners {
                            all_edges[from].insert(to);
                            if !conditional && owners.len() == 1 {
                                unconditional[from].insert(to);
                            }
                        }
                    }
                    if !objective_ids.contains(objective) {
                        b.errors
                            .insert(CoverageFinding::DanglingPredicateObjective {
                                referrer: referrer.clone(),
                                site,
                                objective: objective.clone(),
                            });
                    }
                }
                Found::Item(item) => {
                    if !item_keys.contains(item) {
                        b.errors.insert(CoverageFinding::DanglingPredicateItem {
                            referrer: referrer.clone(),
                            site,
                            item: item.clone(),
                        });
                    }
                }
                Found::Area(map) => {
                    if !map_keys.contains(map) {
                        b.errors
                            .insert(CoverageFinding::UndeclaredPredicateAreaMap {
                                referrer: referrer.clone(),
                                site,
                                map: map.clone(),
                            });
                    }
                }
            }
        }
    }

    // `quest_ids` indexes quests in ID order, so sorted indices are sorted IDs.
    let by_index: Vec<&QuestId> = quest_ids.keys().copied().collect();
    let names = |component: &[usize]| -> Vec<QuestId> {
        component.iter().map(|&i| by_index[i].clone()).collect()
    };
    let definite = cyclic_components(&unconditional);
    for component in &definite {
        b.errors.insert(CoverageFinding::QuestPrerequisiteCycle {
            quests: names(component),
        });
    }
    let definite_members: BTreeSet<_> = definite.iter().flat_map(|c| c.iter().copied()).collect();
    for component in cyclic_components(&all_edges) {
        if !component.iter().any(|id| definite_members.contains(id)) {
            b.gaps
                .insert(CoverageFinding::PossibleQuestPrerequisiteCycle {
                    quests: names(&component),
                });
        }
    }
    let quest_graph = QuestGraphCoverage {
        quests: quest_ids.len(),
        unconditional_edges: unconditional.iter().map(BTreeSet::len).sum(),
        conditional_edges: all_edges.iter().map(BTreeSet::len).sum::<usize>()
            - unconditional.iter().map(BTreeSet::len).sum::<usize>(),
    };

    // Atlas coverage: positions that cannot be placed on imagery are gaps.
    let mut atlas = AtlasCoverage::default();
    let projection_index = ProjectionIndex::new(dataset);
    let mut place = |b: &mut Builder, record: RecordRef, position: &Position| {
        atlas.positions_checked += 1;
        match projection_index.project(position, &context) {
            Ok(_) => atlas.projected += 1,
            Err(gap) => {
                let kind = ProjectionGapKind::from(&gap);
                *atlas.gaps.entry(kind).or_default() += 1;
                b.gaps
                    .insert(CoverageFinding::UnprojectablePosition { record, gap: kind });
            }
        }
    };
    for n in &dataset.nodes {
        place(&mut b, RecordRef::Node { id: n.id.clone() }, &n.position);
    }
    for q in &dataset.quests {
        for position in q.locations.value().into_iter().flatten() {
            place(&mut b, RecordRef::Quest { id: q.id.clone() }, position);
        }
    }

    let errors: Vec<_> = b.errors.into_iter().collect();
    ContentReport {
        report_version: REPORT_VERSION,
        passes: errors.is_empty(),
        outcome: ReportOutcome::Decoded,
        dataset: Some(header),
        coverage: Some(DatasetCoverage {
            collections: BTreeMap::from([
                (Collection::Evidence, evidence),
                (Collection::Items, items),
                (Collection::Acquisitions, acquisitions),
                (Collection::Quests, quests),
                (Collection::MapSpaces, map_spaces),
                (Collection::Nodes, nodes),
                (Collection::Edges, edges),
                (Collection::Objectives, objectives),
                (Collection::Rules, rules),
                (Collection::MapImages, map_images),
                (Collection::MapEmbeddings, map_embeddings),
            ]),
            evidence_records,
            edge_status,
            quest_graph,
            atlas,
        }),
        errors,
        gaps: b.gaps.into_iter().collect(),
    }
}

#[cfg(test)]
mod capacity_tests {
    use super::*;
    #[test]
    fn long_prerequisite_chains_and_cycles_use_an_explicit_stack() {
        let mut graph = vec![BTreeSet::new(); 50_000];
        for (i, edges) in graph.iter_mut().enumerate().skip(1) {
            edges.insert(i - 1);
        }
        assert!(cyclic_components(&graph).is_empty());
        let last = graph.len() - 1;
        graph[0].insert(last);
        let cycles = cyclic_components(&graph);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 50_000);
    }
}
