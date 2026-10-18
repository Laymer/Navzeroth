//! One immutable logical v2 catalog and its exact bounded material closure.
use super::*;
use azeroth_core::{
    content_v2::{self, ReferenceContent, authored::validate_authored},
    release_party::SupportMatrix,
    sources::AccessMethod,
};
pub const MAX_CATALOG_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_CLOSURE_BYTES: usize = 1024 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    generator_version: String,
    purpose: String,
    sources: FileRecord,
    catalog: FileRecord,
    assets: Vec<FileRecord>,
    party_support: Option<FileRecord>,
    supporting_files: Vec<FileRecord>,
    coverage_gaps: u64,
    distribution_eligible: bool,
}
fn distribution(content: &ReferenceContent, registry: &SourceRegistry) -> Res<bool> {
    let data = &content.reference.payload;
    let ids: BTreeSet<_> = data
        .evidence
        .iter()
        .map(|e| &e.source)
        .chain(data.map_images.iter().map(|im| &im.source))
        .collect();
    let mut permitted = !ids.is_empty();
    for id in ids {
        let source = registry
            .ensure_registered(id)
            .map_err(|_| "Unregistered v2 source")?;
        permitted &= source.redistribution.value() == Some(&Redistribution::Permitted)
            && source.license.value().is_some()
            && source.attribution.value().is_some()
            && source.allowed_fields.value().is_some()
            && source.transformations.value().is_some()
            && source
                .editions
                .value()
                .is_some_and(|e| e.contains(&content.reference.game.edition));
    }
    Ok(permitted)
}
/// A pack that registers an authored source carries original writing only: it
/// must register exactly that one source, and the authored-content rules run on
/// top of the graph validation. A mixed registry would let game facts cite the
/// authored source unchecked, so it is refused. A fixture catalog is synthetic
/// engineering data, flagged as a fixture (which the release gate refuses), and
/// is not authored content.
fn authored(content: &ReferenceContent, registry: &SourceRegistry) -> Res<()> {
    if content.reference.fixture
        || !registry
            .iter()
            .any(|s| s.access.value() == Some(&AccessMethod::Authored))
    {
        return Ok(());
    }
    let mut sources = registry.iter();
    match (sources.next(), sources.next()) {
        (Some(source), None) => validate_authored(content, &source.id),
        _ => Err("Authored pack must declare exactly one source"),
    }
}
fn gaps(content: &ReferenceContent) -> Res<u64> {
    let base = content_report(&content.reference);
    if !base.errors.is_empty() {
        return Err("V2 content graph has errors");
    }
    // Count every declared unavailable fact, including unknown/unsupported/restricted
    // observations, without treating false, zero or empty as missing.
    // Extension provenance marked disputed/stale is rejected by core validation.
    fn unavailable(value: &Value) -> u64 {
        match value {
            Value::Object(map) => {
                let own = u64::from(
                    map.get("status")
                        .and_then(Value::as_str)
                        .is_some_and(|s| matches!(s, "unknown" | "unsupported" | "restricted")),
                );
                own + map.values().map(unavailable).sum::<u64>()
            }
            Value::Array(values) => values.iter().map(unavailable).sum(),
            _ => 0,
        }
    }
    let mut extensions = serde_json::to_value(content).map_err(|_| "Invalid v2 content")?;
    extensions
        .as_object_mut()
        .ok_or("Invalid v2 content")?
        .remove("reference");
    Ok(base.gaps.len() as u64 + unavailable(&extensions))
}
fn support_records(matrix: &SupportMatrix) -> Res<Vec<FileRecord>> {
    let mut files = BTreeMap::new();
    for reference in matrix.evidence_files() {
        let record = FileRecord {
            path: reference.path.clone(),
            bytes: reference.bytes,
            sha256: reference.sha256.clone(),
        };
        if let Some(old) = files.insert(record.path.clone(), record.clone())
            && old != record
        {
            return Err("Conflicting party supporting file pins");
        }
    }
    Ok(files.into_values().collect())
}
fn expected(manifest: &Manifest) -> Res<BTreeSet<AssetPath>> {
    if manifest.assets.len() > MAX_ASSETS || manifest.supporting_files.len() > 4096 {
        return Err("V2 material file count exceeds limit");
    }
    let mut files = BTreeSet::from([AssetPath::new("manifest.json").expect("constant")]);
    for file in std::iter::once(&manifest.sources)
        .chain(std::iter::once(&manifest.catalog))
        .chain(&manifest.assets)
        .chain(&manifest.party_support)
        .chain(&manifest.supporting_files)
    {
        if !files.insert(file.path.clone()) {
            return Err("Duplicate v2 material path");
        }
    }
    expected_tree(&files)?;
    Ok(files)
}
/// Verified import bytes; no caller-selected asset path is read after this point.
pub struct VerifiedAsset {
    pub path: AssetPath,
    pub bytes: Vec<u8>,
}
pub struct VerifiedBundle {
    pub content: ReferenceContent,
    pub catalog_sha256: Sha256Hex,
    pub manifest_sha256: Sha256Hex,
    pub party_support: Option<SupportMatrix>,
    pub sources: Vec<SourceRecord>,
    pub assets: Vec<VerifiedAsset>,
}
pub fn read_verified(root: &Path) -> Res<VerifiedBundle> {
    inspect(root, true)?
        .1
        .ok_or("Verified bundle was not collected")
}
pub fn verify(root: &Path) -> Res<Report> {
    Ok(inspect(root, false)?.0)
}
fn inspect(root: &Path, collect_assets: bool) -> Res<(Report, Option<VerifiedBundle>)> {
    path_checked(root, true)?;
    let mut budget = Budget {
        bytes: 0,
        limit: MAX_CLOSURE_BYTES,
    };
    let manifest_bytes = budget.read(&root.join("manifest.json"), MAX_MANIFEST_BYTES)?;
    let manifest: Manifest = typed(&manifest_bytes)?;
    if manifest.schema_version != 2
        || manifest.purpose != "local_reference"
        || manifest.generator_version.is_empty()
        || manifest.generator_version.len() > 64
        || !manifest
            .generator_version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-+".contains(&b))
        || manifest.sources.path.as_str() != "sources.json"
        || manifest.catalog.path.as_str() != "catalog.json"
        || manifest
            .party_support
            .as_ref()
            .is_some_and(|f| f.path.as_str() != "party-support.json")
    {
        return Err("Invalid v2 pack header");
    }
    if canonical(&manifest)? != manifest_bytes {
        return Err("V2 manifest is not canonical");
    }
    let files = expected(&manifest)?;
    exact_tree(root, &files)?;
    let sources_bytes = check_record(root, &manifest.sources, MAX_SOURCES_BYTES, &mut budget)?;
    let registry = read_sources(&sources_bytes)?;
    if source_bytes(&registry)? != sources_bytes {
        return Err("Sources are not canonical");
    }
    let catalog_bytes = check_record(root, &manifest.catalog, MAX_CATALOG_BYTES, &mut budget)?;
    let content = content_v2::decode(&catalog_bytes).map_err(|_| "Invalid v2 content graph")?;
    if canonical(&content)? != catalog_bytes {
        return Err("V2 catalog is not canonical");
    }
    authored(&content, &registry)?;
    let layers = layers_data(&content.reference.payload)?;
    if layers.keys().collect::<BTreeSet<_>>() != manifest.assets.iter().map(|f| &f.path).collect() {
        return Err("V2 imagery closure differs");
    }
    let mut verified_assets = Vec::new();
    for asset in &manifest.assets {
        let bytes = check_record(root, asset, MAX_ASSET_BYTES, &mut budget)?;
        png(&bytes, &layers[&asset.path])?;
        if collect_assets {
            verified_assets.push(VerifiedAsset {
                path: asset.path.clone(),
                bytes,
            });
        }
    }
    let mut verified_party_support = None;
    if let Some(support) = &manifest.party_support {
        let bytes = check_record(
            root,
            support,
            azeroth_core::release::MAX_RELEASE_BYTES,
            &mut budget,
        )?;
        let matrix = SupportMatrix::decode(&bytes).map_err(|_| "Invalid party support matrix")?;
        // A pending (schema 2, `null`) target binds no build here; the release
        // gate refuses it until the target is frozen and then requires equality.
        if let Some(target) = &matrix.target
            && (content.reference.game.edition.as_str() != target.edition
                || content.reference.game.build.as_str()
                    != format!("{}.{}", target.version, target.build)
                || content.reference.game.interface.as_str() != target.interface.to_string())
        {
            return Err("Party support target differs from content catalog");
        }
        if canonical(&matrix)? != bytes
            || matrix.catalog_sha256 != manifest.catalog.sha256
            || support_records(&matrix)? != manifest.supporting_files
        {
            return Err("Party support differs from catalog or declared evidence closure");
        }
        matrix
            .bind(
                manifest.catalog.sha256.as_str(),
                &azeroth_core::content_v2::records::content_records(&content)?,
            )
            .map_err(|_| "Party support dependencies do not bind concrete content")?;
        for file in &manifest.supporting_files {
            check_record(root, file, MAX_CATALOG_BYTES, &mut budget)?;
        }
        verified_party_support = Some(matrix);
    } else if !manifest.supporting_files.is_empty() {
        return Err("Unowned supporting evidence");
    }
    let permitted = distribution(&content, &registry)?;
    let coverage_gaps = gaps(&content)?;
    if permitted != manifest.distribution_eligible || coverage_gaps != manifest.coverage_gaps {
        return Err("V2 declared distribution or coverage differs");
    }
    let report = Report {
        report_version: 2,
        datasets: 1,
        fixtures: usize::from(content.reference.fixture),
        files: files.len(),
        total_bytes: budget.bytes,
        coverage_gaps,
        distribution_eligible: permitted,
    };
    let bundle = collect_assets.then_some(VerifiedBundle {
        content,
        catalog_sha256: manifest.catalog.sha256,
        manifest_sha256: hash(&manifest_bytes),
        party_support: verified_party_support,
        sources: registry.into(),
        assets: verified_assets,
    });
    Ok((report, bundle))
}
pub fn build(input: &Path, output: &Path) -> Res<Report> {
    path_checked(input, true)?;
    let stage = stage_with_limit(output, MAX_CLOSURE_BYTES)?;
    let mut budget = Budget {
        bytes: 0,
        limit: MAX_CLOSURE_BYTES,
    };
    let content = content_v2::decode(&budget.read(&input.join("catalog.json"), MAX_CATALOG_BYTES)?)
        .map_err(|_| "Invalid authored v2 content")?;
    // Refuse invented game facts before staging any catalog bytes.
    let registry = read_sources(&budget.read(&input.join("sources.json"), MAX_SOURCES_BYTES)?)?;
    authored(&content, &registry)?;
    let bytes = canonical(&content)?;
    if bytes.len() > MAX_CATALOG_BYTES {
        return Err("Canonical catalog exceeds limit");
    }
    let catalog = record("catalog.json", &bytes)?;
    stage.write(&stage.root.join("catalog.json"), &bytes)?;
    let bytes = source_bytes(&registry)?;
    if bytes.len() > MAX_SOURCES_BYTES {
        return Err("Canonical source registry exceeds limit");
    }
    let sources = record("sources.json", &bytes)?;
    stage.write(&stage.root.join("sources.json"), &bytes)?;
    let mut assets = vec![];
    for (path, spec) in layers_data(&content.reference.payload)? {
        let bytes = budget.read(&root_join(input, &path), MAX_ASSET_BYTES)?;
        png(&bytes, &spec)?;
        let target = root_join(&stage.root, &path);
        stage.directories(target.parent().ok_or("Invalid asset path")?)?;
        stage.write(&target, &bytes)?;
        assets.push(record(path.as_str(), &bytes)?);
    }
    let mut party_support = None;
    let mut supporting_files = vec![];
    let support_path = input.join("party-support.json");
    let has_support = match fs::symlink_metadata(&support_path) {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => return Err("Cannot inspect authored party support"),
    };
    if has_support {
        let matrix = SupportMatrix::decode(
            &budget.read(&support_path, azeroth_core::release::MAX_RELEASE_BYTES)?,
        )
        .map_err(|_| "Invalid authored party support")?;
        if matrix.catalog_sha256 != catalog.sha256 {
            return Err("Party support uses different catalog hash");
        }
        let bytes = canonical(&matrix)?;
        party_support = Some(record("party-support.json", &bytes)?);
        stage.write(&stage.root.join("party-support.json"), &bytes)?;
        supporting_files = support_records(&matrix)?;
        for file in &supporting_files {
            let bytes = check_record(input, file, MAX_CATALOG_BYTES, &mut budget)?;
            let target = root_join(&stage.root, &file.path);
            stage.directories(target.parent().ok_or("Invalid supporting file")?)?;
            stage.write(&target, &bytes)?;
        }
    }
    let manifest = Manifest {
        schema_version: 2,
        generator_version: env!("CARGO_PKG_VERSION").into(),
        purpose: "local_reference".into(),
        sources,
        catalog,
        assets,
        party_support,
        supporting_files,
        coverage_gaps: gaps(&content)?,
        distribution_eligible: distribution(&content, &registry)?,
    };
    let bytes = canonical(&manifest)?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err("V2 manifest exceeds byte limit");
    }
    stage.write(&stage.root.join("manifest.json"), &bytes)?;
    let report = verify(&stage.root)?;
    publish_with_limit(
        &stage.root,
        output,
        &expected(&manifest)?,
        MAX_CLOSURE_BYTES,
        MAX_CATALOG_BYTES,
    )?;
    Ok(report)
}
