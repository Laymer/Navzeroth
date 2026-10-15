//! A reference dataset revision for one build and namespace. Local references
//! resolve inside the dataset. Not resolved here (M3 content work): quest or
//! objective IDs used only inside predicates, travel-graph cycles (legitimate),
//! image source registry IDs, and references to other datasets. Validated as
//! part of its envelope. `map_images` and `map_embeddings` are additive version 1
//! members (ADR 0002); absent means no imagery or embeddings are recorded.
use super::{ContractKind, EnvelopeHeader, Payload, no_dependencies, same_context, unresolved};
use crate::{entities::*, error::Result, types::*};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ReferenceDataset {
    pub revision: DatasetRevision,
    pub evidence: Vec<EvidenceRecord>,
    pub items: Vec<ItemDefinition>,
    pub acquisitions: Vec<AcquisitionSource>,
    pub quests: Vec<QuestDefinition>,
    pub map_spaces: Vec<MapSpace>,
    pub nodes: Vec<TravelNode>,
    pub edges: Vec<TravelEdge>,
    pub objectives: Vec<Objective>,
    pub rules: Vec<ContextRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub map_images: Vec<MapImage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub map_embeddings: Vec<MapEmbedding>,
}
impl ReferenceDataset {
    fn contexts(&self) -> impl Iterator<Item = &DataContext> {
        let e = self.evidence.iter().map(|r| &r.context);
        let i = self.items.iter().map(|r| &r.key.context);
        let a = self.acquisitions.iter().map(|r| &r.context);
        let q = self.quests.iter().map(|r| &r.context);
        let m = self.map_spaces.iter().map(|r| &r.key.context);
        let n = self.nodes.iter().map(|r| &r.position.map.context);
        let g = self.edges.iter().map(|r| &r.context);
        let o = self.objectives.iter().map(|r| &r.context);
        let r = self.rules.iter().map(|r| &r.context);
        let im = self.map_images.iter().map(|r| &r.map.context);
        let em = self
            .map_embeddings
            .iter()
            .flat_map(|r| [&r.child.context, &r.parent.context]);
        e.chain(i)
            .chain(a)
            .chain(q)
            .chain(m)
            .chain(n)
            .chain(g)
            .chain(o)
            .chain(r)
            .chain(im)
            .chain(em)
    }
    fn evidence_refs(&self) -> impl Iterator<Item = &EvidenceId> {
        let e = self.evidence.iter().flat_map(|r| &r.conflicts);
        let i = self.items.iter().flat_map(|r| &r.evidence);
        let a = self.acquisitions.iter().flat_map(|r| &r.evidence);
        let q = self.quests.iter().flat_map(|r| &r.evidence);
        let m = self.map_spaces.iter().flat_map(|r| &r.evidence);
        let g = self.edges.iter().flat_map(|r| &r.evidence);
        let o = self.objectives.iter().flat_map(|r| &r.evidence);
        let r = self.rules.iter().flat_map(|r| &r.evidence);
        let im = self.map_images.iter().flat_map(|r| {
            r.evidence
                .iter()
                .chain(&r.calibration_evidence)
                .chain(r.pins.iter().flat_map(|p| &p.evidence))
                .chain(r.applicable_builds.iter().flat_map(|b| &b.evidence))
        });
        let em = self.map_embeddings.iter().flat_map(|r| &r.evidence);
        e.chain(i)
            .chain(a)
            .chain(q)
            .chain(m)
            .chain(g)
            .chain(o)
            .chain(r)
            .chain(im)
            .chain(em)
    }
}

