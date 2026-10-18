//! Read-only bound research guide replay; private Lua stays inert data.
use crate::{Result, hash, read_file};
use azeroth_core::local_data_inventory::Mode;
use azeroth_ingest::{
    rxp::{self, Character},
    rxp_verify::{self, FactOrigin, JsonFacts},
};
use serde::Serialize;
use std::path::Path;
#[derive(Debug, Serialize)]
pub struct Review {
    pub format_version: u32,
    pub delivery_sha256: String,
    pub guide_sha256: String,
    pub facts_manifest_sha256: String,
    pub mode: Mode,
    pub acceptance_ready: bool,
    pub duo_eligibility_verified: bool,
    pub limitations: Vec<&'static str>,
    pub report: rxp_verify::Report,
}
pub fn verify(
    root: &Path,
    delivery_digest: &str,
    guide_path: &Path,
    character: Character,
    mode: Mode,
) -> Result<Review> {
    let delivery = crate::delivery::verify(root, delivery_digest)?;
    let facts_digest = delivery
        .guide_facts_sha256
        .as_deref()
        .ok_or("Delivery has no bound research guide facts")?;
    let manifest = crate::guide_facts::verify(root, facts_digest)?;
    let bytes = read_file(
        &root.join("objects").join(&manifest.facts_sha256),
        rxp_verify::MAX_FACTS_BYTES,
    )?
    .0;
    if hash(&bytes) != manifest.facts_sha256 {
        return Err("Guide facts changed after verification");
    }
    let facts = JsonFacts::parse(&bytes).map_err(|_| "Bound guide facts refused")?;
    if facts.origin != FactOrigin::UnverifiedResearch {
        return Err("Bound guide facts must remain unverified research");
    }
    let guide = read_file(guide_path, rxp::MAX_FILE_BYTES)?.0;
    let parsed = rxp::parse(&guide).map_err(|_| "Selected guide data refused")?;
    let report = rxp_verify::verify(
        &parsed,
        &facts,
        character,
        Some(FactOrigin::UnverifiedResearch),
    );
    if report.facts_are_evidence || report.passes {
        return Err("Research guide replay attempted an evidence claim");
    }
    let mut limitations = vec![
        "observations_do_not_establish_universal_prerequisites_or_eligibility",
        "research_replay_is_not_release_evidence",
        "class_training_travel_and_rewards_remain_unverified",
    ];
    if mode == Mode::Duo {
        limitations.push("single_character_replay_cannot_verify_duo_eligibility_or_fairness");
    }
    Ok(Review {
        format_version: 1,
        delivery_sha256: delivery_digest.into(),
        guide_sha256: hash(&guide),
        facts_manifest_sha256: facts_digest.into(),
        mode,
        acceptance_ready: false,
        duo_eligibility_verified: false,
        limitations,
        report,
    })
}
