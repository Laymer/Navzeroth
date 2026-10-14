//! Pure v1 compiler. Only the application recomputes and supplies the report;
//! frontend results are never trusted. No clock, clipboard, filesystem or Lua I/O.
use crate::planning::PlanningReport;
use azeroth_core::{
    contract::{AnyExchange, Envelope, decode_envelope},
    entities::{CharacterSnapshot, ObjectiveTarget, RoutePlan},
    error::{AppError, ErrorCode, Result},
    observation::{FreshnessContext, Observation, ObservationStatus},
    predicate::{Predicate, PredicateNode, QuestState, RuleLimits},
    types::{PlanId, UtcMillis},
};
use azeroth_storage::content::ContentRevision;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const MAX_EXPORT_BYTES: usize = 32768;
pub const RULESET_VERSION: &str = "planner-same-node-clusters-v1";
pub const MODEL_VERSION: &str = "manual-utility-v1";
#[derive(Clone, Debug, Serialize)]
pub struct PlanExport {
    pub text: String,
    pub byte_count: usize,
    pub plan_id: PlanId,
    pub expires_at_utc_ms: UtcMillis,
}
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
pub(crate) fn compile(
    report: &PlanningReport,
    snapshot: &Envelope<CharacterSnapshot>,
    content: &ContentRevision,
    clock: &FreshnessContext,
) -> Result<PlanExport> {
    if report.result.selected.is_empty() {
        return Err(invalid("No selected objectives to export"));
    }
    let now = clock
        .now_utc_ms
        .ok_or_else(|| invalid("Export generation time is unavailable"))?;
    let guard = crate::planning_job_service::PublicationGuard::new(
        None,
        std::slice::from_ref(snapshot),
        content,
        clock,
    )?;
    let deadline = guard
        .valid_until_utc_ms()
        .ok_or_else(|| invalid("Typed freshness deadline is unavailable"))?;
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
    let snapshot_hash = azeroth_storage::snapshot_content_hash(snapshot)?;
    let content_bytes =
        serde_json::to_vec(content).map_err(|_| invalid("Cannot canonicalize content identity"))?;
    let content_hash = format!("{:x}", Sha256::digest(content_bytes));
    let canonical = serde_json::to_vec(&json!({
        "request": report.request, "result": report.result,
        "context": report.context, "fixture": report.fixture,
        "snapshot_hash": snapshot_hash, "content_hash": content_hash,
        "ruleset": RULESET_VERSION, "model": MODEL_VERSION,
    }))
    .map_err(|_| invalid("Cannot canonicalize planning inputs"))?;
    let digest = format!("{:x}", Sha256::digest(canonical));
    let plan_id = PlanId::new(format!("plan-{digest}"))?;
    let dataset = &content.envelope.payload;
    let mut route_predicates = Vec::new();
    let mut travel = Vec::new();
    let mut travel_assumptions = Vec::new();
    let mut evidence = BTreeSet::new();
    for route in report
        .result
        .selected
        .iter()
        .map(|s| &s.travel)
        .chain(std::iter::once(&report.result.stopping))
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
    for (index, choice) in report.result.selected.iter().enumerate() {
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
            "id":format!("step-{digest}-{index}"), "objective":objective.id, "participants":objective.participants,
            "alternatives":alternatives, "prerequisite":{"status":"known","value":prerequisite},
            "invalidation":{"status":"known","value":invalidation}, "required_fields":[],
            "expected_ms":{"status":"known","value":choice.incremental_ms}, "estimate":{"status":"unknown"},
            "explanation":format!("Manual utility {}; expected incremental {} ms (travel {} ms, completion {} ms, service {} ms). Greedy utility/time order with deterministic ties; includes feasible stopping. Revalidate when readiness changes; invalidation does not prove completion.",choice.utility,choice.incremental_ms.get(),choice.travel.total_ms.get(),choice.completion_ms.get(),choice.service_ms.get()),
            "evidence":step_evidence
        }));
    }
    let mut assumptions: Vec<String> = report.assumptions.iter().map(|s| (*s).to_owned()).collect();
    assumptions.push(format!("Manual origin {}; stopping node {}; stopping service {} ms; expected total {} ms. Goal {}.", report.request.origin.as_str(), report.request.stopping_node.as_str(), report.request.stopping_service_ms.get(), report.result.expected_total_ms.get(), report.request.goal_profile.as_str()));
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
    for entry in &report.result.trace {
        assumptions.push(format!(
            "Iteration {}: {} {:?}",
            entry.iteration,
            entry.candidate.as_str(),
            entry.decision
        ));
    }
    let inputs = json!([{"id":snapshot.payload.id,"character":snapshot.payload.character,"context":report.context}]);
    let document = json!({
        "format":"azeroth-navigator","kind":"route_plan","schema_version":1,"fixture":report.fixture,
        "producer":{"component":"desktop","version":env!("CARGO_PKG_VERSION")},"game":snapshot.game,"namespace":snapshot.namespace,
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lua_collection_string_node_and_depth_limits_are_not_truncated() {
        assert!(lua_limits(&json!("a".repeat(4096)), 1, &mut 0).is_ok());
        assert!(lua_limits(&json!("a".repeat(4097)), 1, &mut 0).is_err());
        assert!(lua_limits(&json!(vec![0; 1024]), 1, &mut 0).is_ok());
        assert!(lua_limits(&json!(vec![0; 1025]), 1, &mut 0).is_err());
        let mut nested = json!([]);
        for _ in 1..32 {
            nested = json!([nested]);
        }
        assert!(lua_limits(&nested, 1, &mut 0).is_ok());
        assert!(lua_limits(&json!([nested]), 1, &mut 0).is_err());
        assert!(lua_limits(&json!(0), 1, &mut 8192).is_err());
    }
}
