//! Exact imported identities and unverified authored route designs, independent of I/O.
use crate::{local_data::is_sha256, local_data_inputs::InputTuple, release::DOMAINS};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RecordKind {
    ClientProjection,
    PermittedObservation,
    AuthoredDesign,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub id: String,
    pub source_sha256: String,
    pub kind: RecordKind,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Domain {
    pub domain: String,
    pub imported_count: u32,
    pub records: Vec<Record>,
    pub source_available: bool,
    pub gaps: Vec<String>,
    /// None is unknown, not an empty independently enumerated target.
    pub independently_required_count: Option<u32>,
    pub independently_complete: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    pub format_version: u32,
    pub delivery_sha256: String,
    pub tuple: InputTuple,
    pub authored_manifest_sha256: Option<String>,
    pub authored_catalog_sha256: Option<String>,
    pub domains: Vec<Domain>,
    pub local_only: bool,
    pub acceptance_ready: bool,
}
impl Inventory {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.tuple.validate()?;
        if self.format_version != 1
            || !is_sha256(&self.delivery_sha256)
            || !self.local_only
            || self.acceptance_ready
            || self
                .domains
                .iter()
                .map(|d| d.domain.as_str())
                .collect::<Vec<_>>()
                != DOMAINS
            || self.authored_manifest_sha256.is_some() != self.authored_catalog_sha256.is_some()
            || [
                &self.authored_manifest_sha256,
                &self.authored_catalog_sha256,
            ]
            .into_iter()
            .flatten()
            .any(|h| !is_sha256(h))
        {
            return Err("Invalid research inventory");
        }
        let mut total = 0usize;
        for domain in &self.domains {
            total += domain.records.len();
            if total > 250_000
                || domain.imported_count as usize != domain.records.len()
                || domain.independently_complete
                || domain.independently_required_count.is_some()
                || domain.gaps.len() > 4096
                || domain.gaps.iter().any(|g| g.is_empty() || g.len() > 512)
                || domain.gaps.windows(2).any(|w| w[0] >= w[1])
                || domain.records.windows(2).any(|w| w[0] >= w[1])
                || !domain.source_available && !domain.records.is_empty()
            {
                return Err("Invalid imported inventory domain");
            }
            let mut identities = BTreeSet::new();
            for record in &domain.records {
                if !is_sha256(&record.source_sha256)
                    || record.id.len() > 1024
                    || record.id.is_empty()
                    || !identities.insert(&record.id)
                {
                    return Err("Invalid exact inventory identity");
                }
                let id: Vec<serde_json::Value> = serde_json::from_str(&record.id)
                    .map_err(|_| "Invalid inventory identity encoding")?;
                if id.len() < 3
                    || id.len() > 8
                    || serde_json::to_string(&id).map_err(|_| "Invalid inventory identity")?
                        != record.id
                {
                    return Err("Noncanonical inventory identity");
                }
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Solo,
    Duo,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RouteDesign {
    pub id: String,
    pub class: String,
    pub mode: Mode,
    pub level_min: u8,
    pub level_max: u8,
    /// Authored checkpoints are a proposed review order, never calibrated travel.
    pub proposed_checkpoints: Vec<String>,
    pub required_facts: Vec<String>,
    pub authored_model_reviews: Vec<String>,
    pub verified_quest_sequence: Option<Vec<u32>>,
    pub eligibility_verified: bool,
    pub actionable_advice: bool,
    pub limitations: Vec<String>,
}
/// Original draft designs for the requested slice; private generator facts are not copied.
pub fn human_route_designs() -> Vec<RouteDesign> {
    let mut designs = Vec::new();
    for class in ["warrior", "paladin", "rogue", "priest", "mage", "warlock"] {
        for mode in [Mode::Solo, Mode::Duo] {
            let mode_name = if mode == Mode::Solo { "solo" } else { "duo" };
            let mut required_facts = vec![
                "quest_prerequisites",
                "quest_eligibility",
                "objective_access",
                "quest_rewards_and_xp",
                "travel_and_coordinate_calibration",
                "class_training_and_proficiency",
                "current_character_state",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
            if mode == Mode::Duo {
                required_facts.extend([
                    "each_participant_progress".into(),
                    "shared_objective_semantics".into(),
                    "minimum_participant_advancement".into(),
                ]);
            }
            designs.push(RouteDesign {
                id: format!("human-{class}-1-20-{mode_name}-draft-v1"),
                class: class.into(),
                mode,
                level_min: 1,
                level_max: 20,
                proposed_checkpoints: vec![
                    "Review Northshire candidates",
                    "Review Elwynn candidates",
                    "Review Westfall candidates",
                    "Review Redridge candidates",
                    "Review optional Deadmines preparation",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
                required_facts,
                authored_model_reviews: match class {
                    "warrior" => vec!["model:warrior-tank-1-20", "model:warrior-melee-damage-1-20"],
                    "paladin" => vec![
                        "model:paladin-healer-1-20",
                        "model:paladin-melee-damage-1-20",
                    ],
                    "rogue" => vec!["model:rogue-melee-damage-1-20"],
                    "priest" => vec![
                        "model:priest-healer-1-20",
                        "model:priest-ranged-damage-1-20",
                    ],
                    "mage" => vec!["model:mage-ranged-damage-1-20"],
                    _ => vec!["model:warlock-ranged-damage-1-20"],
                }
                .into_iter()
                .map(str::to_owned)
                .collect(),
                verified_quest_sequence: None,
                eligibility_verified: false,
                actionable_advice: false,
                limitations: vec![
                    "Authored proposal; class list and zone order are scope assumptions".into(),
                    "No verified quest route can be derived from observed turn-ins alone".into(),
                    "Training, class-specific detours and dungeon applicability remain unverified"
                        .into(),
                ],
            });
        }
    }
    designs
}
