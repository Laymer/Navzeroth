//! Explicit authored-over-historical dungeon enrichment (ADR 0007 §4).
//! Callers establish source categories; this pure helper never promotes evidence.
use crate::{
    content_v2::{Dungeon, DungeonCheck},
    error::{Result, invalid},
    observation::{Observation, ObservationStatus},
    types::SourceId,
};
use serde::{Deserialize, Serialize};

/// Internal persisted conflict keys, not public addon exchange fields.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DungeonField {
    Title,
    Map,
    Entrance,
    Access,
    PreparationQuests,
    TravelEdges,
    Rewards,
    ChecklistCollection,
    Checklist { check: SourceId },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Disagreement {
    pub field: DungeonField,
    /// Only the documented pending-map sentinel chooses generated geometry.
    pub generated_preferred: bool,
}

fn union<T: Clone + Ord>(preferred: &mut Vec<T>, other: &[T]) {
    let mut seen: std::collections::BTreeSet<_> = preferred.iter().cloned().collect();
    preferred.extend(
        other
            .iter()
            .filter(|value| seen.insert((*value).clone()))
            .cloned(),
    );
}

fn observation<T: Clone + PartialEq>(
    preferred: &Observation<T>,
    other: &Observation<T>,
    field: DungeonField,
    conflicts: &mut Vec<Disagreement>,
) -> Observation<T> {
    // Only an explicitly unknown authored declaration can inherit another state.
    // Restricted, unsupported, not-applicable and known false/zero/empty survive.
    if preferred.status() == ObservationStatus::Unknown {
        return if other.status() == ObservationStatus::Unknown {
            preferred.clone()
        } else {
            other.clone()
        };
    }
    if other.status() != ObservationStatus::Unknown
        && (preferred.status() != other.status() || preferred.value() != other.value())
    {
        conflicts.push(Disagreement {
            field,
            generated_preferred: false,
        });
    }
    preferred.clone()
}

fn checklist(
    preferred: &Observation<Vec<DungeonCheck>>,
    other: &Observation<Vec<DungeonCheck>>,
    conflicts: &mut Vec<Disagreement>,
) -> Result<Observation<Vec<DungeonCheck>>> {
    let (Some(high), Some(low)) = (preferred.value(), other.value()) else {
        // The key identifies a whole restricted/unsupported collection here.
        return Ok(observation(
            preferred,
            other,
            DungeonField::ChecklistCollection,
            conflicts,
        ));
    };
    let mut checks = high.clone();
    for candidate in low {
        if let Some(check) = checks.iter_mut().find(|check| check.id == candidate.id) {
            let field = DungeonField::Checklist {
                check: check.id.clone(),
            };
            if check.label != candidate.label {
                conflicts.push(Disagreement {
                    field: field.clone(),
                    generated_preferred: false,
                });
            }
            check.condition = observation(&check.condition, &candidate.condition, field, conflicts);
            union(&mut check.evidence, &candidate.evidence);
        } else {
            checks.push(candidate.clone());
        }
    }
    // Independently identified checks are additive, including estimate warnings.
    Observation::known(checks, preferred.metadata().clone())
}

pub fn enrich(authored: &Dungeon, historical: &Dungeon) -> Result<(Dungeon, Vec<Disagreement>)> {
    if authored.id != historical.id {
        return Err(invalid("Dungeon enrichment requires one exact identity"));
    }
    authored
        .map
        .context
        .ensure_compatible(&historical.map.context)?;
    authored.reference_limit()?;
    historical.reference_limit()?;
    let mut merged = authored.clone();
    let mut conflicts = vec![];
    if authored.title != historical.title {
        conflicts.push(Disagreement {
            field: DungeonField::Title,
            generated_preferred: false,
        });
    }
    if authored.map != historical.map {
        // Exact marker in the admitted authored slice-v1 contract; never match
        // arbitrary authored:* names or real authored geometry.
        let pending = authored.id.as_str() == "dungeon:the-deadmines"
            && authored.map.map_id.as_str() == "authored:the-deadmines"
            && authored.map.floor.as_str() == "authored:pending"
            && authored.map.instance.is_none();
        if pending {
            merged.map = historical.map.clone();
        }
        conflicts.push(Disagreement {
            field: DungeonField::Map,
            generated_preferred: pending,
        });
    }
    merged.entrance = observation(
        &authored.entrance,
        &historical.entrance,
        DungeonField::Entrance,
        &mut conflicts,
    );
    merged.access = observation(
        &authored.access,
        &historical.access,
        DungeonField::Access,
        &mut conflicts,
    );
    // Bounds are structural, not a game fact. Both inputs were validated; retaining
    // the larger profile allows inherited complete lists without changing precedence.
    merged.reference_contract_version = authored
        .reference_contract_version
        .into_iter()
        .chain(historical.reference_contract_version)
        .max();
    merged.preparation_quests = observation(
        &authored.preparation_quests,
        &historical.preparation_quests,
        DungeonField::PreparationQuests,
        &mut conflicts,
    );
    merged.travel_edges = observation(
        &authored.travel_edges,
        &historical.travel_edges,
        DungeonField::TravelEdges,
        &mut conflicts,
    );
    merged.rewards = observation(
        &authored.rewards,
        &historical.rewards,
        DungeonField::Rewards,
        &mut conflicts,
    );
    merged.checklist = checklist(&authored.checklist, &historical.checklist, &mut conflicts)?;
    union(&mut merged.bosses, &historical.bosses);
    union(&mut merged.evidence, &historical.evidence);
    conflicts.sort();
    conflicts.dedup();
    Ok((merged, conflicts))
}
