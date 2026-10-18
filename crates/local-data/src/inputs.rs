//! Explicit private research inputs. No input automatically overlays or activates.
use crate::{
    Result, bounded_json, checked_path, extraction::Store, hash, read_file, verify_catalog,
};
use azeroth_core::{
    contract::AnyExchange,
    local_data::{PerformancePreset, is_sha256},
    local_data_inputs::{
        INPUT_VERSION, InputKind, InputTuple, MAX_CACHE_BYTES, ResearchInput, parse_cache,
    },
    observation::{ClockTrust, FreshnessContext},
    types::Millis,
};
use serde::Serialize;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    path::Path,
};
pub const INPUT_PROFILE_SHA256: &str = env!("NAVZEROTH_INPUT_PROFILE_SHA256");
const MAX_ANALYSIS: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
pub struct StagedInput {
    pub input_sha256: String,
    pub manifest: ResearchInput,
}

fn read_selected(path: &Path, maximum: usize) -> Result<(Vec<u8>, File)> {
    let selected = checked_path(path)?;
    let meta = fs::metadata(&selected).map_err(|_| "Cannot inspect selected input")?;
    if !meta.is_file() || meta.len() > maximum as u64 {
        return Err("Input is not a bounded regular file");
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Existing writers prevent opening; new writes/deletion cannot race the read.
        options.share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ);
    }
    let mut file = options
        .open(&selected)
        .map_err(|_| "Selected input unavailable or still open for writing")?;
    let before = crate::stamp(&file.metadata().map_err(|_| "Cannot inspect input handle")?)?;
    if before.bytes > maximum as u64 {
        return Err("Input exceeds capacity");
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read selected input")?;
    file.seek(SeekFrom::Start(0))
        .map_err(|_| "Cannot recheck input")?;
    use sha2::{Digest, Sha256};
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut count = 0usize;
    loop {
        let n = file.read(&mut buffer).map_err(|_| "Cannot recheck input")?;
        if n == 0 {
            break;
        }
        count = count.checked_add(n).ok_or("Input size overflow")?;
        if count > maximum {
            return Err("Input changed or exceeds capacity");
        }
        digest.update(&buffer[..n]);
    }
    let after = crate::stamp(&file.metadata().map_err(|_| "Cannot recheck input")?)?;
    let current_file = options
        .open(checked_path(path)?)
        .map_err(|_| "Selected input disappeared or changed")?;
    crate::input_guard::same_file(&file, &current_file)?;
    let current = crate::stamp(
        &current_file
            .metadata()
            .map_err(|_| "Cannot recheck selected path")?,
    )?;
    if bytes.len() > maximum
        || count != bytes.len()
        || before != after
        || after != current
        || hash(&bytes) != format!("{:x}", digest.finalize())
    {
        return Err("Selected input changed during reading");
    }
    Ok((bytes, file))
}

fn tuple(destination: &Path, catalog: &str, locale: &str) -> Result<InputTuple> {
    let verified = verify_catalog(destination, catalog)?;
    if !verified
        .installation
        .declared_locales
        .iter()
        .any(|v| v == locale)
    {
        return Err("Locale not declared by selected catalog");
    }
    let tuple = InputTuple {
        product: verified.installation.product,
        version: verified.installation.version,
        build: verified.installation.build,
        locale: locale.into(),
    };
    tuple.validate()?;
    Ok(tuple)
}

fn cache_analysis(bytes: &[u8], kind: InputKind, tuple: &InputTuple) -> Result<Vec<u8>> {
    bounded_json(&parse_cache(bytes, kind, tuple.build)?, MAX_ANALYSIS)
}

