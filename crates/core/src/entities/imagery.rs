//! Map imagery records: raster layers, image-space pins, calibration and
//! zone-in-parent embeddings (ADR 0002). Core carries declared file digests and
//! dimensions only; decoding and hashing happen outside this crate. Pixels are
//! exact measurements on the image; normalized positions are not, so pins never
//! become travel nodes here.
use super::{CoordinateTransform, bounded, unique};
use crate::{
    error::{Result, Validate, Validated, invalid},
    observation::Observation,
    types::*,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Units string a `MapImage.calibration` transform must declare.
pub const PIXEL_UNITS: &str = "pixel";
/// Tolerance, in pixels, when a declared control-point residual is checked
/// against the distance recomputed from the calibration.
pub const RESIDUAL_TOLERANCE_PX: f64 = 0.5;
/// Longest allowed chain of `MapEmbedding` records (city -> zone -> continent needs 2).
pub const MAX_EMBEDDING_DEPTH: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImageLayerKind {
    Terrain,
    /// Terrain with a manually traced road overlay baked in; not verified edges.
    RoadsTraced,
    /// Terrain with a reference flight network baked in; not a character's unlocks.
    FlightReference,
    /// Downscaled copy of another layer for small views.
    Overview,
    Other,
}

/// One raster file of a `MapImage`. Pixel coordinates in pins and calibration
/// always refer to the full frame; a layer with `frame_divisor` 2 is half size.
/// `registration_offset_px` is the offset applied to the layer's source crop so
/// it registers with the frame (zero once the panels are aligned).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ImageLayer {
    pub id: LayerId,
    pub kind: ImageLayerKind,
    /// Relative to the content root of the dataset's namespace.
    pub file: AssetPath,
    pub sha256: Sha256Hex,
    pub width_px: PixelExtent,
    pub height_px: PixelExtent,
    pub frame_divisor: PixelExtent,
    pub registration_offset_px: PixelPoint,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum FlightAccess {
    Alliance,
    Horde,
    TowerRoute,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PinKind {
    FlightMaster { access: Vec<FlightAccess> },
    Settlement,
    Landmark,
}

/// A labelled point measured on the image, in full-frame pixels. Reference
/// geography, not an unlock.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ImagePin {
    pub id: PinId,
    pub label: Text,
    pub kind: PinKind,
    pub pixel: PixelPoint,
    /// The layer the pixel was measured on.
    pub measured_on: LayerId,
    pub evidence: Vec<EvidenceId>,
}

/// A pin paired with a travel node whose measured normalized position supplies
/// the other side of a calibration pair.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ControlPoint {
    pub pin: PinId,
    pub node: NodeId,
    /// Distance in pixels between the pin and the node projected through the
    /// calibration; requires a known calibration.
    pub residual_px: Observation<FiniteNumber>,
}

/// A build this imagery may be reused for. The image's own build needs no
/// evidence; every other build requires evidence that the imagery still applies.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ApplicableBuild {
    pub build: BuildKey,
    pub evidence: Vec<EvidenceId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "MapImage")]