impl ReferenceDataset {
    fn validate_local(&self, collection_limit: usize) -> Result<()> {
        for len in [
            self.evidence.len(),
            self.items.len(),
            self.acquisitions.len(),
            self.quests.len(),
            self.map_spaces.len(),
            self.nodes.len(),
            self.edges.len(),
            self.objectives.len(),
            self.rules.len(),
            self.map_images.len(),
            self.map_embeddings.len(),
        ] {
            if len > collection_limit {
                return Err(crate::error::invalid(
                    "Reference collection exceeds capacity",
                ));
            }
        }
        unique_catalog(self.evidence.iter().map(|r| &r.id))?;
        unique_catalog(self.items.iter().map(|r| &r.key))?;
        unique_catalog(self.acquisitions.iter().map(|r| &r.id))?;
        unique_catalog(self.quests.iter().map(|r| &r.id))?;
        unique_catalog(self.map_spaces.iter().map(|r| &r.key))?;
        unique_catalog(self.nodes.iter().map(|r| &r.id))?;
        unique_catalog(self.edges.iter().map(|r| &r.id))?;
        unique_catalog(self.objectives.iter().map(|r| &r.id))?;
        unique_catalog(self.rules.iter().map(|r| &r.id))?;
        unique_catalog(self.map_images.iter().map(|r| &r.id))?;
        unique_catalog(self.map_embeddings.iter().map(|r| &r.child))?;

        let evidence: BTreeSet<_> = self.evidence.iter().map(|r| &r.id).collect();
        let items: BTreeSet<_> = self.items.iter().map(|r| &r.key).collect();
        let acquisitions: BTreeSet<_> = self.acquisitions.iter().map(|r| &r.id).collect();
        let quests: BTreeMap<_, _> = self.quests.iter().map(|r| (&r.id, &r.stages)).collect();
        let maps: BTreeSet<_> = self.map_spaces.iter().map(|r| &r.key).collect();
        let nodes: BTreeMap<_, _> = self
            .nodes
            .iter()
            .map(|r| (&r.id, &r.position.map))
            .collect();
        let node_resolves = |n: &NodeRef| nodes.get(&n.id) == Some(&&n.map);

        let resolved = self.evidence_refs().all(|id| evidence.contains(id))
            && self
                .acquisitions
                .iter()
                .flat_map(|a| &a.items)
                .all(|k| items.contains(k))
            && self
                .quests
                .iter()
                .filter_map(|q| q.rewards.value())
                .flatten()
                .all(|k| items.contains(k))
            && self
                .quests
                .iter()
                .filter_map(|q| q.locations.value())
                .flatten()
                .all(|p| maps.contains(&p.map))
            && self.nodes.iter().all(|n| maps.contains(&n.position.map))
            && self
                .edges
                .iter()
                .all(|e| node_resolves(&e.from) && node_resolves(&e.to))
            && self
                .objectives
                .iter()
                .flat_map(|o| &o.targets)
                .all(|t| match t {
                    ObjectiveTarget::QuestStage { quest, stage } => quests
                        .get(quest)
                        .is_some_and(|stages| stages.contains(stage)),
                    ObjectiveTarget::Acquisition { source } => acquisitions.contains(source),
                    ObjectiveTarget::Service { node } => node_resolves(node),
                });
        if !resolved {
            return Err(unresolved("Dataset reference does not resolve"));
        }
        self.validate_imagery(&maps, &nodes)
    }

