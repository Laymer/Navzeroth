//! Explicit agreed-party projection for one recipient; no remote consent or game messaging.
use crate::{
    joint_planning::{self, JointPlanningCatalog, JointPlanningReport, JointPlanningRequest},
    plan_export::PlanExport,
    planning_job_service::{PublicationGuard, advanced_now},
    snapshots::{SnapshotApplication, context, profile_id},
};
use azeroth_core::{
    contract::{AnyExchange, Envelope, decode_envelope},
    entities::{CharacterSnapshot, ObjectiveTarget, RoutePlan},
    error::{AppError, ErrorCode, Result},
    observation::{FreshnessContext, Observation, ObservationStatus},
    predicate::{Predicate, PredicateNode, QuestState, RuleLimits},
    types::{PartyId, PlanId, Sha256Hex, UtcMillis},
};
use azeroth_storage::{
    SqliteStore,
    content::ContentRevision,
    party_proposals::{PartyProposal, ProposalGuard},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const MAX_EXPORT_BYTES: usize = 32768;
pub const RULESET_VERSION: &str = "joint-greedy-v1";
pub const MODEL_VERSION: &str = "manual-minimum-member-utility-v1";
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
fn too_large() -> AppError {
    AppError::new(
        ErrorCode::ImportTooLarge,
        "Plan exceeds addon limits; reduce objectives, alternatives or declared conditions",
        false,
    )
}
fn declared(obs: &Observation<Predicate>, predicates: &mut Vec<PredicateNode>) -> Result<()> {
    if let Some(predicate) = obs.value() {
        predicates.push((**predicate).clone());
    } else if obs.status() != ObservationStatus::NotApplicable {
        return Err(invalid(
            "Required readiness cannot be represented faithfully for export",
        ));
    }
    Ok(())
}
/// Mirrors the fixed Lua decoder, including scalar nodes and UTF-8 byte lengths.
fn lua_limits(value: &Value, depth: usize, nodes: &mut usize) -> Result<()> {
    *nodes += 1;
    if *nodes > 8192 {
        return Err(too_large());
    }
    match value {
        Value::String(s) if s.len() > 4096 => return Err(too_large()),
        Value::Array(items) => {
            if depth > 32 || items.len() > 1024 {
                return Err(too_large());
            }
            for item in items {
                lua_limits(item, depth + 1, nodes)?;
            }
        }
        Value::Object(items) => {
            if depth > 32 || items.len() > 1024 {
                return Err(too_large());
            }
            for (key, item) in items {
                if key.len() > 4096 {
                    return Err(too_large());
                }
                lua_limits(item, depth + 1, nodes)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn compile(
    report: &JointPlanningReport,
    snapshots: &[Envelope<CharacterSnapshot>],
    recipient: &Envelope<CharacterSnapshot>,
    proposal: &PartyProposal,
    content: &ContentRevision,
    clock: &FreshnessContext,
    deadline: i64,
) -> Result<PlanExport> {
    if report.result.plan.selected.is_empty() {
        return Err(invalid("No selected objectives to export"));
    }
    let now = clock
        .now_utc_ms
        .ok_or_else(|| invalid("Export generation time is unavailable"))?;
    let expiry = deadline.min(
        now.get()
            .checked_add(1_800_000)
            .ok_or_else(|| invalid("Export expiry overflows"))?,
    );
    let expires = UtcMillis::new(expiry)?;
    if expires <= now {
        return Err(invalid(
            "Snapshot freshness deadline has expired; import a new snapshot",
        ));
    }
    let snapshot_hashes = snapshots
        .iter()
        .map(azeroth_storage::snapshot_content_hash)
        .collect::<Result<Vec<_>>>()?;
    let content_hash = digest(content)?;
    let canonical = serde_json::to_vec(&json!({"agreement":proposal.guard,"recipient":recipient.payload.character,"request":report.request,"result":report.result,"context":report.context,"fixture":report.fixture,"snapshots":snapshot_hashes,"content":content_hash,"ruleset":RULESET_VERSION,"model":MODEL_VERSION})).map_err(|_| invalid("Cannot canonicalize planning inputs"))?;
    let digest = format!("{:x}", Sha256::digest(canonical));
    let plan_id = PlanId::new(format!("plan-{digest}"))?;
    let dataset = &content.envelope.payload;
    let mut route_predicates = Vec::new();
    let mut travel = Vec::new();
    let mut travel_assumptions = Vec::new();
    let mut evidence = BTreeSet::new();
    for route in report
        .result
        .plan
        .selected
        .iter()
        .map(|s| &s.travel)
        .chain(std::iter::once(&report.result.plan.stopping))
    {
        for hop in &route.hops {
            if hop.stateful {
                return Err(invalid(
                    "Stateful travel readiness cannot yet be represented faithfully for export",
                ));
            }
            let edge = dataset
                .edges
                .iter()
                .find(|e| e.id == hop.edge)
                .ok_or_else(|| invalid("Selected edge is absent"))?;
            declared(&edge.requirements, &mut route_predicates)?;
            evidence.extend(edge.evidence.iter().cloned());
            travel_assumptions.push(format!("Travel {}: {} to {}; {:?}; expected {:?}, wait {:?}, service {} ms, total {} ms. Declared status {:?}.", hop.edge.as_str(), hop.from.id.as_str(), hop.to.id.as_str(), hop.mode, hop.expected_ms.value().map(|v| v.get()), hop.wait_ms.value().map(|v| v.get()), hop.service_ms.get(), hop.cost_ms.get(), hop.status));
        }
        travel.extend(route.to_travel_segments());
    }
    let mut steps = Vec::new();
    for (index, choice) in report.result.plan.selected.iter().enumerate() {
        let objective = dataset
            .objectives
            .iter()
            .find(|o| o.id == choice.candidate)
            .ok_or_else(|| invalid("Selected objective is absent"))?;
        let mut predicates = route_predicates.clone();
        declared(&objective.requirements, &mut predicates)?;
        let mut step_evidence: BTreeSet<_> = objective.evidence.iter().cloned().collect();
        for target in &objective.targets {
            match target {
                ObjectiveTarget::QuestStage { quest, stage } => {
                    let definition = dataset
                        .quests
                        .iter()
                        .find(|q| &q.id == quest)
                        .ok_or_else(|| invalid("Selected quest is absent"))?;
                    step_evidence.extend(definition.evidence.iter().cloned());
                    predicates.extend([
                        PredicateNode::QuestState {
                            quest: quest.clone(),
                            state: QuestState::Accepted,
                            value: true,
                        },
                        PredicateNode::QuestState {
                            quest: quest.clone(),
                            state: QuestState::Completed,
                            value: false,
                        },
                        PredicateNode::QuestState {
                            quest: quest.clone(),
                            state: QuestState::TurnedIn,
                            value: false,
                        },
                        PredicateNode::ObjectiveState {
                            objective: stage.clone(),
                            completed: false,
                        },
                    ]);
                }
                ObjectiveTarget::Service { .. } => {}
                ObjectiveTarget::Acquisition { .. } => {
                    return Err(invalid("Acquisition readiness cannot be exported"));
                }
            }
        }
        evidence.extend(step_evidence.iter().cloned());
        let prerequisite = Predicate::try_new(PredicateNode::All { predicates })?;
        let invalidation = Predicate::try_new(PredicateNode::Not {
            predicate: Box::new((*prerequisite).clone()),
        })?;
        let limits = RuleLimits::new(12, 1024, 1024)?;
        prerequisite
            .validate_with(limits)
            .map_err(|_| too_large())?;
        invalidation
            .validate_with(limits)
            .map_err(|_| too_large())?;
        let alternatives: Vec<_> = report
            .result
            .plan
            .alternatives
            .get(index)
            .into_iter()
            .flatten()
            .map(|c| c.candidate.clone())
            .collect();
        if alternatives.len() > 8 {
            return Err(too_large());
        }
        steps.push(json!({
            "id":format!("step-{digest}-{index}"), "objective":objective.id, "participants":[recipient.payload.character],
            "alternatives":alternatives, "prerequisite":{"status":"known","value":prerequisite},
            "invalidation":{"status":"known","value":invalidation}, "required_fields":[],
            "expected_ms":{"status":"known","value":choice.incremental_ms}, "estimate":{"status":"unknown"},
            "explanation":format!("Declared minimum-member group utility {}; expected incremental {} ms (travel {} ms, completion {} ms, service {} ms). Original joint greedy utility/time order with deterministic ties; includes feasible stopping. Revalidate when readiness changes; invalidation does not prove completion.",choice.utility,choice.incremental_ms.get(),choice.travel.total_ms.get(),choice.completion_ms.get(),choice.service_ms.get()),
            "evidence":step_evidence
        }));
    }
    let mut assumptions: Vec<String> = report.assumptions.iter().map(|s| (*s).to_owned()).collect();
    assumptions.push(format!("Manual origin {}; stopping node {}; stopping service {} ms; expected total {} ms. Goal {}.", report.request.origin.as_str(), report.request.stopping_node.as_str(), report.request.stopping_service_ms.get(), report.result.plan.expected_total_ms.get(), report.request.goal_profile.as_str()));
    assumptions.extend(travel_assumptions);
    for estimate in &report.request.candidates {
        assumptions.push(
            serde_json::to_string(estimate)
                .map_err(|_| invalid("Cannot encode manual estimates"))?,
        );
    }
    for filtered in &report.filtered {
        assumptions.push(format!(
            "{}: {}",
            filtered.objective.as_str(),
            filtered.reason
        ));
    }
    for entry in &report.result.plan.trace {
        assumptions.push(format!(
            "Iteration {}: {} {:?}",
            entry.iteration,
            entry.candidate.as_str(),
            entry.decision
        ));
    }
    assumptions.push(format!("Local proposal {}; agreement revision {}; recipient {}. Every local participant explicitly accepted. Desktop checked imported cohort readiness; this does not authenticate remote consent or synchronize game progress. Re-import and agree again when other members diverge.", proposal.guard.proposal_id.as_str(), proposal.guard.revision, recipient.payload.character.as_str()));
    let inputs = snapshots
        .iter()
        .map(
            |s| json!({"id":s.payload.id,"character":s.payload.character,"context":report.context}),
        )
        .collect::<Vec<_>>();
    let document = json!({
        "format":"azeroth-navigator","kind":"route_plan","schema_version":1,"fixture":report.fixture,
        "producer":{"component":"desktop","version":env!("CARGO_PKG_VERSION")},"game":recipient.game,"namespace":recipient.namespace,
        "provenance":{"source":if report.fixture {"synthetic_fixture"} else {"manual_entry"},"clock_trust":"trusted","created_at_utc_ms":now},
        "dependencies":{"dataset_revision":report.request.revision,"ruleset_version":RULESET_VERSION,"score_model_version":MODEL_VERSION,"snapshots":inputs},
        "payload":{"schema_version":1,"id":plan_id,"context":report.context,"generated_at_utc_ms":now,"inputs":inputs,
            "dataset_revision":report.request.revision,"ruleset_version":RULESET_VERSION,"score_model_version":MODEL_VERSION,
            "capability_assumptions":{},"goal":report.request.goal_profile,"time_budget_ms":report.request.budget_ms,"exclusions":report.request.excluded,
            "manual_assumptions":assumptions,"steps":steps,"travel":travel,"evidence":evidence,"expires_at_utc_ms":expires,
            "revalidation":["dataset_revision_changed","build_changed","snapshot_superseded","ruleset_changed"]}
    });
    lua_limits(&document, 1, &mut 0)?;
    let envelope: Envelope<RoutePlan> = decode_envelope(&document.to_string())?;
    let text = azeroth_ingest::anav1::encode_text(&AnyExchange::RoutePlan(Box::new(envelope)))?;
    if text.len() > MAX_EXPORT_BYTES {
        return Err(too_large());
    }
    Ok(PlanExport {
        byte_count: text.len(),
        text,
        plan_id,
        expires_at_utc_ms: expires,
    })
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgreedRecipientExportRequest {
    pub party_id: PartyId,
    pub expected: ProposalGuard,
    pub recipient_profile_id: String,
}
fn digest<T: Serialize>(value: &T) -> Result<Sha256Hex> {
    let bytes =
        serde_json::to_vec(value).map_err(|_| invalid("Cannot encode agreed planning identity"))?;
    Sha256Hex::new(format!("{:x}", Sha256::digest(bytes)))
}
fn changed() -> AppError {
    AppError::new(
        ErrorCode::SnapshotOutdated,
        "Agreement or planning inputs changed; refresh and agree again",
        false,
    )
}
fn read_agreed(store: &SqliteStore, input: &AgreedRecipientExportRequest) -> Result<PartyProposal> {
    let proposal = store.party_proposal(&input.party_id)?.ok_or_else(changed)?;
    if proposal.guard != input.expected {
        return Err(changed());
    }
    let request: JointPlanningRequest =
        serde_json::from_str(&proposal.request).map_err(|_| changed())?;
    let members = request
        .participants
        .iter()
        .map(|p| &p.profile_id)
        .collect::<BTreeSet<_>>();
    if request.party_id != input.party_id
        || request.participants.len() != members.len()
        || !(2..=40).contains(&members.len())
        || request.candidates.len() > 128
        || request.excluded.len() > 128
        || !members.contains(&proposal.proposer_profile_id)
        || !members.contains(&input.recipient_profile_id)
        || proposal.acceptances.keys().collect::<BTreeSet<_>>() != members
    {
        return Err(changed());
    }
    Ok(proposal)
}
struct Captured {
    proposal: PartyProposal,
    request: JointPlanningRequest,
    catalog: JointPlanningCatalog,
    snapshots: Vec<Envelope<CharacterSnapshot>>,
    content: ContentRevision,
    guard: PublicationGuard,
    now: Option<UtcMillis>,
    recipient: usize,
}
struct PreparedExport {
    input: AgreedRecipientExportRequest,
    guard: PublicationGuard,
    export: PlanExport,
    proposal_hash: Sha256Hex,
}
impl SnapshotApplication {
    /// Recompute the stored agreed joint request, never a caller-supplied report
    /// or solo optimization. Compile outside the store lock; publish under one
    /// lock after exact agreement and current-input revalidation.
    pub fn export_agreed_party_plan(
        &self,
        text: &str,
        now: Option<UtcMillis>,
    ) -> Result<PlanExport> {
        let started = std::time::Instant::now();
        let prepared = self.prepare_agreed_export(text, now, started)?;
        self.publish_agreed_export(prepared, now, started)
    }
    fn prepare_agreed_export(
        &self,
        text: &str,
        now: Option<UtcMillis>,
        started: std::time::Instant,
    ) -> Result<PreparedExport> {
        if text.len() > 4096 {
            return Err(AppError::new(
                ErrorCode::ImportTooLarge,
                "Recipient export request exceeds limit",
                false,
            ));
        }
        let input: AgreedRecipientExportRequest = serde_json::from_str(text)
            .map_err(|_| invalid("Invalid agreed recipient export request"))?;
        if input.recipient_profile_id.is_empty() || input.recipient_profile_id.len() > 128 {
            return Err(invalid("Invalid export recipient"));
        }
        let captured = self.with_store_mut(|store| {
            store.with_immediate_read(|store| {
                let proposal = read_agreed(store, &input)?;
                let request: JointPlanningRequest =
                    serde_json::from_str(&proposal.request).map_err(|_| changed())?;
                let capture_now = advanced_now(now, started.elapsed());
                let (catalog, snapshots, content) =
                    joint_planning::view(store, &input.party_id, capture_now)?;
                let content = content.ok_or_else(changed)?;
                if catalog.context != proposal.context
                    || catalog.fixture != proposal.fixture
                    || catalog.revision.as_ref() != Some(&request.revision)
                {
                    return Err(changed());
                }
                let expected = request
                    .participants
                    .iter()
                    .map(|p| (&p.profile_id, &p.snapshot_id))
                    .collect::<std::collections::BTreeMap<_, _>>();
                if catalog.participants.len() != expected.len()
                    || catalog.participants.iter().any(|p| {
                        p.snapshot_id
                            .as_ref()
                            .is_none_or(|s| expected.get(&p.profile_id) != Some(&s))
                    })
                {
                    return Err(changed());
                }
                let snapshots = snapshots
                    .into_iter()
                    .map(|s| s.ok_or_else(changed))
                    .collect::<Result<Vec<_>>>()?;
                let recipient = snapshots
                    .iter()
                    .position(|s| {
                        profile_id(&azeroth_storage::ProfileKey::of(s))
                            .ok()
                            .as_ref()
                            == Some(&input.recipient_profile_id)
                    })
                    .ok_or_else(changed)?;
                let clock = context(capture_now);
                let guard = PublicationGuard::new(
                    Some(input.party_id.clone()),
                    &snapshots,
                    &content,
                    &clock,
                )?;
                Ok(Captured {
                    proposal,
                    request,
                    catalog,
                    snapshots,
                    content,
                    guard,
                    now: capture_now,
                    recipient,
                })
            })
        })?;
        let report = joint_planning::calculate(
            captured.request.clone(),
            captured.catalog,
            captured.snapshots.clone(),
            &captured.content,
            captured.now,
        )?;
        if report.context != captured.proposal.context
            || report.fixture != captured.proposal.fixture
            || digest(&report.result)? != captured.proposal.result_hash
            || report
                .result
                .plan
                .selected
                .iter()
                .map(|c| &c.candidate)
                .collect::<Vec<_>>()
                != captured
                    .proposal
                    .selected_objectives
                    .iter()
                    .collect::<Vec<_>>()
            || report.result.plan.expected_total_ms != captured.proposal.expected_total_ms
        {
            return Err(changed());
        }
        let deadline = captured.guard.valid_until_utc_ms().ok_or_else(changed)?;
        let export = compile(
            &report,
            &captured.snapshots,
            &captured.snapshots[captured.recipient],
            &captured.proposal,
            &captured.content,
            &context(captured.now),
            deadline,
        )?;
        let proposal_hash = digest(&captured.proposal)?;
        Ok(PreparedExport {
            input,
            guard: captured.guard,
            export,
            proposal_hash,
        })
    }
    fn publish_agreed_export(
        &self,
        prepared: PreparedExport,
        now: Option<UtcMillis>,
        started: std::time::Instant,
    ) -> Result<PlanExport> {
        let PreparedExport {
            input,
            guard,
            export,
            proposal_hash,
        } = prepared;
        self.with_store_mut(|store| {
            store.with_immediate_read(|store| {
                if digest(&read_agreed(store, &input)?)? != proposal_hash {
                    return Err(changed());
                }
                guard.validate(store, || advanced_now(now, started.elapsed()))?;
                if advanced_now(now, started.elapsed())
                    .is_none_or(|n| n >= export.expires_at_utc_ms)
                {
                    return Err(changed());
                }
                Ok(export)
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(dead_code)]
    mod support {
        use crate as azeroth_app;
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/recipient_export.rs"
        ));
    }
    #[test]
    fn withdrawal_after_compilation_refuses_the_prepared_export() {
        let db = support::setup(|_| {});
        support::members(&db, 23, support::NOW);
        let p = support::agree(&db);
        let now = Some(UtcMillis::new(support::NOW).unwrap());
        let started = std::time::Instant::now();
        let prepared = db
            .app
            .prepare_agreed_export(&support::export_request(&p, 0), now, started)
            .unwrap();
        support::accept(&db, &p, 1, false).unwrap();
        assert_eq!(
            db.app
                .publish_agreed_export(prepared, now, started)
                .unwrap_err()
                .code,
            ErrorCode::SnapshotOutdated
        );
    }
    #[test]
    fn independent_field_expiry_after_compilation_refuses_the_prepared_export() {
        let db = support::setup(|_| {});
        let one = support::import_edit(&db.app, "one", 22, support::NOW, |v| {
            v["level"]["observed_at_utc_ms"] = json!(support::NOW - 1_790_000)
        });
        let two = support::import(&db.app, "two", 23, support::NOW);
        db.app.save_party(&support::id(), &[one, two]).unwrap();
        let p = support::agree(&db);
        let started = std::time::Instant::now();
        let prepared = db
            .app
            .prepare_agreed_export(
                &support::export_request(&p, 1),
                Some(UtcMillis::new(support::NOW).unwrap()),
                started,
            )
            .unwrap();
        assert_eq!(
            db.app
                .publish_agreed_export(
                    prepared,
                    Some(UtcMillis::new(support::NOW + 20_000).unwrap()),
                    started
                )
                .unwrap_err()
                .code,
            ErrorCode::SnapshotOutdated
        );
    }
}
