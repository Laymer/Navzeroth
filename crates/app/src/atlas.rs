//! Atlas command handlers and the hash-verified atlas asset resolver (ADR 0002
//! PR-B, M6.1). Everything here is pure: no clock, no filesystem, no network.
//! The desktop shell passes `now_utc_ms` and adapts `resolve_atlas_asset` to a
//! URI scheme; this module owns every decision.
//!
//! Until stored datasets exist (M2.4), the only atlas is the synthetic demo
//! dataset embedded below, and every response says so (`data_mode: demo`).
use crate::{DataMode, demo_snapshot};
use azeroth_core::atlas::{Projection, ProjectionGap, project_position};
use azeroth_core::contract::{ReferenceDataset, decode_envelope};
use azeroth_core::entities::{
    CharacterSnapshot, Confidence, ImageLayer, ImageLayerKind, ImagePin, MapImage,
};
use azeroth_core::error::{AppError, ErrorCode};
use azeroth_core::observation::{
    ClockTrust, Freshness, FreshnessContext, Observation, ObservationStatus, freshness,
};
use azeroth_core::types::{
    AssetPath, DataContext, DatasetNamespace, DatasetRevision, ImageId, LayerId, MapKey, Millis,
    PixelExtent, Position, Sha256Hex, SnapshotId, Text, UtcMillis,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const DEMO_DATASET: &str =
    include_str!("../../../fixtures/contracts/v1/reference-dataset-imagery.json");

/// The convention shared with the other bounded lists (plan section 13).
pub const MAX_POSITIONS: usize = 64;
/// A snapshot older than this is stale (plan section 12.6; ADR 0002).
pub const SNAPSHOT_MAX_AGE_MS: u64 = 30 * 60 * 1000;
/// Every embedded layer is a PNG; nothing else is ever served.
pub const ATLAS_CONTENT_TYPE: &str = "image/png";

// ---------------------------------------------------------------------------
// Embedded demo assets. AssetPath values are relative to `content/demo`.
// ---------------------------------------------------------------------------

fn embedded_bytes(file: &AssetPath) -> Option<&'static [u8]> {
    match file.as_str() {
        "atlas/synthetic-continent/layers/overview.png" => Some(include_bytes!(
            "../../../content/demo/atlas/synthetic-continent/layers/overview.png"
        )),
        "atlas/synthetic-continent/layers/roads.png" => Some(include_bytes!(
            "../../../content/demo/atlas/synthetic-continent/layers/roads.png"
        )),
        "atlas/synthetic-continent/layers/flight.png" => Some(include_bytes!(
            "../../../content/demo/atlas/synthetic-continent/layers/flight.png"
        )),
        _ => None,
    }
}

fn digest_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// The decoded demo dataset plus the one-time integrity result per layer.
struct DemoAtlas {
    context: DataContext,
    fixture: bool,
    dataset: ReferenceDataset,
    integrity: BTreeMap<(ImageId, LayerId), LayerCheck>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LayerCheck {
    Verified,
    Mismatch,
    NotEmbedded,
}

fn check_layer(layer: &ImageLayer, bytes: Option<&[u8]>) -> LayerCheck {
    match bytes {
        None => LayerCheck::NotEmbedded,
        Some(bytes) if digest_hex(bytes) == layer.sha256.as_str() => LayerCheck::Verified,
        Some(_) => LayerCheck::Mismatch,
    }
}

fn demo_atlas() -> Result<&'static DemoAtlas, AppError> {
    static ATLAS: OnceLock<Result<DemoAtlas, AppError>> = OnceLock::new();
    ATLAS
        .get_or_init(|| {
            let envelope = decode_envelope::<ReferenceDataset>(DEMO_DATASET)?;
            let context = envelope.data_context();
            let fixture = envelope.fixture;
            let dataset = envelope.into_inner().payload;
            let mut integrity = BTreeMap::new();
            for image in &dataset.map_images {
                for layer in &image.layers {
                    integrity.insert(
                        (image.id.clone(), layer.id.clone()),
                        check_layer(layer, embedded_bytes(&layer.file)),
                    );
                }
            }
            Ok(DemoAtlas {
                context,
                fixture,
                dataset,
                integrity,
            })
        })
        .as_ref()
        .map_err(Clone::clone)
}

