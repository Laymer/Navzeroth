//! Release-only contracts. These are not addon exchange schemas or gameplay facts.
//! A passing report checks declarations; the I/O boundary must also verify file
//! hashes, pack contents and the tested Git revision before publication.
use crate::types::{AssetPath, Sha256Hex};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_RELEASE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_UNITS: usize = 4096;
/// Required and shipped record identities. ADR 0007 §7: a whole-world pack
/// (expected at 350,000 to 800,000 identities) plus the authored pack fits
/// with headroom.
pub const MAX_RECORDS: usize = 2_000_000;
/// Schema 3 (ADR 0007, task V2.8e): the target comes from
/// `release/v1/world-data-target.json`, the `LIVE-*` checks and the live slice
/// matrix are no longer required, and evidence binds the accepted-origin policy.
pub const SCHEMA_WORLD_DATA: u32 = 3;
pub const DOMAINS: [&str; 9] = [
    "quests",
    "equipment",
    "acquisition",
    "geography",
    "travel",
    "dungeons",
    "class_roles",
    "party",
    "reminders",
];
pub const LIVE_CHECKS: [&str; 15] = [
    "LIVE-BUILD",
    "LIVE-CAPTURE",
    "LIVE-TRANSFER",
    "LIVE-RELOAD",
    "LIVE-RESTRICTIONS",
    "LIVE-HUD",
    "LIVE-PLAN",
    "LIVE-ADVANCE",
    "LIVE-DEVIATION",
    "LIVE-DUO",
    "LIVE-SESSION",
    "LIVE-NEXT-PLAN",
    "LIVE-REMINDERS",
    "LIVE-GEAR",
    "LIVE-DUNGEON",
];
pub const WINDOWS_CHECKS: [&str; 10] = [
    "WIN-INSTALL",
    "WIN-UPGRADE",
    "WIN-UNINSTALL",
    "WIN-RECOVERY",
    "WIN-OFFLINE",
    "WIN-FOCUS",
    "WIN-REFERENCE",
    "WIN-DISPLAY",
    "WIN-ACCESSIBILITY",
    "WIN-PERFORMANCE",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub edition: String,
    pub version: String,
    pub build: String,
    pub interface: u32,
}
impl Target {
    pub fn forever_70374() -> Self {
        Self {
            edition: "wow_forever".into(),
            version: "1.60.1".into(),
            build: "70374".into(),
            interface: 16001,
        }
    }
}
/// ADR 0006: the 1.0.0 target is the first stable live build after the
/// 4 Nov 2026 launch. Until task V4.2 freezes it here, schema 2 manifests must
/// declare `"target": null` and every release gate blocks. A manifest cannot
/// choose its own target.
pub fn frozen_release_target() -> Option<Target> {
    None
}
/// The target a manifest of this schema version must declare. Schema 3 takes
/// its target from the world-data target file; see [`ReleasePolicy`].
pub fn expected_target(schema_version: u32) -> Option<Target> {
    match schema_version {
        1 => Some(Target::forever_70374()),
        _ => frozen_release_target(),
    }
}
/// ADR 0007 §6: the owner's detected client build, recorded in
/// `release/v1/world-data-target.json`. Unknown fields are refused.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldDataTarget {
    format: String,
    schema_version: u32,
    decision: String,
    edition: String,
    build: String,
    interface: String,
    locale: String,
    detected_from: String,
}
pub const WORLD_DATA_TARGET_FORMAT: &str = "navzeroth-world-data-target";
/// Decode the world-data target file into a release target. The build is
/// `major.minor.patch.build`, all decimal; the interface is decimal.
pub fn decode_world_data_target(bytes: &[u8]) -> Result<Target, &'static str> {
    let value = crate::strict_json::value(bytes, 64 * 1024)
        .map_err(|_| "Invalid world-data target file")?;
    let file: WorldDataTarget =
        serde_json::from_value(value).map_err(|_| "Invalid world-data target file")?;
    let digits = |p: &str| !p.is_empty() && p.len() <= 9 && p.bytes().all(|b| b.is_ascii_digit());
    let parts: Vec<_> = file.build.split('.').collect();
    if file.format != WORLD_DATA_TARGET_FORMAT
        || file.schema_version != 1
        || [
            &file.decision,
            &file.edition,
            &file.locale,
            &file.detected_from,
        ]
        .into_iter()
        .any(|t| !bounded_text(t))
        || crate::types::EditionId::new(file.edition.clone()).is_err()
        || parts.len() != 4
        || !parts.iter().all(|p| digits(p))
        || !digits(&file.interface)
    {
        return Err("Invalid world-data target file");
    }
    Ok(Target {
        edition: file.edition,
        version: parts[..3].join("."),
        build: parts[3].to_owned(),
        interface: file
            .interface
            .parse()
            .map_err(|_| "Invalid world-data target file")?,
    })
}
/// Release inputs outside the manifests (ADR 0007). Schema 1 and 2 ignore them.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReleasePolicy<'a> {
    /// The decoded `world-data-target.json`; schema 3 requires it.
    pub target: Option<&'a Target>,
    /// The decoded `accepted-origins.json`; schema 3 requires it.
    pub origins: Option<&'a crate::origin_policy::OriginPolicy>,
    /// SHA-256 of the exact policy bytes, which evidence must bind.
    pub origin_policy_sha256: Option<&'a str>,
}
/// The target a manifest must declare under the supplied release inputs.
pub fn expected_target_with(schema_version: u32, policy: &ReleasePolicy<'_>) -> Option<Target> {
    if schema_version == SCHEMA_WORLD_DATA {
        policy.target.cloned()
    } else {
        expected_target(schema_version)
    }
}
/// Mandatory acceptance rows and their methods for a manifest schema. Schema 3
/// drops the `LIVE-*` rows (ADR 0007 §5); the `WIN-*` rows are unchanged.
pub fn required_checks(schema_version: u32) -> Vec<(&'static str, Method)> {
    let windows = WINDOWS_CHECKS
        .into_iter()
        .map(|id| (id, Method::InteractiveWindows));
    if schema_version == SCHEMA_WORLD_DATA {
        windows.collect()
    } else {
        LIVE_CHECKS
            .into_iter()
            .map(|id| (id, Method::LiveGame))
            .chain(windows)
            .collect()
    }
}
/// Sources whose target validation is satisfied by the accepted-origin policy
/// (ADR 0007 §3): every domain whose coverage cites the source accepts its
/// origin edition. A source no coverage row cites is never accepted this way.
pub fn origin_accepted_sources(
    inventory: &Inventory,
    evidence: &Evidence,
    policy: &ReleasePolicy<'_>,
) -> BTreeSet<String> {
    let Some(origins) = policy.origins else {
        return BTreeSet::new();
    };
    if inventory.schema_version != SCHEMA_WORLD_DATA
        || policy
            .target
            .is_none_or(|t| t.edition != origins.target_edition.as_str())
    {
        return BTreeSet::new();
    }
    let unit_domain: BTreeMap<_, _> = inventory
        .domains
        .iter()
        .flat_map(|d| {
            d.units
                .iter()
                .map(move |u| (u.id.as_str(), d.domain.as_str()))
        })
        .collect();
    let mut citing: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for row in &evidence.coverage {
        if let Some(domain) = unit_domain.get(row.unit_id.as_str()) {
            for source in &row.source_ids {
                citing.entry(source.as_str()).or_default().insert(domain);
            }
        }
    }
    evidence
        .sources
        .iter()
        .filter(|s| {
            citing.get(s.id.as_str()).is_some_and(|domains| {
                domains
                    .iter()
                    .all(|d| origins.accepts_origin(d, &s.origin_edition))
            })
        })
        .map(|s| s.id.clone())
        .collect()
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRef {
    pub path: AssetPath,
    pub bytes: u64,
    pub sha256: Sha256Hex,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    Unknown,
    Complete,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Unverified,
    Fail,
    Pass,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    LiveGame,
    InteractiveWindows,
    Automated,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Unknown,
    Conditional,
    Prohibited,
    Permitted,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub id: String,
    pub scope: String,
    /// None is an unknown inventory; [] is an explicitly enumerated empty unit.
    pub required_record_ids: Option<Vec<String>>,
}
/// Schema 2: content deliberately outside the declared slice. This is neither
/// `unknown` (unassessed) nor `unsupported`; release notes list every boundary.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Boundary {
    pub id: String,
    pub scope: String,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainInventory {
    pub domain: String,
    pub completeness: Completeness,
    pub enumeration_evidence: Vec<FileRef>,
    pub units: Vec<Unit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub out_of_scope: Vec<Boundary>,
}
/// Schema 2: the declared release slice (ADR 0006 section 2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slice {
    pub id: String,
    pub races: Vec<String>,
    pub zones: Vec<String>,
    pub dungeons: Vec<String>,
    pub level_min: u8,
    pub level_max: u8,
    /// None until the launch class list is known.
    pub classes: Option<Vec<String>>,
    pub party_sizes: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRequirement {
    pub id: String,
    pub revision: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    pub schema_version: u32,
    pub inventory_id: String,
    /// Always present; null (schema 2 only) means the target is pending.
    pub target: Option<Target>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slice: Option<Slice>,
    pub domains: Vec<DomainInventory>,
    pub sources: Vec<SourceRequirement>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acceptance {
    pub id: String,
    pub status: Status,
    pub method: Method,
    pub operator: Option<String>,
    pub recorded_at: Option<String>,
    pub environment: Option<String>,
    pub artifacts: Vec<FileRef>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitCoverage {
    pub unit_id: String,
    pub status: Status,
    pub included_record_ids: Vec<String>,
    pub source_ids: Vec<String>,
    pub unresolved_conflicts: u32,
    pub unsupported_corrections: u32,
    pub validation_evidence: Vec<FileRef>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEvidence {
    pub id: String,
    pub revision: String,
    pub origin_edition: String,
    pub origin_build: Option<String>,
    pub redistribution: Permission,
    pub files: Vec<FileRef>,
    pub license_files: Vec<FileRef>,
    pub attribution_files: Vec<FileRef>,
    pub permission_evidence: Vec<FileRef>,
    pub target_validation: Status,
    pub target_evidence: Vec<FileRef>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackRef {
    pub manifest: FileRef,
    pub dataset_revisions: Vec<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionalFeature {
    OfficialApi,
    PartyMessaging,
    CombatLogAnalysis,
    HostedDistribution,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deferral {
    pub feature: OptionalFeature,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub schema_version: u32,
    /// Always present; must equal the inventory target, including null.
    pub target: Option<Target>,
    pub inventory_sha256: Sha256Hex,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admission_registry: Option<FileRef>,
    /// The actual tested ancestor, not the later commit that records evidence.
    pub tested_source_commit: Option<String>,
    pub production_sha256: Option<Sha256Hex>,
    pub checks: Vec<Acceptance>,
    pub coverage: Vec<UnitCoverage>,
    pub sources: Vec<SourceEvidence>,
    pub packs: Vec<PackRef>,
    pub optional_deferrals: Vec<Deferral>,
    /// Schema 2 (task V1.4): live rows that cover the declared slice.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub matrix: Vec<MatrixRow>,
    /// Schema 3 (ADR 0007): the exact `accepted-origins.json` the gate applies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_policy: Option<FileRef>,
}
/// One live acceptance run inside the slice: a class in a level band, in a
/// zone, at a party size. Coverage is derived in code from the slice.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixRow {
    pub id: String,
    pub class: String,
    pub level_min: u8,
    pub level_max: u8,
    pub zone: String,
    pub party_size: u8,
    pub status: Status,
    pub method: Method,
    pub operator: Option<String>,
    pub recorded_at: Option<String>,
    pub environment: Option<String>,
    pub artifacts: Vec<FileRef>,
}
/// Ten-level bands of the slice's level range (1-10, 11-20, ...).
pub fn level_bands(slice: &Slice) -> Vec<(u8, u8)> {
    let (min, max) = (u16::from(slice.level_min), u16::from(slice.level_max));
    let mut bands = vec![];
    let mut start = min.max(1);
    while start <= max {
        let end = ((start - 1) / 10 * 10 + 10).min(max);
        // Both values are at most `max`, which came from a u8.
        bands.push((start as u8, end as u8));
        start = end + 1;
    }
    bands
}
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PendingRequirement {
    InitialTestedBinding,
    Observation { check_id: String },
}
#[derive(Debug, Serialize)]
pub struct Readiness {
    pub report_version: u32,
    pub version: String,
    pub stable: bool,
    pub stable_ready: bool,
    pub candidate_ready: bool,
    pub pending: Vec<PendingRequirement>,
    pub publication_allowed: bool,
    pub blockers: Vec<String>,
}
impl Readiness {
    fn defer(&mut self, requirement: PendingRequirement, message: impl Into<String>) {
        self.pending.push(requirement);
        self.blockers.push(message.into());
        self.stable_ready = false;
        self.publication_allowed = !self.stable;
    }

    pub fn block(&mut self, message: impl Into<String>) {
        self.blockers.push(message.into());
        self.stable_ready = false;
        self.candidate_ready = false;
        self.publication_allowed = !self.stable;
    }
}
pub struct Bindings<'a> {
    pub inventory_sha256: &'a str,
    pub production_sha256: &'a str,
    /// I/O caller verifies that this commit exists, is an ancestor, and has the
    /// same production fingerprint as the candidate (including version files).
    pub tested_source_commit: &'a str,
}
fn bounded_text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 4096 && !value.chars().any(char::is_control)
}
fn sha1(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn unique<'a>(values: impl IntoIterator<Item = &'a str>) -> bool {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .all(|v| bounded_text(v) && seen.insert(v))
}
fn files_present(files: &[FileRef]) -> bool {
    !files.is_empty()
        && files.len() <= MAX_UNITS
        && files.iter().all(|f| f.bytes > 0)
        && unique(files.iter().map(|f| f.path.as_str()))
}

/// Strict SemVer subset used by the packaging declarations. Malformed versions
/// fail instead of accidentally becoming a prerelease and bypassing stable gates.
pub fn is_stable(version: &str) -> Result<bool, &'static str> {
    let (base, pre) = version
        .split_once('-')
        .map_or((version, None), |(a, b)| (a, Some(b)));
    let parts: Vec<_> = base.split('.').collect();
    if parts.len() != 3
        || parts.iter().any(|p| {
            p.is_empty()
                || p.len() > 9
                || !p.bytes().all(|b| b.is_ascii_digit())
                || (p.len() > 1 && p.starts_with('0'))
        })
    {
        return Err("Invalid release version");
    }
    if let Some(pre) = pre
        && (pre.len() > 128
            || pre.split('.').any(|p| {
                p.is_empty()
                    || !p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    || (p.len() > 1 && p.bytes().all(|b| b.is_ascii_digit()) && p.starts_with('0'))
            }))
    {
        return Err("Invalid prerelease version");
    }
    Ok(parts[0] != "0" && pre.is_none())
}

pub fn decode_inventory(bytes: &[u8]) -> Result<Inventory, &'static str> {
    let inventory: Inventory = decode(bytes)?;
    let shape_matches = match inventory.schema_version {
        1 => {
            inventory.slice.is_none() && inventory.domains.iter().all(|d| d.out_of_scope.is_empty())
        }
        2 | SCHEMA_WORLD_DATA => inventory.slice.is_some(),
        _ => false,
    };
    if !shape_matches {
        return Err("Unsupported inventory version");
    }
    Ok(inventory)
}
pub fn decode_evidence(bytes: &[u8]) -> Result<Evidence, &'static str> {
    let evidence: Evidence = decode(bytes)?;
    let world = evidence.schema_version == SCHEMA_WORLD_DATA;
    if !matches!(evidence.schema_version, 1 | 2 | SCHEMA_WORLD_DATA)
        || (matches!(evidence.schema_version, 1 | SCHEMA_WORLD_DATA) && !evidence.matrix.is_empty())
        || world != evidence.origin_policy.is_some()
    {
        return Err("Unsupported evidence version");
    }
    Ok(evidence)
}
fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, &'static str> {
    if bytes.len() > MAX_RELEASE_BYTES {
        return Err("Release manifest exceeds size limit");
    }
    // Typed structs reject duplicate and unknown fields. Arrays are checked below.
    let value = serde_json::from_slice(bytes).map_err(|_| "Invalid release manifest")?;
    // An optional target would otherwise default to null when the key is omitted.
    #[derive(Deserialize)]
    struct TargetPresent {
        #[allow(dead_code)]
        target: serde::de::IgnoredAny,
    }
    serde_json::from_slice::<TargetPresent>(bytes).map_err(|_| "Invalid release manifest")?;
    Ok(value)
}

