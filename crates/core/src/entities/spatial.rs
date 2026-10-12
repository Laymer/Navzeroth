use super::{bounded, requirement_context, unique};
use crate::{
    error::{Result, Validate, Validated, invalid},
    observation::Observation,
    predicate::Predicate,
    types::*,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "CoordinateTransform")]
pub struct CoordinateTransformData {
    pub units: Text,
    pub scale_x: FiniteNumber,
    pub scale_y: FiniteNumber,
    pub offset_x: FiniteNumber,
    pub offset_y: FiniteNumber,
}
pub type CoordinateTransform = Validated<CoordinateTransformData>;
impl Validate for CoordinateTransformData {
    fn validate(&self) -> Result<()> {
        if self.units.as_str().is_empty() || self.scale_x.get() == 0. || self.scale_y.get() == 0. {
            return Err(invalid(
                "Coordinate transform requires named units and nonzero scale",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "MapSpace")]
pub struct MapSpaceData {
    pub key: MapKey,
    pub transform: Observation<CoordinateTransform>,
    pub evidence: Vec<EvidenceId>,
}
pub type MapSpace = Validated<MapSpaceData>;
impl Validate for MapSpaceData {
    fn validate(&self) -> Result<()> {
        unique(self.evidence.iter())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    Junction,
    Vendor,
    Repair,
    Trainer,
    Flight,
    Transport,
    Dungeon,
    Other,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "TravelNode")]
pub struct TravelNodeData {
    pub id: NodeId,
    pub position: Position,
    pub service: Observation<ServiceKind>,
}
pub type TravelNode = Validated<TravelNodeData>;
impl Validate for TravelNodeData {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct NodeRef {
    pub id: NodeId,
    pub map: MapKey,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TravelMode {
    Walking,
    Flight,
    Boat,
    Tram,
    Teleport,
    Return,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EdgeStatus {
    Verified,
    Estimated,
    TemporarilyUnavailable,
    Unverified,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "DurationRange")]
pub struct DurationRangeData {
    pub min_ms: Millis,
    pub max_ms: Millis,
}
pub type DurationRange = Validated<DurationRangeData>;
impl Validate for DurationRangeData {
    fn validate(&self) -> Result<()> {
        if self.min_ms > self.max_ms {
            return Err(invalid("Duration range is inverted"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "TravelEdge")]
pub struct TravelEdgeData {
    pub id: EdgeId,
    pub context: DataContext,
    pub from: NodeRef,
    pub to: NodeRef,
    pub mode: TravelMode,
    pub requirements: Observation<Predicate>,
    pub expected_ms: Observation<Millis>,
    pub wait_ms: Observation<Millis>,
    pub uncertainty: Observation<DurationRange>,
    pub status: EdgeStatus,
    pub evidence: Vec<EvidenceId>,
    pub consumes_resource: Observation<bool>,
    pub cooldown_ms: Observation<Millis>,
}
pub type TravelEdge = Validated<TravelEdgeData>;
impl Validate for TravelEdgeData {
    fn validate(&self) -> Result<()> {
        requirement_context(&self.requirements, &self.context)?;
        self.context.ensure_compatible(&self.from.map.context)?;
        self.context.ensure_compatible(&self.to.map.context)?;
        unique(self.evidence.iter())?;
        if self.status == EdgeStatus::Verified && self.evidence.is_empty() {
            return Err(invalid("Verified edge requires evidence"));
        }
        if let (Some(expected), Some(range)) = (self.expected_ms.value(), self.uncertainty.value())
            && (*expected < range.min_ms || *expected > range.max_ms)
        {
            return Err(invalid("Expected duration lies outside uncertainty range"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObjectiveTarget {
    QuestStage { quest: QuestId, stage: ObjectiveId },
    Acquisition { source: AcquisitionId },
    Service { node: NodeRef },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Objective")]
pub struct ObjectiveData {
    pub id: ObjectiveId,
    pub context: DataContext,
    pub participants: Vec<CharacterId>,
    pub targets: Vec<ObjectiveTarget>,
    pub requirements: Observation<Predicate>,
    pub expected_ms: Observation<Millis>,
    pub evidence: Vec<EvidenceId>,
}
pub type Objective = Validated<ObjectiveData>;
impl Validate for ObjectiveData {
    fn validate(&self) -> Result<()> {
        requirement_context(&self.requirements, &self.context)?;
        unique(self.participants.iter())?;
        unique(self.evidence.iter())?;
        bounded(self.targets.len())?;
        if self.participants.is_empty() || self.targets.is_empty() {
            return Err(invalid("Objective requires participants and targets"));
        }
        for target in &self.targets {
            if let ObjectiveTarget::Service { node } = target {
                self.context.ensure_compatible(&node.map.context)?;
            }
        }
        Ok(())
    }
}
