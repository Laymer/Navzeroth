//! Explicit manual source locations and guarded independent route opportunities.
use crate::catalog::CatalogView;
use crate::{
    gear::{self, AcquisitionInput, EffortInput, GearReport, GearRequest},
    planning::{self, PlanningReport, PlanningRequest},
    snapshots::SnapshotApplication,
};
use azeroth_core::{
    contract::Envelope,
    entities::{CharacterSnapshot, NodeRef, ObjectiveTarget},
    error::{AppError, ErrorCode, Result},
    evaluation::{EvaluationContext, StalePolicy},
    gear::{
        AcquisitionAssessment, AcquisitionEffort, BalanceBasis, CostSchedule, ProbabilityModel,
        assess_acquisition,
    },
    observation::{FreshnessContext, Observation, ObservationMetadata, StateSource},
    predicate::RuleLimits,
    travel::{Route, RouteOptions, RouteOutcome, RouteSearch, shortest_route},
    types::*,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
pub const MAX_OPPORTUNITIES: usize = 128;
pub const MAX_REPORT_BYTES: usize = 4 * 1024 * 1024;
fn bounded<T: Serialize>(value: &T) -> Result<()> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > MAX_REPORT_BYTES - self.0 {
                return Err(std::io::Error::other(
                    "Route acquisition report exceeds limit",
                ));
            }
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Counter(0), value).map_err(|_| {
        AppError::new(
            ErrorCode::ImportTooLarge,
            "Route acquisition report exceeds limit",
            false,
        )
    })
}
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
fn parse<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    if text.len() > MAX_REQUEST_BYTES {
        return Err(AppError::new(
            ErrorCode::ImportTooLarge,
            "Route acquisition request exceeds limit",
            false,
        ));
    }
    serde_json::from_str(text).map_err(|_| invalid("Invalid route acquisition request"))
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteOpportunityInput {
    pub source: AcquisitionId,
    pub target: ItemKey,
    /// A user declaration, never an inferred source location. None stays unknown.
    pub node: Option<NodeRef>,
    pub leg_index: usize,
    pub formation_ms: Observation<Millis>,
    pub attempt_ms: Observation<Millis>,
    pub available_copper: Observation<Copper>,
    pub max_attempts: u32,
    pub independent_player_attempts: bool,
    pub cost_per_attempt: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteAcquisitionRequest {
    pub schema_version: u32,
    pub planning: PlanningRequest,
    pub expected_route_fingerprint: Sha256Hex,
    pub opportunities: Vec<RouteOpportunityInput>,
    /// Gear assumptions only; each supplied acquisition must be None.
    pub gear: Option<GearRequest>,
}
#[derive(Clone, Debug, Serialize)]
pub struct AcquisitionRouteLeg {
    pub index: usize,
    pub stopping: bool,
    pub route: Route,
}
#[derive(Clone, Debug, Serialize)]
pub struct RouteAcquisitionBaseline {
    pub schema_version: u32,
    pub route_fingerprint: Sha256Hex,
    pub planning_report: PlanningReport,
    pub legs: Vec<AcquisitionRouteLeg>,
}
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DetourGap {
    MissingManualBinding,
    OutboundIncomplete,
    RejoinIncomplete,
    UnsafeStatefulAction,
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DetourOutcome {
    Complete {
        outbound: RouteSearch,
        rejoin: RouteSearch,
        baseline_leg_ms: Millis,
        incremental_ms: Millis,
    },
    Unknown {
        reason: DetourGap,
        outbound: Option<RouteSearch>,
        rejoin: Option<RouteSearch>,
    },
}
#[derive(Clone, Debug, Serialize)]
pub struct RouteOpportunityReport {
    pub input: RouteOpportunityInput,
    pub detour: DetourOutcome,
    pub assessment: AcquisitionAssessment,
    pub reward_accounting_complete: bool,
    pub total_with_reservation_ms: Option<Millis>,
}
#[derive(Clone, Debug, Serialize)]
pub struct RouteAcquisitionReport {
    pub schema_version: u32,
    pub baseline: RouteAcquisitionBaseline,
    pub remaining_budget_ms: Millis,
    pub opportunities: Vec<RouteOpportunityReport>,
    pub gear: Option<GearReport>,
    pub assumptions: Vec<&'static str>,
}
fn measured(value: Millis) -> Result<Observation<Millis>> {
    Observation::known(
        value,
        ObservationMetadata {
            source: StateSource::ManualEntry,
            ..Default::default()
        },
    )
}
fn sum(a: u64, b: u64) -> Result<Millis> {
    Millis::new(
        a.checked_add(b)
            .ok_or_else(|| invalid("Route acquisition duration overflows"))?,
    )
}
fn fingerprint(
    report: &PlanningReport,
    snapshot: &Envelope<CharacterSnapshot>,
    content: &dyn CatalogView,
) -> Result<Sha256Hex> {
    // Hash exact captured bytes and route result, not generated time/hardware timing.
    let mut bytes = serde_json::to_vec(&(
        &report.request,
        &report.context,
        report.fixture,
        &report.result.selected,
        &report.result.stopping,
        snapshot,
    ))
    .map_err(|_| invalid("Route identity cannot be encoded"))?;
    bytes.pop(); // tuple closing bracket; retain the legacy v1 byte identity
    bytes.push(b',');
    bytes.extend(content.encoded()?);
    bytes.push(b']');
    Sha256Hex::new(format!("{:x}", Sha256::digest(bytes)))
}
fn baseline(
    request: PlanningRequest,
    snapshot: &Envelope<CharacterSnapshot>,
    content: &dyn CatalogView,
    clock: &FreshnessContext,
) -> Result<RouteAcquisitionBaseline> {
    let report = planning::calculate(request, snapshot, content, clock)?;
    let fingerprint = fingerprint(&report, snapshot, content)?;
    let legs = report
        .result
        .selected
        .iter()
        .map(|s| s.travel.clone())
        .chain(std::iter::once(report.result.stopping.clone()))
        .enumerate()
        .map(|(index, route)| AcquisitionRouteLeg {
            index,
            stopping: index == report.result.selected.len(),
            route,
        })
        .collect();
    let value = RouteAcquisitionBaseline {
        schema_version: 1,
        route_fingerprint: fingerprint,
        planning_report: report,
        legs,
    };
    bounded(&value)?;
    Ok(value)
}
fn complete(search: &RouteSearch) -> Option<&Route> {
    match &search.outcome {
        RouteOutcome::Complete { route } => Some(route),
        _ => None,
    }
}
fn detour(
    input: &RouteOpportunityInput,
    leg: &AcquisitionRouteLeg,
    baseline: &RouteAcquisitionBaseline,
    content: &dyn CatalogView,
    context: &EvaluationContext<'_>,
) -> Result<DetourOutcome> {
    let Some(node) = &input.node else {
        return Ok(DetourOutcome::Unknown {
            reason: DetourGap::MissingManualBinding,
            outbound: None,
            rejoin: None,
        });
    };
    let data = &content.reference();
    context
        .snapshot
        .context
        .ensure_compatible(&node.map.context)?;
    if !data
        .nodes
        .iter()
        .any(|n| n.id == node.id && n.position.map == node.map)
    {
        return Err(invalid(
            "Manual source node is absent from exact active content",
        ));
    }
    let options = RouteOptions::default();
    let outbound = shortest_route(
        &data.nodes,
        &data.edges,
        &leg.route.start.id,
        &node.id,
        context,
        &options,
    )?;
    let Some(a) = complete(&outbound) else {
        return Ok(DetourOutcome::Unknown {
            reason: DetourGap::OutboundIncomplete,
            outbound: Some(outbound),
            rejoin: None,
        });
    };
    let rejoin = shortest_route(
        &data.nodes,
        &data.edges,
        &node.id,
        &leg.route.end().id,
        context,
        &options,
    )?;
    let Some(c) = complete(&rejoin) else {
        return Ok(DetourOutcome::Unknown {
            reason: DetourGap::RejoinIncomplete,
            outbound: Some(outbound),
            rejoin: Some(rejoin),
        });
    };
    // Reject baseline resource actions outside the final route and any outbound
    // reserved action. The replacement must not make one action reusable.
    let unsafe_baseline = baseline.legs.iter().any(|l| {
        l.route
            .hops
            .iter()
            .enumerate()
            .any(|(i, h)| h.stateful && (!l.stopping || i + 1 != l.route.hops.len()))
    });
    let stateful_other = baseline
        .legs
        .iter()
        .filter(|l| l.index != leg.index)
        .flat_map(|l| &l.route.hops)
        .filter(|h| h.stateful)
        .count();
    let unsafe_detour = a.hops.iter().any(|h| h.stateful)
        || c.hops
            .iter()
            .enumerate()
            .any(|(i, h)| h.stateful && (!leg.stopping || i + 1 != c.hops.len()))
        || stateful_other + c.hops.iter().filter(|h| h.stateful).count() > 1;
    if unsafe_baseline || unsafe_detour {
        return Ok(DetourOutcome::Unknown {
            reason: DetourGap::UnsafeStatefulAction,
            outbound: Some(outbound),
            rejoin: Some(rejoin),
        });
    }
    let replacement = sum(a.total_ms.get(), c.total_ms.get())?;
    let incremental = Millis::new(
        replacement
            .get()
            .checked_sub(leg.route.total_ms.get())
            .ok_or_else(|| invalid("Detour is cheaper than guarded baseline leg"))?,
    )?;
    Ok(DetourOutcome::Complete {
        outbound,
        rejoin,
        baseline_leg_ms: leg.route.total_ms,
        incremental_ms: incremental,
    })
}
fn counted_rewards(
    baseline: &RouteAcquisitionBaseline,
    content: &dyn CatalogView,
) -> Result<(BTreeSet<ItemKey>, bool)> {
    let mut items = BTreeSet::new();
    let mut known = true;
    let data = &content.reference();
    for choice in &baseline.planning_report.result.selected {
        let objective = data
            .objectives
            .iter()
            .find(|o| o.id == choice.candidate)
            .ok_or_else(|| invalid("Selected objective is absent"))?;
        for target in &objective.targets {
            match target {
                ObjectiveTarget::QuestStage { quest, .. } => {
                    let quest = data
                        .quests
                        .iter()
                        .find(|q| &q.id == quest)
                        .ok_or_else(|| invalid("Selected quest is absent"))?;
                    match quest.rewards.value() {
                        Some(rewards) => items.extend(rewards.iter().cloned()),
                        None if quest.rewards.status()
                            == azeroth_core::observation::ObservationStatus::NotApplicable => {}
                        None => known = false,
                    }
                }
                ObjectiveTarget::Acquisition { source } => {
                    let source = data
                        .acquisitions
                        .iter()
                        .find(|s| &s.id == source)
                        .ok_or_else(|| invalid("Selected source is absent"))?;
                    items.extend(source.items.iter().cloned());
                }
                ObjectiveTarget::Service { .. } => {}
            }
        }
    }
    Ok((items, known))
}
fn calculate(
    request: RouteAcquisitionRequest,
    snapshot: &Envelope<CharacterSnapshot>,
    content: &dyn CatalogView,
    clock: &FreshnessContext,
) -> Result<RouteAcquisitionReport> {
    let base = baseline(request.planning.clone(), snapshot, content, clock)?;
    if base.route_fingerprint != request.expected_route_fingerprint {
        return Err(AppError::new(
            ErrorCode::SnapshotOutdated,
            "Calculated route changed; preview the current route again",
            false,
        ));
    }
    let remaining = Millis::new(
        request
            .planning
            .budget_ms
            .get()
            .checked_sub(base.planning_report.result.expected_total_ms.get())
            .ok_or_else(|| invalid("Baseline exceeds the declared session budget"))?,
    )?;
    let evaluation = EvaluationContext {
        snapshot: &snapshot.payload,
        party: None,
        freshness: clock,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    let (counted, coverage_complete) = counted_rewards(&base, content)?;
    let mut reports = Vec::new();
    for input in request.opportunities {
        let leg = base
            .legs
            .get(input.leg_index)
            .ok_or_else(|| invalid("Selected route leg is absent"))?;
        let source = content
            .reference()
            .acquisitions
            .iter()
            .find(|s| s.id == input.source)
            .ok_or_else(|| invalid("Acquisition source is absent from active content"))?;
        let route = detour(&input, leg, &base, content, &evaluation)?;
        let travel = match &route {
            DetourOutcome::Complete { incremental_ms, .. } => measured(*incremental_ms)?,
            _ => Observation::unknown(),
        };
        let effort = AcquisitionEffort {
            incremental_travel_ms: travel,
            formation_ms: input.formation_ms.clone(),
            attempt_ms: input.attempt_ms.clone(),
            stopping_ms: measured(Millis::new(0)?)?,
            available_copper: input.available_copper.clone(),
            balance_basis: BalanceBasis::CallerBudget,
            session_budget_ms: remaining,
            max_attempts: input.max_attempts,
            probability_model: if input.independent_player_attempts {
                ProbabilityModel::IndependentPlayerAttempts
            } else {
                ProbabilityModel::DeclaredOnly
            },
            cost_schedule: if input.cost_per_attempt {
                CostSchedule::PerAttempt
            } else {
                CostSchedule::Once
            },
        };
        let effective_source = content
            .content_v2()
            .map(|c| crate::authored_content::acquisition_facts(source, &input.target, c))
            .transpose()?;
        let mut assessment = assess_acquisition(
            effective_source.as_ref().unwrap_or(source),
            &input.target,
            &effort,
            &counted,
            &evaluation,
        )?;
        if !coverage_complete {
            assessment.credited_reward = None;
            if assessment.status != azeroth_core::gear::AcquisitionStatus::Unavailable {
                assessment.status = azeroth_core::gear::AcquisitionStatus::Uncertain;
            }
        }
        let total = assessment
            .budgeted_effort_ms
            .map(|effort| {
                sum(
                    base.planning_report.result.expected_total_ms.get(),
                    effort.get(),
                )
            })
            .transpose()?;
        reports.push(RouteOpportunityReport {
            input,
            detour: route,
            assessment,
            reward_accounting_complete: coverage_complete,
            total_with_reservation_ms: total,
        });
    }
    let gear = request
        .gear
        .map(|mut gear| {
            if gear.profile_id != request.planning.profile_id
                || gear.snapshot_id != request.planning.snapshot_id
                || gear.revision != request.planning.revision
                || gear.candidates.iter().any(|c| c.acquisition.is_some())
            {
                return Err(invalid(
                    "Gear must use the route capture and contain only model assumptions",
                ));
            }
            for candidate in &mut gear.candidates {
                let opportunities = reports
                    .iter()
                    .filter(|r| r.input.target == candidate.item)
                    .collect::<Vec<_>>();
                if opportunities.len() > 1 {
                    return Err(invalid("Select one independent source per gear candidate"));
                }
                if let Some(report) = opportunities.first().filter(|r| {
                    r.assessment.credited_reward.is_some() && r.reward_accounting_complete
                }) {
                    let e = &report.assessment.effort;
                    candidate.acquisition = Some(AcquisitionInput {
                        source: report.input.source.clone(),
                        effort: EffortInput {
                            incremental_travel_ms: e.incremental_travel_ms.clone(),
                            formation_ms: e.formation_ms.clone(),
                            attempt_ms: e.attempt_ms.clone(),
                            stopping_ms: e.stopping_ms.clone(),
                            available_copper: e.available_copper.clone(),
                            session_budget_ms: e.session_budget_ms,
                            max_attempts: e.max_attempts,
                            independent_player_attempts: report.input.independent_player_attempts,
                            cost_per_attempt: report.input.cost_per_attempt,
                        },
                    });
                }
            }
            gear::calculate(gear, snapshot, content, clock)
        })
        .transpose()?;
    let value = RouteAcquisitionReport {
        schema_version: 1,
        baseline: base,
        remaining_budget_ms: remaining,
        opportunities: reports,
        gear,
        assumptions: vec![
            "Source locations, route objective nodes, utilities and trial effort are explicit manual assumptions; no source location is inferred",
            "Each opportunity is an independent alternative, not a combined acquisition schedule; unknown reward coverage blocks additional credit",
            "Incremental travel replaces one guarded route leg and includes rejoin/return; baseline travel and stopping are reserved once",
            "Graph routes preserve estimated/unknown edges and do not establish walkable road geometry; no shared-kill savings or useful lifespan is inferred",
        ],
    };
    bounded(&value)?;
    Ok(value)
}
impl SnapshotApplication {
    pub fn preview_acquisition_route(
        &self,
        text: &str,
        now: Option<UtcMillis>,
    ) -> Result<RouteAcquisitionBaseline> {
        self.with_planning(text, now, baseline)
    }
    pub fn assess_route_acquisition(
        &self,
        text: &str,
        now: Option<UtcMillis>,
    ) -> Result<RouteAcquisitionReport> {
        let request: RouteAcquisitionRequest = parse(text)?;
        if request.schema_version != 1
            || request.opportunities.is_empty()
            || request.opportunities.len() > MAX_OPPORTUNITIES
        {
            return Err(invalid(
                "Unsupported route acquisition schema or opportunity count",
            ));
        }
        let mut unique = BTreeSet::new();
        for o in &request.opportunities {
            if !unique.insert((&o.source, &o.target)) {
                return Err(invalid("Duplicate independent opportunity"));
            }
        }
        let planning = serde_json::to_string(&request.planning)
            .map_err(|_| invalid("Invalid planning assumptions"))?;
        self.with_planning(&planning, now, move |_, snapshot, content, clock| {
            calculate(request, snapshot, content, clock)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_serialization_is_bounded_without_allocating_an_output_buffer() {
        assert!(bounded(&"x".repeat(MAX_REPORT_BYTES)).is_err());
        bounded(&"short").unwrap();
    }
}
