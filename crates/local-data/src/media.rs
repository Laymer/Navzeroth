//! Quota-bounded private media staging. Native reads run only in the isolated worker.
use crate::tables::{Cell, DecodedTable};
use crate::{
    Result, bounded_json, checked_path,
    extraction::{SourceState, Store},
    hash, read_file, verify_catalog,
};
use azeroth_core::{
    local_data::{PerformancePreset, is_sha256},
    local_data_media::{Asset, Candidate, Discovery, Disposition, MediaManifest, ReadRefusal},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
const MAX_OBJECT: usize = 32 * 1024 * 1024;
const MAX_MANIFEST: usize = 2 * 1024 * 1024;
const MAX_BATCH: usize = 128 * 1024 * 1024;
pub const WORKER_ARGUMENT: &str = "--navzeroth-media-worker-v1";
const SOURCES: &[(&str, &str, &[&str])] = &[
    ("Item", "9a2a4834", &["IconFileDataID"]),
    (
        "SpellMisc",
        "434b3607",
        &["SpellIconFileDataID", "ActiveIconFileDataID"],
    ),
    ("UiMapArtTile", "2da5b77b", &["FileDataID"]),
    ("UiMapArt", "3ae7d144", &["HighlightFileDataID"]),
];
fn profile() -> &'static str {
    crate::inputs::INPUT_PROFILE_SHA256
}
fn object(root: &Path, digest: &str, maximum: usize) -> Result<Vec<u8>> {
    if !is_sha256(digest) {
        return Err("Invalid media identity");
    }
    let b = read_file(&root.join("objects").join(digest), maximum)?.0;
    if hash(&b) != digest {
        return Err("Media object hash mismatch");
    }
    Ok(b)
}
pub fn discover(root: &Path, catalog_digest: &str, locale: &str) -> Result<Discovery> {
    let catalog = verify_catalog(root, catalog_digest)?;
    if catalog.installation.product != "wow_classic_beta"
        || !matches!(catalog.installation.build, 70374 | 70425 | 70450)
        || catalog.installation.version != format!("1.60.1.{}", catalog.installation.build)
        || !catalog
            .installation
            .declared_locales
            .iter()
            .any(|l| l == locale)
    {
        return Err("Unsupported media tuple");
    }
    let mut candidates = BTreeSet::new();
    let mut gaps = BTreeSet::new();
    for (table, layout, fields) in SOURCES {
        let Some(entry) = catalog
            .tables
            .iter()
            .find(|e| e.table == *table && e.locale.as_deref() == Some(locale))
        else {
            gaps.insert(format!("{table}:missing"));
            continue;
        };
        if entry.layout_hash.as_deref() != Some(*layout) {
            gaps.insert(format!("{table}:unsupported_layout"));
            continue;
        }
        let Some(digest) = &entry.decoded_sha256 else {
            gaps.insert(format!("{table}:undecoded:{}", entry.reason));
            continue;
        };
        let decoded: DecodedTable =
            serde_json::from_slice(&object(root, digest, crate::extraction::MAX_DECODED_BYTES)?)
                .map_err(|_| "Invalid media discovery table")?;
        if format!("{:08x}", decoded.header.layout_hash) != *layout {
            return Err("Media discovery table binding mismatch");
        }
        for gap in &decoded.gaps {
            gaps.insert(format!("{table}:{gap}"));
        }
        let mut seen = BTreeSet::new();
        for row in decoded.rows {
            if !seen.insert(row.id) {
                gaps.insert(format!("{table}:duplicate_record_identity"));
            }
            for field in *fields {
                use azeroth_core::local_data_projection::Fact;
                let id = match row.fields.get(*field) {
                    Some(Cell::Unsigned(v)) => {
                        u32::try_from(*v).map_or(Fact::Unsupported, Fact::Known)
                    }
                    Some(Cell::Signed(v)) => {
                        u32::try_from(*v).map_or(Fact::Unsupported, Fact::Known)
                    }
                    Some(Cell::Unknown) => Fact::Unknown,
                    None => Fact::Missing,
                    _ => Fact::Unsupported,
                };
                match id {
                    Fact::Known(0) => {
                        gaps.insert(format!("{table}:{field}:zero_sentinel"));
                    }
                    Fact::Known(id) => {
                        candidates.insert(Candidate {
                            table: (*table).into(),
                            row: row.id,
                            field: (*field).into(),
                            file_data_id: id,
                        });
                    }
                    state => {
                        let label = match state {
                            Fact::Unknown => "unknown",
                            Fact::Missing => "missing",
                            _ => "unsupported",
                        };
                        gaps.insert(format!("{table}:{field}:{label}"));
                    }
                }
                if candidates.len() > 16_384 {
                    return Err("Media discovery candidate quota exceeded");
                }
            }
        }
    }
    gaps.insert("coordinates_uncalibrated_and_usage_rights_unverified".into());
    Ok(Discovery {
        format_version: 1,
        catalog_sha256: catalog_digest.into(),
        locale: locale.into(),
        candidates: candidates.into_iter().collect(),
        gaps: gaps.into_iter().collect(),
    })
}
fn png(texture: &azeroth_ingest::texture::Texture) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, texture.width, texture.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|_| "PNG header failed")?;
        writer
            .write_image_data(&texture.rgba)
            .map_err(|_| "PNG encoding failed")?;
    }
    if bytes.len() > MAX_OBJECT {
        return Err("PNG exceeds capacity");
    }
    Ok(bytes)
}
fn decode_asset(store: &mut Store, id: u32, bytes: &[u8]) -> Result<Asset> {
    let source = store.put("objects", bytes)?;
    let mut asset = Asset {
        file_data_id: id,
        disposition: Disposition::Unsupported,
        source_sha256: Some(source),
        read_refusal: None,
        png_sha256: None,
        width: None,
        height: None,
    };
    match azeroth_ingest::texture::decode(bytes) {
        Ok(texture) => {
            asset.disposition = Disposition::Decoded;
            asset.png_sha256 = Some(store.put("objects", &png(&texture)?)?);
            asset.width = Some(texture.width);
            asset.height = Some(texture.height);
        }
        Err(azeroth_ingest::texture::Refusal::Unsupported) => {}
        Err(azeroth_ingest::texture::Refusal::Invalid) => asset.disposition = Disposition::Invalid,
        Err(azeroth_ingest::texture::Refusal::Capacity) => {
            asset.disposition = Disposition::Capacity
        }
    }
    Ok(asset)
}
fn seal(
    store: &mut Store,
    discovery: Discovery,
    assets: Vec<Asset>,
    fixture: bool,
) -> Result<String> {
    let discovery_digest = store.put("objects", &bounded_json(&discovery, MAX_MANIFEST)?)?;
    let manifest = MediaManifest {
        format_version: 1,
        catalog_sha256: discovery.catalog_sha256,
        locale: discovery.locale,
        discovery_sha256: discovery_digest,
        profile_sha256: profile().into(),
        fixture,
        local_only: true,
        calibrated: false,
        verified_for_gameplay: false,
        assets,
    };
    manifest.validate()?;
    store.put("media", &bounded_json(&manifest, MAX_MANIFEST)?)
}
/// Explicit synthetic-only entry point. It cannot create a production media manifest.
pub fn stage_fixture(
    root: &Path,
    catalog: &str,
    locale: &str,
    files: &BTreeMap<u32, Vec<u8>>,
    preset: PerformancePreset,
) -> Result<String> {
    let mut store = Store::open(root, preset.limits().total_storage_bytes)?;
    let discovery = discover(root, catalog, locale)?;
    validate_selection(
        &discovery,
        files.keys().copied().collect::<Vec<_>>().as_slice(),
    )?;
    if files.values().any(|b| b.len() > MAX_OBJECT)
        || files.values().map(Vec::len).sum::<usize>() > MAX_BATCH
    {
        return Err("Media batch capacity exceeded");
    }
    let assets = files
        .iter()
        .map(|(id, b)| decode_asset(&mut store, *id, b))
        .collect::<Result<Vec<_>>>()?;
    seal(&mut store, discovery, assets, true)
}
fn validate_selection(discovery: &Discovery, ids: &[u32]) -> Result<()> {
    if ids.is_empty()
        || ids.len() > 128
        || ids.windows(2).any(|p| p[0] >= p[1])
        || ids
            .iter()
            .any(|id| !discovery.candidates.iter().any(|c| c.file_data_id == *id))
    {
        return Err("Media selection must be sorted unique discovered IDs, at most 128");
    }
    Ok(())
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub format_version: u32,
    pub installation: PathBuf,
    pub destination: PathBuf,
    pub catalog: String,
    pub locale: String,
    pub ids: Vec<u32>,
    pub preset: PerformancePreset,
}
fn extract(request: &Request) -> Result<String> {
    extract_using(request, crate::native::Archive::open)
}

