//! Accepted-origin policy (ADR 0007 §3 and §5, task V2.8e).
//!
//! `release/v1/accepted-origins.json` names, per release domain, the origin
//! editions whose data the release gate accepts as content for the target
//! edition. Records keep their true origin, and nothing here rewrites an
//! edition, a confidence or an observation status.
//!
//! Origin rule (integrator decision for V2.8, so storage stays unchanged): a
//! pack source lists its true origin *and* the pack's target edition, for
//! example `editions = ["vanilla_1_12", "wow_forever"]`. A source's origin is
//! every listed edition other than the target edition; a source that lists
//! only the target edition is of Forever origin. Evidence from a source with a
//! non-target origin may claim at most `cross_edition_unverified` or
//! `inferred`; a stronger claim is a relabel and always blocks the release
//! (see [`GateClassification::relabelled_evidence`]).
//!
//! The policy changes only how the gate classifies coverage findings:
//!
//! - Unverified evidence (`cross_edition_unverified`, or `inferred` when
//!   `accept_inferred` is set) from a source whose editions are accepted for the
//!   record's domain becomes an *accepted origin* instead of a gap.
//!   `disputed_or_stale`, `limited_sample`, conflicting or unregistered evidence
//!   stays a gap.
//! - When `accept_unknowns` is set, unknown, unsupported and restricted
//!   observations (and positions that cannot be placed because their imagery is
//!   unknown) become *accepted unknowns*. They are counted and listed; they are
//!   never coerced to zero, false or empty, and `not_applicable` stays distinct.
//!
//! Every other finding remains a gap. The module is pure: it performs no I/O.
use crate::{
    content_v2::ReferenceContent,
    coverage::{CoverageFinding, FieldStatus, ProjectionGapKind, RecordRef, content_report},
    entities::{Confidence, EvidenceRecord},
    release::DOMAINS,
    sources::SourceRecord,
    types::{EditionId, EvidenceId, SourceId},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const FORMAT: &str = "navzeroth-accepted-origins";
pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_POLICY_BYTES: usize = 64 * 1024;
/// Findings listed per category; counts are always complete.
pub const MAX_LISTED: usize = 1000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OriginPolicy {
    pub format: String,
    pub schema_version: u32,
    /// The decision that authorizes the policy, for example `ADR 0007 §3`.
    pub decision: String,
    /// The edition the release ships for; accepted origins are other editions.
    pub target_edition: EditionId,
    /// Release domain to accepted origin editions.
    pub domains: BTreeMap<String, Vec<EditionId>>,
    pub accept_inferred: bool,
    pub accept_unknowns: bool,
    /// Conflict precedence (ADR 0007 §4), recorded for the conflict report.
    pub precedence: Vec<String>,
}

fn text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

impl OriginPolicy {
    /// Strict decoding: unknown fields, duplicate members, unknown domains,
    /// empty or duplicate edition lists and a listed target edition are refused.
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        let value = crate::strict_json::value(bytes, MAX_POLICY_BYTES)
            .map_err(|_| "Invalid accepted-origin policy")?;
        let policy: Self =
            serde_json::from_value(value).map_err(|_| "Invalid accepted-origin policy")?;
        policy.validate()?;
        Ok(policy)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.format != FORMAT || self.schema_version != SCHEMA_VERSION {
            return Err("Unsupported accepted-origin policy version");
        }
        if !text(&self.decision) {
            return Err("Accepted-origin policy must name its decision");
        }
        for (domain, editions) in &self.domains {
            let unique: BTreeSet<_> = editions.iter().collect();
            if !DOMAINS.contains(&domain.as_str())
                || editions.is_empty()
                || unique.len() != editions.len()
                || unique.contains(&self.target_edition)
            {
                return Err("Accepted-origin policy domains or editions are invalid");
            }
        }
        let precedence: BTreeSet<_> = self.precedence.iter().collect();
        if self.precedence.is_empty()
            || self.precedence.len() > 64
            || precedence.len() != self.precedence.len()
            || !self.precedence.iter().all(|p| text(p))
        {
            return Err("Accepted-origin policy precedence is invalid");
        }
        Ok(())
    }

    /// True when `edition` is an accepted origin for `domain`.
    pub fn accepts_origin(&self, domain: &str, edition: &str) -> bool {
        self.domains
            .get(domain)
            .is_some_and(|editions| editions.iter().any(|e| e.as_str() == edition))
    }

    /// Accepted origins named in at least one domain.
    pub fn origins(&self) -> BTreeSet<&str> {
        self.domains
            .values()
            .flatten()
            .map(EditionId::as_str)
            .collect()
    }

    /// A source's origin editions (its editions other than the target) are all
    /// accepted for `domain`; `foreign` additionally requires at least one such
    /// origin edition. A source listing only the target edition is of target
    /// origin.
    fn source_accepted(&self, domain: &str, source: &SourceRecord, foreign: bool) -> bool {
        let Some(editions) = source.editions.value() else {
            return false;
        };
        let accepted = |e: &EditionId| self.accepts_origin(domain, e.as_str());
        !editions.is_empty()
            && editions
                .iter()
                .all(|e| *e == self.target_edition || accepted(e))
            && (!foreign || editions.iter().any(accepted))
    }

    /// Whether one evidence record from `source` is acceptable for `domain`
    /// although it is not verified on build or publisher documented.
    pub fn evidence_accepted(
        &self,
        domain: &str,
        evidence: &EvidenceRecord,
        source: Option<&SourceRecord>,
    ) -> bool {
        let Some(source) = source else {
            return false;
        };
        if !evidence.conflicts.is_empty()
            || evidence.context.build.edition != self.target_edition
            || source.id != evidence.source
        {
            return false;
        }
        match evidence.confidence {
            Confidence::CrossEditionUnverified => self.source_accepted(domain, source, true),
            Confidence::Inferred => {
                self.accept_inferred && self.source_accepted(domain, source, false)
            }
            Confidence::VerifiedOnBuild
            | Confidence::PublisherDocumented
            | Confidence::LimitedSample
            | Confidence::DisputedOrStale => false,
        }
    }
}

