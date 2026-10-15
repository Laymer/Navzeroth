//! Canonical content identities and their source bindings, without I/O.
//! Callers still enforce fixture/target/source rights and evidence-file integrity.
use crate::{contract::ReferenceDataset, release};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
type Result<T> = std::result::Result<T, &'static str>;
#[derive(Default)]
struct Catalog {
    records: crate::release_party::ConcreteCatalog,
    revisions: BTreeSet<String>,
}
pub fn reference_records(data: &ReferenceDataset) -> Result<crate::release_party::ConcreteCatalog> {
    let mut catalog = Catalog::default();
    catalog_data(&mut catalog, data)?;
    Ok(catalog.records)
}
fn add_record(
    catalog: &mut Catalog,
    domain: &str,
    key: Value,
    sources: &BTreeSet<String>,
) -> Result<()> {
    let key = serde_json::to_string(&key).map_err(|_| "Record identity cannot be encoded")?;
    if catalog
        .records
        .entry(domain.into())
        .or_default()
        .insert(key, sources.clone())
        .is_some()
    {
        return Err("Duplicate record identity across release packs");
    }
    if catalog.records.values().map(BTreeMap::len).sum::<usize>() > release::MAX_RECORDS {
        return Err("Release catalog exceeds record limit");
    }
    Ok(())
}
fn catalog_data(catalog: &mut Catalog, data: &ReferenceDataset) -> Result<()> {
    let revision = data.revision.as_str();
    if !catalog.revisions.insert(revision.to_owned()) {
        return Err("Duplicate dataset revision across release packs");
    }
    let evidence: BTreeMap<_, _> = data
        .evidence
        .iter()
        .map(|e| (&e.id, e.source.as_str().to_owned()))
        .collect();
    let sources = |ids: &[crate::types::EvidenceId]| -> Result<BTreeSet<String>> {
        ids.iter()
            .map(|id| {
                evidence
                    .get(id)
                    .cloned()
                    .ok_or("Unresolved record provenance")
            })
            .collect()
    };
    for quest in &data.quests {
        add_record(
            catalog,
            "quests",
            json!([revision, "quests", quest.id]),
            &sources(&quest.evidence)?,
        )?;
        for stage in &quest.stages {
            add_record(
                catalog,
                "quests",
                json!([revision, "quest_stages", quest.id, stage]),
                &sources(&quest.evidence)?,
            )?;
        }
    }
    for item in &data.items {
        add_record(
            catalog,
            "equipment",
            json!([revision, "items", item.key.item_id, item.key.variant]),
            &sources(&item.evidence)?,
        )?;
    }
    for source in &data.acquisitions {
        add_record(
            catalog,
            "acquisition",
            json!([revision, "acquisitions", source.id]),
            &sources(&source.evidence)?,
        )?;
    }
    for map in &data.map_spaces {
        add_record(
            catalog,
            "geography",
            json!([
                revision,
                "map_spaces",
                map.key.map_id,
                map.key.floor,
                map.key.instance
            ]),
            &sources(&map.evidence)?,
        )?;
    }
    for image in &data.map_images {
        let mut image_sources = sources(&image.evidence)?;
        image_sources.insert(image.source.as_str().to_owned());
        image_sources.extend(sources(&image.calibration_evidence)?);
        for applicable in &image.applicable_builds {
            image_sources.extend(sources(&applicable.evidence)?);
        }
        add_record(
            catalog,
            "geography",
            json!([revision, "map_images", image.id]),
            &image_sources,
        )?;
        for pin in &image.pins {
            add_record(
                catalog,
                "geography",
                json!([revision, "map_pins", image.id, pin.id]),
                &sources(&pin.evidence)?,
            )?;
        }
    }
    for map in &data.map_embeddings {
        add_record(
            catalog,
            "geography",
            json!([
                revision,
                "map_embeddings",
                map.child.map_id,
                map.child.floor,
                map.child.instance,
                map.parent.map_id,
                map.parent.floor,
                map.parent.instance
            ]),
            &sources(&map.evidence)?,
        )?;
    }
    for node in &data.nodes {
        add_record(
            catalog,
            "travel",
            json!([revision, "nodes", node.id]),
            &BTreeSet::new(),
        )?;
    }
    for edge in &data.edges {
        add_record(
            catalog,
            "travel",
            json!([revision, "edges", edge.id]),
            &sources(&edge.evidence)?,
        )?;
    }
    for objective in &data.objectives {
        add_record(
            catalog,
            "quests",
            json!([revision, "objectives", objective.id]),
            &sources(&objective.evidence)?,
        )?;
    }
    for rule in &data.rules {
        add_record(
            catalog,
            "reminders",
            json!([revision, "rules", rule.id]),
            &sources(&rule.evidence)?,
        )?;
    }
    Ok(())
}
pub fn content_records(
    content: &crate::content_v2::ReferenceContent,
) -> Result<crate::release_party::ConcreteCatalog> {
    let mut catalog = Catalog::default();
    catalog_data(&mut catalog, &content.reference.payload)?;
    let revision = content.reference.payload.revision.as_str();
    let evidence: BTreeMap<_, _> = content
        .reference
        .payload
        .evidence
        .iter()
        .map(|e| (&e.id, e.source.as_str().to_owned()))
        .collect();
    let sources = |ids: &[crate::types::EvidenceId]| -> Result<BTreeSet<String>> {
        ids.iter()
            .map(|id| evidence.get(id).cloned().ok_or("Unresolved v2 provenance"))
            .collect()
    };
    for row in &content.models {
        add_record(
            &mut catalog,
            "class_roles",
            json!([revision, "models", row.id]),
            &sources(&row.evidence)?,
        )?;
    }
    for row in &content.dungeons {
        add_record(
            &mut catalog,
            "dungeons",
            json!([revision, "dungeons", row.id]),
            &sources(&row.evidence)?,
        )?;
    }
    for row in &content.equipment {
        add_record(
            &mut catalog,
            "equipment",
            json!([
                revision,
                "equipment_facts",
                row.item.item_id,
                row.item.variant
            ]),
            &sources(&row.evidence)?,
        )?;
    }
    for row in &content.quest_facts {
        add_record(
            &mut catalog,
            "quests",
            json!([revision, "quest_facts", row.quest]),
            &sources(&row.evidence)?,
        )?;
    }
    for row in &content.acquisition_facts {
        add_record(
            &mut catalog,
            "acquisition",
            json!([revision, "acquisition_facts", row.acquisition]),
            &sources(&row.evidence)?,
        )?;
    }
    Ok(catalog.records)
}
