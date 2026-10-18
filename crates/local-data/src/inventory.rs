//! Exact private imported IDs; no imported count establishes target completeness.
use crate::{Result, bounded_json, extraction::Store, hash, read_file};
use azeroth_core::{
    local_data::{PerformancePreset, is_sha256},
    local_data_inventory::{Domain, Inventory, Record, RecordKind},
    local_data_projection::{LocalProjection, SourceState},
    release::DOMAINS,
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
const MAX: usize = 64 * 1024 * 1024;
struct Builder {
    records: BTreeMap<String, BTreeMap<String, Record>>,
    available: BTreeSet<String>,
    gaps: BTreeMap<String, BTreeSet<String>>,
}
impl Builder {
    fn add(&mut self, domain: &str, id: String, source: &str, kind: RecordKind) -> Result<()> {
        let records = self.records.entry(domain.into()).or_default();
        if records
            .insert(
                id.clone(),
                Record {
                    id,
                    source_sha256: source.into(),
                    kind,
                },
            )
            .is_some()
        {
            return Err("Duplicate imported inventory identity");
        }
        self.available.insert(domain.into());
        if self.records.values().map(BTreeMap::len).sum::<usize>() > 250_000 {
            return Err("Inventory record quota exceeded");
        }
        Ok(())
    }
    fn client(&mut self, domain: &str, kind: &str, id: u32, projection: &str) -> Result<()> {
        self.add(
            domain,
            serde_json::to_string(&(projection, kind, id))
                .map_err(|_| "Inventory identity failed")?,
            projection,
            RecordKind::ClientProjection,
        )
    }
}
fn projection(root: &Path, digest: &str) -> Result<LocalProjection> {
    let bytes = read_file(
        &root.join("projections").join(digest),
        crate::projection::MAX_PROJECTION_BYTES,
    )?
    .0;
    if hash(&bytes) != digest {
        return Err("Inventory projection changed after verification");
    }
    let p: LocalProjection =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid inventory projection")?;
    p.validate()?;
    Ok(p)
}
fn derive(root: &Path, delivery_digest: &str, authored: Option<&Path>) -> Result<Inventory> {
    let delivery = crate::delivery::verify(root, delivery_digest)?;
    let projection = projection(root, &delivery.projection_sha256)?;
    let source = delivery.projection_sha256.as_str();
    let mut b = Builder {
        records: BTreeMap::new(),
        available: BTreeSet::new(),
        gaps: BTreeMap::new(),
    };
    for source in &projection.sources {
        if source.state == SourceState::Projected {
            b.available.insert(source.domain.clone());
        } else {
            b.gaps
                .entry(source.domain.clone())
                .or_default()
                .insert(format!("{}:{}", source.table, source.reason));
        }
    }
    for gap in &projection.gaps {
        let domain = gap.split(':').next().unwrap_or("");
        if DOMAINS.contains(&domain) {
            b.gaps.entry(domain.into()).or_default().insert(gap.clone());
        }
    }
    for r in &projection.equipment.items {
        b.client("equipment", "equipment_items", r.item_id, source)?;
    }
    for r in &projection.geography.maps {
        b.client("geography", "maps", r.map_id, source)?;
    }
    for r in &projection.geography.ui_maps {
        b.client("geography", "ui_maps", r.ui_map_id, source)?;
    }
    for r in &projection.geography.ui_map_assignments {
        b.client("geography", "ui_map_assignments", r.assignment_id, source)?;
    }
    for r in &projection.travel.nodes {
        b.client("travel", "taxi_nodes", r.node_id, source)?;
    }
    for r in &projection.travel.paths {
        b.client("travel", "taxi_paths", r.path_id, source)?;
        for point in &r.waypoints {
            b.client("travel", "taxi_path_nodes", point.waypoint_id, source)?;
        }
    }
    for r in &projection.class_roles.classes {
        b.client("class_roles", "classes", r.class_id, source)?;
    }
    for r in &projection.class_roles.races {
        b.client("class_roles", "races", r.race_id, source)?;
    }
    if let Some(expanded) = &projection.expanded {
        for table in &expanded.tables {
            let domain = projection
                .sources
                .iter()
                .find(|s| s.table == table.table)
                .ok_or("Inventory expanded source missing")?
                .domain
                .as_str();
            for row in table.records.iter().flatten() {
                b.client(domain, &table.table, row.id, source)?;
            }
        }
        for model in &expanded.item_stats {
            b.client(
                "equipment",
                "unverified_item_budget_models",
                model.item_id,
                source,
            )?;
        }
    }
    if let Some(digest) = &delivery.guide_facts_sha256 {
        let facts = crate::guide_facts::verify(root, digest)?;
        let bytes = read_file(
            &root.join("objects").join(&facts.observations_sha256),
            16 * 1024 * 1024,
        )?
        .0;
        if hash(&bytes) != facts.observations_sha256 {
            return Err("Inventory observations changed after verification");
        }
        let observations: azeroth_core::local_data_guide::Observations =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid inventory observations")?;
        for (index, candidate) in observations.candidates.iter().enumerate() {
            b.add(
                "quests",
                serde_json::to_string(&(digest, "observations", index, &candidate.quest_id))
                    .map_err(|_| "Inventory observation identity failed")?,
                digest,
                RecordKind::PermittedObservation,
            )?;
        }
        if !observations.conflicts.is_empty() {
            b.gaps.entry("quests".into()).or_default().insert(format!(
                "observation_conflict_groups:{}",
                observations.conflicts.len()
            ));
        }
    }
    let mut authored_manifest = None;
    let mut authored_catalog = None;
    if let Some(path) = authored {
        let bundle = azeroth_ingest::pack::v2::read_verified(path)?;
        if bundle.content.reference.fixture
            || bundle.sources.len() != 1
            || bundle.sources[0].access.value()
                != Some(&azeroth_core::sources::AccessMethod::Authored)
            || bundle.content.reference.game.edition.as_str() != "wow_forever"
            || bundle.content.reference.game.build.as_str() != delivery.tuple.version
            || bundle.content.reference.game.interface.as_str() != "16001"
            || bundle.content.reference.game.locale.as_str() != delivery.tuple.locale
        {
            return Err("Authored inventory requires exact-tuple original authoring pack");
        }
        let mut records = azeroth_core::content_v2::records::content_records(&bundle.content)?;
        if let Some(matrix) = &bundle.party_support {
            records.insert(
                "party".into(),
                matrix.bind(bundle.catalog_sha256.as_str(), &records)?,
            );
        }
        for (domain, records) in records {
            for (id, _) in records {
                b.add(
                    &domain,
                    id,
                    bundle.catalog_sha256.as_str(),
                    RecordKind::AuthoredDesign,
                )?;
            }
        }
        authored_manifest = Some(bundle.manifest_sha256.as_str().into());
        authored_catalog = Some(bundle.catalog_sha256.as_str().into());
    }
    let domains = DOMAINS
        .into_iter()
        .map(|domain| {
            let records = b
                .records
                .remove(domain)
                .unwrap_or_default()
                .into_values()
                .collect::<Vec<_>>();
            let mut gaps = b.gaps.remove(domain).unwrap_or_default();
            gaps.insert("independent_target_enumeration_unverified".into());
            if !b.available.contains(domain) {
                gaps.insert("no_imported_source_for_domain".into());
            }
            Domain {
                domain: domain.into(),
                imported_count: records.len() as u32,
                records,
                source_available: b.available.contains(domain),
                gaps: gaps.into_iter().collect(),
                independently_required_count: None,
                independently_complete: false,
            }
        })
        .collect();
    let inventory = Inventory {
        format_version: 1,
        delivery_sha256: delivery_digest.into(),
        tuple: delivery.tuple,
        authored_manifest_sha256: authored_manifest,
        authored_catalog_sha256: authored_catalog,
        domains,
        local_only: true,
        acceptance_ready: false,
    };
    inventory.validate()?;
    Ok(inventory)
}
pub fn build(
    root: &Path,
    delivery: &str,
    authored: Option<&Path>,
    preset: PerformancePreset,
) -> Result<String> {
    let mut store = Store::open(root, preset.limits().total_storage_bytes)?;
    let inventory = derive(root, delivery, authored)?;
    store.put("inventories", &bounded_json(&inventory, MAX)?)
}
pub fn verify(root: &Path, digest: &str, authored: Option<&Path>) -> Result<Inventory> {
    if !is_sha256(digest) {
        return Err("Invalid inventory identity");
    }
    let bytes = read_file(&root.join("inventories").join(digest), MAX)?.0;
    if hash(&bytes) != digest {
        return Err("Inventory hash mismatch");
    }
    let inventory: Inventory = serde_json::from_slice(&bytes).map_err(|_| "Invalid inventory")?;
    inventory.validate()?;
    if bounded_json(&inventory, MAX)? != bytes
        || derive(root, &inventory.delivery_sha256, authored)? != inventory
    {
        return Err(
            "Inventory source closure differs; reselect the same authored pack if required",
        );
    }
    Ok(inventory)
}
#[derive(Debug, Serialize)]
pub struct Summary {
    pub revision: String,
    pub domains: Vec<DomainSummary>,
    pub acceptance_ready: bool,
}
#[derive(Debug, Serialize)]
pub struct DomainSummary {
    pub domain: String,
    pub imported_count: u32,
    pub source_available: bool,
    pub independently_required_count: Option<u32>,
    pub independently_complete: bool,
    pub gaps: Vec<String>,
}
pub fn summary(root: &Path, digest: &str, authored: Option<&Path>) -> Result<Summary> {
    let inventory = verify(root, digest, authored)?;
    Ok(Summary {
        revision: digest.into(),
        domains: inventory
            .domains
            .into_iter()
            .map(|d| DomainSummary {
                domain: d.domain,
                imported_count: d.imported_count,
                source_available: d.source_available,
                independently_required_count: None,
                independently_complete: false,
                gaps: d.gaps,
            })
            .collect(),
        acceptance_ready: false,
    })
}
#[derive(Debug, Serialize)]
pub struct Page {
    pub revision: String,
    pub domain: String,
    pub offset: u32,
    pub total: u32,
    pub records: Vec<Record>,
    pub next_offset: Option<u32>,
    pub source_available: bool,
    pub independently_complete: bool,
}
pub fn page(
    root: &Path,
    digest: &str,
    domain: &str,
    offset: u32,
    limit: u32,
    authored: Option<&Path>,
) -> Result<Page> {
    if limit == 0 || limit > 200 {
        return Err("Inventory page limit must be 1 through 200");
    }
    let inventory = verify(root, digest, authored)?;
    let domain = inventory
        .domains
        .into_iter()
        .find(|d| d.domain == domain)
        .ok_or("Unknown inventory domain")?;
    if offset as usize > domain.records.len() {
        return Err("Inventory page offset out of range");
    }
    let end = (offset as usize + limit as usize).min(domain.records.len());
    Ok(Page {
        revision: digest.into(),
        domain: domain.domain,
        offset,
        total: domain.imported_count,
        records: domain.records[offset as usize..end].to_vec(),
        next_offset: (end < domain.records.len()).then_some(end as u32),
        source_available: domain.source_available,
        independently_complete: false,
    })
}
#[derive(Debug, Serialize)]
pub struct Designs {
    pub format_version: u32,
    pub inventory_sha256: String,
    pub delivery_sha256: String,
    pub authored_manifest_sha256: Option<String>,
    pub designs: Vec<Design>,
    pub acceptance_ready: bool,
}
#[derive(Debug, Serialize)]
pub struct Design {
    pub draft: azeroth_core::local_data_inventory::RouteDesign,
    pub authored_models_present: bool,
    pub eligible_quest_sequence_verified: bool,
}
pub fn designs(root: &Path, digest: &str, authored: Option<&Path>) -> Result<Designs> {
    let inventory = verify(root, digest, authored)?;
    let models = inventory
        .domains
        .iter()
        .find(|d| d.domain == "class_roles")
        .ok_or("Class-role inventory unavailable")?
        .records
        .iter()
        .filter(|r| r.kind == RecordKind::AuthoredDesign)
        .filter_map(|r| {
            let id: Vec<serde_json::Value> = serde_json::from_str(&r.id).ok()?;
            (id.get(1)?.as_str()? == "models")
                .then(|| id.last()?.as_str().map(str::to_owned))
                .flatten()
        })
        .collect::<BTreeSet<_>>();
    let designs = azeroth_core::local_data_inventory::human_route_designs()
        .into_iter()
        .map(|draft| Design {
            authored_models_present: draft
                .authored_model_reviews
                .iter()
                .all(|m| models.contains(m)),
            draft,
            eligible_quest_sequence_verified: false,
        })
        .collect();
    Ok(Designs {
        format_version: 1,
        inventory_sha256: digest.into(),
        delivery_sha256: inventory.delivery_sha256,
        authored_manifest_sha256: inventory.authored_manifest_sha256,
        designs,
        acceptance_ready: false,
    })
}