/// The release domain a reference record belongs to; evidence records have none.
pub fn record_domain(record: &RecordRef) -> Option<&'static str> {
    Some(match record {
        RecordRef::Item { .. } => "equipment",
        RecordRef::Acquisition { .. } => "acquisition",
        RecordRef::Quest { .. } | RecordRef::Objective { .. } => "quests",
        RecordRef::MapSpace { .. }
        | RecordRef::MapImage { .. }
        | RecordRef::MapEmbedding { .. } => "geography",
        RecordRef::Node { .. } | RecordRef::Edge { .. } => "travel",
        RecordRef::Rule { .. } => "reminders",
        RecordRef::Evidence { .. } => return None,
    })
}

/// An unknown, unsupported or restricted observation in a content-v2
/// extension collection, by JSON pointer.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct UnavailableField {
    pub path: String,
    pub status: String,
}

/// Count every declared unavailable observation (`unknown`, `unsupported`,
/// `restricted`) outside the reference dataset, listing the first
/// [`MAX_LISTED`]. False, zero, empty and `not_applicable` are not counted.
pub fn extension_unavailable(
    content: &ReferenceContent,
) -> Result<(u64, Vec<UnavailableField>), &'static str> {
    fn walk(value: &Value, path: &mut String, count: &mut u64, listed: &mut Vec<UnavailableField>) {
        match value {
            Value::Object(map) => {
                if let Some(status) = map.get("status").and_then(Value::as_str)
                    && matches!(status, "unknown" | "unsupported" | "restricted")
                {
                    *count += 1;
                    if listed.len() < MAX_LISTED {
                        listed.push(UnavailableField {
                            path: path.clone(),
                            status: status.to_owned(),
                        });
                    }
                }
                for (key, child) in map {
                    let length = path.len();
                    path.push('/');
                    path.push_str(&key.replace('~', "~0").replace('/', "~1"));
                    walk(child, path, count, listed);
                    path.truncate(length);
                }
            }
            Value::Array(values) => {
                for (index, child) in values.iter().enumerate() {
                    let length = path.len();
                    path.push('/');
                    path.push_str(&index.to_string());
                    walk(child, path, count, listed);
                    path.truncate(length);
                }
            }
            _ => {}
        }
    }
    let mut extensions = serde_json::to_value(content).map_err(|_| "Invalid v2 content")?;
    extensions
        .as_object_mut()
        .ok_or("Invalid v2 content")?
        .remove("reference");
    let (mut count, mut listed) = (0, vec![]);
    walk(&extensions, &mut String::new(), &mut count, &mut listed);
    Ok((count, listed))
}

/// One accepted unknown: a reference coverage finding or an extension field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AcceptedUnknown {
    Finding { finding: Box<CoverageFinding> },
    Extension { field: UnavailableField },
}

/// How the gate classifies one catalog's coverage findings under a policy.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct GateClassification {
    /// Unverified-evidence findings accepted through an accepted origin.
    pub accepted_origins: u64,
    /// Unknown, unsupported and restricted observations accepted as unknowns.
    pub accepted_unknowns: u64,
    /// Findings the policy does not accept; the release requires zero.
    pub remaining_gaps: u64,
    /// Sources whose evidence was accepted through their origin.
    pub accepted_origin_sources: BTreeSet<SourceId>,
    pub listed_accepted_origins: Vec<CoverageFinding>,
    pub listed_accepted_unknowns: Vec<AcceptedUnknown>,
    pub listed_remaining_gaps: Vec<CoverageFinding>,
    /// Evidence claiming `verified_on_build` or `publisher_documented` although
    /// its source has a non-target origin edition: a relabel. Not part of the
    /// strict gap count; the release requires zero.
    pub relabelled_evidence: u64,
    pub listed_relabelled_evidence: Vec<EvidenceId>,
}
impl GateClassification {
    /// `accepted_origins + accepted_unknowns + remaining_gaps`: equal to the
    /// pack's strict `coverage_gaps` count.
    pub fn total(&self) -> u64 {
        self.accepted_origins + self.accepted_unknowns + self.remaining_gaps
    }
}

