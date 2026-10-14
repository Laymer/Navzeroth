//! Shortest routes over the directed travel graph (plan sections 9.1 and 9.2).
//!
//! - The graph is directed. A reverse connection exists only if the data holds
//!   a second edge; no edge is mirrored, interpolated or inferred from
//!   positions. Positions on different map spaces are never compared and no
//!   geometric heuristic is used.
//! - The search is Dijkstra over nonnegative expected milliseconds:
//!   `expected + wait + caller-supplied service time`, summed with checked
//!   arithmetic. Waits are declared expected waits only; there are no
//!   time-dependent schedules.
//! - Unknown is never treated as available. Only an edge whose requirement
//!   evaluates to `Truth::True` (or that declares no requirement) is usable.
//!   `Truth::False` and `Truth::Unknown` stay distinct exclusion reasons, and
//!   an `Err` from evaluation propagates instead of becoming Unknown.
//! - An incomplete route reports the known reachable segment and never
//!   invents a connection across the gap.
//! - Stateful actions (return or teleport modes, consumed resources, known
//!   cooldowns) are never relaxed mid-route. At most one verified stateful
//!   edge may close a route as its final hop.
//!
//! The module is pure: it reads no clock, performs no I/O and uses only
//! ordered collections, so equal inputs give an identical result regardless of
//! the order in which nodes and edges are supplied.
use crate::search_control::{NoopSearchControl, SearchControl, SearchPhase};
use crate::{
    entities::{
        DurationRange, EdgeStatus, NodeRef, TravelEdge, TravelMode, TravelNode, TravelSegment,
        unique,
    },
    error::{Result, invalid},
    evaluation::{EvaluationContext, EvaluationIssue, Truth, evaluate},
    observation::{Observation, ObservationStatus},
    types::*,
};
use serde::Serialize;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, BinaryHeap},
};

/// Caller-supplied inputs that the contract has no field for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RouteOptions {
    /// Service time charged each time the keyed edge is used, for example
    /// boarding a boat. Edges without an entry add 0. Keys must name edges of
    /// the graph. Danger or preference penalties never belong here.
    pub service_ms: BTreeMap<EdgeId, Millis>,
}

/// Cost term that made an edge unusable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CostTerm {
    Expected,
    Wait,
}