// ---------------------------------------------------------------------------
// Asset resolver
// ---------------------------------------------------------------------------

/// A verified layer file ready to be served.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasAsset<'a> {
    pub content_type: &'static str,
    pub bytes: &'a [u8],
    /// The digest the dataset declares and the bytes were verified against.
    pub sha256: &'a Sha256Hex,
}

/// Why an asset request was refused. No variant carries request text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtlasAssetError {
    /// The path is not exactly `<image id>/<layer id>` in the safe alphabet.
    InvalidPath,
    /// No dataset-declared image or layer has these IDs.
    NotFound,
    /// The layer is declared, but no file is available for it.
    NotEmbedded,
    /// The bytes do not match the declared digest.
    IntegrityMismatch,
    /// The dataset itself could not be loaded.
    DatasetUnavailable,
}
impl AtlasAssetError {
    /// HTTP status the scheme adapter answers with (an empty body).
    pub fn http_status(self) -> u16 {
        match self {
            Self::InvalidPath | Self::NotFound | Self::NotEmbedded => 404,
            Self::IntegrityMismatch => 403,
            Self::DatasetUnavailable => 500,
        }
    }
}

fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Percent-decode exactly once. Anything malformed is refused.
fn percent_decode_once(segment: &str) -> Option<String> {
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = segment.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Split `<image>/<layer>` (no leading slash) into two decoded, safe IDs.
fn parse_asset_path(path: &str) -> Result<(String, String), AtlasAssetError> {
    let mut parts = path.split('/');
    let (Some(image), Some(layer), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err(AtlasAssetError::InvalidPath);
    };
    let decode = |raw: &str| {
        percent_decode_once(raw)
            .filter(|decoded| is_safe_id(decoded))
            .ok_or(AtlasAssetError::InvalidPath)
    };
    Ok((decode(image)?, decode(layer)?))
}

fn find_layer<'a>(
    dataset: &'a ReferenceDataset,
    path: &str,
) -> Result<(&'a MapImage, &'a ImageLayer), AtlasAssetError> {
    let (image_id, layer_id) = parse_asset_path(path)?;
    let image = dataset
        .map_images
        .iter()
        .find(|i| i.id.as_str() == image_id)
        .ok_or(AtlasAssetError::NotFound)?;
    let layer = image
        .layers
        .iter()
        .find(|l| l.id.as_str() == layer_id)
        .ok_or(AtlasAssetError::NotFound)?;
    Ok((image, layer))
}

/// Resolve `path` against `dataset`, reading bytes only through `source` and
/// hashing them on every call. This is the injectable form used by tests; the
/// shell uses [`resolve_atlas_asset`].
pub fn resolve_asset_with<'a>(
    dataset: &'a ReferenceDataset,
    path: &str,
    source: &dyn Fn(&AssetPath) -> Option<&'a [u8]>,
) -> Result<AtlasAsset<'a>, AtlasAssetError> {
    let (_, layer) = find_layer(dataset, path)?;
    let bytes = source(&layer.file);
    match check_layer(layer, bytes) {
        LayerCheck::Verified => Ok(AtlasAsset {
            content_type: ATLAS_CONTENT_TYPE,
            bytes: bytes.ok_or(AtlasAssetError::NotEmbedded)?,
            sha256: &layer.sha256,
        }),
        LayerCheck::Mismatch => Err(AtlasAssetError::IntegrityMismatch),
        LayerCheck::NotEmbedded => Err(AtlasAssetError::NotEmbedded),
    }
}