fn push<T>(list: &mut Vec<T>, value: T) {
    if list.len() < MAX_LISTED {
        list.push(value);
    }
}

fn unknown_status(status: FieldStatus) -> bool {
    matches!(
        status,
        FieldStatus::Unknown | FieldStatus::Unsupported | FieldStatus::Restricted
    )
}

/// A finding that only says a value is unknown, unsupported or restricted.
fn is_unknown(finding: &CoverageFinding) -> bool {
    match finding {
        CoverageFinding::UnknownMapTransform { status, .. }
        | CoverageFinding::UncalibratedImage { status, .. } => unknown_status(*status),
        // The imagery the position would be placed on is unknown (absent,
        // uncalibrated, or an instance without imagery). Out-of-bounds, floor,
        // build and namespace mismatches are content faults and stay gaps.
        CoverageFinding::UnprojectablePosition { gap, .. } => matches!(
            gap,
            ProjectionGapKind::NoImageForMap
                | ProjectionGapKind::InInstance
                | ProjectionGapKind::UncalibratedImage
        ),
        _ => false,
    }
}

/// Classify the strict coverage findings of one content-v2 catalog. Without a
/// policy, or for a policy of another target edition, every finding remains a
/// gap. Content with graph errors is refused.
pub fn classify(
    content: &ReferenceContent,
    sources: &[SourceRecord],
    policy: Option<&OriginPolicy>,
) -> Result<GateClassification, &'static str> {
    let report = content_report(&content.reference);
    if !report.errors.is_empty() {
        return Err("V2 content graph has errors");
    }
    let (extensions, fields) = extension_unavailable(content)?;
    let policy = policy.filter(|p| p.target_edition == content.reference.game.edition);
    let sources: BTreeMap<&SourceId, &SourceRecord> = sources.iter().map(|s| (&s.id, s)).collect();
    let evidence: BTreeMap<&EvidenceId, &EvidenceRecord> = content
        .reference
        .payload
        .evidence
        .iter()
        .map(|e| (&e.id, e))
        .collect();
    let mut result = GateClassification::default();
    let target = &content.reference.game.edition;
    for e in &content.reference.payload.evidence {
        let foreign = sources
            .get(&e.source)
            .and_then(|s| s.editions.value())
            .is_some_and(|editions| editions.iter().any(|edition| edition != target));
        if foreign
            && matches!(
                e.confidence,
                Confidence::VerifiedOnBuild | Confidence::PublisherDocumented
            )
        {
            result.relabelled_evidence += 1;
            push(&mut result.listed_relabelled_evidence, e.id.clone());
        }
    }
    for finding in report.gaps {
        let accepted_origin = match (&finding, policy) {
            (
                CoverageFinding::UnverifiedEvidence {
                    record,
                    evidence: ids,
                },
                Some(policy),
            ) => record_domain(record).and_then(|domain| {
                let cited: Option<Vec<&EvidenceRecord>> =
                    ids.iter().map(|id| evidence.get(id).copied()).collect();
                let cited = cited.filter(|c| !c.is_empty())?;
                cited
                    .iter()
                    .all(|e| policy.evidence_accepted(domain, e, sources.get(&e.source).copied()))
                    .then(|| cited.iter().map(|e| e.source.clone()).collect::<Vec<_>>())
            }),
            _ => None,
        };
        if let Some(cited) = accepted_origin {
            result.accepted_origins += 1;
            result.accepted_origin_sources.extend(cited);
            push(&mut result.listed_accepted_origins, finding);
        } else if policy.is_some_and(|p| p.accept_unknowns) && is_unknown(&finding) {
            result.accepted_unknowns += 1;
            push(
                &mut result.listed_accepted_unknowns,
                AcceptedUnknown::Finding {
                    finding: Box::new(finding),
                },
            );
        } else {
            result.remaining_gaps += 1;
            push(&mut result.listed_remaining_gaps, finding);
        }
    }
    if policy.is_some_and(|p| p.accept_unknowns) {
        result.accepted_unknowns += extensions;
        for field in fields {
            push(
                &mut result.listed_accepted_unknowns,
                AcceptedUnknown::Extension { field },
            );
        }
    } else {
        result.remaining_gaps += extensions;
    }
    Ok(result)
}