/// Synthetic path deliberately always marks the resulting input as a fixture.
/// This is not an alternate route to real-client acceptance or actionable data.
pub fn stage_cache_fixture(
    destination: &Path,
    catalog: &str,
    path: &Path,
    kind: InputKind,
    locale: &str,
    preset: PerformancePreset,
) -> Result<StagedInput> {
    stage_cache_impl(destination, catalog, path, kind, locale, preset, true)
}
pub fn stage_cache(
    destination: &Path,
    catalog: &str,
    path: &Path,
    kind: InputKind,
    locale: &str,
    preset: PerformancePreset,
) -> Result<StagedInput> {
    crate::input_guard::require_closed()?;
    stage_cache_impl(destination, catalog, path, kind, locale, preset, false)
}
fn stage_cache_impl(
    destination: &Path,
    catalog: &str,
    path: &Path,
    kind: InputKind,
    locale: &str,
    preset: PerformancePreset,
    fixture: bool,
) -> Result<StagedInput> {
    let expected = kind
        .cache_filename()
        .ok_or("Not an allowlisted cache input")?;
    if path.file_name().and_then(|v| v.to_str()) != Some(expected)
        || path
            .parent()
            .and_then(Path::file_name)
            .and_then(|v| v.to_str())
            != Some(locale)
    {
        return Err("Cache selection must match allowlisted filename and exact locale directory");
    }
    let root = checked_path(destination)?;
    let source = checked_path(path)?;
    if source.starts_with(&root) || root.starts_with(&source) {
        return Err("Source and destination must be separate");
    }
    let mut store = Store::open(&root, preset.limits().total_storage_bytes)?;
    let tuple = tuple(&root, catalog, locale)?;
    let (bytes, _held_read_handle) = read_selected(&source, MAX_CACHE_BYTES)?;
    let analysis = cache_analysis(&bytes, kind, &tuple)?;
    if !fixture {
        crate::input_guard::require_closed()?;
    }
    seal(
        &mut store,
        catalog,
        tuple,
        kind,
        &bytes,
        &analysis,
        fixture,
        "selected_cache_directory_only",
        vec![
            "hotfix_field_layouts_unverified".into(),
            "cache_locale_not_embedded".into(),
            "playable_availability_unknown".into(),
        ],
    )
}

