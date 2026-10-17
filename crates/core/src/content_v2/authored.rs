//! Authored-content rules (task V2.4): original writing may state assumptions,
//! checklists and advice, never game facts. Run after [`super::decode`]; this
//! adds refusals on top of the graph validation and never relaxes it.
//!
//! An authored catalog cites only its declared authored source and carries no
//! game-fact records: no quests, objectives, items, acquisitions, travel nodes or
//! edges, imagery, or equipment/quest/acquisition/node facts. Fields of a dungeon
//! or model that are game facts (entrance, access, preparation quests, travel,
//! rewards, proficiency, slot layout) must stay unavailable. Predicates may only
//! read declared state (level, role, preferences), never quest, objective,
//! equipment, location or participant facts. Map spaces exist only as
//! `authored:` placeholder identities with no transform, and model classes use
//! the authored `class:` vocabulary until a projection join supplies the client
//! identity. Passing these rules is not a claim that the advice is correct.
use super::ReferenceContent;
use crate::{
    entities::Confidence,
    gear::WeightSource,
    observation::{Observation, ObservationStatus},
    predicate::{Field, Predicate, PredicateNode},
    types::SourceId,
};

type Res = std::result::Result<(), &'static str>;

/// Prefix of authored placeholder map identities (never a client map ID).
pub const AUTHORED_MAP_PREFIX: &str = "authored:";
/// Prefix of the authored class vocabulary, joined to client classes later.
pub const AUTHORED_CLASS_PREFIX: &str = "class:";

fn declared_state(node: &PredicateNode) -> bool {
    let field = |f: &Field| matches!(f, Field::Level | Field::Role | Field::Preference { .. });
    match node {
        PredicateNode::All { predicates } | PredicateNode::Any { predicates } => {
            predicates.iter().all(declared_state)
        }
        PredicateNode::Not { predicate } => declared_state(predicate),
        PredicateNode::Known { field: f }
        | PredicateNode::Equals { field: f, .. }
        | PredicateNode::GreaterThanOrEqual { field: f, .. } => field(f),
        PredicateNode::LevelAtLeast { .. }
        | PredicateNode::RoleIs { .. }
        | PredicateNode::UserPreference { .. } => true,
        PredicateNode::QuestState { .. }
        | PredicateNode::ObjectiveState { .. }
        | PredicateNode::EquipmentMatches { .. }
        | PredicateNode::LocationInSupportedArea { .. }
        | PredicateNode::ParticipantState { .. } => false,
    }
}
fn predicate(p: &Observation<Predicate>) -> Res {
    match p.value() {
        Some(p) if !declared_state(p) => {
            Err("Authored predicate reads game facts instead of declared state")
        }
        _ => Ok(()),
    }
}
fn pending<T>(value: &Observation<T>) -> Res {
    if value.status() == ObservationStatus::Known {
        return Err("Authored content cannot declare a game fact");
    }
    Ok(())
}

/// Refuse game facts in a decoded authored catalog whose evidence must all come
/// from `source`.
pub fn validate_authored(content: &ReferenceContent, source: &SourceId) -> Res {
    let reference = &content.reference;
    let data = &reference.payload;
    if reference.fixture {
        return Err("Authored content is not a fixture");
    }
    if data.evidence.is_empty()
        || data.evidence.iter().any(|e| {
            &e.source != source
                || matches!(
                    e.confidence,
                    Confidence::VerifiedOnBuild | Confidence::CrossEditionUnverified
                )
        })
    {
        return Err("Authored evidence must cite only the authored source, unverified on build");
    }
    if !(data.quests.is_empty()
        && data.objectives.is_empty()
        && data.items.is_empty()
        && data.acquisitions.is_empty()
        && data.nodes.is_empty()
        && data.edges.is_empty()
        && data.map_images.is_empty()
        && data.map_embeddings.is_empty()
        && content.equipment.is_empty()
        && content.quest_facts.is_empty()
        && content.acquisition_facts.is_empty()
        && content.node_facts.is_empty())
    {
        return Err("Authored content cannot contain game-fact records");
    }
    for map in &data.map_spaces {
        if !map.key.map_id.as_str().starts_with(AUTHORED_MAP_PREFIX)
            || !map.key.floor.as_str().starts_with(AUTHORED_MAP_PREFIX)
        {
            return Err("Authored map spaces must be placeholder identities");
        }
        pending(&map.transform)?;
    }
    for model in &content.models {
        if !model.class.as_str().starts_with(AUTHORED_CLASS_PREFIX) {
            return Err("Authored model class must use the authored vocabulary");
        }
        match &model.profile.weight_source {
            WeightSource::DocumentedModel { source: s } if s == source => {}
            _ => return Err("Authored model weights must cite the authored source"),
        }
        pending(&model.proficiency)?;
        pending(&model.layout)?;
        predicate(&model.applicability)?;
        if let Some(exclusions) = model.exclusions.value()
            && !exclusions.iter().all(|p| declared_state(p))
        {
            return Err("Authored predicate reads game facts instead of declared state");
        }
    }
    for dungeon in &content.dungeons {
        if !dungeon.map.map_id.as_str().starts_with(AUTHORED_MAP_PREFIX) {
            return Err("Authored map spaces must be placeholder identities");
        }
        pending(&dungeon.entrance)?;
        pending(&dungeon.access)?;
        pending(&dungeon.preparation_quests)?;
        pending(&dungeon.travel_edges)?;
        pending(&dungeon.rewards)?;
        for check in dungeon.checklist.value().into_iter().flatten() {
            predicate(&check.condition)?;
        }
    }
    for rule in &data.rules {
        if !declared_state(&rule.predicate) {
            return Err("Authored predicate reads game facts instead of declared state");
        }
    }
    Ok(())
}
