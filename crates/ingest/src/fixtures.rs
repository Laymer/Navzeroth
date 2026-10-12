//! Fixture importer stub (plan section 17, M1.5) behind `azeroth-cli validate-fixtures`.
//!
//! Two manifests are checked against their declared outcomes, each as an explicit
//! directory; nothing here globs `fixtures/**`:
//! - the contract fixtures run through `decode_any` (kind `expect`
//!   `valid`, `schema_invalid` or `semantic_invalid` with an `error_code`);
//! - the transport fixtures run through the real import service
//!   ([`crate::import::import_file`]), `expect` `valid` or `invalid`.
//!
//! Every file in the directory must be listed exactly once. Failures are reported
//! as a file name taken from the manifest plus a closed [`Problem`]; fixture
//! content is never echoed.
use crate::import::{ExportLocation, FileFormat, ImportOptions, import_file, read_bounded};
use azeroth_core::{
    contract::{MAX_DATASET_BYTES, decode_any},
    error::ErrorCode,
    observation::{ClockTrust, FreshnessContext},
    types::Millis,
};
use serde::Deserialize;
use std::{collections::BTreeSet, path::Path};

pub const MANIFEST_FILE: &str = "manifest.json";
const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_ENTRIES: usize = 1024;
const MAX_MISMATCHES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestReport {
    pub checked: usize,
    /// Entries declared valid that were accepted.
    pub valid: usize,
    /// Entries declared invalid that were rejected with the declared error code.
    pub rejected: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    ManifestUnreadable,
    ManifestInvalid,
    TooManyEntries,
    UnsafeFileName,
    ListedTwice,
    Unlisted,
    Missing,
    Unreadable,
    UnknownExpectation,
    MissingErrorCode,
    AcceptedButDeclaredInvalid,
    RejectedButDeclaredValid,
    WrongErrorCode,
    WrongKind,
    Internal,
}
impl Problem {
    pub fn message(self) -> &'static str {
        match self {
            Self::ManifestUnreadable => "manifest cannot be read",
            Self::ManifestInvalid => "manifest is not valid",
            Self::TooManyEntries => "manifest lists too many files",
            Self::UnsafeFileName => "file name is not a plain safe name",
            Self::ListedTwice => "file is listed more than once",
            Self::Unlisted => "file exists but is not listed",
            Self::Missing => "file is listed but missing",
            Self::Unreadable => "file cannot be read",
            Self::UnknownExpectation => "unknown expectation or format",
            Self::MissingErrorCode => "invalid entry declares no error code",
            Self::AcceptedButDeclaredInvalid => "accepted but declared invalid",
            Self::RejectedButDeclaredValid => "rejected but declared valid",
            Self::WrongErrorCode => "rejected with a different error code",
            Self::WrongKind => "decoded to a different kind",
            Self::Internal => "internal fixture check error",
        }
    }
}

/// One failed check. `file` is a validated plain file name from the manifest or
/// the directory, never fixture content; it is empty for manifest-level problems.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch {
    pub file: String,
    pub problem: Problem,
}

#[derive(Deserialize)]
struct ContractManifest {
    fixtures: Vec<ContractEntry>,
}
#[derive(Deserialize)]
struct ContractEntry {
    file: String,
    #[serde(default)]
    kind: Option<String>,
    expect: String,
    #[serde(default)]
    error_code: Option<String>,
}
#[derive(Deserialize)]
struct TransportManifest {
    #[serde(default)]
    saved_variables: Option<LocationSpec>,
    fixtures: Vec<TransportEntry>,
}
#[derive(Deserialize)]
struct LocationSpec {
    global: String,
    key: String,
}
#[derive(Deserialize)]
struct TransportEntry {
    file: String,
    format: String,
    expect: String,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    error_code: Option<String>,
}

fn mismatch(file: &str, problem: Problem) -> Mismatch {
    Mismatch {
        file: file.to_owned(),
        problem,
    }
}
fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.starts_with('.')
        && !name.contains("..")
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}
fn code_name(code: ErrorCode) -> String {
    serde_json::to_value(code)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Read and parse the manifest, then check the listing against the directory:
/// every entry safe and unique, every non-manifest file listed exactly once.
fn load<M, F>(dir: &Path, files_of: F) -> Result<M, Vec<Mismatch>>
where
    M: serde::de::DeserializeOwned,
    F: Fn(&M) -> Vec<&str>,
{
    let text = read_bounded(&dir.join(MANIFEST_FILE), MAX_MANIFEST_BYTES)
        .map_err(|_| vec![mismatch("", Problem::ManifestUnreadable)])?;
    let manifest: M =
        serde_json::from_str(&text).map_err(|_| vec![mismatch("", Problem::ManifestInvalid)])?;
    let listed = files_of(&manifest);
    if listed.len() > MAX_ENTRIES {
        return Err(vec![mismatch("", Problem::TooManyEntries)]);
    }
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for name in &listed {
        if !safe_name(name) || *name == MANIFEST_FILE {
            problems.push(mismatch("", Problem::UnsafeFileName));
        } else if !seen.insert(*name) {
            problems.push(mismatch(name, Problem::ListedTwice));
        }
    }
    let on_disk = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Err(vec![mismatch("", Problem::ManifestUnreadable)]),
    };
    let mut disk_names = BTreeSet::new();
    for entry in on_disk {
        let Ok(entry) = entry else {
            problems.push(mismatch("", Problem::Unreadable));
            continue;
        };
        match entry.file_name().into_string() {
            Ok(name) if name == MANIFEST_FILE => {}
            Ok(name) if safe_name(&name) => {
                disk_names.insert(name);
            }
            _ => problems.push(mismatch("", Problem::UnsafeFileName)),
        }
    }
    for name in &disk_names {
        if !seen.contains(name.as_str()) {
            problems.push(mismatch(name, Problem::Unlisted));
        }
    }
    for name in &seen {
        if !disk_names.contains(*name) {
            problems.push(mismatch(name, Problem::Missing));
        }
    }
    if problems.is_empty() {
        Ok(manifest)
    } else {
        Err(problems)
    }
}