fn matches_build(value: &str, tuple: &InputTuple) -> bool {
    value == tuple.version || value == tuple.build.to_string()
}
fn observation_analysis(
    bytes: &[u8],
    kind: InputKind,
    tuple: &InputTuple,
) -> Result<(Vec<u8>, bool, &'static str)> {
    let text = std::str::from_utf8(bytes).map_err(|_| "Observation is not UTF-8")?;
    match kind {
        InputKind::CharacterSnapshot => {
            let options = azeroth_ingest::import::ImportOptions::snapshots(FreshnessContext {
                now_utc_ms: None,
                clock_trust: ClockTrust::Uncertain,
                session: None,
                max_age_ms: Millis::new(0).map_err(|_| "Invalid freshness policy")?,
            });
            let parsed = azeroth_ingest::import::import_text(text, &options)
                .map_err(|_| "Invalid bounded addon snapshot")?;
            let AnyExchange::CharacterSnapshot(ref snapshot) = parsed.exchange else {
                return Err("Only a character snapshot is supported for this input");
            };
            if !matches_build(snapshot.game.build.as_str(), tuple)
                || snapshot.game.locale.as_str() != tuple.locale
            {
                return Err("Observation build or locale differs from selected catalog");
            }
            let fixture = parsed.is_fixture_or_demo();
            if !fixture
                && parsed.producer_component != azeroth_core::contract::ProducerComponent::Addon
            {
                return Err("Observation input requires an addon export or labelled fixture");
            }
            // Preserve all typed observation states, metadata, false, zero and empty.
            Ok((
                bounded_json(&parsed.exchange, MAX_ANALYSIS)?,
                fixture,
                "recorded_addon_context",
            ))
        }
        InputKind::SessionEvents => {
            let parsed = azeroth_ingest::session::decode_text(text)
                .map_err(|_| "Invalid bounded addon session")?;
            if !matches_build(parsed.batch.context.build.build.as_str(), tuple) {
                return Err("Session build differs from selected catalog");
            }
            if parsed
                .locale
                .as_ref()
                .is_some_and(|locale| locale.as_str() != tuple.locale)
            {
                return Err("Session locale differs from selected catalog");
            }
            let evidence = if parsed.locale.is_some() {
                "recorded_addon_context"
            } else {
                "unrecorded_addon_locale"
            };
            // Historical v1 sessions do not record locale. Bind as research with an
            // explicit gap; never pretend the requested locale was observed.
            Ok((
                bounded_json(&parsed, MAX_ANALYSIS)?,
                parsed.fixture,
                evidence,
            ))
        }
        _ => Err("Not a supported observation transport"),
    }
}
pub fn stage_observation(
    destination: &Path,
    catalog: &str,
    path: &Path,
    kind: InputKind,
    locale: &str,
    preset: PerformancePreset,
) -> Result<StagedInput> {
    let root = checked_path(destination)?;
    let source = checked_path(path)?;
    if source.starts_with(&root) {
        return Err("Source and destination must be separate");
    }
    let maximum = match kind {
        InputKind::CharacterSnapshot => azeroth_ingest::anav1::MAX_TEXT_BYTES,
        InputKind::SessionEvents => azeroth_ingest::session::MAX_TEXT_BYTES,
        _ => return Err("Not a supported observation transport"),
    };
    let mut store = Store::open(&root, preset.limits().total_storage_bytes)?;
    let tuple = tuple(&root, catalog, locale)?;
    let (bytes, _held_read_handle) = read_selected(&source, maximum)?;
    let (analysis, fixture, evidence) = observation_analysis(&bytes, kind, &tuple)?;
    let mut gaps = vec![
        "contextual_observation_only".into(),
        "universal_eligibility_unknown".into(),
        "playable_availability_unknown".into(),
        "edition_product_binding_unverified".into(),
    ];
    if evidence == "unrecorded_addon_locale" {
        gaps.push("observation_locale_unknown".into());
    }
    seal(
        &mut store, catalog, tuple, kind, &bytes, &analysis, fixture, evidence, gaps,
    )
}
#[allow(clippy::too_many_arguments)]
fn seal(
    store: &mut Store,
    catalog: &str,
    tuple: InputTuple,
    kind: InputKind,
    source: &[u8],
    analysis: &[u8],
    fixture: bool,
    locale_evidence: &str,
    gaps: Vec<String>,
) -> Result<StagedInput> {
    let (unsupported, record_count, conflict_count) = if kind.cache_filename().is_some() {
        let envelope = parse_cache(source, kind, tuple.build)?;
        let counts = if envelope.unsupported.is_none() {
            (
                Some(envelope.records.len() as u32),
                Some(envelope.conflicting_records.len() as u32),
            )
        } else {
            (None, None)
        };
        (envelope.unsupported, counts.0, counts.1)
    } else {
        (None, None, None)
    };
    let manifest = ResearchInput {
        classification: azeroth_core::local_data_catalog::DistributionClass::LocalOnly,
        format_version: INPUT_VERSION,
        catalog_sha256: catalog.into(),
        catalog_tuple: tuple,
        kind,
        source_sha256: store.put("objects", source)?,
        analysis_sha256: store.put("objects", analysis)?,
        analyzer_profile_sha256: INPUT_PROFILE_SHA256.into(),
        fixture,
        unsupported,
        record_count,
        conflict_count,
        locale_evidence: locale_evidence.into(),
        gaps,
    };
    manifest.validate()?;
    let input_sha256 = store.put("inputs", &bounded_json(&manifest, 64 * 1024)?)?;
    Ok(StagedInput {
        input_sha256,
        manifest,
    })
}
pub fn verify(destination: &Path, digest: &str) -> Result<ResearchInput> {
    if !is_sha256(digest) {
        return Err("Invalid input digest");
    }
    let root = checked_path(destination)?;
    let bytes = read_file(&root.join("inputs").join(digest), 64 * 1024)?.0;
    if hash(&bytes) != digest {
        return Err("Input manifest hash mismatch");
    }
    let manifest: ResearchInput =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid research input manifest")?;
    manifest.validate()?;
    if manifest.analyzer_profile_sha256 != INPUT_PROFILE_SHA256 {
        return Err("Unsupported input analyzer profile");
    }
    if tuple(
        &root,
        &manifest.catalog_sha256,
        &manifest.catalog_tuple.locale,
    )? != manifest.catalog_tuple
    {
        return Err("Input tuple differs from bound catalog");
    }
    let maximum = if manifest.kind.cache_filename().is_some() {
        MAX_CACHE_BYTES
    } else {
        azeroth_ingest::anav1::MAX_TEXT_BYTES
    };
    let source = read_file(&root.join("objects").join(&manifest.source_sha256), maximum)?.0;
    let analysis = read_file(
        &root.join("objects").join(&manifest.analysis_sha256),
        MAX_ANALYSIS,
    )?
    .0;
    if hash(&source) != manifest.source_sha256 || hash(&analysis) != manifest.analysis_sha256 {
        return Err("Input object hash mismatch");
    }
    let expected = if manifest.kind.cache_filename().is_some() {
        let envelope = parse_cache(&source, manifest.kind, manifest.catalog_tuple.build)?;
        if envelope.unsupported != manifest.unsupported
            || (envelope
                .unsupported
                .is_none()
                .then_some(envelope.records.len() as u32))
                != manifest.record_count
            || (envelope
                .unsupported
                .is_none()
                .then_some(envelope.conflicting_records.len() as u32))
                != manifest.conflict_count
        {
            return Err("Cache disposition differs from retained source");
        }
        cache_analysis(&source, manifest.kind, &manifest.catalog_tuple)?
    } else {
        let (bytes, fixture, evidence) =
            observation_analysis(&source, manifest.kind, &manifest.catalog_tuple)?;
        if fixture != manifest.fixture || evidence != manifest.locale_evidence {
            return Err("Observation lineage mismatch");
        }
        bytes
    };
    if analysis != expected {
        return Err("Input analysis does not match retained source");
    }
    Ok(manifest)
}