/// Explicit v1 to v2 migration (task V1.1): the target becomes pending, the
/// slice is declared and each domain receives its out-of-scope boundaries.
/// Units, evidence references and sources are kept unchanged.
pub fn upgrade_inventory(
    mut inventory: Inventory,
    slice: Slice,
    mut boundaries: BTreeMap<String, Vec<Boundary>>,
) -> Result<Inventory, &'static str> {
    if inventory.schema_version != 1 || inventory.slice.is_some() {
        return Err("Only a schema 1 inventory can be upgraded");
    }
    if boundaries.keys().any(|k| !DOMAINS.contains(&k.as_str())) {
        return Err("Boundaries name an unsupported domain");
    }
    inventory.schema_version = 2;
    inventory.target = frozen_release_target();
    inventory.slice = Some(slice);
    for domain in &mut inventory.domains {
        domain.out_of_scope = boundaries.remove(&domain.domain).unwrap_or_default();
    }
    Ok(inventory)
}
pub fn upgrade_evidence(
    mut evidence: Evidence,
    inventory_sha256: Sha256Hex,
) -> Result<Evidence, &'static str> {
    if evidence.schema_version != 1 {
        return Err("Only schema 1 evidence can be upgraded");
    }
    evidence.schema_version = 2;
    evidence.target = frozen_release_target();
    evidence.inventory_sha256 = inventory_sha256;
    Ok(evidence)
}
/// Explicit v2 to v3 migration (ADR 0007, task V2.8e): the target becomes the
/// world-data target. Slice, boundaries, units and sources are kept.
pub fn upgrade_inventory_v3(
    mut inventory: Inventory,
    target: Target,
) -> Result<Inventory, &'static str> {
    if inventory.schema_version != 2 {
        return Err("Only a schema 2 inventory can be upgraded to schema 3");
    }
    inventory.schema_version = SCHEMA_WORLD_DATA;
    inventory.target = Some(target);
    Ok(inventory)
}
/// Evidence v2 to v3: the `LIVE-*` rows and live slice matrix are removed
/// (ADR 0007 §5), the `WIN-*` rows are kept unchanged and the accepted-origin
/// policy file is bound.
pub fn upgrade_evidence_v3(
    mut evidence: Evidence,
    inventory_sha256: Sha256Hex,
    target: Target,
    origin_policy: FileRef,
) -> Result<Evidence, &'static str> {
    if evidence.schema_version != 2 {
        return Err("Only schema 2 evidence can be upgraded to schema 3");
    }
    evidence.schema_version = SCHEMA_WORLD_DATA;
    evidence.target = Some(target);
    evidence.inventory_sha256 = inventory_sha256;
    evidence
        .checks
        .retain(|c| !LIVE_CHECKS.contains(&c.id.as_str()));
    evidence.matrix.clear();
    evidence.origin_policy = Some(origin_policy);
    Ok(evidence)
}

