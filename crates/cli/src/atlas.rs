//! `prepare-atlas`: deterministic preparation of map imagery from an
//! owner-supplied poster (ADR 0002). Decoding and hashing happen here, never
//! in core. Messages name rules and paths, never file contents.
use azeroth_core::{
    entities::ImageLayerKind,
    types::{AssetPath, ImageId, LayerId, PixelExtent, Sha256Hex, SourceId},
};
use image::{
    ExtendedColorType, ImageEncoder, ImageFormat, ImageReader, Limits, RgbImage,
    codecs::png::{CompressionType, FilterType, PngEncoder},
    imageops,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

pub const GENERATED_FILE: &str = "atlas.generated.json";
pub const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
/// Hard caps a manifest cannot raise.
pub const MAX_SOURCE_BYTES_CAP: u64 = 64 * 1024 * 1024;
pub const MAX_DIMENSION_CAP: u32 = 8192;
const MAX_DECODE_ALLOC: u64 = 256 * 1024 * 1024;
/// Registration search radius per axis, in source pixels.
pub const ALIGNMENT_SEARCH_PX: i32 = 16;
const ALIGNMENT_STRIDE: u32 = 8;
const ALIGNMENT_TRUNCATION: u32 = 64;
const LFS_POINTER_PREFIX: &[u8] = b"version https://git-lfs.github.com/spec/v1";
const LFS_POINTER_MAX_BYTES: u64 = 1024;

#[derive(Debug, PartialEq)]
pub struct PrepareError {
    pub message: String,
}
impl std::fmt::Display for PrepareError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
fn fail<T>(message: impl Into<String>) -> Result<T, PrepareError> {
    Err(PrepareError {
        message: message.into(),
    })
}
type Res<T> = Result<T, PrepareError>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtlasManifest {
    pub atlas_manifest_version: u32,
    pub image_id: ImageId,
    pub source: SourceSpec,
    pub frame: Frame,
    pub layers: Vec<LayerSpec>,
    #[serde(default)]
    pub overview: Option<OverviewSpec>,
    #[serde(default)]
    pub alignment: Option<AlignmentSpec>,
    pub limits: ManifestLimits,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSpec {
    /// Relative to the content root.
    pub file: AssetPath,
    pub sha256: Sha256Hex,
    pub size_bytes: u64,
    pub width_px: PixelExtent,
    pub height_px: PixelExtent,
    pub registry_source_id: SourceId,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub width_px: PixelExtent,
    pub height_px: PixelExtent,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerSpec {
    pub id: LayerId,
    pub kind: ImageLayerKind,
    pub crop: CropRect,
    /// Relative to the content root (or the output override).
    pub output: AssetPath,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewSpec {
    pub id: LayerId,
    pub from_layer: LayerId,
    pub divisor: PixelExtent,
    pub output: AssetPath,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AlignmentSpec {
    pub reference_layer: LayerId,
    /// Mean truncated grey difference (0..64) tolerated at the best offset.
    pub max_residual: f64,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestLimits {
    pub max_source_bytes: u64,
    pub max_dimension_px: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct Offset {
    pub x: i32,
    pub y: i32,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct GeneratedLayer {
    pub id: LayerId,
    pub kind: ImageLayerKind,
    pub file: AssetPath,
    pub width_px: u32,
    pub height_px: u32,
    pub frame_divisor: u32,
    pub size_bytes: u64,
    pub sha256: Sha256Hex,
    pub registration_offset_px: Offset,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct AlignmentResult {
    pub layer: LayerId,
    pub offset_px: Offset,
    pub residual: f64,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct GeneratedSource {
    pub file: AssetPath,
    pub sha256: Sha256Hex,
    pub size_bytes: u64,
    pub width_px: u32,
    pub height_px: u32,
}
/// Written next to the manifest. Deterministic: no timestamps.
#[derive(Debug, Deserialize, Serialize)]
pub struct Generated {
    pub generator: String,
    pub atlas_manifest_version: u32,
    pub image_id: ImageId,
    pub manifest_sha256: Sha256Hex,
    pub source: GeneratedSource,
    pub frame: Frame,
    pub layers: Vec<GeneratedLayer>,
    pub alignment: Option<Vec<AlignmentResult>>,
}

pub struct Paths<'a> {
    pub manifest: &'a Path,
    pub content_root: &'a Path,
    /// Base directory for outputs; defaults to the content root.
    pub output: Option<&'a Path>,
}

fn sha256_hex(bytes: &[u8]) -> Sha256Hex {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    Sha256Hex::new(hex).expect("hex digest")
}
fn read_bounded(path: &Path, limit: u64, what: &str) -> Res<Vec<u8>> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            return fail(format!(
                "{what} {} is not readable: {}",
                path.display(),
                e.kind()
            ));
        }
    };
    let mut bytes = Vec::new();
    match file.take(limit + 1).read_to_end(&mut bytes) {
        Ok(_) => {}
        Err(e) => {
            return fail(format!(
                "{what} {} is not readable: {}",
                path.display(),
                e.kind()
            ));
        }
    }
    if bytes.len() as u64 > limit {
        return fail(format!("{what} {} exceeds {limit} bytes", path.display()));
    }
    Ok(bytes)
}
/// Resolve an asset path under a root and refuse symlinks anywhere below it.
fn resolve(root: &Path, asset: &AssetPath) -> Res<PathBuf> {
    let mut path = root.to_path_buf();
    for segment in asset.segments() {
        path.push(segment);
        if let Ok(meta) = fs::symlink_metadata(&path)
            && meta.file_type().is_symlink()
        {
            return fail(format!("{} is a symbolic link; refused", path.display()));
        }
    }
    Ok(path)
}
fn is_lfs_pointer(bytes: &[u8]) -> bool {
    bytes.starts_with(LFS_POINTER_PREFIX)
}

pub fn read_manifest(path: &Path) -> Res<(AtlasManifest, Sha256Hex)> {
    let bytes = read_bounded(path, MAX_MANIFEST_BYTES, "manifest")?;
    if bytes.contains(&b'\r') {
        return fail("manifest has CRLF line endings; check .gitattributes (content JSON is LF)");
    }
    let manifest: AtlasManifest = match serde_json::from_slice(&bytes) {
        Ok(m) => m,
        Err(_) => {
            return fail(format!(
                "manifest {} is not a valid atlas manifest",
                path.display()
            ));
        }
    };
    if manifest.atlas_manifest_version != 1 {
        return fail("manifest version is not 1");
    }
    if manifest.limits.max_source_bytes > MAX_SOURCE_BYTES_CAP
        || manifest.limits.max_dimension_px > MAX_DIMENSION_CAP
    {
        return fail("manifest limits exceed the hard caps (64 MiB, 8192 px)");
    }
    if manifest.layers.is_empty() {
        return fail("manifest declares no layers");
    }
    let mut ids: Vec<&str> = manifest.layers.iter().map(|l| l.id.as_str()).collect();
    let mut outputs: Vec<&str> = manifest.layers.iter().map(|l| l.output.as_str()).collect();
    if let Some(o) = &manifest.overview {
        ids.push(o.id.as_str());
        outputs.push(o.output.as_str());
        if !manifest.layers.iter().any(|l| l.id == o.from_layer) {
            return fail("overview names an undeclared layer");
        }
    }
    for list in [&mut ids, &mut outputs] {
        list.sort_unstable();
        if list.windows(2).any(|w| w[0] == w[1]) {
            return fail("layer ids and output paths must be unique");
        }
    }
    for output in &outputs {
        let name = output.rsplit('/').next().unwrap_or(output);
        if *output == manifest.source.file.as_str()
            || name == GENERATED_FILE
            || name.ends_with(".manifest.json")
        {
            return fail(
                "layer outputs must not overwrite the source, the manifest or the generated record",
            );
        }
    }
    if let Some(a) = &manifest.alignment
        && !manifest.layers.iter().any(|l| l.id == a.reference_layer)
    {
        return fail("alignment names an undeclared reference layer");
    }
    for layer in &manifest.layers {
        let (w, h) = (
            manifest.frame.width_px.get(),
            manifest.frame.height_px.get(),
        );
        if layer.crop.width != w || layer.crop.height != h {
            return fail(format!(
                "layer {} crop size differs from the frame",
                layer.id.as_str()
            ));
        }
        let (sw, sh) = (
            manifest.source.width_px.get(),
            manifest.source.height_px.get(),
        );
        if layer.crop.x.checked_add(w).is_none_or(|r| r > sw)
            || layer.crop.y.checked_add(h).is_none_or(|b| b > sh)
        {
            return fail(format!(
                "layer {} crop lies outside the source",
                layer.id.as_str()
            ));
        }
    }
    Ok((manifest, sha256_hex(&bytes)))
}

fn decode_source(manifest: &AtlasManifest, path: &Path) -> Res<RgbImage> {
    let bytes = read_bounded(path, manifest.limits.max_source_bytes, "source")?;
    if is_lfs_pointer(&bytes) {
        return fail(format!(
            "source {} is not materialized (Git LFS pointer); run `git lfs install && git lfs pull`",
            path.display()
        ));
    }
    if bytes.len() as u64 != manifest.source.size_bytes {
        return fail("source size differs from the manifest");
    }
    if sha256_hex(&bytes) != manifest.source.sha256 {
        return fail("source digest differs from the manifest");
    }
    let mut limits = Limits::no_limits();
    limits.max_image_width = Some(manifest.limits.max_dimension_px);
    limits.max_image_height = Some(manifest.limits.max_dimension_px);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    reader.limits(limits);
    let decoded = match reader.decode() {
        Ok(d) => d,
        Err(_) => return fail("source is not a decodable PNG within the declared limits"),
    };
    if decoded.width() != manifest.source.width_px.get()
        || decoded.height() != manifest.source.height_px.get()
    {
        return fail("source dimensions differ from the manifest");
    }
    Ok(decoded.into_rgb8())
}

pub fn encode_png(image: &RgbImage) -> Vec<u8> {
    let mut bytes = Vec::new();
    PngEncoder::new_with_quality(&mut bytes, CompressionType::Default, FilterType::Adaptive)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgb8,
        )
        .expect("in-memory PNG encoding");
    bytes
}

fn grey(image: &RgbImage, x: u32, y: u32) -> i32 {
    let p = image.get_pixel(x, y).0;
    (i32::from(p[0]) + i32::from(p[1]) + i32::from(p[2])) / 3
}
/// Best integer offset of `candidate` relative to `reference` within the search
/// radius, by mean truncated grey difference over a stride-sampled grid.
/// Overlays differ between panels; truncation keeps them from dominating.
pub fn measure_alignment(reference: &RgbImage, candidate: &RgbImage) -> (Offset, f64) {
    let (w, h) = (reference.width() as i32, reference.height() as i32);
    let mut best = (Offset { x: 0, y: 0 }, f64::INFINITY);
    for dy in -ALIGNMENT_SEARCH_PX..=ALIGNMENT_SEARCH_PX {
        for dx in -ALIGNMENT_SEARCH_PX..=ALIGNMENT_SEARCH_PX {
            let (mut total, mut count) = (0u64, 0u64);
            let mut y = 0;
            while y < h {
                let mut x = 0;
                while x < w {
                    let (cx, cy) = (x + dx, y + dy);
                    if cx >= 0 && cy >= 0 && cx < w && cy < h {
                        let d = (grey(reference, x as u32, y as u32)
                            - grey(candidate, cx as u32, cy as u32))
                        .unsigned_abs()
                        .min(ALIGNMENT_TRUNCATION);
                        total += u64::from(d);
                        count += 1;
                    }
                    x += ALIGNMENT_STRIDE as i32;
                }
                y += ALIGNMENT_STRIDE as i32;
            }
            if count == 0 {
                continue;
            }
            let mean = total as f64 / count as f64;
            let candidate_offset = Offset { x: dx, y: dy };
            let closer = |o: Offset| (o.x.abs() + o.y.abs(), o.y, o.x);
            if mean < best.1 || (mean == best.1 && closer(candidate_offset) < closer(best.0)) {
                best = (candidate_offset, mean);
            }
        }
    }
    (best.0, best.1)
}

fn write_output(base: &Path, asset: &AssetPath, bytes: &[u8]) -> Res<PathBuf> {
    let path = resolve(base, asset)?;
    if let Some(parent) = path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        return fail(format!("cannot create {}: {}", parent.display(), e.kind()));
    }
    if let Err(e) = fs::write(&path, bytes) {
        return fail(format!("cannot write {}: {}", path.display(), e.kind()));
    }
    Ok(path)
}
fn generated_path(paths: &Paths) -> Res<PathBuf> {
    let manifest_dir = paths.manifest.parent().unwrap_or(Path::new("."));
    match paths.output {
        None => Ok(manifest_dir.join(GENERATED_FILE)),
        Some(output) => {
            // Lexical normalization only; symlinks are refused separately.
            let (dir, root) = match (
                std::path::absolute(manifest_dir),
                std::path::absolute(paths.content_root),
            ) {
                (Ok(d), Ok(r)) => (d, r),
                _ => return fail("manifest or content root path cannot be made absolute"),
            };
            match dir.strip_prefix(&root) {
                Ok(relative) => Ok(output.join(relative).join(GENERATED_FILE)),
                Err(_) => fail(format!(
                    "manifest directory {} is not under content root {}; --output requires that",
                    dir.display(),
                    root.display()
                )),
            }
        }
    }
}

/// Crop the declared layers, write the overview and the generated record.
/// Two runs over the same inputs produce byte-identical outputs.
pub fn prepare(paths: &Paths) -> Res<Generated> {
    let (manifest, manifest_sha256) = read_manifest(paths.manifest)?;
    let source_path = resolve(paths.content_root, &manifest.source.file)?;
    let source = decode_source(&manifest, &source_path)?;
    let base = paths.output.unwrap_or(paths.content_root);
    let mut crops = Vec::new();
    for layer in &manifest.layers {
        let c = layer.crop;
        let crop = imageops::crop_imm(&source, c.x, c.y, c.width, c.height).to_image();
        crops.push((layer, crop));
    }
    let mut alignment = None;
    if let Some(spec) = &manifest.alignment {
        let reference = &crops
            .iter()
            .find(|(l, _)| l.id == spec.reference_layer)
            .expect("validated reference layer")
            .1;
        let mut results = Vec::new();
        for (layer, crop) in &crops {
            let (offset, residual) = if layer.id == spec.reference_layer {
                (Offset { x: 0, y: 0 }, 0.)
            } else {
                measure_alignment(reference, crop)
            };
            if offset != (Offset { x: 0, y: 0 }) {
                return fail(format!(
                    "layer {} is offset by ({}, {}) px from {}; move its crop origin by that amount",
                    layer.id.as_str(),
                    offset.x,
                    offset.y,
                    spec.reference_layer.as_str()
                ));
            }
            if residual > spec.max_residual {
                return fail(format!(
                    "layer {} differs from {} beyond the alignment tolerance (residual {residual:.2})",
                    layer.id.as_str(),
                    spec.reference_layer.as_str()
                ));
            }
            results.push(AlignmentResult {
                layer: layer.id.clone(),
                offset_px: offset,
                residual,
            });
        }
        alignment = Some(results);
    }
    let mut layers = Vec::new();
    for (layer, crop) in &crops {
        let bytes = encode_png(crop);
        write_output(base, &layer.output, &bytes)?;
        layers.push(GeneratedLayer {
            id: layer.id.clone(),
            kind: layer.kind,
            file: layer.output.clone(),
            width_px: crop.width(),
            height_px: crop.height(),
            frame_divisor: 1,
            size_bytes: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
            registration_offset_px: Offset { x: 0, y: 0 },
        });
    }
    if let Some(overview) = &manifest.overview {
        let (_, from) = crops
            .iter()
            .find(|(l, _)| l.id == overview.from_layer)
            .expect("validated overview source");
        let d = overview.divisor.get();
        let (w, h) = (from.width().div_ceil(d), from.height().div_ceil(d));
        let small = imageops::resize(from, w, h, imageops::FilterType::Triangle);
        let bytes = encode_png(&small);
        write_output(base, &overview.output, &bytes)?;
        layers.push(GeneratedLayer {
            id: overview.id.clone(),
            kind: ImageLayerKind::Overview,
            file: overview.output.clone(),
            width_px: w,
            height_px: h,
            frame_divisor: d,
            size_bytes: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
            registration_offset_px: Offset { x: 0, y: 0 },
        });
    }
    let generated = Generated {
        generator: format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        atlas_manifest_version: 1,
        image_id: manifest.image_id.clone(),
        manifest_sha256,
        source: GeneratedSource {
            file: manifest.source.file.clone(),
            sha256: manifest.source.sha256.clone(),
            size_bytes: manifest.source.size_bytes,
            width_px: source.width(),
            height_px: source.height(),
        },
        frame: manifest.frame,
        layers,
        alignment,
    };
    let path = generated_path(paths)?;
    let mut text = serde_json::to_string_pretty(&generated).expect("generated record serializes");
    text.push('\n');
    if let Some(parent) = path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        return fail(format!("cannot create {}: {}", parent.display(), e.kind()));
    }
    if let Err(e) = fs::write(&path, text) {
        return fail(format!("cannot write {}: {}", path.display(), e.kind()));
    }
    Ok(generated)
}

fn parse_pointer(bytes: &[u8]) -> Option<(String, u64)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut oid = None;
    let mut size = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("oid sha256:") {
            oid = Some(rest.trim().to_owned());
        } else if let Some(rest) = line.strip_prefix("size ") {
            size = rest.trim().parse().ok();
        }
    }
    Some((oid?, size?))
}
fn check_one(path: &Path, sha256: &Sha256Hex, size: u64) -> Res<()> {
    let bytes = read_bounded(path, LFS_POINTER_MAX_BYTES, "pointer").or_else(|_| {
        fail(format!(
            "{} is not a Git LFS pointer (raw blob or missing)",
            path.display()
        ))
    })?;
    if !is_lfs_pointer(&bytes) {
        return fail(format!(
            "{} is not a Git LFS pointer (raw blob or missing)",
            path.display()
        ));
    }
    let Some((oid, declared_size)) = parse_pointer(&bytes) else {
        return fail(format!("{} is a malformed Git LFS pointer", path.display()));
    };
    if oid != sha256.as_str() || declared_size != size {
        return fail(format!(
            "{} pointer does not match the declared digest or size",
            path.display()
        ));
    }
    Ok(())
}
/// CI mode: every declared PNG must be a Git LFS pointer whose oid and size
/// match the manifest and the generated record. Materialized bytes fail, which
/// catches a raw blob committed without the LFS clean filter.
pub fn check_pointers(paths: &Paths) -> Res<usize> {
    let (manifest, manifest_sha256) = read_manifest(paths.manifest)?;
    let mut checked = 1;
    check_one(
        &resolve(paths.content_root, &manifest.source.file)?,
        &manifest.source.sha256,
        manifest.source.size_bytes,
    )?;
    let generated_path = generated_path(paths)?;
    let bytes = read_bounded(&generated_path, MAX_MANIFEST_BYTES, "generated record")?;
    let generated: Generated = match serde_json::from_slice(&bytes) {
        Ok(g) => g,
        Err(_) => {
            return fail(format!(
                "{} is not a valid generated record",
                generated_path.display()
            ));
        }
    };
    if generated.manifest_sha256 != manifest_sha256 {
        return fail(
            "the generated record was produced from a different manifest; rerun prepare-atlas",
        );
    }
    for layer in &generated.layers {
        check_one(
            &resolve(paths.content_root, &layer.file)?,
            &layer.sha256,
            layer.size_bytes,
        )?;
        checked += 1;
    }
    Ok(checked)
}

/// Deterministic synthetic poster for demo content and tests: two 64x96
/// panels of the same procedural terrain at x=8 and x=88 (y=12) with distinct
/// overlays, on a dark background. Not game data.
pub fn synthetic_poster() -> RgbImage {
    let mut image = RgbImage::from_pixel(160, 120, image::Rgb([20, 30, 50]));
    for x in 0..160 {
        for y in 0..8 {
            image.put_pixel(x, y, image::Rgb([40, 50, 70]));
        }
    }
    let terrain = |x: u32, y: u32| {
        let v = ((x * 31 + y * 17 + (x * y) % 13) % 97) as u8;
        image::Rgb([60 + v, 90 + v / 2, 50 + v / 3])
    };
    for (origin, overlay) in [(8u32, 0u8), (88u32, 1u8)] {
        for x in 0..64 {
            for y in 0..96 {
                image.put_pixel(origin + x, 12 + y, terrain(x, y));
            }
        }
        if overlay == 0 {
            for x in 10..50 {
                image.put_pixel(origin + x, 12 + 40, image::Rgb([230, 190, 80]));
            }
        } else {
            for y in 10..80 {
                image.put_pixel(origin + 30, 12 + y, image::Rgb([80, 160, 255]));
            }
            image.put_pixel(origin + 6, 12 + 9, image::Rgb([255, 60, 60]));
        }
    }
    image
}
pub fn synthetic_poster_png() -> Vec<u8> {
    encode_png(&synthetic_poster())
}