fn extract_using(
    request: &Request,
    open: impl FnOnce(&Path, &str, u32, u32) -> Result<crate::native::Archive>,
) -> Result<String> {
    if request.format_version != 1 {
        return Err("Unsupported media request");
    }
    let root = checked_path(&request.installation)?;
    let destination = checked_path(&request.destination)?;
    if root.starts_with(&destination) || destination.starts_with(&root) {
        return Err("Media source and store overlap");
    }
    let mut store = Store::open(&destination, request.preset.limits().total_storage_bytes)?;
    let discovery = discover(&destination, &request.catalog, &request.locale)?;
    validate_selection(&discovery, &request.ids)?;
    let catalog = verify_catalog(&destination, &request.catalog)?;
    let inspection = crate::inspect(&root, &catalog.installation.product)?;
    if inspection.metadata != catalog.installation {
        return Err("Media installation differs from catalog");
    }
    let before = SourceState::capture_checked(&root, &inspection.metadata, || Ok(()))?;
    if before.hashes != catalog.input_hashes {
        return Err("Media source fingerprint differs from catalog");
    }
    let locale_mask = crate::media::locale_mask(&request.locale)?;
    let archive = open(
        &root,
        &catalog.installation.product,
        locale_mask,
        catalog.installation.build,
    )?;
    let mut assets = Vec::new();
    let mut used = 0usize;
    for id in &request.ids {
        let mut asset = Asset {
            file_data_id: *id,
            disposition: Disposition::Unavailable,
            source_sha256: None,
            read_refusal: Some(ReadRefusal::Unavailable),
            png_sha256: None,
            width: None,
            height: None,
        };
        match archive.read_file_id(*id, MAX_OBJECT) {
            Ok(crate::native::Logical::Whole(bytes)) => {
                used = used
                    .checked_add(bytes.len())
                    .filter(|v| *v <= MAX_BATCH)
                    .ok_or("Media batch capacity exceeded")?;
                asset = decode_asset(&mut store, *id, &bytes)?;
            }
            Ok(crate::native::Logical::Partial(_)) => {
                asset.read_refusal = Some(ReadRefusal::EncryptedFrames);
                asset.disposition = Disposition::Encrypted;
            }
            Err(error) => {
                use crate::native::ReadFailure::*;
                let refusal = match error {
                    Missing => ReadRefusal::Missing,
                    Encrypted => ReadRefusal::Encrypted,
                    EncryptedFrames => ReadRefusal::EncryptedFrames,
                    Unsupported => ReadRefusal::Unsupported,
                    Invalid => ReadRefusal::Invalid,
                    Truncated => ReadRefusal::Truncated,
                    Capacity => ReadRefusal::Capacity,
                    Unavailable => ReadRefusal::Unavailable,
                    ShortRead => ReadRefusal::ShortRead,
                };
                asset.disposition = refusal.disposition();
                asset.read_refusal = Some(refusal);
            }
        }
        assets.push(asset);
    }
    drop(archive);
    if SourceState::capture_checked(&root, &inspection.metadata, || Ok(()))?.hashes != before.hashes
    {
        return Err("Media source changed during extraction");
    }
    seal(&mut store, discovery, assets, false)
}

