//! Pure projection of normalized positions onto map imagery, and an
//! axis-aligned calibration solver (ADR 0002). A gap is an outcome the UI
//! explains, never a guessed marker. No clock, no I/O.
use crate::{
    contract::ReferenceDataset,
    entities::{Confidence, CoordinateTransformData, MAX_EMBEDDING_DEPTH, MapImage, PIXEL_UNITS},
    observation::ObservationStatus,
    types::*,
};
use schemars::JsonSchema;
use serde::Serialize;

/// Why a position could not be placed on an image.
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
#[serde(tag = "gap", rename_all = "snake_case")]
pub enum ProjectionGap {
    /// The position's namespace differs from the dataset's (demo vs reference).
    NamespaceMismatch,
    /// No image and no embedding chain exists for the map.
    NoImageForMap { map: MapKey },
    /// The position lies inside an instance; instance maps have no imagery here.
    InInstance { map: MapKey },
    /// An image exists for the map but on another floor.
    DifferentFloor {
        map: MapKey,
        available_floors: Vec<FloorId>,
    },
    /// An image or embedding exists for the map, but not for the position's build.
    BuildMismatch {
        position_build: BuildKey,
        image: Option<ImageId>,
        applicable_builds: Vec<BuildKey>,
    },
    /// Defensive guard; datasets reject such chains at validation.
    EmbeddingChainTooDeep { via: Vec<MapKey> },
    /// The image has no known calibration.
    UncalibratedImage {
        image: ImageId,
        status: ObservationStatus,
    },
    /// The calibrated pixel falls outside the image frame.
    OutOfBounds { image: ImageId, pixel: PixelPoint },
}
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
#[serde(tag = "notice", rename_all = "snake_case")]
pub enum ProjectionNotice {
    /// The position's build is listed as applicable but is not the image build.
    CrossBuildReuse { image_build: BuildKey },
    /// The position was mapped through zone-in-parent rectangles.
    EmbeddingFallback { via: Vec<MapKey> },
}
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
pub struct Projection {
    pub image: ImageId,
    /// Full-frame pixels.
    pub pixel: PixelPoint,
    pub notices: Vec<ProjectionNotice>,
    pub calibration_evidence: Vec<EvidenceId>,
    /// Weakest confidence among the calibration evidence records found in the
    /// dataset; absent when none of them is present.
    pub calibration_confidence: Option<Confidence>,
}

fn same_space(a: &MapKey, b: &MapKey) -> bool {
    a.map_id == b.map_id && a.floor == b.floor && a.instance == b.instance
}
fn confidence_rank(c: Confidence) -> u8 {
    match c {
        Confidence::VerifiedOnBuild => 0,
        Confidence::PublisherDocumented => 1,
        Confidence::LimitedSample => 2,
        Confidence::CrossEditionUnverified => 3,
        Confidence::Inferred => 4,
        Confidence::DisputedOrStale => 5,
    }
}
fn weakest_confidence(image: &MapImage, dataset: &ReferenceDataset) -> Option<Confidence> {
    dataset
        .evidence
        .iter()
        .filter(|e| image.calibration_evidence.contains(&e.id))
        .map(|e| e.confidence)
        .max_by_key(|c| confidence_rank(*c))
}

/// Project a position onto the first applicable image in dataset order.
/// A gap is an ordinary outcome carrying the keys the UI explains, so the
/// large `Err` variant is intentional.
#[allow(clippy::result_large_err)]
/// `context` is the dataset envelope's context. The position's build is carried
/// unchanged through embeddings and checked against `applicable_builds`.
pub fn project_position(
    position: &Position,
    context: &DataContext,
    dataset: &ReferenceDataset,
) -> Result<Projection, ProjectionGap> {
    if position.map.context.namespace != context.namespace {
        return Err(ProjectionGap::NamespaceMismatch);
    }
    let build = &position.map.context.build;
    let mut current = position.map.clone();
    let (mut x, mut y) = (position.x, position.y);
    let mut via = Vec::new();
    for _ in 0..=MAX_EMBEDDING_DEPTH {
        let mut floors = Vec::new();
        let mut build_mismatch = None;
        for image in &dataset.map_images {
            if image.map.map_id != current.map_id || image.map.instance != current.instance {
                continue;
            }
            if image.map.floor != current.floor {
                floors.push(image.map.floor.clone());
                continue;
            }
            if !image.applies_to_build(build) {
                build_mismatch.get_or_insert_with(|| ProjectionGap::BuildMismatch {
                    position_build: build.clone(),
                    image: Some(image.id.clone()),
                    applicable_builds: image
                        .applicable_builds
                        .iter()
                        .map(|b| b.build.clone())
                        .collect(),
                });
                continue;
            }
            return place(image, dataset, build, x, y, via);
        }
        if let Some(gap) = build_mismatch {
            return Err(gap);
        }
        let embedding = dataset.map_embeddings.iter().find(|e| e.child == current);
        match embedding {
            Some(embedding) => {
                let (nx, ny) = embedding
                    .to_parent(x, y)
                    .expect("validated bounds keep normalized points in range");
                x = nx;
                y = ny;
                via.push(current);
                current = embedding.parent.clone();
            }
            None => {
                if dataset
                    .map_embeddings
                    .iter()
                    .any(|e| same_space(&e.child, &current) && e.child.context != current.context)
                {
                    return Err(ProjectionGap::BuildMismatch {
                        position_build: build.clone(),
                        image: None,
                        applicable_builds: Vec::new(),
                    });
                }
                if current.instance.is_some() {
                    return Err(ProjectionGap::InInstance { map: current });
                }
                if !floors.is_empty() {
                    return Err(ProjectionGap::DifferentFloor {
                        map: current,
                        available_floors: floors,
                    });
                }
                return Err(ProjectionGap::NoImageForMap { map: current });
            }
        }
    }
    Err(ProjectionGap::EmbeddingChainTooDeep { via })
}

