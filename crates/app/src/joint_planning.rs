//! Local manual planning over a consistent party/content/snapshot view.
use crate::catalog::CatalogView;
use crate::snapshots::{SnapshotApplication, context, profile_id};
use azeroth_core::search_control::{NoopSearchControl, SearchControl, SearchPhase};
use azeroth_core::{
    contract::Envelope,
    entities::{
        CharacterSnapshot, Objective, ObjectiveTarget, PartySnapshot, PartySnapshotData, TravelNode,
    },
    error::{AppError, ErrorCode, Result},
    evaluation::{EvaluationContext, StalePolicy},
    joint_planner::{JointCandidate, JointInput, JointResult, plan_joint_clustered_controlled},
    observation::{Freshness, ObservationStatus, freshness},
    predicate::{Predicate, PredicateNode, QuestState, RuleLimits},
    types::*,
};
use azeroth_storage::content::ActiveContentRevision;
use azeroth_storage::{SnapshotStore, SqliteStore, content::ContentScope};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Serialize)]
pub struct JointMember {
    pub profile_id: String,
    pub character: CharacterId,
    pub snapshot_id: Option<SnapshotId>,
    pub freshness: Option<Freshness>,
}
#[derive(Clone, Debug, Serialize)]
pub struct JointPlanningCatalog {
    pub context: DataContext,
    pub fixture: bool,
    pub revision: Option<DatasetRevision>,
    pub participants: Vec<JointMember>,
    pub nodes: Vec<TravelNode>,
    pub objectives: Vec<Objective>,
}
pub use azeroth_storage::today::{JointEstimate, JointParticipantInput, JointPlanningRequest};
#[derive(Clone, Debug, Serialize)]
pub struct JointFilteredCandidate {
    pub objective: ObjectiveId,
    pub reason: &'static str,
}
#[derive(Clone, Debug, Serialize)]
pub struct JointPlanningReport {
    pub request: JointPlanningRequest,
    pub context: DataContext,
    pub fixture: bool,
    pub participants: Vec<JointMember>,
    pub result: JointResult,
    pub clustering: crate::clustering::ClusteringMetadata,
    pub filtered: Vec<JointFilteredCandidate>,
    pub assumptions: Vec<&'static str>,
}
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
type JointView = (
    JointPlanningCatalog,
    Vec<Option<Envelope<CharacterSnapshot>>>,
    Option<ActiveContentRevision>,
);
pub(crate) fn view(store: &SqliteStore, id: &PartyId, now: Option<UtcMillis>) -> Result<JointView> {
    view_controlled(store, id, now, &mut NoopSearchControl)
}
fn view_controlled(
    store: &SqliteStore,
    id: &PartyId,
    now: Option<UtcMillis>,
    control: &mut dyn SearchControl,
) -> Result<JointView> {
    control.checkpoint(SearchPhase::ParticipantReadiness)?;
    let party = store
        .party(id)?
        .ok_or_else(|| invalid("No such local party"))?;
    if !(2..=40).contains(&party.participants.len()) {
        return Err(invalid("Joint planning requires at least two participants"));
    }
    let first = &party.participants[0];
    let content = store.active_catalog_revision(&ContentScope {
        context: first.context.clone(),
        fixture: first.fixture,
    })?;
    let clock = context(now);
    let mut snapshots = Vec::new();
    let mut participants = Vec::new();
    for key in &party.participants {
        control.checkpoint(SearchPhase::ParticipantReadiness)?;
        first.context.ensure_compatible(&key.context)?;
        if key.fixture != first.fixture {
            return Err(invalid("Party fixture modes differ"));
        }
        let snapshot = store.current_snapshot(key)?;
        participants.push(JointMember {
            profile_id: profile_id(key)?,
            character: key.character.clone(),
            snapshot_id: snapshot.as_ref().map(|s| s.payload.id.clone()),
            freshness: snapshot
                .as_ref()
                .map(|s| freshness(&s.payload.capture, &clock)),
        });
        snapshots.push(snapshot);
    }
    let catalog = JointPlanningCatalog {
        context: first.context.clone(),
        fixture: first.fixture,
        revision: content.as_ref().map(|c| c.reference().revision.clone()),
        participants,
        nodes: content
            .as_ref()
            .map(|c| c.reference().nodes.clone())
            .unwrap_or_default(),
        objectives: content
            .as_ref()
            .map(|c| c.reference().objectives.clone())
            .unwrap_or_default(),
    };
    Ok((catalog, snapshots, content))
}
impl SnapshotApplication {
    pub fn joint_planning_catalog(
        &self,
        id: &PartyId,
        now: Option<UtcMillis>,
    ) -> Result<JointPlanningCatalog> {
        self.with_store_mut(|store| Ok(view(store, id, now)?.0))
    }
    pub fn plan_joint_session(
        &self,
        text: &str,
        now: Option<UtcMillis>,
    ) -> Result<JointPlanningReport> {
        self.with_joint_planning(text, now, |request, catalog, snapshots, content| {
            calculate(request, catalog, snapshots, content, now)
        })
    }
    pub(crate) fn with_joint_planning<T>(
        &self,
        text: &str,
        now: Option<UtcMillis>,
        calculate: impl FnOnce(
            JointPlanningRequest,
            JointPlanningCatalog,
            Vec<Envelope<CharacterSnapshot>>,
            &dyn CatalogView,
        ) -> Result<T>,
    ) -> Result<T> {
        self.with_joint_planning_controlled(
            text,
            now,
            &mut NoopSearchControl,
            |request, catalog, snapshots, content, _| {
                calculate(request, catalog, snapshots, content)
            },
        )
    }
    pub(crate) fn with_joint_planning_controlled<T>(
        &self,
        text: &str,
        now: Option<UtcMillis>,
        control: &mut dyn SearchControl,
        calculate: impl FnOnce(
            JointPlanningRequest,
            JointPlanningCatalog,
            Vec<Envelope<CharacterSnapshot>>,
            &dyn CatalogView,
            &mut dyn SearchControl,
        ) -> Result<T>,
    ) -> Result<T> {
        self.with_joint_planning_guarded(
            text,
            now,
            control,
            |request, catalog, snapshots, content, control, _| {
                calculate(request, catalog, snapshots, content, control)
            },
        )
    }
    pub(crate) fn with_joint_planning_guarded<T>(
        &self,
        text: &str,
        now: Option<UtcMillis>,
        control: &mut dyn SearchControl,
        calculate: impl FnOnce(
            JointPlanningRequest,
            JointPlanningCatalog,
            Vec<Envelope<CharacterSnapshot>>,
            &dyn CatalogView,
            &mut dyn SearchControl,
            &crate::planning_job_service::PublicationGuard,
        ) -> Result<T>,
    ) -> Result<T> {
        control.checkpoint(SearchPhase::ValidateInputs)?;
        let started = std::time::Instant::now();
        if text.len() > crate::planning::MAX_REQUEST_BYTES {
            return Err(AppError::new(
                ErrorCode::ImportTooLarge,
                "Joint planning request exceeds size limit",
                false,
            ));
        }
        let request: JointPlanningRequest =
            serde_json::from_str(text).map_err(|_| invalid("Invalid joint planning request"))?;
        if request.candidates.len() > 128
            || request.excluded.len() > 128
            || !(2..=40).contains(&request.participants.len())
        {
            return Err(invalid("Joint input count exceeds limits"));
        }
        let (catalog, snapshots, content, guard) = self.with_store_mut(|store| store.with_immediate_read(|store| {
            let (catalog, snapshots, content) =
                view_controlled(store, &request.party_id, now, control)?;
            let content = content.ok_or_else(|| {
                AppError::new(
                    ErrorCode::DatasetIncomplete,
                    "No active party content",
                    false,
                )
            })?;
            if catalog.revision.as_ref() != Some(&request.revision) {
                return Err(invalid("Active content changed; refresh joint planning"));
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
            {
                return Err(AppError::new(
                    ErrorCode::SnapshotOutdated,
                    "Party membership or a participant snapshot changed; refresh joint planning",
                    false,
                ));
            }
            if catalog
                .participants
                .iter()
                .any(|p| !matches!(p.freshness, Some(Freshness::Fresh { .. })))
            {
                return Err(AppError::new(
                    ErrorCode::SnapshotOutdated,
                    "Every participant requires a fresh trusted capture",
                    false,
                ));
            }
            let snapshots = snapshots
                .into_iter()
                .map(|s| s.expect("expected snapshot checked"))
                .collect::<Vec<_>>();
            let guard=crate::planning_job_service::PublicationGuard::new(Some(request.party_id.clone()),&snapshots,&content,&context(now))?.bind_disposition(store.usable_catalog_revision(&content.key())?.ok_or_else(||invalid("Active content unavailable"))?.1.guard())?;
            Ok((catalog,snapshots,content,guard))
        }))?;
        let result = calculate(request, catalog, snapshots, &content, control, &guard)?;
        guard.publish(
            self,
            crate::planning_job_service::advanced_now(now, started.elapsed()),
            &mut || Ok(()),
        )?;
        Ok(result)
    }
}
pub(crate) fn calculate(
    request: JointPlanningRequest,
    catalog: JointPlanningCatalog,
    snapshots: Vec<Envelope<CharacterSnapshot>>,
    content: &dyn CatalogView,
    now: Option<UtcMillis>,
) -> Result<JointPlanningReport> {
    calculate_controlled(
        request,
        catalog,
        snapshots,
        content,
        now,
        &mut NoopSearchControl,
    )
}
pub(crate) fn calculate_controlled(
    request: JointPlanningRequest,
    catalog: JointPlanningCatalog,
    snapshots: Vec<Envelope<CharacterSnapshot>>,
    content: &dyn CatalogView,
    now: Option<UtcMillis>,
    control: &mut dyn SearchControl,
) -> Result<JointPlanningReport> {
    control.checkpoint(SearchPhase::ValidateInputs)?;
    if request.candidates.len() > 128
        || request.excluded.len() > 128
        || !(2..=40).contains(&request.participants.len())
    {
        return Err(invalid("Joint planning input exceeds bounds"));
    }
    let dataset = &content.reference();
    if request
        .excluded
        .iter()
        .any(|id| !dataset.objectives.iter().any(|o| &o.id == id))
    {
        return Err(invalid(
            "Excluded joint objective is absent from active content",
        ));
    }
    let clock = context(now);
    let party = PartySnapshot::try_new(PartySnapshotData {
        id: request.party_id.clone(),
        context: catalog.context.clone(),
        participants: snapshots.iter().map(|s| s.payload.clone()).collect(),
    })?;
    let contexts = snapshots
        .iter()
        .map(|s| EvaluationContext {
            snapshot: &s.payload,
            party: Some(&party),
            freshness: &clock,
            stale_policy: StalePolicy::Reject,
            limits: RuleLimits::default(),
        })
        .collect::<Vec<_>>();
    let members = catalog
        .participants
        .iter()
        .map(|p| p.character.clone())
        .collect::<BTreeSet<_>>();
    let profile_ids = catalog
        .participants
        .iter()
        .map(|p| p.profile_id.clone())
        .collect::<BTreeSet<_>>();
    let mut prepared = Vec::new();
    let mut filtered = Vec::new();
    let mut seen = BTreeSet::new();
    for estimate in &request.candidates {
        control.checkpoint(SearchPhase::CandidateAssessment)?;
        if !seen.insert(&estimate.objective)
            || estimate.utilities.keys().cloned().collect::<BTreeSet<_>>() != profile_ids
        {
            return Err(invalid(
                "Duplicate candidate or utilities do not exactly match party profiles",
            ));
        }
        let objective = dataset
            .objectives
            .iter()
            .find(|o| o.id == estimate.objective)
            .ok_or_else(|| invalid("Joint objective is absent from active content"))?;
        if !dataset.nodes.iter().any(|n| n.id == estimate.node) {
            return Err(invalid(
                "Joint objective node is absent from active content",
            ));
        }
        if objective
            .participants
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            != members
        {
            filtered.push(JointFilteredCandidate { objective: objective.id.clone(), reason: "Authored objective participants differ from this party; no participant contribution is inferred" });
            continue;
        }
        let mut predicates = Vec::new();
        let mut rewards = BTreeSet::new();
        let mut reason = None;
        match (
            objective.requirements.value(),
            objective.requirements.status(),
        ) {
            (Some(p), _) => predicates.push((**p).clone()),
            (None, ObservationStatus::NotApplicable) => {}
            _ => reason = Some("Objective requirements are unavailable"),
        }
        for target in &objective.targets {
            control.checkpoint(SearchPhase::CandidateAssessment)?;
            match target {
                ObjectiveTarget::QuestStage { quest, stage } => {
                    let definition = dataset
                        .quests
                        .iter()
                        .find(|q| &q.id == quest)
                        .ok_or_else(|| invalid("Quest target is absent from active content"))?;
                    if !definition.stages.contains(stage) {
                        return Err(invalid("Stage is not declared by its quest"));
                    }
                    // Never plan speculative catch-up: accepted/incomplete stage facts
                    // are independently required from each imported participant.
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
                    rewards.insert(SourceId::new(format!("quest:{}", quest.as_str()))?);
                }
                ObjectiveTarget::Service { node } => {
                    if node.id != estimate.node
                        || !dataset
                            .nodes
                            .iter()
                            .any(|n| n.id == node.id && n.position.map == node.map)
                    {
                        reason = Some("Service objective must use its declared node");
                    }
                    rewards.insert(SourceId::new(format!("service:{}", node.id.as_str()))?);
                }
                ObjectiveTarget::Acquisition { .. } => {
                    reason = Some("Acquisition requirements are not integrated into joint planning")
                }
            }
        }
        if let Some(reason) = reason {
            filtered.push(JointFilteredCandidate {
                objective: objective.id.clone(),
                reason,
            });
            continue;
        }
        prepared.push((
            estimate,
            Predicate::try_new(PredicateNode::All { predicates })?,
            rewards,
        ));
    }
    let candidates = prepared
        .iter()
        .map(|(e, predicate, rewards)| JointCandidate {
            id: e.objective.clone(),
            node: e.node.clone(),
            readiness: vec![predicate; contexts.len()],
            utilities: catalog
                .participants
                .iter()
                .map(|p| e.utilities[&p.profile_id])
                .collect(),
            completion_ms: e.completion_ms,
            service_ms: e.service_ms,
            rewards: rewards.clone(),
        })
        .collect::<Vec<_>>();
    filtered.sort_by(|a, b| a.objective.cmp(&b.objective));
    let (result, clustered) = plan_joint_clustered_controlled(
        &JointInput {
            contexts: &contexts,
            candidates: &candidates,
            goal_profile: request.goal_profile.clone(),
            excluded: &request.excluded,
            nodes: &dataset.nodes,
            edges: &dataset.edges,
            origin: request.origin.clone(),
            stopping_node: request.stopping_node.clone(),
            budget_ms: request.budget_ms,
            stopping_service_ms: request.stopping_service_ms,
        },
        control,
    )?;
    Ok(JointPlanningReport {
        request,
        context: catalog.context,
        fixture: catalog.fixture,
        participants: catalog.participants,
        result,
        clustering: crate::clustering::ClusteringMetadata::from(clustered),
        filtered,
        assumptions: vec![
            "Utilities share an explicit goal profile; cluster utility is the minimum participant sum of original contributions and every member must benefit",
            "Origin, stopping point, utility and duration are manual assumptions, not observed positions or measured rewards",
            "Joint readiness and travel requirements are checked independently for every participant; blocked catch-up requires separate eligibility review",
            "Expected-time same-node clustered greedy estimate; no inferred shared-kill/service savings, speculative unlocks, quest sharing claim or live synchronization",
        ],
    })
}