fn slice_valid(slice: &Slice) -> bool {
    let names = |values: &[String]| {
        !values.is_empty() && values.len() <= MAX_UNITS && unique(values.iter().map(String::as_str))
    };
    let mut sizes = BTreeSet::new();
    bounded_text(&slice.id)
        && names(&slice.races)
        && names(&slice.zones)
        && (slice.dungeons.is_empty() || names(&slice.dungeons))
        && slice.classes.as_deref().is_none_or(names)
        && (1..=slice.level_max).contains(&slice.level_min)
        && slice.level_max <= 60
        && !slice.party_sizes.is_empty()
        && slice
            .party_sizes
            .iter()
            .all(|n| (1..=5).contains(n) && sizes.insert(*n))
}

fn check_limits(inventory: &Inventory, evidence: Option<&Evidence>) -> Result<(), &'static str> {
    let units = inventory.domains.iter().flat_map(|d| &d.units);
    if inventory.domains.len() > DOMAINS.len()
        || inventory.sources.len() > MAX_UNITS
        || units.clone().count() > MAX_UNITS
        || inventory
            .domains
            .iter()
            .map(|d| d.out_of_scope.len())
            .sum::<usize>()
            > MAX_UNITS
        || units
            .map(|u| u.required_record_ids.as_ref().map_or(0, Vec::len))
            .sum::<usize>()
            > MAX_RECORDS
    {
        return Err("Release inventory exceeds collection limits");
    }
    if let Some(e) = evidence
        && (e.checks.len() > LIVE_CHECKS.len() + WINDOWS_CHECKS.len()
            || e.coverage.len() > MAX_UNITS
            || e.sources.len() > MAX_UNITS
            || e.packs.len() > MAX_UNITS
            || e.optional_deferrals.len() > 4
            || e.matrix.len() > MAX_UNITS
            || e.coverage
                .iter()
                .map(|c| c.included_record_ids.len())
                .sum::<usize>()
                > MAX_RECORDS
            || e.coverage.iter().any(|c| c.source_ids.len() > MAX_UNITS)
            || evidence_files(inventory, e).len() > MAX_RECORDS)
    {
        return Err("Release evidence exceeds collection limits");
    }
    Ok(())
}