/// Serve a demo atlas layer: `path` is the URI path without its leading slash,
/// exactly `<image id>/<layer id>`. Only dataset-declared layers whose embedded
/// bytes match the declared digest are returned.
pub fn resolve_atlas_asset(path: &str) -> Result<AtlasAsset<'static>, AtlasAssetError> {
    let atlas = demo_atlas().map_err(|_| AtlasAssetError::DatasetUnavailable)?;
    let (image, layer) = find_layer(&atlas.dataset, path)?;
    match atlas.integrity.get(&(image.id.clone(), layer.id.clone())) {
        Some(LayerCheck::Verified) => Ok(AtlasAsset {
            content_type: ATLAS_CONTENT_TYPE,
            bytes: embedded_bytes(&layer.file).ok_or(AtlasAssetError::NotEmbedded)?,
            sha256: &layer.sha256,
        }),
        Some(LayerCheck::Mismatch) => Err(AtlasAssetError::IntegrityMismatch),
        Some(LayerCheck::NotEmbedded) | None => Err(AtlasAssetError::NotEmbedded),
    }
}

/// What the `atlas` URI scheme answers with. Errors have an empty body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasReply {
    pub status: u16,
    pub content_type: Option<&'static str>,
    pub body: &'static [u8],
}

/// Pure request handler behind the `atlas` scheme: GET only, `uri_path` is the
/// request URI path including its single leading slash. The shell adds
/// `Cache-Control: no-store` and `X-Content-Type-Options: nosniff`.
pub fn serve_atlas(method: &str, uri_path: &str) -> AtlasReply {
    let refuse = |status| AtlasReply {
        status,
        content_type: None,
        body: &[],
    };
    if method != "GET" {
        return refuse(405);
    }
    let Some(path) = uri_path.strip_prefix('/') else {
        return refuse(404);
    };
    match resolve_atlas_asset(path) {
        Ok(asset) => AtlasReply {
            status: 200,
            content_type: Some(asset.content_type),
            body: asset.bytes,
        },
        Err(error) => refuse(error.http_status()),
    }
}

// ---------------------------------------------------------------------------
// URLs
// ---------------------------------------------------------------------------

/// How the webview addresses the `atlas` custom scheme on this platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemeStyle {
    /// Linux and macOS: `atlas://localhost/<image>/<layer>`.
    Custom,
    /// Windows and Android: `http://atlas.localhost/<image>/<layer>`.
    HttpLocalhost,
}
impl SchemeStyle {
    pub fn for_platform() -> Self {
        if cfg!(windows) {
            Self::HttpLocalhost
        } else {
            Self::Custom
        }
    }
    /// The single place layer URLs are built. `None` when an ID is outside the
    /// safe alphabet, because such a layer could not be requested anyway.
    pub fn layer_url(self, image: &ImageId, layer: &LayerId) -> Option<String> {
        if !is_safe_id(image.as_str()) || !is_safe_id(layer.as_str()) {
            return None;
        }
        Some(match self {
            Self::Custom => format!("atlas://localhost/{}/{}", image.as_str(), layer.as_str()),
            Self::HttpLocalhost => {
                format!(
                    "http://atlas.localhost/{}/{}",
                    image.as_str(),
                    layer.as_str()
                )
            }
        })
    }
}

// ---------------------------------------------------------------------------
// DTOs (snake_case wire names; mirrored by hand in apps/desktop/src/commands)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerAvailability {
    Available,
    /// Declared, but no embedded file exists.
    NotEmbedded,
    /// The embedded file does not match the declared digest; it is not served.
    IntegrityMismatch,
    /// The IDs cannot be addressed through the scheme.
    UnservableId,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AtlasLayer {
    pub id: LayerId,
    pub kind: ImageLayerKind,
    pub url: Option<String>,
    pub availability: LayerAvailability,
    pub sha256: Sha256Hex,
    pub width_px: PixelExtent,
    pub height_px: PixelExtent,
    pub frame_divisor: PixelExtent,
}