#[allow(clippy::result_large_err)]
fn place(
    image: &MapImage,
    dataset: &ReferenceDataset,
    build: &BuildKey,
    x: Normalized,
    y: Normalized,
    via: Vec<MapKey>,
) -> Result<Projection, ProjectionGap> {
    let Some(transform) = image.calibration.value() else {
        return Err(ProjectionGap::UncalibratedImage {
            image: image.id.clone(),
            status: image.calibration.status(),
        });
    };
    let (px, py) = transform.apply(x, y);
    let clamp = |v: f64| {
        FiniteNumber::new(v.clamp(-(MAX_SAFE_INTEGER as f64), MAX_SAFE_INTEGER as f64))
            .expect("clamped values are finite and within range")
    };
    let pixel = PixelPoint {
        x: clamp(px),
        y: clamp(py),
    };
    if !image.frame_contains(&pixel) {
        return Err(ProjectionGap::OutOfBounds {
            image: image.id.clone(),
            pixel,
        });
    }
    let mut notices = Vec::new();
    if &image.map.context.build != build {
        notices.push(ProjectionNotice::CrossBuildReuse {
            image_build: image.map.context.build.clone(),
        });
    }
    if !via.is_empty() {
        notices.push(ProjectionNotice::EmbeddingFallback { via });
    }
    Ok(Projection {
        image: image.id.clone(),
        pixel,
        notices,
        calibration_evidence: image.calibration_evidence.clone(),
        calibration_confidence: weakest_confidence(image, dataset),
    })
}

/// One measured pair for calibration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalibrationPair {
    pub normalized_x: Normalized,
    pub normalized_y: Normalized,
    pub pixel: PixelPoint,
}
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
pub struct CalibrationSolution {
    pub transform: CoordinateTransformData,
    pub residuals_px: Vec<FiniteNumber>,
    pub rms_px: FiniteNumber,
    pub max_px: FiniteNumber,
}
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum CalibrationError {
    TooFewPoints {
        needed: usize,
        got: usize,
    },
    /// The normalized inputs do not spread along both axes.
    DegenerateSpread,
    NotFinite,
}
pub const MIN_CALIBRATION_POINTS: usize = 3;
const MIN_SPREAD: f64 = 1e-6;

fn fit_axis(
    samples: impl Iterator<Item = (f64, f64)> + Clone,
) -> Result<(f64, f64), CalibrationError> {
    let n = samples.clone().count() as f64;
    let mean_n = samples.clone().map(|(a, _)| a).sum::<f64>() / n;
    let mean_p = samples.clone().map(|(_, b)| b).sum::<f64>() / n;
    let var = samples
        .clone()
        .map(|(a, _)| (a - mean_n).powi(2))
        .sum::<f64>();
    if var < MIN_SPREAD {
        return Err(CalibrationError::DegenerateSpread);
    }
    let cov = samples
        .map(|(a, b)| (a - mean_n) * (b - mean_p))
        .sum::<f64>();
    let scale = cov / var;
    if scale == 0. {
        return Err(CalibrationError::DegenerateSpread);
    }
    Ok((scale, mean_p - scale * mean_n))
}

/// Least-squares per-axis affine fit (no rotation or shear). Needs at least
/// three pairs spread along both axes. Residuals above a consumer's threshold
/// mean the imagery is not axis-aligned to the map and calibration must stay
/// unknown.
pub fn solve_axis_aligned_calibration(
    pairs: &[CalibrationPair],
) -> Result<CalibrationSolution, CalibrationError> {
    if pairs.len() < MIN_CALIBRATION_POINTS {
        return Err(CalibrationError::TooFewPoints {
            needed: MIN_CALIBRATION_POINTS,
            got: pairs.len(),
        });
    }
    let (scale_x, offset_x) = fit_axis(
        pairs
            .iter()
            .map(|p| (p.normalized_x.get(), p.pixel.x.get())),
    )?;
    let (scale_y, offset_y) = fit_axis(
        pairs
            .iter()
            .map(|p| (p.normalized_y.get(), p.pixel.y.get())),
    )?;
    let finite = |v: f64| FiniteNumber::new(v).map_err(|_| CalibrationError::NotFinite);
    let transform = CoordinateTransformData {
        units: Text::new(PIXEL_UNITS).expect("constant units text"),
        scale_x: finite(scale_x)?,
        scale_y: finite(scale_y)?,
        offset_x: finite(offset_x)?,
        offset_y: finite(offset_y)?,
    };
    let mut residuals = Vec::with_capacity(pairs.len());
    for pair in pairs {
        let (px, py) = transform.apply(pair.normalized_x, pair.normalized_y);
        residuals.push(finite(
            ((px - pair.pixel.x.get()).powi(2) + (py - pair.pixel.y.get()).powi(2)).sqrt(),
        )?);
    }
    let rms =
        (residuals.iter().map(|r| r.get().powi(2)).sum::<f64>() / residuals.len() as f64).sqrt();
    let max = residuals.iter().map(|r| r.get()).fold(0., f64::max);
    Ok(CalibrationSolution {
        transform,
        rms_px: finite(rms)?,
        max_px: finite(max)?,
        residuals_px: residuals,
    })
}