/// Evaluate the complete, fixed release requirements. A caller cannot make a
/// mandatory row optional, omit a domain, or substitute an automated live pass.
pub fn assess(
    version: &str,
    inventory: &Inventory,
    evidence: Option<&Evidence>,
    bindings: &Bindings<'_>,
) -> Result<Readiness, &'static str> {
    assess_with(
        version,
        inventory,
        evidence,
        bindings,
        &ReleasePolicy::default(),
    )
}
/// [`assess`] with the release inputs outside the manifests (ADR 0007).
pub fn assess_with(
    version: &str,
    inventory: &Inventory,
    evidence: Option<&Evidence>,
    bindings: &Bindings<'_>,
    policy: &ReleasePolicy<'_>,
) -> Result<Readiness, &'static str> {
    let stable = is_stable(version)?;
    check_limits(inventory, evidence)?;
    if !matches!(inventory.schema_version, 1 | 2 | SCHEMA_WORLD_DATA)
        || evidence.is_some_and(|e| !matches!(e.schema_version, 1 | 2 | SCHEMA_WORLD_DATA))
    {
        return Err("Unsupported release manifest version");
    }
    let world = inventory.schema_version == SCHEMA_WORLD_DATA;
    let mut report = Readiness {
        report_version: if world { 3 } else { 2 },
        version: version.into(),
        stable,
        stable_ready: true,
        candidate_ready: true,
        pending: vec![],
        publication_allowed: true,
        blockers: vec![],
    };
    if world && policy.target.is_none() {
        report.block("World-data target file is missing or invalid (ADR 0007 §6)");
    } else if inventory.target != expected_target_with(inventory.schema_version, policy)
        || !bounded_text(&inventory.inventory_id)
    {
        report.block("Inventory version, identifier or frozen target does not match");
    }
    if world && policy.origins.is_none() {
        report.block("Accepted-origin policy is missing or invalid (ADR 0007 §3)");
    }
    if world
        && let Some(origins) = policy.origins
        && policy
            .target
            .is_some_and(|t| t.edition != origins.target_edition.as_str())
    {
        report.block("Accepted-origin policy names a different target edition");
    }
    if matches!(inventory.schema_version, 2 | SCHEMA_WORLD_DATA) {
        if !world && frozen_release_target().is_none() {
            report.block(
                "Release target is pending the first stable live build after launch (ADR 0006)",
            );
        }
        match &inventory.slice {
            Some(slice) if slice_valid(slice) => {
                if slice.classes.is_none() {
                    report.block("Release slice classes are unknown");
                }
            }
            _ => report.block("Release slice is missing or invalid"),
        }
    }
    // A slice always leaves content outside it; an omitted or empty list would
    // merge "not declared" into "nothing outside" (ADR 0006 section 2).
    if matches!(inventory.schema_version, 2 | SCHEMA_WORLD_DATA) {
        for domain in inventory
            .domains
            .iter()
            .filter(|d| d.out_of_scope.is_empty())
        {
            report.block(format!(
                "Out-of-scope boundaries are not declared: {}",
                domain.domain
            ));
        }
    }
    let boundaries: Vec<_> = inventory
        .domains
        .iter()
        .flat_map(|d| &d.out_of_scope)
        .collect();
    if !unique(boundaries.iter().map(|b| b.id.as_str()))
        || boundaries
            .iter()
            .any(|b| !bounded_text(&b.scope) || !bounded_text(&b.reason))
    {
        report.block("Out-of-scope boundaries are invalid, duplicated or unexplained");
    }
    let domains: BTreeSet<_> = inventory
        .domains
        .iter()
        .map(|d| d.domain.as_str())
        .collect();
    if domains != BTreeSet::from(DOMAINS) || inventory.domains.len() != DOMAINS.len() {
        report.block("Mandatory content domains are missing, duplicated or unsupported");
    }
    let units: Vec<_> = inventory.domains.iter().flat_map(|d| &d.units).collect();
    if units.len() > MAX_UNITS || !unique(units.iter().map(|u| u.id.as_str())) {
        report.block("Invalid, duplicate or excessive inventory units");
    }
    let mut records = 0usize;
    for domain in &inventory.domains {
        if domain.completeness != Completeness::Complete
            || domain.units.is_empty()
            || !domain.units.iter().any(|u| {
                u.required_record_ids
                    .as_ref()
                    .is_some_and(|ids| !ids.is_empty())
            })
            || !files_present(&domain.enumeration_evidence)
        {
            report.block(format!(
                "Content inventory is incomplete: {}",
                domain.domain
            ));
        }
        for unit in &domain.units {
            if !bounded_text(&unit.scope) {
                report.block(format!("Unit scope is missing: {}", unit.id));
            }
            match &unit.required_record_ids {
                None => report.block(format!("Required records are unknown: {}", unit.id)),
                Some(ids) => {
                    records = records.saturating_add(ids.len());
                    if !unique(ids.iter().map(String::as_str)) {
                        report.block(format!(
                            "Invalid or duplicate required records: {}",
                            unit.id
                        ));
                    }
                }
            }
        }
    }
    if records == 0 || records > MAX_RECORDS {
        report.block("Required record inventory is empty or excessive");
    }
    if inventory.sources.is_empty()
        || inventory.sources.len() > MAX_UNITS
        || !unique(inventory.sources.iter().map(|s| s.id.as_str()))
        || inventory.sources.iter().any(|s| !sha1(&s.revision))
    {
        report.block("Accepted source revisions are missing or invalid");
    }
    let Some(evidence) = evidence else {
        report.block("Acceptance evidence is missing");
        return Ok(report);
    };
    if !inventory.sources.is_empty() && evidence.admission_registry.is_none() {
        report.block("Accepted sources require a pinned admission registry");
    }
    if evidence.schema_version != inventory.schema_version || evidence.target != inventory.target {
        report.block("Acceptance version or target differs from inventory");
    }
    if evidence.inventory_sha256.as_str() != bindings.inventory_sha256 {
        report.block("Evidence refers to a different inventory revision");
    }
    if world
        && evidence
            .origin_policy
            .as_ref()
            .map(|f| f.sha256.as_str())
            .is_none_or(|sha| Some(sha) != policy.origin_policy_sha256)
    {
        report.block("Evidence does not bind the applied accepted-origin policy");
    }
    let initial_binding =
        evidence.tested_source_commit.is_none() && evidence.production_sha256.is_none();
    if initial_binding {
        report.defer(
            PendingRequirement::InitialTestedBinding,
            "Initial tested source revision and production fingerprint are pending",
        );
    } else if !sha1(bindings.tested_source_commit)
        || evidence.tested_source_commit.as_deref() != Some(bindings.tested_source_commit)
        || evidence.production_sha256.as_ref().map(Sha256Hex::as_str)
            != Some(bindings.production_sha256)
    {
        report.block("Tested source revision or production fingerprint does not match");
    }
    let required = required_checks(inventory.schema_version);
    let expected_checks: BTreeSet<_> = required.iter().map(|(id, _)| *id).collect();
    if evidence.checks.len() != expected_checks.len()
        || evidence
            .checks
            .iter()
            .map(|c| c.id.as_str())
            .collect::<BTreeSet<_>>()
            != expected_checks
    {
        report.block("Mandatory acceptance rows are missing, duplicated or unsupported");
    }
    for check in &evidence.checks {
        let method = required
            .iter()
            .find(|(id, _)| *id == check.id)
            .map_or(Method::InteractiveWindows, |(_, method)| *method);
        if check.status == Status::Unverified
            && check.method == method
            && check.operator.is_none()
            && check.recorded_at.is_none()
            && check.environment.is_none()
            && check.artifacts.is_empty()
            && expected_checks.contains(check.id.as_str())
        {
            report.defer(
                PendingRequirement::Observation {
                    check_id: check.id.clone(),
                },
                format!("Required acceptance observation is pending: {}", check.id),
            );
            continue;
        }
        if initial_binding
            || check.status != Status::Pass
            || check.method != method
            || [&check.operator, &check.recorded_at, &check.environment]
                .into_iter()
                .any(|s| !s.as_deref().is_some_and(bounded_text))
            || !files_present(&check.artifacts)
        {
            report.block(format!(
                "Required acceptance has not passed with recorded environment evidence: {}",
                check.id
            ));
        }
    }
    // Schema 3 has no live slice matrix: 1.0.0 ships without live-client
    // evidence (ADR 0007 §5 and Consequences).
    if inventory.schema_version == 2 {
        assess_matrix(&mut report, inventory, evidence, initial_binding);
    }
    if evidence.coverage.len() != units.len()
        || !unique(evidence.coverage.iter().map(|u| u.unit_id.as_str()))
    {
        report.block("Unit coverage is missing, duplicated or excessive");
    }
    let coverage_by_id: BTreeMap<_, _> = evidence
        .coverage
        .iter()
        .map(|c| (c.unit_id.as_str(), c))
        .collect();
    let source_ids: BTreeSet<_> = inventory.sources.iter().map(|s| s.id.as_str()).collect();
    for unit in units {
        let coverage = coverage_by_id.get(unit.id.as_str());
        if coverage.is_none_or(|c| {
            c.status != Status::Pass
                || c.unresolved_conflicts != 0
                || c.unsupported_corrections != 0
                || !files_present(&c.validation_evidence)
                || c.source_ids.is_empty()
                || !unique(c.source_ids.iter().map(String::as_str))
                || c.source_ids
                    .iter()
                    .any(|id| !source_ids.contains(id.as_str()))
                || !unique(c.included_record_ids.iter().map(String::as_str))
                || unit.required_record_ids.as_ref().is_none_or(|ids| {
                    ids.iter().collect::<BTreeSet<_>>()
                        != c.included_record_ids.iter().collect::<BTreeSet<_>>()
                })
        }) {
            report.block(format!(
                "Mandatory unit coverage has not passed: {}",
                unit.id
            ));
        }
    }
    if evidence.sources.len() != inventory.sources.len()
        || !unique(evidence.sources.iter().map(|s| s.id.as_str()))
    {
        report.block("Source evidence inventory differs from accepted sources");
    }
    let sources_by_id: BTreeMap<_, _> = evidence
        .sources
        .iter()
        .map(|s| (s.id.as_str(), s))
        .collect();
    let origin_accepted = origin_accepted_sources(inventory, evidence, policy);
    for required in &inventory.sources {
        if sources_by_id.get(required.id.as_str()).is_none_or(|s| {
            let target_ok = (s.target_validation == Status::Pass
                && files_present(&s.target_evidence))
                || origin_accepted.contains(&s.id);
            s.revision != required.revision
                || s.redistribution != Permission::Permitted
                || !bounded_text(&s.origin_edition)
                || s.origin_build.as_deref().is_some_and(|b| !bounded_text(b))
                || !files_present(&s.files)
                || !files_present(&s.license_files)
                || !files_present(&s.attribution_files)
                || !files_present(&s.permission_evidence)
                || !target_ok
        }) {
            report.block(format!(
                "Source permission, provenance or target validation is incomplete: {}",
                required.id
            ));
        }
    }
    if evidence.packs.is_empty()
        || evidence.packs.len() > MAX_UNITS
        || !unique(evidence.packs.iter().map(|p| p.manifest.path.as_str()))
        || evidence.packs.iter().any(|p| {
            p.manifest.bytes == 0
                || p.dataset_revisions.is_empty()
                || !unique(p.dataset_revisions.iter().map(String::as_str))
        })
    {
        report.block("Validated offline content packs are missing or invalid");
    }
    let mut deferrals = BTreeSet::new();
    if evidence
        .optional_deferrals
        .iter()
        .any(|d| !deferrals.insert(d.feature) || !bounded_text(&d.reason))
    {
        report.block("Optional deferrals are duplicate or unexplained");
    }
    Ok(report)
}

