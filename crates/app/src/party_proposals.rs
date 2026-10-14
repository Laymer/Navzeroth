//! Explicit local agreement declarations, never authenticated remote consent.
use crate::{
    joint_planning::{self, JointPlanningReport, JointPlanningRequest},
    snapshots::{SnapshotApplication, context, profile_id},
};
use azeroth_core::{
    entities::QuestProgress,
    error::{AppError, ErrorCode, Result},
    observation::{Freshness, freshness},
    types::{DataContext, DatasetRevision, PartyId, QuestId, Sha256Hex, UtcMillis},
};
use azeroth_storage::{
    SnapshotStore, SqliteStore,
    party_proposals::{LocalAcceptance, PartyProposal, ProposalGuard},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProposePartyPlan {
    pub planning: JointPlanningRequest,
    pub proposer_profile_id: String,
    pub expected: Option<ProposalGuard>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptPartyPlan {
    pub party_id: PartyId,
    pub profile_id: String,
    pub expected: ProposalGuard,
    pub accepted: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgreementState {
    AwaitingAcceptance,
    Agreed,
    Invalidated,
}
#[derive(Clone, Debug, Serialize)]
pub struct QuestProgressDifference {
    pub quest: QuestId,
    pub previous: Option<QuestProgress>,
    pub current: Option<QuestProgress>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ParticipantAgreement {
    pub profile_id: String,
    pub expected_snapshot_id: azeroth_core::types::SnapshotId,
    pub current_snapshot_id: Option<azeroth_core::types::SnapshotId>,
    pub accepted: bool,
    pub accepted_at_utc_ms: Option<UtcMillis>,
    pub freshness: Option<Freshness>,
    pub reasons: Vec<&'static str>,
    pub progress_changes: Vec<QuestProgressDifference>,
}
#[derive(Clone, Debug, Serialize)]
pub struct PartyProposalReport {
    pub party_id: PartyId,
    pub schema_version: u32,
    pub guard: ProposalGuard,
    pub proposer_profile_id: String,
    pub context: DataContext,
    pub fixture: bool,
    pub revision: DatasetRevision,
    pub selected_objectives: Vec<azeroth_core::types::ObjectiveId>,
    pub expected_total_ms: azeroth_core::types::Millis,
    pub created_at_utc_ms: Option<UtcMillis>,
    pub state: AgreementState,
    pub reasons: Vec<&'static str>,
    pub participants: Vec<ParticipantAgreement>,
    pub request: JointPlanningRequest,
}
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
fn digest<T: Serialize>(value: &T) -> Result<Sha256Hex> {
    let data = serde_json::to_vec(value).map_err(|_| invalid("Plan identity cannot be encoded"))?;
    Sha256Hex::new(format!("{:x}", Sha256::digest(data)))
}
fn parse<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    if text.len() > 2 * crate::planning::MAX_REQUEST_BYTES {
        return Err(AppError::new(
            ErrorCode::ImportTooLarge,
            "Agreement request exceeds size limit",
            false,
        ));
    }
    serde_json::from_str(text).map_err(|_| invalid("Invalid local plan agreement request"))
}
/// Recompute within the caller's service lock; never call another locking service.
fn recalculate(
    store: &SqliteStore,
    request: JointPlanningRequest,
    now: Option<UtcMillis>,
) -> Result<JointPlanningReport> {
    let (catalog, snapshots, content) = joint_planning::view(store, &request.party_id, now)?;
    let content = content.ok_or_else(|| invalid("Active party content is unavailable"))?;
    if catalog.revision.as_ref() != Some(&request.revision) {
        return Err(invalid("Party content revision changed"));
    }
    let expected = request
        .participants
        .iter()
        .map(|p| (&p.profile_id, &p.snapshot_id))
        .collect::<BTreeMap<_, _>>();
    if expected.len() != request.participants.len()
        || expected.len() != catalog.participants.len()
        || catalog.participants.iter().any(|p| {
            p.snapshot_id
                .as_ref()
                .is_none_or(|id| expected.get(&p.profile_id) != Some(&id))
        })
        || catalog
            .participants
            .iter()
            .any(|p| !matches!(p.freshness, Some(Freshness::Fresh { .. })))
    {
        return Err(invalid("Party snapshots, membership or freshness changed"));
    }
    joint_planning::calculate(
        request,
        catalog,
        snapshots
            .into_iter()
            .map(|s| s.expect("snapshot checked"))
            .collect(),
        &content,
        now,
    )
}
fn same_progress(old: Option<&QuestProgress>, new: Option<&QuestProgress>) -> bool {
    let same = |a: &azeroth_core::observation::Observation<bool>,
                b: &azeroth_core::observation::Observation<bool>| {
        a.status() == b.status() && a.value() == b.value()
    };
    match (old, new) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            same(&a.accepted, &b.accepted)
                && same(&a.completed, &b.completed)
                && same(&a.turned_in, &b.turned_in)
                && a.objectives.len() == b.objectives.len()
                && a.objectives
                    .iter()
                    .all(|(id, value)| b.objectives.get(id).is_some_and(|new| same(value, new)))
        }
        _ => false,
    }
}
fn inspect(
    store: &SqliteStore,
    proposal: PartyProposal,
    now: Option<UtcMillis>,
) -> Result<PartyProposalReport> {
    let request: JointPlanningRequest = parse(&proposal.request)?;
    let identities = request
        .participants
        .iter()
        .map(|p| &p.profile_id)
        .collect::<BTreeSet<_>>();
    if request.party_id != proposal.party_id
        || identities.len() != request.participants.len()
        || !(2..=40).contains(&identities.len())
        || !identities.contains(&proposal.proposer_profile_id)
        || proposal
            .acceptances
            .keys()
            .any(|id| !identities.contains(id))
    {
        return Err(invalid(
            "Stored local proposal participants are inconsistent",
        ));
    }
    let party = store
        .party(&proposal.party_id)?
        .ok_or_else(|| invalid("No such local party"))?;
    let clock = context(now);
    let current = party
        .participants
        .iter()
        .map(|key| Ok((profile_id(key)?, key)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut reasons = Vec::new();
    if current.keys().collect::<BTreeSet<_>>() != identities {
        reasons.push("Party membership changed; propose and accept a refreshed plan");
    }
    let mut participants = Vec::new();
    for expected in &request.participants {
        let key = current.get(&expected.profile_id).copied();
        let snapshot = key
            .map(|key| store.current_snapshot(key))
            .transpose()?
            .flatten();
        let member_freshness = snapshot
            .as_ref()
            .map(|s| freshness(&s.payload.capture, &clock));
        let mut member_reasons = Vec::new();
        if key.is_none() {
            member_reasons.push("Participant is no longer in this party");
        }
        if snapshot.is_none() {
            member_reasons.push("Current imported snapshot is missing");
        }
        if snapshot
            .as_ref()
            .is_some_and(|s| s.payload.id != expected.snapshot_id)
        {
            member_reasons
                .push("Participant snapshot changed; previous acceptance no longer applies");
        }
        if !matches!(member_freshness, Some(Freshness::Fresh { .. })) {
            member_reasons.push("Capture age is stale or uncertain; joint advice is invalidated");
        }
        let mut progress_changes = Vec::new();
        if let (Some(key), Some(snapshot)) = (key, snapshot.as_ref())
            && let Some(previous) = store.historical_snapshot(key, &expected.snapshot_id)?
        {
            let quests = previous
                .payload
                .quests
                .keys()
                .chain(snapshot.payload.quests.keys())
                .collect::<BTreeSet<_>>();
            for quest in quests {
                let old = previous.payload.quests.get(quest);
                let new = snapshot.payload.quests.get(quest);
                if !same_progress(old, new) {
                    progress_changes.push(QuestProgressDifference {
                        quest: quest.clone(),
                        previous: old.cloned(),
                        current: new.cloned(),
                    });
                }
            }
        }
        let acceptance = proposal.acceptances.get(&expected.profile_id);
        participants.push(ParticipantAgreement {
            profile_id: expected.profile_id.clone(),
            expected_snapshot_id: expected.snapshot_id.clone(),
            current_snapshot_id: snapshot.as_ref().map(|s| s.payload.id.clone()),
            accepted: acceptance.is_some(),
            accepted_at_utc_ms: acceptance.and_then(|a| a.accepted_at_utc_ms),
            freshness: member_freshness,
            reasons: member_reasons,
            progress_changes,
        });
    }
    let recomputed = recalculate(store, request.clone(), now);
    match recomputed {
        Ok(report) => {
            if report.context != proposal.context
                || report.fixture != proposal.fixture
                || digest(&report.result)? != proposal.result_hash
                || report
                    .result
                    .plan
                    .selected
                    .iter()
                    .map(|c| &c.candidate)
                    .collect::<Vec<_>>()
                    != proposal.selected_objectives.iter().collect::<Vec<_>>()
                || report.result.plan.expected_total_ms != proposal.expected_total_ms
            {
                reasons
                    .push("Joint recommendation or data context changed; agreement is invalidated");
            }
        }
        Err(error) => {
            if matches!(error.code, ErrorCode::StorageError) {
                return Err(error);
            }
            reasons
                .push("Current inputs cannot reproduce this proposal; refresh and propose again");
        }
    }
    let invalidated = !reasons.is_empty() || participants.iter().any(|p| !p.reasons.is_empty());
    let state = if invalidated {
        AgreementState::Invalidated
    } else if participants.iter().all(|p| p.accepted) {
        AgreementState::Agreed
    } else {
        AgreementState::AwaitingAcceptance
    };
    Ok(PartyProposalReport {
        party_id: proposal.party_id,
        schema_version: proposal.schema_version,
        guard: proposal.guard,
        proposer_profile_id: proposal.proposer_profile_id,
        context: proposal.context,
        fixture: proposal.fixture,
        revision: request.revision.clone(),
        selected_objectives: proposal.selected_objectives,
        expected_total_ms: proposal.expected_total_ms,
        created_at_utc_ms: proposal.created_at_utc_ms,
        state,
        reasons,
        participants,
        request,
    })
}
impl SnapshotApplication {
    pub fn party_plan_proposal(
        &self,
        id: &PartyId,
        now: Option<UtcMillis>,
    ) -> Result<Option<PartyProposalReport>> {
        self.with_store_mut(|store| {
            store
                .party_proposal(id)?
                .map(|p| inspect(store, p, now))
                .transpose()
        })
    }
    pub fn propose_party_plan(
        &self,
        text: &str,
        now: Option<UtcMillis>,
    ) -> Result<PartyProposalReport> {
        let mut input: ProposePartyPlan = parse(text)?;
        if input.proposer_profile_id.len() > 128 {
            return Err(invalid("Invalid proposer profile"));
        }
        input
            .planning
            .candidates
            .sort_by(|a, b| a.objective.cmp(&b.objective));
        input
            .planning
            .participants
            .sort_by(|a, b| a.profile_id.cmp(&b.profile_id));
        self.with_store_mut(|store| {
            let report = recalculate(store, input.planning.clone(), now)?;
            if report.result.plan.selected.is_empty() {
                return Err(invalid(
                    "A proposal requires at least one jointly feasible selected objective",
                ));
            }
            if !report
                .participants
                .iter()
                .any(|p| p.profile_id == input.proposer_profile_id)
            {
                return Err(invalid("Proposer is not in this party"));
            }
            let revision = input
                .expected
                .as_ref()
                .map_or(Some(1), |g| g.revision.checked_add(1))
                .ok_or_else(|| invalid("Proposal revision overflows"))?;
            let result_hash = digest(&report.result)?;
            let proposal_id = digest(&(
                1,
                &input.planning,
                &input.proposer_profile_id,
                &result_hash,
                &input.expected,
                now,
            ))?;
            let proposal = PartyProposal {
                schema_version: 1,
                party_id: input.planning.party_id.clone(),
                guard: ProposalGuard {
                    proposal_id,
                    revision,
                },
                proposer_profile_id: input.proposer_profile_id,
                request: serde_json::to_string(&input.planning)
                    .map_err(|_| invalid("Cannot encode proposal request"))?,
                result_hash,
                selected_objectives: report
                    .result
                    .plan
                    .selected
                    .iter()
                    .map(|c| c.candidate.clone())
                    .collect(),
                expected_total_ms: report.result.plan.expected_total_ms,
                context: report.context,
                fixture: report.fixture,
                created_at_utc_ms: now,
                acceptances: BTreeMap::new(),
            };
            store.save_party_proposal(&proposal, input.expected.as_ref())?;
            inspect(store, proposal, now)
        })
    }
    pub fn accept_party_plan(
        &self,
        text: &str,
        now: Option<UtcMillis>,
    ) -> Result<PartyProposalReport> {
        let input: AcceptPartyPlan = parse(text)?;
        self.with_store_mut(|store| {
            let mut proposal = store
                .party_proposal(&input.party_id)?
                .ok_or_else(|| invalid("No local plan proposal"))?;
            if proposal.guard != input.expected {
                return Err(invalid("Local proposal changed; refresh before accepting"));
            }
            let report = inspect(store, proposal.clone(), now)?;
            if !report
                .participants
                .iter()
                .any(|p| p.profile_id == input.profile_id)
            {
                return Err(invalid("Acceptance profile is not a proposal participant"));
            }
            if input.accepted && matches!(report.state, AgreementState::Invalidated) {
                return Err(invalid(
                    "An invalidated proposal cannot be accepted; propose a refreshed plan",
                ));
            }
            if input.accepted {
                proposal.acceptances.insert(
                    input.profile_id,
                    LocalAcceptance {
                        accepted_at_utc_ms: now,
                    },
                );
            } else {
                proposal.acceptances.remove(&input.profile_id);
            }
            proposal.guard.revision = proposal
                .guard
                .revision
                .checked_add(1)
                .ok_or_else(|| invalid("Proposal revision overflows"))?;
            store.save_party_proposal(&proposal, Some(&input.expected))?;
            inspect(store, proposal, now)
        })
    }
}
