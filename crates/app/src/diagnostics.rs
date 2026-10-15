//! Explicit local allowlisted diagnostic metadata. Imported strings are never logs.
use crate::catalog::CatalogView;
use azeroth_core::{
    contract::Envelope,
    coverage::CoverageClass,
    entities::{CapabilityStatus, CharacterSnapshot, GameBuild},
    error::{AppError, ErrorCode, Result},
    observation::{Freshness, FreshnessContext, ObservationStatus, freshness},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, VecDeque},
    io::Write,
    path::Path,
};
const MAX_EVENTS: usize = 64;
pub const MAX_DIAGNOSTIC_BYTES: usize = 64 * 1024;
const MAX_DURATION_MS: u64 = 86_400_000;
fn invalid() -> AppError {
    AppError::new(
        ErrorCode::InvalidInput,
        "Invalid diagnostic metadata",
        false,
    )
}
fn storage() -> AppError {
    AppError::new(
        ErrorCode::StorageError,
        "Diagnostic file cannot be written",
        false,
    )
}
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticOperation {
    SnapshotImport,
    SessionImport,
    ContentImport,
    Planning,
    JointPlanning,
    Backup,
    Restore,
}
#[derive(Clone, Debug, Serialize)]
struct DiagnosticEvent {
    operation: DiagnosticOperation,
    error: Option<ErrorCode>,
    duration_ms: Option<u64>,
}
/// Only fixed enum codes and measured durations enter this ring. No text API exists.
#[derive(Clone, Default)]
pub struct DiagnosticTelemetry {
    events: VecDeque<DiagnosticEvent>,
    truncated: bool,
    import_attempts: Option<u64>,
    import_succeeded: Option<u64>,
    import_failed: Option<u64>,
}
impl DiagnosticTelemetry {
    /// Counts start at actual observed zero for this process collector, not history.
    pub fn observed() -> Self {
        Self {
            import_attempts: Some(0),
            import_succeeded: Some(0),
            import_failed: Some(0),
            ..Self::default()
        }
    }
    pub fn record(
        &mut self,
        operation: DiagnosticOperation,
        duration_ms: Option<u64>,
        error: Option<ErrorCode>,
    ) -> Result<()> {
        if duration_ms.is_some_and(|value| value > MAX_DURATION_MS) {
            return Err(invalid());
        }
        if matches!(
            operation,
            DiagnosticOperation::SnapshotImport
                | DiagnosticOperation::SessionImport
                | DiagnosticOperation::ContentImport
        ) {
            for counter in [
                &mut self.import_attempts,
                if error.is_some() {
                    &mut self.import_failed
                } else {
                    &mut self.import_succeeded
                },
            ] {
                if let Some(value) = counter {
                    *counter = (*value < 9_007_199_254_740_991).then_some(*value + 1);
                }
            }
        }
        if self.events.len() == MAX_EVENTS {
            self.truncated = true;
            self.events.pop_front();
        }
        self.events.push_back(DiagnosticEvent {
            operation,
            error,
            duration_ms,
        });
        Ok(())
    }
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Os {
    Windows,
    Linux,
    Macos,
    Other,
}
#[derive(Serialize)]
struct Versions {
    app: &'static str,
    bundled_addon: &'static str,
    installed_addon: Option<&'static str>,
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Edition {
    Forever,
    Other,
}
#[derive(Serialize)]
struct Game {
    edition: Edition,
    build: Option<u32>,
    interface: Option<u32>,
    fixture: bool,
}
#[derive(Serialize)]
struct Dataset {
    revision_fingerprint_sha256: String,
    coverage_class: CoverageClass,
    collections:
        BTreeMap<azeroth_core::coverage::Collection, azeroth_core::coverage::CollectionCoverage>,
    errors: usize,
    gaps: usize,
}
#[derive(Serialize)]
struct Telemetry {
    scope: &'static str,
    event_limit: usize,
    truncated: Option<bool>,
    import_attempts: Option<u64>,
    import_succeeded: Option<u64>,
    import_failed: Option<u64>,
    events: Vec<DiagnosticEvent>,
}
#[derive(Serialize)]
struct Capability {
    availability: ObservationStatus,
    check: Option<CapabilityStatus>,
    verified_available: Option<bool>,
    freshness: Option<Freshness>,
}
#[derive(Serialize)]
struct HistoricalReport {
    build: u32,
    interface: u32,
    source: &'static str,
    fields: [&'static str; 2],
    installed_revision_match: Option<bool>,
}
#[derive(Serialize)]
pub struct DiagnosticReport {
    format: &'static str,
    schema_version: u32,
    versions: Versions,
    os: Os,
    game: Option<Game>,
    capabilities: BTreeMap<&'static str, Capability>,
    historical_user_report: HistoricalReport,
    snapshot_availability: BTreeMap<&'static str, ObservationStatus>,
    snapshot_freshness: Option<Freshness>,
    field_freshness: BTreeMap<&'static str, Option<Freshness>>,
    dataset: Option<Dataset>,
    dataset_context_mismatch: bool,
    telemetry: Telemetry,
}
#[derive(Clone, Debug, Serialize)]
pub struct DiagnosticReceipt {
    pub schema_version: u32,
    pub bytes: usize,
}
fn decimal(value: &str) -> Option<u32> {
    if value.is_empty() || value.len() > 10 || !value.bytes().all(|b| b.is_ascii_digit()) {
        None
    } else {
        value.parse().ok()
    }
}
/// Capture only selected immutable snapshot/content under the application's lease.
/// Every exported text field is a fixed label, trusted producer version or digest.
pub fn build(
    snapshot: Option<&Envelope<CharacterSnapshot>>,
    content: Option<&dyn CatalogView>,
    telemetry: Option<&DiagnosticTelemetry>,
) -> Result<DiagnosticReport> {
    build_with_context(snapshot, content, telemetry, None, None)
}
/// Optional measured clock/capability inputs; unavailable collectors remain unknown.
pub fn build_with_context(
    snapshot: Option<&Envelope<CharacterSnapshot>>,
    content: Option<&dyn CatalogView>,
    telemetry: Option<&DiagnosticTelemetry>,
    clock: Option<&FreshnessContext>,
    capability_build: Option<&GameBuild>,
) -> Result<DiagnosticReport> {
    let game = snapshot.map(|s| Game {
        edition: if s.payload.context.build.edition.as_str() == "wow_forever" {
            Edition::Forever
        } else {
            Edition::Other
        },
        build: decimal(s.payload.context.build.build.as_str()),
        interface: decimal(s.payload.context.build.interface.as_str()),
        fixture: s.fixture,
    });
    let mismatch = snapshot
        .zip(content)
        .is_some_and(|(s, c)| s.data_context() != c.context() || s.fixture != c.fixture());
    let dataset = if mismatch {
        None
    } else {
        content.map(|content| {
            let report = content.report();
            Dataset {
                revision_fingerprint_sha256: format!(
                    "{:x}",
                    Sha256::digest(content.reference().revision.as_str().as_bytes())
                ),
                coverage_class: report
                    .dataset
                    .expect("validated dataset header")
                    .coverage_class,
                collections: report
                    .coverage
                    .expect("validated dataset coverage")
                    .collections,
                errors: report.errors.len(),
                gaps: report.gaps.len(),
            }
        })
    };
    let has_telemetry = telemetry.is_some();
    let unknown = DiagnosticTelemetry::default();
    let telemetry = telemetry.unwrap_or(&unknown);
    let report = DiagnosticReport {
        format: "azeroth-local-diagnostics",
        schema_version: 1,
        versions: Versions {
            app: env!("CARGO_PKG_VERSION"),
            bundled_addon: env!("CARGO_PKG_VERSION"),
            installed_addon: None,
        },
        os: match std::env::consts::OS {
            "windows" => Os::Windows,
            "linux" => Os::Linux,
            "macos" => Os::Macos,
            _ => Os::Other,
        },
        game,
        capabilities: [
            "addon_party_messages",
            "character_level",
            "equipment_item_links",
            "map_position",
            "quest_completion_history",
            "quest_log",
            "saved_variables_persistence",
        ]
        .into_iter()
        .map(|key| {
            let observed = capability_build
                .filter(|build| snapshot.is_some_and(|s| s.payload.context.build == build.key));
            let checks = observed.and_then(|b| b.capabilities.value());
            let check = checks.and_then(|checks| {
                checks
                    .iter()
                    .find(|(id, _)| id.as_str() == key)
                    .map(|(_, check)| check.status)
            });
            let availability = if check.is_some() {
                ObservationStatus::Known
            } else {
                observed
                    .map(|b| b.capabilities.status())
                    .filter(|s| *s != ObservationStatus::Known)
                    .unwrap_or(ObservationStatus::Unknown)
            };
            (
                key,
                Capability {
                    availability,
                    check,
                    verified_available: match check {
                        Some(CapabilityStatus::VerifiedAvailable) => Some(true),
                        Some(CapabilityStatus::VerifiedUnavailable) => Some(false),
                        _ => None,
                    },
                    freshness: observed
                        .zip(clock)
                        .map(|(b, c)| freshness(b.capabilities.metadata(), c)),
                },
            )
        })
        .collect(),
        historical_user_report: HistoricalReport {
            build: 70338,
            interface: 16001,
            source: "limited_user_report",
            fields: ["level", "clock"],
            installed_revision_match: None,
        },
        snapshot_availability: snapshot
            .map(|s| {
                [
                    ("level", s.payload.level.status()),
                    ("role", s.payload.role.status()),
                    ("location", s.payload.location.status()),
                ]
                .into()
            })
            .unwrap_or_default(),
        field_freshness: snapshot
            .map(|s| {
                [
                    (
                        "level",
                        clock.map(|c| freshness(s.payload.level.metadata(), c)),
                    ),
                    (
                        "role",
                        clock.map(|c| freshness(s.payload.role.metadata(), c)),
                    ),
                    (
                        "location",
                        clock.map(|c| freshness(s.payload.location.metadata(), c)),
                    ),
                ]
                .into()
            })
            .unwrap_or_default(),
        snapshot_freshness: snapshot
            .zip(clock)
            .map(|(s, c)| freshness(&s.payload.capture, c)),
        dataset,
        dataset_context_mismatch: mismatch,
        telemetry: Telemetry {
            scope: "current_process_collector",
            event_limit: MAX_EVENTS,
            truncated: has_telemetry.then_some(telemetry.truncated),
            import_attempts: telemetry.import_attempts,
            import_succeeded: telemetry.import_succeeded,
            import_failed: telemetry.import_failed,
            events: telemetry.events.iter().cloned().collect(),
        },
    };
    serialize(&report)?;
    Ok(report)
}
pub fn serialize(report: &DiagnosticReport) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec_pretty(report).map_err(|_| invalid())?;
    if bytes.len() > MAX_DIAGNOSTIC_BYTES {
        return Err(invalid());
    }
    Ok(bytes)
}
/// New explicitly selected file only. No overwrite, upload or input execution.
pub fn write_new(report: &DiagnosticReport, path: &Path) -> Result<DiagnosticReceipt> {
    let bytes = serialize(report)?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|_| storage())?;
    let result = file.write_all(&bytes).and_then(|()| file.sync_all());
    drop(file);
    if result.is_err() {
        let _ = std::fs::remove_file(path);
        return Err(storage());
    }
    Ok(DiagnosticReceipt {
        schema_version: 1,
        bytes: bytes.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn serialization_enforces_byte_bound_before_file_creation() {
        let mut report = build(None, None, None).unwrap();
        report.format = Box::leak("x".repeat(MAX_DIAGNOSTIC_BYTES).into_boxed_str());
        assert_eq!(
            serialize(&report).unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }
    #[test]
    fn numeric_build_input_preserves_zero_but_rejects_text_and_overflow() {
        assert_eq!(decimal("0"), Some(0));
        assert_eq!(decimal("70338"), Some(70338));
        for value in [
            "",
            "private-secret",
            "4294967296",
            "12345678901",
            " 70338",
            "７０３３８",
        ] {
            assert_eq!(decimal(value), None);
        }
    }
}
