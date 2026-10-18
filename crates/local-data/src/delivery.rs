//! Immutable partial research delivery orchestration and guarded lookup.
use crate::{Result, bounded_json, extraction::Store, hash, read_file};
use azeroth_core::{
    local_data::{PerformancePreset, is_sha256},
    local_data_delivery::{
        Activation, ConsumerGuard, Delivery, DeliveryPolicyV3, DevelopmentMatch,
    },
    local_data_inputs::InputTuple,
    local_data_projection::LocalProjection,
};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};
const MAX: usize = 1024 * 1024;
fn load_projection(root: &Path, digest: &str) -> Result<LocalProjection> {
    if !crate::projection::verify_projection(root, digest)?.verified {
        return Err("Delivery requires reproduced projection");
    }
    let bytes = read_file(
        &root.join("projections").join(digest),
        crate::projection::MAX_PROJECTION_BYTES,
    )?
    .0;
    if hash(&bytes) != digest {
        return Err("Delivery projection changed after verification");
    }
    let projection: LocalProjection =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid delivery projection")?;
    projection.validate()?;
    Ok(projection)
}
fn derive(
    root: &Path,
    projection_digest: &str,
    media: &[String],
    inputs: &[String],
    guide: Option<&str>,
) -> Result<Delivery> {
    if media.len() > 16
        || inputs.len() > 16
        || media.windows(2).any(|w| w[0] >= w[1])
        || inputs.windows(2).any(|w| w[0] >= w[1])
    {
        return Err("Delivery inputs must be bounded sorted unique digests");
    }
    let projection = load_projection(root, projection_digest)?;
    let tuple = InputTuple {
        product: projection.product.clone(),
        version: projection.version.clone(),
        build: projection.build,
        locale: projection.locale.clone(),
    };
    let mut dependencies = BTreeMap::new();
    let catalog = crate::verify_catalog(root, &projection.catalog_sha256)?;
    let mut fixture = catalog.input_hashes.is_empty()
        || catalog.extractor_version == "synthetic"
        || catalog.parser_profile == "synthetic";
    dependencies.insert("catalog".to_owned(), projection.catalog_sha256.clone());
    dependencies.insert("projection".to_owned(), projection_digest.to_owned());
    dependencies.insert(
        "projector_profile".to_owned(),
        projection.projector_profile_sha256.clone(),
    );
    dependencies.insert(
        "metadata".to_owned(),
        projection.catalog_metadata_sha256.clone(),
    );
    dependencies.insert(
        "analyzer_dependency_profile".to_owned(),
        crate::inputs::INPUT_PROFILE_SHA256.to_owned(),
    );
    for digest in media {
        let m = crate::media::verify(root, digest)?;
        if m.catalog_sha256 != projection.catalog_sha256 || m.locale != tuple.locale {
            return Err("Delivery media tuple/catalog mismatch");
        }
        fixture |= m.fixture;
        dependencies.insert(format!("media:{digest}"), digest.clone());
    }
    for digest in inputs {
        let input = crate::inputs::verify(root, digest)?;
        if input.catalog_sha256 != projection.catalog_sha256 || input.catalog_tuple != tuple {
            return Err("Delivery input tuple/catalog mismatch");
        }
        fixture |= input.fixture;
        dependencies.insert(format!("input:{digest}"), digest.clone());
    }
    if let Some(digest) = guide {
        let facts = crate::guide_facts::verify(root, digest)?;
        if facts.projection_sha256 != projection_digest
            || facts.catalog_tuple != tuple
            || facts.input_sha256s != inputs
        {
            return Err("Delivery guide facts do not bind selected projection and inputs");
        }
        dependencies.insert("guide_facts".into(), digest.into());
    }
    let delivery = Delivery {
        format_version: 1,
        tuple,
        catalog_sha256: projection.catalog_sha256,
        projection_sha256: projection_digest.into(),
        projector_profile_sha256: projection.projector_profile_sha256,
        media_sha256: media.to_vec(),
        guide_facts_sha256: guide.map(str::to_owned),
        input_sha256: inputs.to_vec(),
        dependency_profile_sha256: crate::inputs::INPUT_PROFILE_SHA256.into(),
        dependencies_sha256: hash(&bounded_json(&dependencies, MAX)?),
        fixture,
        local_only: true,
        verified_for_planning: false,
    };
    delivery.validate()?;
    Ok(delivery)
}
pub fn build(
    root: &Path,
    projection: &str,
    media: &[String],
    inputs: &[String],
    guide: Option<&str>,
    preset: PerformancePreset,
) -> Result<String> {
    let mut store = Store::open(root, preset.limits().total_storage_bytes)?;
    let delivery = derive(root, projection, media, inputs, guide)?;
    store.put("deliveries", &bounded_json(&delivery, MAX)?)
}
pub fn verify(root: &Path, digest: &str) -> Result<Delivery> {
    if !is_sha256(digest) {
        return Err("Invalid delivery identity");
    }
    let bytes = read_file(&root.join("deliveries").join(digest), MAX)?.0;
    if hash(&bytes) != digest {
        return Err("Delivery hash mismatch");
    }
    let delivery: Delivery =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid delivery manifest")?;
    delivery.validate()?;
    if bounded_json(&delivery, MAX)? != bytes
        || derive(
            root,
            &delivery.projection_sha256,
            &delivery.media_sha256,
            &delivery.input_sha256,
            delivery.guide_facts_sha256.as_deref(),
        )? != delivery
    {
        return Err("Delivery dependency closure mismatch");
    }
    Ok(delivery)
}
fn scope(delivery: &Delivery) -> Result<String> {
    Ok(hash(&bounded_json(
        &("NAVZEROTH-DELIVERY-SCOPE-V1", &delivery.tuple),
        MAX,
    )?))
}
pub fn active(root: &Path, digest: &str) -> Result<Activation> {
    let delivery = verify(root, digest)?;
    crate::lifecycle::index(root)?
        .delivery_active(&scope(&delivery)?)
        .map_err(|_| "Cannot read delivery activation")
}
pub fn activate(root: &Path, digest: &str, expected: u64, rollback: bool) -> Result<Activation> {
    let _store = Store::open(root, PerformancePreset::Max.limits().total_storage_bytes)?;
    let delivery = verify(root, digest)?;
    crate::lifecycle::index(root)?
        .activate_delivery(&scope(&delivery)?, digest, expected, rollback)
        .map_err(|_| "Delivery activation changed or rollback/history invalid")
}
#[derive(Debug, Serialize)]
pub struct Diff {
    pub before: String,
    pub after: String,
    pub changed: Vec<String>,
    pub same_scope: bool,
    pub actionable_advice: bool,
}
pub fn diff(root: &Path, before: &str, after: &str) -> Result<Diff> {
    let a = verify(root, before)?;
    let b = verify(root, after)?;
    let mut changed = Vec::new();
    for (name, differs) in [
        ("tuple", a.tuple != b.tuple),
        ("catalog", a.catalog_sha256 != b.catalog_sha256),
        ("projection", a.projection_sha256 != b.projection_sha256),
        (
            "projector_profile",
            a.projector_profile_sha256 != b.projector_profile_sha256,
        ),
        ("media", a.media_sha256 != b.media_sha256),
        ("inputs", a.input_sha256 != b.input_sha256),
        ("guide_facts", a.guide_facts_sha256 != b.guide_facts_sha256),
        (
            "dependency_profile",
            a.dependency_profile_sha256 != b.dependency_profile_sha256,
        ),
        (
            "dependencies",
            a.dependencies_sha256 != b.dependencies_sha256,
        ),
    ] {
        if differs {
            changed.push(name.into());
        }
    }
    Ok(Diff {
        before: before.into(),
        after: after.into(),
        changed,
        same_scope: a.tuple == b.tuple,
        actionable_advice: false,
    })
}
#[derive(Debug, Serialize)]
pub struct Row {
    pub domain: String,
    pub id: u32,
    pub facts: serde_json::Value,
}
#[derive(Debug, Serialize)]
pub struct Page {
    pub revision: String,
    pub domain: String,
    pub total: Option<u32>,
    pub offset: u32,
    pub rows: Vec<Row>,
    pub next_offset: Option<u32>,
    pub guard: ConsumerGuard,
    pub sources: Vec<azeroth_core::local_data_projection::SourceBinding>,
    pub gaps: Vec<String>,
}
fn slice_rows<T: Serialize>(
    records: &[T],
    identity: impl Fn(&T) -> u32,
    domain: &str,
    offset: u32,
    limit: u32,
) -> Result<(usize, Vec<Row>)> {
    if offset as usize > records.len() {
        return Err("Delivery page offset out of range");
    }
    let end = (offset as usize + limit as usize).min(records.len());
    let rows = records[offset as usize..end]
        .iter()
        .map(|v| {
            Ok(Row {
                domain: domain.into(),
                id: identity(v),
                facts: serde_json::to_value(v).map_err(|_| "Delivery record encoding failed")?,
            })
        })
        .collect::<Result<_>>()?;
    Ok((records.len(), rows))
}
pub fn page(
    root: &Path,
    digest: &str,
    domain: &str,
    offset: u32,
    limit: u32,
    policy: Option<&DeliveryPolicyV3>,
) -> Result<Page> {
    use azeroth_core::local_data_projection::SourceState;
    if limit == 0 || limit > 200 {
        return Err("Delivery page limit must be 1 through 200");
    }
    let delivery = verify(root, digest)?;
    let projection = load_projection(root, &delivery.projection_sha256)?;
    let sources = projection
        .sources
        .iter()
        .filter(|s| {
            if matches!(domain, "equipment" | "geography" | "travel") {
                s.domain == domain
            } else {
                s.table == domain
            }
        })
        .cloned()
        .collect::<Vec<_>>();
    let (available, (count, rows)) = match domain {
        "equipment" => (
            sources.iter().any(|s| {
                s.state == SourceState::Projected
                    && matches!(s.table.as_str(), "Item" | "ItemSparse")
            }),
            slice_rows(
                &projection.equipment.items,
                |v| v.item_id,
                domain,
                offset,
                limit,
            )?,
        ),
        "geography" => (
            sources
                .iter()
                .any(|s| s.state == SourceState::Projected && s.table == "Map"),
            slice_rows(
                &projection.geography.maps,
                |v| v.map_id,
                domain,
                offset,
                limit,
            )?,
        ),
        "travel" => (
            sources
                .iter()
                .any(|s| s.state == SourceState::Projected && s.table == "TaxiNodes"),
            slice_rows(
                &projection.travel.nodes,
                |v| v.node_id,
                domain,
                offset,
                limit,
            )?,
        ),
        name => {
            let expanded = projection
                .expanded
                .as_ref()
                .ok_or("Expanded delivery unavailable")?;
            let table = expanded
                .tables
                .iter()
                .find(|t| t.table == name)
                .ok_or("Unsupported delivery lookup domain/table")?;
            (
                table.records.is_some(),
                slice_rows(
                    table.records.as_deref().unwrap_or(&[]),
                    |v| v.id,
                    domain,
                    offset,
                    limit,
                )?,
            )
        }
    };
    let development = policy.map_or(Ok(DevelopmentMatch::Unconfigured), |p| {
        p.development_match(&delivery.tuple, digest)
    })?;
    let end = offset as usize + rows.len();
    Ok(Page {
        revision: digest.into(),
        domain: domain.into(),
        total: available.then_some(count as u32),
        offset,
        rows,
        next_offset: (end < count).then_some(end as u32),
        guard: azeroth_core::local_data_delivery::consumer_guard(development),
        sources,
        gaps: projection.gaps,
    })
}
#[cfg(test)]
mod page_tests {
    use super::*;
    use std::cell::Cell;
    struct Counted<'a> {
        id: u32,
        count: &'a Cell<usize>,
    }
    impl Serialize for Counted<'_> {
        fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
            self.count.set(self.count.get() + 1);
            self.id.serialize(s)
        }
    }
    #[test]
    fn lookup_serializes_only_requested_records_and_keeps_order() {
        let count = Cell::new(0);
        let records = (0..10_000)
            .map(|id| Counted { id, count: &count })
            .collect::<Vec<_>>();
        let (total, page) = slice_rows(&records, |v| v.id, "test", 9000, 2).unwrap();
        assert_eq!(total, 10_000);
        assert_eq!(count.get(), 2);
        assert_eq!(page.iter().map(|r| r.id).collect::<Vec<_>>(), [9000, 9001]);
    }
}