#[derive(Debug, Serialize)]
pub struct CachePage {
    pub revision: String,
    pub offset: u32,
    pub total: u32,
    pub rows: Vec<azeroth_core::local_data_inputs::CacheRecord>,
    pub next_offset: Option<u32>,
}
/// Revision-bound metadata page. Raw cache payloads are never returned here.
pub fn cache_page(destination: &Path, digest: &str, offset: u32, limit: u32) -> Result<CachePage> {
    if limit == 0 || limit > 200 {
        return Err("Input page limit must be between one and two hundred");
    }
    let manifest = verify(destination, digest)?;
    if manifest.kind.cache_filename().is_none() {
        return Err("Input is not a cache envelope");
    }
    let root = checked_path(destination)?;
    let source = read_file(
        &root.join("objects").join(&manifest.source_sha256),
        MAX_CACHE_BYTES,
    )?
    .0;
    if hash(&source) != manifest.source_sha256 {
        return Err("Input source hash mismatch");
    }
    let envelope = parse_cache(&source, manifest.kind, manifest.catalog_tuple.build)?;
    if envelope.unsupported.is_some() {
        return Err("Unsupported cache has no decoded record page");
    }
    let total = envelope.records.len() as u32;
    if offset > total {
        return Err("Input page offset exceeds record count");
    }
    let end = offset.saturating_add(limit).min(total);
    Ok(CachePage {
        revision: digest.into(),
        offset,
        total,
        rows: envelope.records[offset as usize..end as usize].to_vec(),
        next_offset: (end < total).then_some(end),
    })
}