    /// Image maps, control points and embeddings resolve locally; embeddings form
    /// acyclic chains no deeper than `MAX_EMBEDDING_DEPTH`; declared residuals
    /// agree with the calibration.
    fn validate_imagery(
        &self,
        maps: &BTreeSet<&MapKey>,
        nodes: &BTreeMap<&NodeId, &MapKey>,
    ) -> Result<()> {
        let positions: BTreeMap<_, _> = self.nodes.iter().map(|n| (&n.id, &n.position)).collect();
        for image in &self.map_images {
            if !maps.contains(&image.map) {
                return Err(unresolved("Map image references an undeclared map space"));
            }
            for point in &image.control_points {
                if nodes.get(&point.node) != Some(&&image.map) {
                    return Err(unresolved(
                        "Control point references a node outside the image map",
                    ));
                }
                if let (Some(residual), Some(transform)) =
                    (point.residual_px.value(), image.calibration.value())
                {
                    let pin = image
                        .pins
                        .iter()
                        .find(|p| p.id == point.pin)
                        .expect("validated: control point pin exists");
                    let position = positions[&point.node];
                    let (x, y) = transform.apply(position.x, position.y);
                    let distance =
                        ((x - pin.pixel.x.get()).powi(2) + (y - pin.pixel.y.get()).powi(2)).sqrt();
                    if (distance - residual.get()).abs() > RESIDUAL_TOLERANCE_PX {
                        return Err(crate::error::invalid(
                            "Control-point residual disagrees with the calibration",
                        ));
                    }
                }
            }
        }
        let parents: BTreeMap<_, _> = self
            .map_embeddings
            .iter()
            .map(|e| (&e.child, &e.parent))
            .collect();
        for embedding in &self.map_embeddings {
            if !maps.contains(&embedding.child) || !maps.contains(&embedding.parent) {
                return Err(unresolved(
                    "Map embedding references an undeclared map space",
                ));
            }
            // One parent per child: any repeated key is a cycle and every walk
            // ends within `parents.len()` steps; depth is judged afterwards.
            let mut visited: BTreeSet<&MapKey> = BTreeSet::from([&embedding.child]);
            let mut current = &embedding.parent;
            let mut depth = 1;
            loop {
                if !visited.insert(current) {
                    return Err(crate::error::invalid("Map embeddings form a cycle"));
                }
                match parents.get(current) {
                    Some(next) => {
                        depth += 1;
                        if depth > MAX_EMBEDDING_DEPTH {
                            return Err(crate::error::invalid("Map embedding chain is too deep"));
                        }
                        current = next;
                    }
                    None => break,
                }
            }
            if depth > MAX_EMBEDDING_DEPTH {
                return Err(crate::error::invalid("Map embedding chain is too deep"));
            }
        }
        Ok(())
    }
}
impl Payload for ReferenceDataset {
    const KIND: ContractKind = ContractKind::ReferenceDataset;
    fn validate_payload(&self, header: &EnvelopeHeader) -> Result<()> {
        self.validate_reference(header, 1024)
    }
}
impl ReferenceDataset {
    fn validate_reference(&self, header: &EnvelopeHeader, collection_limit: usize) -> Result<()> {
        no_dependencies(header)?;
        self.validate_local(collection_limit)?;
        for context in self.contexts() {
            same_context(header, context)?;
        }
        if header.fixture
            && self
                .evidence
                .iter()
                .any(|e| e.confidence == Confidence::VerifiedOnBuild)
        {
            return Err(crate::error::invalid(
                "Fixture evidence cannot claim live build verification",
            ));
        }
        Ok(())
    }
}

/// Separate v2 catalog payload. Ordinary v1 decoding retains its 1,024 bound.
pub const MAX_CATALOG_IDENTITIES: usize = 250_000;
pub const MAX_CATALOG_EVIDENCE: usize = 250_000;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct CatalogDataset(pub ReferenceDataset);
impl std::ops::Deref for CatalogDataset {
    type Target = ReferenceDataset;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl AsRef<ReferenceDataset> for CatalogDataset {
    fn as_ref(&self) -> &ReferenceDataset {
        &self.0
    }
}
impl AsRef<ReferenceDataset> for ReferenceDataset {
    fn as_ref(&self) -> &ReferenceDataset {
        self
    }
}
impl ReferenceDataset {
    pub fn identity_count(&self) -> usize {
        self.items.len()
            + self.acquisitions.len()
            + self.quests.len()
            + self.map_spaces.len()
            + self.nodes.len()
            + self.edges.len()
            + self.objectives.len()
            + self.rules.len()
            + self.map_images.len()
            + self.map_embeddings.len()
            + self
                .map_images
                .iter()
                .map(|image| image.pins.len())
                .sum::<usize>()
            + self
                .quests
                .iter()
                .map(|quest| quest.stages.len())
                .sum::<usize>()
    }
}
impl Payload for CatalogDataset {
    const KIND: ContractKind = ContractKind::ReferenceDataset;
    fn validate_payload(&self, header: &EnvelopeHeader) -> Result<()> {
        validate_catalog_capacity(self.identity_count(), self.evidence.len())?;
        self.0.validate_reference(header, MAX_CATALOG_IDENTITIES)
    }
}

pub(crate) fn unique_catalog<T: Ord>(values: impl IntoIterator<Item = T>) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(crate::error::invalid("Duplicate catalog identifier"));
        }
        if seen.len() > MAX_CATALOG_IDENTITIES {
            return Err(crate::error::invalid("Catalog identity capacity exceeded"));
        }
    }
    Ok(())
}

pub fn validate_catalog_capacity(identities: usize, evidence: usize) -> Result<()> {
    if identities > MAX_CATALOG_IDENTITIES || evidence > MAX_CATALOG_EVIDENCE {
        return Err(crate::error::invalid(
            "Reference catalog exceeds identity or evidence capacity",
        ));
    }
    Ok(())
}