/// An image pin plus the display rule that it is reference geography only.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AtlasPin {
    #[serde(flatten)]
    pub pin: ImagePin,
    /// Always true: a pin says where something is on the picture, never that
    /// the character has unlocked it (CASE-004).
    pub reference_only: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AtlasImage {
    pub id: ImageId,
    pub map: MapKey,
    pub width_px: PixelExtent,
    pub height_px: PixelExtent,
    pub layers: Vec<AtlasLayer>,
    pub pins: Vec<AtlasPin>,
    pub calibration_status: ObservationStatus,
    /// Weakest confidence among the calibration evidence records.
    pub calibration_confidence: Option<Confidence>,
    /// Display rule local to this crate, not an evidence precedence: true
    /// unless the calibration is verified on the build, or sampled from at
    /// least three in-game control points.
    pub approximate: bool,
    /// What the imagery does not show or verify.
    pub gaps: Text,
    pub attribution: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AtlasView {
    pub data_mode: DataMode,
    pub fixture: bool,
    pub revision: DatasetRevision,
    pub namespace: DatasetNamespace,
    pub images: Vec<AtlasImage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<crate::stored_atlas::AtlasPage>,
}

/// A position placed on an image, or the explicit reason it was not.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ProjectedPosition {
    Placed(Projection),
    Gap(ProjectionGap),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SnapshotMarker {
    pub profile_id: String,
    pub snapshot_id: SnapshotId,
    pub location_status: ObservationStatus,
    pub observed_at_utc_ms: Option<UtcMillis>,
    pub received_at_utc_ms: Option<UtcMillis>,
    pub freshness: Freshness,
    /// Absent unless the location is known.
    pub projection: Option<ProjectedPosition>,
    /// Present only for a placed marker: true when the calibration is approximate.
    pub approximate: Option<bool>,
    pub fixture: bool,
    pub data_mode: DataMode,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub(crate) fn confidence_rank(c: Confidence) -> u8 {
    match c {
        Confidence::VerifiedOnBuild => 0,
        Confidence::PublisherDocumented => 1,
        Confidence::LimitedSample => 2,
        Confidence::CrossEditionUnverified => 3,
        Confidence::Inferred => 4,
        Confidence::DisputedOrStale => 5,
    }
}

fn calibration_confidence(image: &MapImage, dataset: &ReferenceDataset) -> Option<Confidence> {
    dataset
        .evidence
        .iter()
        .filter(|e| image.calibration_evidence.contains(&e.id))
        .map(|e| e.confidence)
        .max_by_key(|c| confidence_rank(*c))
}

fn is_approximate(image: &MapImage, confidence: Option<Confidence>) -> bool {
    image.calibration.value().is_none()
        || match confidence {
            Some(Confidence::VerifiedOnBuild) => false,
            Some(Confidence::LimitedSample) => image.control_points.len() < 3,
            _ => true,
        }
}

fn attribution(image: &MapImage, namespace: DatasetNamespace) -> Result<Text, AppError> {
    let origin = match namespace {
        DatasetNamespace::Demo => "Synthetic demo imagery, not game data.",
        DatasetNamespace::Reference => "Reference imagery.",
    };
    Text::new(format!(
        "{origin} Source: {}. Licence: not recorded in this view.",
        image.source.as_str()
    ))
    .map_err(|_| {
        AppError::new(
            ErrorCode::DatasetIncomplete,
            "Attribution is unavailable",
            false,
        )
    })
}

fn atlas_image(
    atlas: &DemoAtlas,
    image: &MapImage,
    style: SchemeStyle,
) -> Result<AtlasImage, AppError> {
    catalog_image(
        calibration_confidence(image, &atlas.dataset),
        &atlas.context,
        image,
        style,
        &|layer| {
            atlas
                .integrity
                .get(&(image.id.clone(), layer.id.clone()))
                .copied()
                .unwrap_or(LayerCheck::NotEmbedded)
        },
    )
}

fn catalog_image(
    confidence: Option<Confidence>,
    context: &DataContext,
    image: &MapImage,
    style: SchemeStyle,
    integrity: &dyn Fn(&ImageLayer) -> LayerCheck,
) -> Result<AtlasImage, AppError> {
    let layers = image
        .layers
        .iter()
        .map(|layer| {
            let url = style.layer_url(&image.id, &layer.id);
            let check = integrity(layer);
            let availability = match (&url, check) {
                (None, _) => LayerAvailability::UnservableId,
                (Some(_), LayerCheck::Verified) => LayerAvailability::Available,
                (Some(_), LayerCheck::Mismatch) => LayerAvailability::IntegrityMismatch,
                (Some(_), LayerCheck::NotEmbedded) => LayerAvailability::NotEmbedded,
            };
            AtlasLayer {
                id: layer.id.clone(),
                kind: layer.kind,
                // A layer that would be refused is never advertised.
                url: url.filter(|_| availability == LayerAvailability::Available),
                availability,
                sha256: layer.sha256.clone(),
                width_px: layer.width_px,
                height_px: layer.height_px,
                frame_divisor: layer.frame_divisor,
            }
        })
        .collect();
    Ok(AtlasImage {
        id: image.id.clone(),
        map: image.map.clone(),
        width_px: image.width_px,
        height_px: image.height_px,
        layers,
        pins: image
            .pins
            .iter()
            .map(|pin| AtlasPin {
                pin: pin.clone(),
                reference_only: true,
            })
            .collect(),
        calibration_status: image.calibration.status(),
        calibration_confidence: confidence,
        approximate: is_approximate(image, confidence),
        gaps: image.gaps.clone(),
        attribution: attribution(image, context.namespace)?.as_str().to_owned(),
    })
}

/// The demo atlas, optionally narrowed to one image. Layer URLs use the
/// platform's scheme style.
pub fn query_atlas(image: Option<&str>) -> Result<AtlasView, AppError> {
    query_atlas_with(image, SchemeStyle::for_platform())
}

/// As [`query_atlas`] with an explicit scheme style, so output is the same on
/// every platform.
pub fn query_atlas_with(image: Option<&str>, style: SchemeStyle) -> Result<AtlasView, AppError> {
    let atlas = demo_atlas()?;
    let mut images = Vec::new();
    for candidate in &atlas.dataset.map_images {
        if image.is_none_or(|wanted| wanted == candidate.id.as_str()) {
            images.push(atlas_image(atlas, candidate, style)?);
        }
    }
    if image.is_some() && images.is_empty() {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "No such atlas image",
            false,
        ));
    }
    Ok(AtlasView {
        data_mode: DataMode::Demo,
        fixture: atlas.fixture,
        revision: atlas.dataset.revision.clone(),
        namespace: atlas.context.namespace,
        images,
        page: None,
    })
}