fn finish(
    report: ManifestReport,
    mut problems: Vec<Mismatch>,
) -> Result<ManifestReport, Vec<Mismatch>> {
    problems.truncate(MAX_MISMATCHES);
    if problems.is_empty() {
        Ok(report)
    } else {
        Err(problems)
    }
}

/// Check `fixtures/contracts/v1`-shaped directories against `manifest.json`.
pub fn check_contract_fixtures(dir: &Path) -> Result<ManifestReport, Vec<Mismatch>> {
    let manifest: ContractManifest = load(dir, |m: &ContractManifest| {
        m.fixtures.iter().map(|e| e.file.as_str()).collect()
    })?;
    let mut report = ManifestReport {
        checked: 0,
        valid: 0,
        rejected: 0,
    };
    let mut problems = Vec::new();
    for entry in &manifest.fixtures {
        report.checked += 1;
        let file = entry.file.as_str();
        let Ok(text) = read_bounded(&dir.join(file), MAX_DATASET_BYTES) else {
            problems.push(mismatch(file, Problem::Unreadable));
            continue;
        };
        let result = decode_any(&text);
        match (entry.expect.as_str(), result) {
            ("valid", Ok(exchange)) => {
                if entry
                    .kind
                    .as_deref()
                    .is_some_and(|k| k != exchange.kind().as_str())
                {
                    problems.push(mismatch(file, Problem::WrongKind));
                } else {
                    report.valid += 1;
                }
            }
            ("valid", Err(_)) => problems.push(mismatch(file, Problem::RejectedButDeclaredValid)),
            ("schema_invalid" | "semantic_invalid", Ok(_)) => {
                problems.push(mismatch(file, Problem::AcceptedButDeclaredInvalid));
            }
            ("schema_invalid" | "semantic_invalid", Err(error)) => match &entry.error_code {
                None => problems.push(mismatch(file, Problem::MissingErrorCode)),
                Some(code) if *code == code_name(error.code) => report.rejected += 1,
                Some(_) => problems.push(mismatch(file, Problem::WrongErrorCode)),
            },
            _ => problems.push(mismatch(file, Problem::UnknownExpectation)),
        }
    }
    finish(report, problems)
}

/// Check `fixtures/transport`-shaped directories against `manifest.json`
/// through the real import service, using snapshot-only options and no clock.
pub fn check_transport_fixtures(dir: &Path) -> Result<ManifestReport, Vec<Mismatch>> {
    let manifest: TransportManifest = load(dir, |m: &TransportManifest| {
        m.fixtures.iter().map(|e| e.file.as_str()).collect()
    })?;
    let Ok(max_age_ms) = Millis::new(0) else {
        return Err(vec![mismatch("", Problem::Internal)]);
    };
    let options = ImportOptions::snapshots(FreshnessContext {
        now_utc_ms: None,
        clock_trust: ClockTrust::Uncertain,
        session: None,
        max_age_ms,
    });
    let location = manifest.saved_variables.map(|l| ExportLocation {
        global: l.global,
        key: l.key,
    });
    let mut report = ManifestReport {
        checked: 0,
        valid: 0,
        rejected: 0,
    };
    let mut problems = Vec::new();
    for entry in &manifest.fixtures {
        report.checked += 1;
        let file = entry.file.as_str();
        let format = match (entry.format.as_str(), &location) {
            ("anav1_text", _) => FileFormat::Anav1Text,
            ("saved_variables", Some(location)) => FileFormat::SavedVariables(location.clone()),
            _ => {
                problems.push(mismatch(file, Problem::UnknownExpectation));
                continue;
            }
        };
        let result = import_file(&dir.join(file), &format, &options);
        match (entry.expect.as_str(), result) {
            ("valid", Ok(imported)) => {
                if entry
                    .kind
                    .as_deref()
                    .is_some_and(|k| k != imported.kind.as_str())
                {
                    problems.push(mismatch(file, Problem::WrongKind));
                } else {
                    report.valid += 1;
                }
            }
            ("valid", Err(_)) => problems.push(mismatch(file, Problem::RejectedButDeclaredValid)),
            ("invalid", Ok(_)) => {
                problems.push(mismatch(file, Problem::AcceptedButDeclaredInvalid))
            }
            ("invalid", Err(error)) => match &entry.error_code {
                None => problems.push(mismatch(file, Problem::MissingErrorCode)),
                Some(code) if *code == code_name(error.code) => report.rejected += 1,
                Some(_) => problems.push(mismatch(file, Problem::WrongErrorCode)),
            },
            _ => problems.push(mismatch(file, Problem::UnknownExpectation)),
        }
    }
    finish(report, problems)
}
