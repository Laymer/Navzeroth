//! Release-only contracts. These are not addon exchange schemas or gameplay facts.
//! A passing report checks declarations; the I/O boundary must also verify file
//! hashes, pack contents and the tested Git revision before publication.
use crate::types::{AssetPath, Sha256Hex};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_RELEASE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_UNITS: usize = 4096;
pub const MAX_RECORDS: usize = 250_000;
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
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainInventory {
    pub domain: String,
    pub completeness: Completeness,
    pub enumeration_evidence: Vec<FileRef>,
    pub units: Vec<Unit>,
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
    pub target: Target,
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
    pub target: Target,
    pub inventory_sha256: Sha256Hex,
    /// The actual tested ancestor, not the later commit that records evidence.
    pub tested_source_commit: Option<String>,
    pub production_sha256: Option<Sha256Hex>,
    pub checks: Vec<Acceptance>,
    pub coverage: Vec<UnitCoverage>,
    pub sources: Vec<SourceEvidence>,
    pub packs: Vec<PackRef>,
    pub optional_deferrals: Vec<Deferral>,
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
    if inventory.schema_version != 1 {
        return Err("Unsupported inventory version");
    }
    Ok(inventory)
}
pub fn decode_evidence(bytes: &[u8]) -> Result<Evidence, &'static str> {
    let evidence: Evidence = decode(bytes)?;
    if evidence.schema_version != 1 {
        return Err("Unsupported evidence version");
    }
    Ok(evidence)
}
fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, &'static str> {
    if bytes.len() > MAX_RELEASE_BYTES {
        return Err("Release manifest exceeds size limit");
    }
    // Typed structs reject duplicate and unknown fields. Arrays are checked below.
    serde_json::from_slice(bytes).map_err(|_| "Invalid release manifest")
}

fn check_limits(inventory: &Inventory, evidence: Option<&Evidence>) -> Result<(), &'static str> {
    let units = inventory.domains.iter().flat_map(|d| &d.units);
    if inventory.domains.len() > DOMAINS.len()
        || inventory.sources.len() > MAX_UNITS
        || units.clone().count() > MAX_UNITS
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
    let stable = is_stable(version)?;
    check_limits(inventory, evidence)?;
    if inventory.schema_version != 1 || evidence.is_some_and(|e| e.schema_version != 1) {
        return Err("Unsupported release manifest version");
    }
    let mut report = Readiness {
        report_version: 2,
        version: version.into(),
        stable,
        stable_ready: true,
        candidate_ready: true,
        pending: vec![],
        publication_allowed: true,
        blockers: vec![],
    };
    if inventory.schema_version != 1
        || inventory.target != Target::forever_70374()
        || !bounded_text(&inventory.inventory_id)
    {
        report.block("Inventory version, identifier or frozen target does not match");
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
    if evidence.schema_version != 1 || evidence.target != inventory.target {
        report.block("Acceptance version or target differs from inventory");
    }
    if evidence.inventory_sha256.as_str() != bindings.inventory_sha256 {
        report.block("Evidence refers to a different inventory revision");
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
    let expected_checks: BTreeSet<_> = LIVE_CHECKS.into_iter().chain(WINDOWS_CHECKS).collect();
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
        let method = if LIVE_CHECKS.contains(&check.id.as_str()) {
            Method::LiveGame
        } else {
            Method::InteractiveWindows
        };
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
    for required in &inventory.sources {
        if sources_by_id.get(required.id.as_str()).is_none_or(|s| {
            s.revision != required.revision
                || s.redistribution != Permission::Permitted
                || !bounded_text(&s.origin_edition)
                || s.origin_build.as_deref().is_some_and(|b| !bounded_text(b))
                || !files_present(&s.files)
                || !files_present(&s.license_files)
                || !files_present(&s.attribution_files)
                || !files_present(&s.permission_evidence)
                || s.target_validation != Status::Pass
                || !files_present(&s.target_evidence)
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
        .chain(evidence.packs.iter().map(|p| &p.manifest))
        .collect()
}
