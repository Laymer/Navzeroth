//! Deterministic research guide facts from verified projection/input closures.
use crate::{Result, bounded_json, checked_path, extraction::Store, hash, read_file};
use azeroth_core::{
    contract::AnyExchange,
    local_data::{PerformancePreset, is_sha256},
    local_data_guide::{GuideFactsManifest, Observations, QuestObservation},
    local_data_inputs::{InputKind, InputTuple},
    local_data_projection::{Fact, LocalProjection},
    observation::{Observation, ObservationMetadata, StateSource},
    sessions::{SessionDocument, SessionEventKind},
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
const MAX_OBSERVATIONS: usize = 16_384;
const MAX_BYTES: usize = 16 * 1024 * 1024;
#[derive(Debug, Serialize)]
pub struct Summary {
    pub guide_facts_sha256: String,
    pub manifest: GuideFactsManifest,
    pub observations: u32,
    pub conflicts: u32,
    pub quest_records: u32,
    pub zone_names: u32,
    pub flight_names: u32,
}
fn object(root: &Path, digest: &str, maximum: usize) -> Result<Vec<u8>> {
    if !is_sha256(digest) {
        return Err("Invalid guide object digest");
    }
    let bytes = read_file(&root.join("objects").join(digest), maximum)?.0;
    if hash(&bytes) != digest {
        return Err("Guide object hash mismatch");
    }
    Ok(bytes)
}
fn numeric_quest(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 9
        && !value.starts_with('0')
        && value.bytes().all(|b| b.is_ascii_digit())
}
fn add_name(names: &mut BTreeSet<String>, name: &str, gaps: &mut BTreeSet<String>) -> Result<()> {
    if name.trim() != name || name.is_empty() || name.len() > azeroth_ingest::rxp::MAX_LABEL_BYTES {
        gaps.insert("client_label_unsupported".into());
        return Ok(());
    }
    names.insert(name.into());
    if names.len() > azeroth_ingest::rxp_verify::MAX_FACT_NAMES {
        return Err("Guide name quota exceeded");
    }
    Ok(())
}
fn candidates(
    root: &Path,
    projection: &LocalProjection,
    inputs: &[String],
    gaps: &mut BTreeSet<String>,
) -> Result<Observations> {
    let mut candidates = Vec::new();
    for digest in inputs {
        let manifest = crate::inputs::verify(root, digest)?;
        if manifest.catalog_sha256 != projection.catalog_sha256
            || manifest.catalog_tuple.product != projection.product
            || manifest.catalog_tuple.version != projection.version
            || manifest.catalog_tuple.build != projection.build
            || manifest.catalog_tuple.locale != projection.locale
        {
            return Err("Guide input differs from projection tuple or catalog");
        }
        if manifest.fixture {
            gaps.insert("synthetic_input_present".into());
        }
        let analysis = object(root, &manifest.analysis_sha256, 32 * 1024 * 1024)?;
        match manifest.kind {
            InputKind::CharacterSnapshot => {
                let decoded = azeroth_core::contract::decode_any(
                    std::str::from_utf8(&analysis).map_err(|_| "Invalid snapshot analysis")?,
                )
                .map_err(|_| "Invalid snapshot analysis")?;
                let AnyExchange::CharacterSnapshot(snapshot) = decoded else {
                    return Err("Input is not a character snapshot");
                };
                if snapshot.game.edition.as_str() != "wow_forever"
                    || snapshot.game.interface.as_str() != "16001"
                {
                    return Err("Unsupported observation edition/interface binding");
                }
                for (quest_id, quest) in snapshot.payload.quests.iter() {
                    for (property, value) in [
                        ("completed", &quest.completed),
                        ("turned_in", &quest.turned_in),
                    ] {
                        candidates.push(QuestObservation {
                            input_sha256: digest.clone(),
                            quest_id: quest_id.as_str().into(),
                            character_id: snapshot.payload.character.as_str().into(),
                            context: snapshot.payload.context.clone(),
                            property: property.into(),
                            value: value.clone(),
                        });
                    }
                }
            }
            InputKind::SessionEvents => {
                let document: SessionDocument =
                    serde_json::from_slice(&analysis).map_err(|_| "Invalid session analysis")?;
                if document.batch.context.build.edition.as_str() != "wow_forever"
                    || document.batch.context.build.interface.as_str() != "16001"
                {
                    return Err("Unsupported observation edition/interface binding");
                }
                if document.locale.is_none() {
                    gaps.insert("observation_locale_unknown".into());
                }
                for event in &document.batch.events {
                    if let SessionEventKind::QuestTurnIn { quest } = &event.kind {
                        let metadata = ObservationMetadata {
                            source: if document.fixture {
                                StateSource::SyntheticFixture
                            } else {
                                StateSource::AddonSnapshot
                            },
                            clock_trust: event.clock_trust,
                            observed_at_utc_ms: event.utc,
                            session_id: Some(event.session.clone()),
                            session_elapsed_ms: Some(event.elapsed_ms),
                            sequence: Some(event.sequence),
                            ..ObservationMetadata::default()
                        };
                        candidates.push(QuestObservation {
                            input_sha256: digest.clone(),
                            quest_id: quest.as_str().into(),
                            character_id: document.batch.character.as_str().into(),
                            context: document.batch.context.clone(),
                            property: "turned_in".into(),
                            value: Observation::known(true, metadata)
                                .map_err(|_| "Invalid milestone observation")?,
                        });
                    }
                }
            }
            _ => return Err("Only permitted addon observation inputs can contribute guide facts"),
        }
        if candidates.len() > MAX_OBSERVATIONS {
            return Err("Guide observation quota exceeded");
        }
    }
    let mut groups = BTreeMap::<String, Vec<u32>>::new();
    for (i, candidate) in candidates.iter().enumerate() {
        let metadata = candidate.value.metadata();
        if metadata.session_id.is_some()
            && metadata.sequence.is_some()
            && candidate.value.value().is_some()
        {
            let key = serde_json::to_string(&(
                &candidate.quest_id,
                &candidate.character_id,
                &candidate.context,
                &candidate.property,
                &metadata.session_id,
                metadata.sequence,
            ))
            .map_err(|_| "Invalid observation identity")?;
            groups.entry(key).or_default().push(i as u32);
        }
    }
    let conflicts = groups
        .into_values()
        .filter(|indices| {
            let first = candidates[indices[0] as usize].value.value();
            indices
                .iter()
                .any(|i| candidates[*i as usize].value.value() != first)
        })
        .collect::<Vec<_>>();
    if !conflicts.is_empty() {
        gaps.insert("conflicting_observations_retained".into());
    }
    Ok(Observations {
        format_version: 1,
        candidates,
        conflicts,
    })
}
struct Derived {
    manifest: GuideFactsManifest,
    facts: Vec<u8>,
    observations: Vec<u8>,
    quest_records: u32,
    zone_names: u32,
    flight_names: u32,
}
fn derive(root: &Path, projection_digest: &str, inputs: &[String]) -> Result<Derived> {
    let verified = crate::projection::verify_projection(root, projection_digest)?;
    if !verified.verified {
        return Err("Guide facts require a reproduced projection profile");
    }
    let bytes = read_file(
        &root.join("projections").join(projection_digest),
        crate::projection::MAX_PROJECTION_BYTES,
    )?
    .0;
    if hash(&bytes) != projection_digest {
        return Err("Guide projection changed");
    }
    let projection: LocalProjection =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid guide projection")?;
    if projection.product != "wow_classic_beta"
        || !matches!(projection.build, 70374 | 70425 | 70450)
        || projection.version != format!("1.60.1.{}", projection.build)
    {
        return Err("Unsupported guide projection development tuple");
    }
    let mut gaps: BTreeSet<String> = [
        "research_only_not_evidence",
        "target_build_unconfirmed",
        "universal_prerequisites_unknown",
        "universal_eligibility_unknown",
        "quest_availability_contextual_only",
        "client_name_is_not_access_evidence",
        "coordinates_uncalibrated",
        "spell_trainability_unknown",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let observations = candidates(root, &projection, inputs, &mut gaps)?;
    let mut quests = BTreeMap::<String, serde_json::Value>::new();
    for candidate in &observations.candidates {
        if candidate.value.value() == Some(&true) && numeric_quest(&candidate.quest_id) {
            quests.insert(candidate.quest_id.clone(), serde_json::json!({}));
        } else if !numeric_quest(&candidate.quest_id) {
            gaps.insert("observation_quest_id_unsupported".into());
        }
    }
    let mut zones = BTreeSet::new();
    let mut flights = BTreeSet::new();
    for name in projection
        .geography
        .maps
        .iter()
        .map(|v| &v.name)
        .chain(projection.geography.ui_maps.iter().map(|v| &v.name))
    {
        if let Fact::Known(name) = name {
            add_name(&mut zones, name, &mut gaps)?;
        }
    }
    if let Some(expanded) = &projection.expanded
        && let Some(table) = expanded.table("AreaTable")
    {
        for row in table.records.iter().flatten() {
            if let Some(Fact::Known(azeroth_core::local_data_gameplay::Value::Text(name))) =
                row.fields.get("AreaName_lang")
            {
                add_name(&mut zones, name, &mut gaps)?;
            }
        }
    }

    for node in &projection.travel.nodes {
        if let Fact::Known(name) = &node.name {
            add_name(&mut flights, name, &mut gaps)?;
        }
    }
    let counts = (
        quests.len() as u32,
        zones.len() as u32,
        flights.len() as u32,
    );
    let facts = serde_json::json!({"format":"navzeroth-guide-facts","version":2,"origin":"unverified_research","edition":"forever","client_build":projection.version,"quest_log_capacity":null,"level_xp":null,"zones":{"known":zones,"complete":false},"flight_nodes":{"known":flights,"complete":false},"quests":{"records":quests,"absent":[],"complete":false},"spells":{}});
    let facts = bounded_json(&facts, azeroth_ingest::rxp_verify::MAX_FACTS_BYTES)?;
    azeroth_ingest::rxp_verify::JsonFacts::parse(&facts)
        .map_err(|_| "Derived guide facts violate verifier contract")?;
    let observations = bounded_json(&observations, MAX_BYTES)?;
    let manifest = GuideFactsManifest {
        format_version: 1,
        classification: azeroth_core::local_data_catalog::DistributionClass::LocalOnly,
        projection_sha256: projection_digest.into(),
        catalog_sha256: projection.catalog_sha256,
        catalog_tuple: InputTuple {
            product: projection.product,
            version: projection.version,
            build: projection.build,
            locale: projection.locale,
        },
        input_sha256s: inputs.to_vec(),
        facts_sha256: hash(&facts),
        observations_sha256: hash(&observations),
        verified_for_planning: false,
        gaps: gaps.into_iter().collect(),
    };
    manifest.validate()?;
    Ok(Derived {
        manifest,
        facts,
        observations,
        quest_records: counts.0,
        zone_names: counts.1,
        flight_names: counts.2,
    })
}
pub fn build(
    destination: &Path,
    projection: &str,
    inputs: &[String],
    preset: PerformancePreset,
) -> Result<Summary> {
    if !is_sha256(projection)
        || inputs.len() > 16
        || inputs.iter().any(|v| !is_sha256(v))
        || !inputs.windows(2).all(|w| w[0] < w[1])
    {
        return Err("Guide input identities must be valid sorted unique digests");
    }
    let root = checked_path(destination)?;
    let mut store = Store::open(&root, preset.limits().total_storage_bytes)?;
    let Derived {
        manifest,
        facts,
        observations,
        quest_records,
        zone_names,
        flight_names,
    } = derive(&root, projection, inputs)?;
    let parsed: Observations =
        serde_json::from_slice(&observations).map_err(|_| "Invalid derived observations")?;
    store.put("objects", &facts)?;
    store.put("objects", &observations)?;
    let guide_facts_sha256 = store.put("guide-facts", &bounded_json(&manifest, 64 * 1024)?)?;
    Ok(Summary {
        guide_facts_sha256,
        manifest,
        observations: parsed.candidates.len() as u32,
        conflicts: parsed.conflicts.len() as u32,
        quest_records,
        zone_names,
        flight_names,
    })
}
pub fn verify(destination: &Path, digest: &str) -> Result<GuideFactsManifest> {
    if !is_sha256(digest) {
        return Err("Invalid guide facts digest");
    }
    let root = checked_path(destination)?;
    let bytes = read_file(&root.join("guide-facts").join(digest), 64 * 1024)?.0;
    if hash(&bytes) != digest {
        return Err("Guide facts manifest hash mismatch");
    }
    let manifest: GuideFactsManifest =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid guide facts manifest")?;
    manifest.validate()?;
    let derived = derive(&root, &manifest.projection_sha256, &manifest.input_sha256s)?;
    if derived.manifest != manifest
        || derived.facts
            != object(
                &root,
                &manifest.facts_sha256,
                azeroth_ingest::rxp_verify::MAX_FACTS_BYTES,
            )?
        || derived.observations != object(&root, &manifest.observations_sha256, MAX_BYTES)?
    {
        return Err("Guide facts are not reproducible from bound inputs");
    }
    Ok(manifest)
}