/// Every class must pass in every level band, and every zone and party size of
/// the slice must appear in a passing row. Pristine rows are pending.
fn assess_matrix(
    report: &mut Readiness,
    inventory: &Inventory,
    evidence: &Evidence,
    initial_binding: bool,
) {
    let Some(slice) = &inventory.slice else {
        return;
    };
    let Some(classes) = &slice.classes else {
        report.block("Slice acceptance matrix cannot be checked until the slice classes are known");
        return;
    };
    let bands = level_bands(slice);
    // The SLICE- prefix keeps pending matrix rows distinct from LIVE/WIN checks.
    if !unique(evidence.matrix.iter().map(|r| r.id.as_str()))
        || evidence.matrix.iter().any(|r| !r.id.starts_with("SLICE-"))
    {
        report.block("Slice acceptance matrix rows are invalid or duplicated");
    }
    let inside = |row: &MatrixRow| {
        classes.contains(&row.class)
            && slice.zones.contains(&row.zone)
            && slice.party_sizes.contains(&row.party_size)
            && bands.contains(&(row.level_min, row.level_max))
    };
    for row in &evidence.matrix {
        if !inside(row) {
            report.block(format!(
                "Slice acceptance row is outside the slice: {}",
                row.id
            ));
            continue;
        }
        if row.status == Status::Unverified
            && row.method == Method::LiveGame
            && row.operator.is_none()
            && row.recorded_at.is_none()
            && row.environment.is_none()
            && row.artifacts.is_empty()
        {
            report.defer(
                PendingRequirement::Observation {
                    check_id: row.id.clone(),
                },
                format!("Slice acceptance observation is pending: {}", row.id),
            );
            continue;
        }
        if initial_binding
            || row.status != Status::Pass
            || row.method != Method::LiveGame
            || [&row.operator, &row.recorded_at, &row.environment]
                .into_iter()
                .any(|s| !s.as_deref().is_some_and(bounded_text))
            || !files_present(&row.artifacts)
        {
            report.block(format!(
                "Slice acceptance has not passed with recorded environment evidence: {}",
                row.id
            ));
        }
    }
    // Every inside row is pending, failed or passed and each non-pass blocks or
    // defers above, so coverage counts the inside rows and names the gaps.
    let planned: Vec<_> = evidence.matrix.iter().filter(|r| inside(r)).collect();
    for class in classes {
        for band in &bands {
            if !planned
                .iter()
                .any(|r| &r.class == class && (r.level_min, r.level_max) == *band)
            {
                report.block(format!(
                    "Slice acceptance matrix lacks {class} at levels {}-{}",
                    band.0, band.1
                ));
            }
        }
    }
    for zone in &slice.zones {
        if !planned.iter().any(|r| &r.zone == zone) {
            report.block(format!("Slice acceptance matrix lacks zone {zone}"));
        }
    }
    for size in &slice.party_sizes {
        if !planned.iter().any(|r| r.party_size == *size) {
            report.block(format!("Slice acceptance matrix lacks party size {size}"));
        }
    }
}

/// All evidence files are checked by the CLI without executing their contents.
pub fn evidence_files<'a>(inventory: &'a Inventory, evidence: &'a Evidence) -> Vec<&'a FileRef> {
    inventory
        .domains
        .iter()
        .flat_map(|d| &d.enumeration_evidence)
        .chain(evidence.checks.iter().flat_map(|c| &c.artifacts))
        .chain(
            evidence
                .coverage
                .iter()
                .flat_map(|c| &c.validation_evidence),
        )
        .chain(evidence.sources.iter().flat_map(|s| {
            s.files
                .iter()
                .chain(&s.license_files)
                .chain(&s.attribution_files)
                .chain(&s.permission_evidence)
                .chain(&s.target_evidence)
        }))
        .chain(evidence.admission_registry.iter())
        .chain(evidence.origin_policy.iter())
        .chain(evidence.packs.iter().map(|p| &p.manifest))
        .chain(evidence.matrix.iter().flat_map(|r| &r.artifacts))
        .collect()
}