fn project(atlas: &DemoAtlas, position: &Position) -> ProjectedPosition {
    match project_position(position, &atlas.context, &atlas.dataset) {
        Ok(projection) => ProjectedPosition::Placed(projection),
        Err(gap) => ProjectedPosition::Gap(gap),
    }
}

/// Project up to [`MAX_POSITIONS`] positions onto the demo atlas. Gaps are
/// ordinary results; only a bad request is an error, and its message never
/// echoes the input.
pub fn project_positions(positions: &[Position]) -> Result<Vec<ProjectedPosition>, AppError> {
    if positions.is_empty() || positions.len() > MAX_POSITIONS {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Provide between 1 and 64 positions",
            false,
        ));
    }
    let atlas = demo_atlas()?;
    Ok(positions.iter().map(|p| project(atlas, p)).collect())
}

fn marker_for(
    atlas: &DemoAtlas,
    snapshot: &azeroth_core::contract::Envelope<CharacterSnapshot>,
    now_utc_ms: Option<UtcMillis>,
) -> Result<SnapshotMarker, AppError> {
    catalog_marker(&atlas.dataset, &atlas.context, snapshot, now_utc_ms)
}

pub(crate) fn catalog_marker(
    dataset: &ReferenceDataset,
    data_context: &DataContext,
    snapshot: &azeroth_core::contract::Envelope<CharacterSnapshot>,
    now_utc_ms: Option<UtcMillis>,
) -> Result<SnapshotMarker, AppError> {
    let payload = &snapshot.payload;
    let location: &Observation<Position> = &payload.location;
    let metadata = location.metadata();
    let max_age_ms = Millis::new(SNAPSHOT_MAX_AGE_MS)?;
    let context = FreshnessContext {
        now_utc_ms,
        clock_trust: if now_utc_ms.is_some() {
            ClockTrust::Trusted
        } else {
            ClockTrust::Uncertain
        },
        session: None,
        max_age_ms,
    };
    // Only a known location is projected; unknown, unsupported, restricted and
    // not-applicable locations never become a pixel.
    let projection =
        location.value().map(
            |position| match project_position(position, data_context, dataset) {
                Ok(value) => ProjectedPosition::Placed(value),
                Err(gap) => ProjectedPosition::Gap(gap),
            },
        );
    let approximate = match &projection {
        Some(ProjectedPosition::Placed(placed)) => dataset
            .map_images
            .iter()
            .find(|i| i.id == placed.image)
            .map(|image| is_approximate(image, placed.calibration_confidence)),
        _ => None,
    };
    Ok(SnapshotMarker {
        profile_id: payload.character.as_str().to_owned(),
        snapshot_id: payload.id.clone(),
        location_status: location.status(),
        observed_at_utc_ms: metadata.observed_at_utc_ms,
        received_at_utc_ms: metadata.received_at_utc_ms,
        freshness: freshness(metadata, &context),
        projection,
        approximate,
        fixture: snapshot.fixture,
        data_mode: DataMode::Demo,
    })
}