#[cfg(test)]
pub(crate) fn extract_synthetic(request: &Request) -> Result<String> {
    extract_using(request, crate::native::boundary_tests::open_staging)
}
fn locale_mask(locale: &str) -> Result<u32> {
    Ok(match locale {
        "enUS" => 2,
        "koKR" => 4,
        "frFR" => 16,
        "deDE" => 32,
        "zhCN" => 64,
        "esES" => 128,
        "zhTW" => 256,
        "enGB" => 512,
        "esMX" => 4096,
        "ruRU" => 8192,
        "ptBR" => 16384,
        "itIT" => 32768,
        "ptPT" => 65536,
        _ => return Err("Unsupported media locale"),
    })
}

#[cfg(test)]
mod locale_tests {
    #[test]
    fn installed_content_locale_masks_are_exact_and_unknown_locales_are_refused() {
        for (locale, mask) in [
            ("enUS", 2),
            ("koKR", 4),
            ("frFR", 16),
            ("deDE", 32),
            ("zhCN", 64),
            ("esES", 128),
            ("zhTW", 256),
            ("enGB", 512),
            ("esMX", 4096),
            ("ruRU", 8192),
            ("ptBR", 16384),
            ("itIT", 32768),
            ("ptPT", 65536),
        ] {
            assert_eq!(super::locale_mask(locale), Ok(mask));
        }
        for locale in ["", "enus", "enUS ", "xxXX"] {
            assert_eq!(super::locale_mask(locale), Err("Unsupported media locale"));
        }
    }
}
pub fn verify(root: &Path, digest: &str) -> Result<MediaManifest> {
    if !is_sha256(digest) {
        return Err("Invalid media revision");
    }
    let bytes = read_file(&root.join("media").join(digest), MAX_MANIFEST)?.0;
    if hash(&bytes) != digest {
        return Err("Media revision hash mismatch");
    }
    let manifest: MediaManifest =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid media manifest")?;
    manifest.validate()?;
    if bounded_json(&manifest, MAX_MANIFEST)? != bytes || manifest.profile_sha256 != profile() {
        return Err("Media profile or encoding mismatch");
    }
    let discovery = discover(root, &manifest.catalog_sha256, &manifest.locale)?;
    if hash(&bounded_json(&discovery, MAX_MANIFEST)?) != manifest.discovery_sha256
        || object(root, &manifest.discovery_sha256, MAX_MANIFEST)?
            != bounded_json(&discovery, MAX_MANIFEST)?
    {
        return Err("Media discovery binding mismatch");
    }
    validate_selection(
        &discovery,
        &manifest
            .assets
            .iter()
            .map(|a| a.file_data_id)
            .collect::<Vec<_>>(),
    )?;
    let mut used = 0usize;
    for asset in &manifest.assets {
        if let Some(source) = &asset.source_sha256 {
            let bytes = object(root, source, MAX_OBJECT)?;
            used = used
                .checked_add(bytes.len())
                .filter(|v| *v <= MAX_BATCH)
                .ok_or("Media batch capacity exceeded")?;
            match azeroth_ingest::texture::decode(&bytes) {
                Ok(texture) => {
                    let data = png(&texture)?;
                    if asset.disposition != Disposition::Decoded
                        || asset.width != Some(texture.width)
                        || asset.height != Some(texture.height)
                        || asset.png_sha256.as_deref() != Some(hash(&data).as_str())
                        || object(root, asset.png_sha256.as_ref().unwrap(), MAX_OBJECT)? != data
                    {
                        return Err("Media decoded binding mismatch");
                    }
                }
                Err(error) => {
                    let expected = match error {
                        azeroth_ingest::texture::Refusal::Unsupported => Disposition::Unsupported,
                        azeroth_ingest::texture::Refusal::Invalid => Disposition::Invalid,
                        azeroth_ingest::texture::Refusal::Capacity => Disposition::Capacity,
                    };
                    if asset.disposition != expected {
                        return Err("Media disposition mismatch");
                    }
                }
            }
        } else if asset.disposition == Disposition::Decoded {
            return Err("Media source missing");
        }
    }
    Ok(manifest)
}
fn asset_id(revision: &str, png: &str) -> String {
    hash(format!("NAVZEROTH-PRIVATE-ASSET-V1\0{revision}\0{png}").as_bytes())
}
#[derive(Debug, Serialize)]
pub struct AssetView {
    pub id: String,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Serialize)]