pub struct MapImageData {
    pub id: ImageId,
    pub map: MapKey,
    /// Registry ID in docs/data/SOURCES.md; not resolved inside a dataset.
    pub source: SourceId,
    pub evidence: Vec<EvidenceId>,
    pub applicable_builds: Vec<ApplicableBuild>,
    pub width_px: PixelExtent,
    pub height_px: PixelExtent,
    pub layers: Vec<ImageLayer>,
    pub pins: Vec<ImagePin>,
    /// Normalized map coordinate -> full-frame pixel; units must be `pixel`.
    pub calibration: Observation<CoordinateTransform>,
    pub calibration_evidence: Vec<EvidenceId>,
    pub control_points: Vec<ControlPoint>,
    /// What the imagery does not show or verify (plan section 12.3).
    pub gaps: Text,
}
pub type MapImage = Validated<MapImageData>;
impl MapImageData {
    pub fn frame_contains(&self, pixel: &PixelPoint) -> bool {
        (0.0..=f64::from(self.width_px.get())).contains(&pixel.x.get())
            && (0.0..=f64::from(self.height_px.get())).contains(&pixel.y.get())
    }
    pub fn applies_to_build(&self, build: &BuildKey) -> bool {
        self.applicable_builds.iter().any(|b| &b.build == build)
    }
}
impl Validate for MapImageData {
    fn validate(&self) -> Result<()> {
        bounded(self.layers.len())?;
        bounded(self.pins.len())?;
        bounded(self.control_points.len())?;
        unique(self.layers.iter().map(|l| &l.id))?;
        unique(self.pins.iter().map(|p| &p.id))?;
        unique(self.control_points.iter().map(|c| &c.pin))?;
        unique(self.control_points.iter().map(|c| &c.node))?;
        unique(self.evidence.iter())?;
        unique(self.calibration_evidence.iter())?;
        unique(self.applicable_builds.iter().map(|b| &b.build))?;
        if self.layers.is_empty() {
            return Err(invalid("Map image requires at least one layer"));
        }
        unique(self.layers.iter().map(|l| &l.file))?;
        for layer in &self.layers {
            let d = layer.frame_divisor.get();
            if layer.width_px.get() != self.width_px.get().div_ceil(d)
                || layer.height_px.get() != self.height_px.get().div_ceil(d)
            {
                return Err(invalid(
                    "Layer size disagrees with the image frame and divisor",
                ));
            }
        }
        for pin in &self.pins {
            unique(pin.evidence.iter())?;
            if !self.frame_contains(&pin.pixel) {
                return Err(invalid("Pin lies outside the image frame"));
            }
            if !self.layers.iter().any(|l| l.id == pin.measured_on) {
                return Err(invalid("Pin was measured on an undeclared layer"));
            }
        }
        for point in &self.control_points {
            if !self.pins.iter().any(|p| p.id == point.pin) {
                return Err(invalid("Control point names an undeclared pin"));
            }
            if point.residual_px.value().is_some() && self.calibration.value().is_none() {
                return Err(invalid(
                    "Control-point residual requires a known calibration",
                ));
            }
            if point.residual_px.value().is_some_and(|r| r.get() < 0.) {
                return Err(invalid("Control-point residual cannot be negative"));
            }
        }
        let own = &self.map.context.build;
        if !self.applies_to_build(own) {
            return Err(invalid("Applicable builds must include the image build"));
        }
        for applicable in &self.applicable_builds {
            unique(applicable.evidence.iter())?;
            if applicable.build.edition != own.edition {
                return Err(invalid("Imagery cannot be reused across editions"));
            }
            if &applicable.build != own && applicable.evidence.is_empty() {
                return Err(invalid(
                    "Reusing imagery for another build requires evidence",
                ));
            }
        }
        if let Some(transform) = self.calibration.value() {
            if transform.units.as_str() != PIXEL_UNITS {
                return Err(invalid("Image calibration units must be pixel"));
            }
            if self.calibration_evidence.is_empty() {
                return Err(invalid("Known calibration requires evidence"));
            }
        }
        Ok(())
    }
}

/// The child map's rectangle inside its parent map, in the parent's normalized
/// space (GetMapRectOnMap style). Both keys share one exact context.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "MapEmbedding")]
pub struct MapEmbeddingData {
    pub child: MapKey,
    pub parent: MapKey,
    pub bounds: Bounds,
    pub evidence: Vec<EvidenceId>,
}
pub type MapEmbedding = Validated<MapEmbeddingData>;
impl MapEmbeddingData {
    /// Map a child-normalized point into the parent's normalized space.
    pub fn to_parent(&self, x: Normalized, y: Normalized) -> Result<(Normalized, Normalized)> {
        let b = &self.bounds;
        let px = b.min_x.get() + x.get() * (b.max_x.get() - b.min_x.get());
        let py = b.min_y.get() + y.get() * (b.max_y.get() - b.min_y.get());
        Ok((Normalized::new(px)?, Normalized::new(py)?))
    }
}
impl Validate for MapEmbeddingData {
    fn validate(&self) -> Result<()> {
        unique(self.evidence.iter())?;
        if self.child == self.parent {
            return Err(invalid("A map cannot embed itself"));
        }
        self.child.context.ensure_compatible(&self.parent.context)?;
        if self.bounds.min_x.get() >= self.bounds.max_x.get()
            || self.bounds.min_y.get() >= self.bounds.max_y.get()
        {
            return Err(invalid(
                "Embedding bounds must have positive width and height",
            ));
        }
        Ok(())
    }
}