/// Markers for the imported snapshots (demo only until M2.4). `now_utc_ms` is
/// supplied by the caller; `None` means no trusted clock, so ages are uncertain.
pub fn get_snapshot_markers(
    now_utc_ms: Option<UtcMillis>,
) -> Result<Vec<SnapshotMarker>, AppError> {
    let atlas = demo_atlas()?;
    let snapshot = demo_snapshot()?;
    Ok(vec![marker_for(atlas, &snapshot, now_utc_ms)?])
}

/// As [`get_snapshot_markers`] for an explicit snapshot (used by tests to cover
/// unknown and restricted locations without new committed fixtures).
pub fn snapshot_marker_for(
    snapshot: &azeroth_core::contract::Envelope<CharacterSnapshot>,
    now_utc_ms: Option<UtcMillis>,
) -> Result<SnapshotMarker, AppError> {
    marker_for(demo_atlas()?, snapshot, now_utc_ms)
}

/// Without a selected catalog an observed location remains unprojected.
pub(crate) fn marker_without_catalog(
    snapshot: &azeroth_core::contract::Envelope<CharacterSnapshot>,
    now: Option<UtcMillis>,
) -> Result<SnapshotMarker, AppError> {
    let mut marker = snapshot_marker_for(snapshot, now)?;
    marker.projection = None;
    marker.approximate = None;
    Ok(marker)
}
pub(crate) fn stored_image(
    confidence: Option<Confidence>,
    context: &DataContext,
    image: &MapImage,
) -> Result<AtlasImage, AppError> {
    catalog_image(
        confidence,
        context,
        image,
        SchemeStyle::for_platform(),
        &|_| LayerCheck::NotEmbedded,
    )
}