/// Why an edge is not part of the search. One reason per edge, chosen in the
/// order: map, status, requirement, cost, stateful availability.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum TravelExclusion {
    /// An endpoint reference names a different map space than the node's position.
    MapMismatch,
    TemporarilyUnavailable,
    Unverified,
    /// The requirement observation is unknown, unsupported or restricted.
    RequirementUnavailable {
        status: ObservationStatus,
    },
    /// The requirement evaluated to `Truth::False`.
    RequirementNotMet,
    /// The requirement evaluated to `Truth::Unknown`; the issues explain why.
    RequirementUnknown {
        issues: Vec<EvaluationIssue>,
    },
    /// A cost term is not known, so the edge cost cannot be stated.
    CostUnknown {
        term: CostTerm,
    },
    /// A stateful edge is only estimated; the reserved action must be verified.
    StatefulNotVerified,
    /// Resource use or cooldown of a stateful edge is not known.
    StatefulAvailabilityUnknown {
        consumes_resource: ObservationStatus,
        cooldown: ObservationStatus,
    },
    /// A stateful edge may only end a route, and this one does not reach the target.
    StatefulNotFinalHop,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExcludedEdge {
    pub edge: EdgeId,
    pub reason: TravelExclusion,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RouteHop {
    pub edge: EdgeId,
    pub from: NodeRef,
    pub to: NodeRef,
    pub mode: TravelMode,
    /// `Estimated` hops make the route an estimate; stateful hops are `Verified`.
    pub status: EdgeStatus,
    pub expected_ms: Observation<Millis>,
    pub wait_ms: Observation<Millis>,
    pub service_ms: Millis,
    /// `expected + wait + service`.
    pub cost_ms: Millis,
    pub range: Observation<DurationRange>,
    /// True for the reserved final stateful action.
    pub stateful: bool,
    pub consumes_resource: Observation<bool>,
    pub cooldown_ms: Observation<Millis>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Route {
    pub start: NodeRef,
    pub hops: Vec<RouteHop>,
    /// Sum of the hop costs.
    pub total_ms: Millis,
}
impl Route {
    pub fn end(&self) -> &NodeRef {
        self.hops.last().map_or(&self.start, |hop| &hop.to)
    }
    /// True if any hop rests on an estimated edge.
    pub fn is_estimate(&self) -> bool {
        self.hops
            .iter()
            .any(|hop| hop.status == EdgeStatus::Estimated)
    }
    /// Contract segments for a `RoutePlan`. Each segment carries the edge's
    /// own travel duration and range (waits and service time are in
    /// `total_ms`), so segments are contiguous and inside their ranges.
    pub fn to_travel_segments(&self) -> Vec<TravelSegment> {
        self.hops
            .iter()
            .map(|hop| TravelSegment {
                from: hop.from.clone(),
                to: hop.to.clone(),
                expected_ms: hop.expected_ms.clone(),
                range: hop.range.clone(),
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RouteOutcome {
    Complete {
        route: Route,
    },
    /// The target cannot be reached through usable edges. `reached` is the
    /// known segment ending at the reachable node with the highest cost (ties
    /// by smallest node id); it makes no claim of being nearer the target,
    /// because no geometry is used. `reachable` lists every node the search
    /// settled. Nothing joins the gap to the target.
    Incomplete {
        target: NodeId,
        reached: Route,
        reachable: Vec<NodeId>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RouteSearch {
    pub outcome: RouteOutcome,
    /// Sorted by edge id.
    pub exclusions: Vec<ExcludedEdge>,
}

struct Prepared<'a> {
    edge: &'a TravelEdge,
    cost: u64,
    service: Millis,
    stateful: bool,
}

type Label = (u64, usize, Vec<EdgeId>);

fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b)
        .filter(|sum| *sum <= MAX_SAFE_INTEGER)
        .ok_or_else(|| invalid("Route duration exceeds the safe integer range"))
}

fn is_stateful(edge: &TravelEdge) -> bool {
    matches!(edge.mode, TravelMode::Return | TravelMode::Teleport)
        || edge.consumes_resource.value() == Some(&true)
        || edge.cooldown_ms.value().is_some_and(|ms| ms.get() > 0)
}

fn assess<'a>(
    edge: &'a TravelEdge,
    context: &EvaluationContext<'_>,
    options: &RouteOptions,
) -> Result<std::result::Result<Prepared<'a>, TravelExclusion>> {
    match edge.status {
        EdgeStatus::TemporarilyUnavailable => {
            return Ok(Err(TravelExclusion::TemporarilyUnavailable));
        }
        EdgeStatus::Unverified => return Ok(Err(TravelExclusion::Unverified)),
        EdgeStatus::Verified | EdgeStatus::Estimated => {}
    }
    match (edge.requirements.value(), edge.requirements.status()) {
        (Some(predicate), _) => {
            let result = evaluate(predicate, context)?;
            match result.truth {
                Truth::True => {}
                Truth::False => return Ok(Err(TravelExclusion::RequirementNotMet)),
                Truth::Unknown => {
                    return Ok(Err(TravelExclusion::RequirementUnknown {
                        issues: result.reasons,
                    }));
                }
            }
        }
        (None, ObservationStatus::NotApplicable) => {}
        (None, status) => return Ok(Err(TravelExclusion::RequirementUnavailable { status })),
    }
    let Some(expected) = edge.expected_ms.value() else {
        return Ok(Err(TravelExclusion::CostUnknown {
            term: CostTerm::Expected,
        }));
    };
    let wait = match (edge.wait_ms.value(), edge.wait_ms.status()) {
        (Some(wait), _) => wait.get(),
        (None, ObservationStatus::NotApplicable) => 0,
        (None, _) => {
            return Ok(Err(TravelExclusion::CostUnknown {
                term: CostTerm::Wait,
            }));
        }
    };
    let service = options
        .service_ms
        .get(&edge.id)
        .copied()
        .unwrap_or(Millis::new(0)?);
    let cost = add(add(expected.get(), wait)?, service.get())?;
    let stateful = is_stateful(edge);
    if stateful {
        if edge.status != EdgeStatus::Verified {
            return Ok(Err(TravelExclusion::StatefulNotVerified));
        }
        let (consumes, cooldown) = (edge.consumes_resource.status(), edge.cooldown_ms.status());
        if consumes != ObservationStatus::Known || cooldown != ObservationStatus::Known {
            return Ok(Err(TravelExclusion::StatefulAvailabilityUnknown {
                consumes_resource: consumes,
                cooldown,
            }));
        }
    }
    Ok(Ok(Prepared {
        edge,
        cost,
        service,
        stateful,
    }))
}

fn build_route(
    start: NodeRef,
    label: &Label,
    prepared: &BTreeMap<&EdgeId, &Prepared<'_>>,
    control: &mut dyn SearchControl,
) -> Result<Route> {
    let mut hops = Vec::with_capacity(label.2.len());
    for id in &label.2 {
        control.checkpoint(SearchPhase::RouteExpansion)?;
        let item = prepared
            .get(id)
            .ok_or_else(|| invalid("Route references an unusable edge"))?;
        let edge = item.edge;
        hops.push(RouteHop {
            edge: edge.id.clone(),
            from: edge.from.clone(),
            to: edge.to.clone(),
            mode: edge.mode,
            status: edge.status,
            expected_ms: edge.expected_ms.clone(),
            wait_ms: edge.wait_ms.clone(),
            service_ms: item.service,
            cost_ms: Millis::new(item.cost)?,
            range: edge.uncertainty.clone(),
            stateful: item.stateful,
            consumes_resource: edge.consumes_resource.clone(),
            cooldown_ms: edge.cooldown_ms.clone(),
        });
    }
    Ok(Route {
        start,
        hops,
        total_ms: Millis::new(label.0)?,
    })
}

/// Cheapest route from `origin` to `target`.
///
/// Ties resolve by lowest total milliseconds, then fewer hops, then the
/// lexicographically smallest sequence of edge ids. The caller supplies the
/// snapshot, freshness and limits inside `context`; nothing reads a clock.
///
/// Errors: `InvalidInput` for more than 1024 nodes or edges, duplicate ids,
/// dangling node references, unknown origin, target or option keys and
/// duration overflow; `BuildMismatch` if any node, edge or the snapshot lies
/// in a different data context; evaluation errors propagate unchanged.
pub fn shortest_route(
    nodes: &[TravelNode],
    edges: &[TravelEdge],
    origin: &NodeId,
    target: &NodeId,
    context: &EvaluationContext<'_>,
    options: &RouteOptions,
) -> Result<RouteSearch> {
    shortest_route_controlled(
        nodes,
        edges,
        origin,
        target,
        context,
        options,
        &mut NoopSearchControl,
    )
}

/// Cooperatively controlled shortest route; callback errors abort without a route.
pub fn shortest_route_controlled(
    nodes: &[TravelNode],
    edges: &[TravelEdge],
    origin: &NodeId,
    target: &NodeId,
    context: &EvaluationContext<'_>,
    options: &RouteOptions,
    control: &mut dyn SearchControl,
) -> Result<RouteSearch> {
    control.checkpoint(SearchPhase::ValidateInputs)?;
    unique(nodes.iter().map(|node| &node.id))?;
    unique(edges.iter().map(|edge| &edge.id))?;
    for node in nodes {
        control.checkpoint(SearchPhase::ValidateInputs)?;
        node.position
            .map
            .context
            .ensure_compatible(&context.snapshot.context)?;
    }
    for edge in edges {
        control.checkpoint(SearchPhase::ValidateInputs)?;
        edge.context.ensure_compatible(&context.snapshot.context)?;
    }
    let by_node: BTreeMap<&NodeId, &TravelNode> = nodes.iter().map(|n| (&n.id, n)).collect();
    crate::entities::bounded(options.service_ms.len())?;
    let edge_ids: BTreeSet<&EdgeId> = edges.iter().map(|edge| &edge.id).collect();
    if options.service_ms.keys().any(|id| !edge_ids.contains(id)) {
        return Err(invalid("Service time names an unknown edge"));
    }
    let (Some(start_node), Some(_)) = (by_node.get(origin), by_node.get(target)) else {
        return Err(invalid("Route endpoint is not a node of the graph"));
    };
    let start = NodeRef {
        id: start_node.id.clone(),
        map: start_node.position.map.clone(),
    };

    let mut sorted: Vec<&TravelEdge> = edges.iter().collect();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));

    let mut exclusions = Vec::new();
    let mut usable = Vec::new();
    for edge in sorted {
        control.checkpoint(SearchPhase::EdgeAssessment)?;
        let (Some(from), Some(to)) = (by_node.get(&edge.from.id), by_node.get(&edge.to.id)) else {
            return Err(invalid("Travel edge references an unknown node"));
        };
        if edge.from.map != from.position.map || edge.to.map != to.position.map {
            exclusions.push(ExcludedEdge {
                edge: edge.id.clone(),
                reason: TravelExclusion::MapMismatch,
            });
            continue;
        }
        match assess(edge, context, options)? {
            Ok(prepared) if prepared.stateful && edge.to.id != *target => {
                exclusions.push(ExcludedEdge {
                    edge: edge.id.clone(),
                    reason: TravelExclusion::StatefulNotFinalHop,
                });
            }
            Ok(prepared) => usable.push(prepared),
            Err(reason) => exclusions.push(ExcludedEdge {
                edge: edge.id.clone(),
                reason,
            }),
        }
    }
    let prepared: BTreeMap<&EdgeId, &Prepared<'_>> =
        usable.iter().map(|item| (&item.edge.id, item)).collect();
    let mut adjacency: BTreeMap<&NodeId, Vec<&Prepared<'_>>> = BTreeMap::new();
    for item in usable.iter().filter(|item| !item.stateful) {
        control.checkpoint(SearchPhase::EdgeAssessment)?;
        adjacency.entry(&item.edge.from.id).or_default().push(item);
    }

    // Dijkstra. The label (cost, hops, edge ids) is totally ordered and grows
    // strictly along any extension, so settled labels are optimal even with
    // zero-cost edges and cycles.
    let mut best: BTreeMap<NodeId, Label> = BTreeMap::new();
    let mut settled: BTreeMap<NodeId, Label> = BTreeMap::new();
    let mut heap = BinaryHeap::new();
    best.insert(origin.clone(), (0, 0, Vec::new()));
    heap.push(Reverse((
        0u64,
        0usize,
        Vec::<EdgeId>::new(),
        origin.clone(),
    )));
    while let Some(Reverse((cost, hops, path, node))) = heap.pop() {
        control.checkpoint(SearchPhase::RouteExpansion)?;
        if settled.contains_key(&node) {
            continue;
        }
        for item in adjacency.get(&node).into_iter().flatten() {
            control.checkpoint(SearchPhase::RouteExpansion)?;
            let next = &item.edge.to.id;
            if settled.contains_key(next) {
                continue;
            }
            let mut next_path = path.clone();
            next_path.push(item.edge.id.clone());
            let label = (add(cost, item.cost)?, hops + 1, next_path);
            if best.get(next).is_none_or(|known| label < *known) {
                best.insert(next.clone(), label.clone());
                heap.push(Reverse((label.0, label.1, label.2, next.clone())));
            }
        }
        settled.insert(node, (cost, hops, path));
    }

    // Candidates: the plain route and every verified stateful edge that closes
    // a settled node onto the target. The same label order decides.
    let mut chosen: Option<Label> = settled.get(target).cloned();
    for item in usable.iter().filter(|item| item.stateful) {
        control.checkpoint(SearchPhase::RouteExpansion)?;
        let Some(base) = settled.get(&item.edge.from.id) else {
            continue;
        };
        let mut path = base.2.clone();
        path.push(item.edge.id.clone());
        let label = (add(base.0, item.cost)?, base.1 + 1, path);
        if chosen.as_ref().is_none_or(|known| label < *known) {
            chosen = Some(label);
        }
    }

    let outcome = match chosen {
        Some(label) => RouteOutcome::Complete {
            route: build_route(start, &label, &prepared, control)?,
        },
        None => {
            let mut frontier: Option<&Label> = None;
            for label in settled.values() {
                control.checkpoint(SearchPhase::RouteExpansion)?;
                if frontier.is_none_or(|known| label.0 > known.0) {
                    frontier = Some(label);
                }
            }
            let label = frontier.ok_or_else(|| invalid("Search did not settle the origin"))?;
            RouteOutcome::Incomplete {
                target: target.clone(),
                reached: build_route(start, label, &prepared, control)?,
                reachable: settled.keys().cloned().collect(),
            }
        }
    };
    control.checkpoint(SearchPhase::RouteExpansion)?;
    Ok(RouteSearch {
        outcome,
        exclusions,
    })
}