pub struct Page {
    pub revision: String,
    pub total: u32,
    pub offset: u32,
    pub assets: Vec<AssetView>,
    pub next_offset: Option<u32>,
}
pub fn page(root: &Path, revision: &str, offset: u32, limit: u32) -> Result<Page> {
    if limit == 0 || limit > 200 {
        return Err("Media page limit must be 1 through 200");
    }
    let manifest = verify(root, revision)?;
    let ready = manifest
        .assets
        .iter()
        .filter(|a| a.disposition == Disposition::Decoded)
        .collect::<Vec<_>>();
    if offset as usize > ready.len() {
        return Err("Media offset out of range");
    }
    let end = (offset as usize + limit as usize).min(ready.len());
    Ok(Page {
        revision: revision.into(),
        total: ready.len() as u32,
        offset,
        assets: ready[offset as usize..end]
            .iter()
            .map(|a| AssetView {
                id: asset_id(revision, a.png_sha256.as_ref().unwrap()),
                width: a.width.unwrap(),
                height: a.height.unwrap(),
            })
            .collect(),
        next_offset: (end < ready.len()).then_some(end as u32),
    })
}
/// Only a verified revision and opaque ID can select served bytes. Return PNG only.
pub fn asset(root: &Path, revision: &str, id: &str) -> Result<Vec<u8>> {
    if !is_sha256(id) {
        return Err("Invalid opaque asset identity");
    }
    let manifest = verify(root, revision)?;
    let digest = manifest
        .assets
        .iter()
        .filter_map(|a| a.png_sha256.as_ref())
        .find(|h| asset_id(revision, h) == id)
        .ok_or("Asset is not in this media revision")?;
    object(root, digest, MAX_OBJECT)
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Response {
    format_version: u32,
    revision: Option<String>,
    error: Option<String>,
}
pub fn worker_entry() -> i32 {
    let request: Request = match crate::worker::read_frame(&mut std::io::stdin().lock()) {
        Ok(r) => r,
        Err(_) => return 1,
    };
    let outcome = crate::isolation::worker_limit(request.preset.limits().worker_memory_bytes)
        .and_then(|()| extract(&request));
    let response = match outcome {
        Ok(revision) => Response {
            format_version: 1,
            revision: Some(revision),
            error: None,
        },
        Err(error) => Response {
            format_version: 1,
            revision: None,
            error: Some(error.into()),
        },
    };
    let exit = i32::from(response.error.is_some());
    if crate::worker::write_frame(&mut std::io::stdout().lock(), &response).is_err() {
        return 1;
    }
    exit
}
/// Parent applies the native process boundary before sending source paths over the pipe.
pub fn extract_worker(executable: &Path, request: &Request) -> Result<String> {
    use std::process::{Command, Stdio};
    let mut child = Command::new(executable)
        .arg(WORKER_ARGUMENT)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Cannot start media worker")?;
    let outcome = (|| {
        let _limit = crate::isolation::ProcessLimit::attach(
            &child,
            request.preset.limits().worker_memory_bytes,
        )?;
        let mut input = child.stdin.take().ok_or("Media worker pipe unavailable")?;
        let mut output = child.stdout.take().ok_or("Media worker pipe unavailable")?;
        crate::worker::write_frame(&mut input, request)?;
        drop(input);
        let response: Response = crate::worker::read_frame(&mut output)?;
        if response.format_version != 1 || response.error.is_some() {
            return Err("Media worker refused extraction");
        }
        let revision = response.revision.ok_or("Media worker revision missing")?;
        if !child
            .wait()
            .map_err(|_| "Media worker exit unavailable")?
            .success()
        {
            return Err("Media worker failed");
        }
        let manifest = verify(&request.destination, &revision)?;
        if manifest.fixture
            || manifest.catalog_sha256 != request.catalog
            || manifest.locale != request.locale
            || manifest
                .assets
                .iter()
                .map(|a| a.file_data_id)
                .collect::<Vec<_>>()
                != request.ids
        {
            return Err("Media worker response binding mismatch");
        }
        Ok(revision)
    })();
    if outcome.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    outcome
}
/// Research-only plain-text evaluation over a reproduced projection's spell descriptions.
pub fn tooltip(
    root: &Path,
    projection: &str,
    spell: u32,
) -> Result<azeroth_core::tooltip::Evaluation> {
    if !crate::projection::verify_projection(root, projection)?.verified {
        return Err("Tooltip projection profile is not reproduced");
    }
    let bytes = read_file(
        &root.join("projections").join(projection),
        crate::projection::MAX_PROJECTION_BYTES,
    )?
    .0;
    if hash(&bytes) != projection {
        return Err("Tooltip projection changed after verification");
    }
    let projection: azeroth_core::local_data_projection::LocalProjection =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid tooltip projection")?;
    projection.validate()?;
    let expanded = projection
        .expanded
        .ok_or("Tooltip expanded projection unavailable")?;
    let mut index = BTreeMap::new();
    if let Some(table) = expanded.tables.iter().find(|t| t.table == "Spell") {
        for row in table.records.iter().flatten() {
            if let Some(azeroth_core::local_data_projection::Fact::Known(
                azeroth_core::local_data_gameplay::Value::Text(text),
            )) = row.fields.get("Description_lang")
            {
                index.insert(row.id, text);
            }
        }
    }
    let mut documents = BTreeMap::new();
    let mut pending = BTreeSet::from([spell]);
    let mut seen = BTreeSet::new();
    let mut used = 0usize;
    while let Some(id) = pending.pop_first() {
        if !seen.insert(id) {
            continue;
        }
        if seen.len() > azeroth_core::tooltip::MAX_DOCUMENTS {
            return Err("Tooltip document quota exceeded");
        }
        let Some(text) = index.get(&id) else {
            continue;
        };
        used += text.len();
        if used > 4 * 1024 * 1024 {
            return Err("Tooltip closure text quota exceeded");
        }
        pending.extend(azeroth_core::tooltip::references(text)?);
        documents.insert(
            id,
            azeroth_core::tooltip::Document {
                text: (*text).clone(),
                scalars: BTreeMap::new(),
            },
        );
    }
    azeroth_core::tooltip::evaluate(&documents, spell)
}
#[derive(Debug, Serialize)]
pub struct DiscoveryPage {
    pub revision: String,
    pub catalog_sha256: String,
    pub locale: String,
    pub total: u32,
    pub offset: u32,
    pub candidates: Vec<Candidate>,
    pub gaps: Vec<String>,
    pub next_offset: Option<u32>,
}
pub fn discovery_page(
    root: &Path,
    catalog: &str,
    locale: &str,
    offset: u32,
    limit: u32,
) -> Result<DiscoveryPage> {
    if limit == 0 || limit > 200 {
        return Err("Discovery page limit must be 1 through 200");
    }
    let discovery = discover(root, catalog, locale)?;
    if offset as usize > discovery.candidates.len() {
        return Err("Discovery offset out of range");
    }
    let revision = hash(&bounded_json(&discovery, MAX_MANIFEST)?);
    let end = (offset as usize + limit as usize).min(discovery.candidates.len());
    Ok(DiscoveryPage {
        revision,
        catalog_sha256: catalog.into(),
        locale: locale.into(),
        total: discovery.candidates.len() as u32,
        offset,
        candidates: discovery.candidates[offset as usize..end].to_vec(),
        gaps: discovery.gaps,
        next_offset: (end < discovery.candidates.len()).then_some(end as u32),
    })
}
