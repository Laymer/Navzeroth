//! The import service: one validation path for every input shape (plan section 17, M2).
//!
//! `import_text`, `import_saved_variables` and `import_file` only differ in how
//! they obtain the JSON document. All of them end in [`normalize`], which
//! enforces the 4 MiB and nesting ceilings before parsing, applies the accepted
//! kind policy, validates through the core contract decoder and summarizes the
//! outcome. The caller supplies the clock inside [`ImportOptions`]; this module
//! never reads one. Imported state is imported, never live.
use crate::{
    anav1::{self, MAX_TEXT_BYTES},
    error::{invalid, too_large, unreadable},
    savedvars::{self, MAX_FILE_BYTES},
};
use azeroth_core::{
    contract::{
        AnyExchange, ContractKind, Envelope, EnvelopeData, Payload, ProducerComponent, decode_any,
    },
    entities::{CharacterSnapshot, PartySnapshot},
    error::Result,
    observation::{Freshness, FreshnessContext, ObservationMetadata, freshness},
    types::{BuildKey, CharacterId, DatasetNamespace, Sequence, SessionId, SnapshotId, UtcMillis},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::Path,
};

/// Which kinds a transport accepts and how freshness is judged.
#[derive(Clone, Debug)]
pub struct ImportOptions {
    pub accepted_kinds: BTreeSet<ContractKind>,
    /// Caller-supplied clock, trust and maximum age. With no trusted clock the
    /// freshness of a snapshot is `Uncertain`, never assumed fresh.
    pub freshness: FreshnessContext,
}
impl ImportOptions {
    /// Character and party snapshots only: the kinds the addon produces.
    pub fn snapshots(freshness: FreshnessContext) -> Self {
        Self {
            accepted_kinds: BTreeSet::from([
                ContractKind::CharacterSnapshot,
                ContractKind::PartySnapshot,
            ]),
            freshness,
        }
    }
}

/// Where a SavedVariables file keeps the export string. Both names are
/// parameters because the addon layout is UNVERIFIED until M0.5.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportLocation {
    pub global: String,
    pub key: String,
}

/// How a file's bytes are interpreted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileFormat {
    /// A file holding exactly one `ANAV1:` transport text.
    Anav1Text,
    /// A SavedVariables file holding an `ANAV1:` string at the given location.
    SavedVariables(ExportLocation),
}

/// Per-snapshot facts. Freshness is judged against the caller's clock, and only
/// states that clock supports: otherwise `Uncertain`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotSummary {
    pub snapshot_id: SnapshotId,
    pub freshness: Freshness,
    pub observed_at_utc_ms: Option<UtcMillis>,
    pub capture_session_id: Option<SessionId>,
    pub capture_sequence: Option<Sequence>,
}

/// The outcome of a successful import. The same envelope gives the same result
/// from text, from a file, or from SavedVariables. Nothing here claims the data
/// is live, and nothing decides whether fixture or demo data is acceptable:
/// production callers must check [`ImportResult::is_fixture_or_demo`].
#[derive(Clone, Debug, PartialEq)]
pub struct ImportResult {
    pub kind: ContractKind,
    /// Synthetic engineering data (CASE-015). Production planning rejects it.
    pub fixture: bool,
    pub namespace: DatasetNamespace,
    pub build: BuildKey,
    pub producer_component: ProducerComponent,
    pub producer_session_id: Option<SessionId>,
    pub producer_sequence: Option<Sequence>,
    /// The document's recorded creation time; absent means none was recorded.
    pub created_at_utc_ms: Option<UtcMillis>,
    /// One entry per character snapshot (a party has one per participant),
    /// keyed by character in deterministic order. Empty for other kinds.
    pub snapshots: BTreeMap<CharacterId, SnapshotSummary>,
    /// The validated document.
    pub exchange: AnyExchange,
}
impl ImportResult {
    pub fn is_fixture_or_demo(&self) -> bool {
        self.fixture || self.namespace == DatasetNamespace::Demo
    }
}

/// Import one `ANAV1:` text.
pub fn import_text(text: &str, options: &ImportOptions) -> Result<ImportResult> {
    normalize(anav1::decode_text(text)?, options)
}

/// Import from the text of a SavedVariables file. The file is parsed as data
/// within bounds, the export string is read at `location`, and it then follows
/// the same `ANAV1:` path as [`import_text`].
pub fn import_saved_variables(
    source: &str,
    location: &ExportLocation,
    options: &ImportOptions,
) -> Result<ImportResult> {
    let file = savedvars::parse(source)?;
    let export = savedvars::extract_export(&file, &location.global, &location.key)?;
    normalize(anav1::decode_text(export)?, options)
}

/// Import one explicitly named regular file. The size is checked before and
/// while reading, directories and symbolic links are refused, and nothing is
/// scanned recursively. Content must be UTF-8 without a byte order mark.
pub fn import_file(
    path: &Path,
    format: &FileFormat,
    options: &ImportOptions,
) -> Result<ImportResult> {
    let limit = match format {
        FileFormat::Anav1Text => MAX_TEXT_BYTES,
        FileFormat::SavedVariables(_) => MAX_FILE_BYTES,
    };
    let text = read_bounded(path, limit)?;
    match format {
        FileFormat::Anav1Text => import_text(&text, options),
        FileFormat::SavedVariables(location) => import_saved_variables(&text, location, options),
    }
}

pub(crate) fn read_bounded(path: &Path, limit: usize) -> Result<String> {
    let meta =
        std::fs::symlink_metadata(path).map_err(|_| unreadable("Import file cannot be read"))?;
    if !meta.file_type().is_file() {
        return Err(invalid("Import path is not a regular file"));
    }
    if meta.len() > limit as u64 {
        return Err(too_large("Import file exceeds its size limit"));
    }
    let file = std::fs::File::open(path).map_err(|_| unreadable("Import file cannot be read"))?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unreadable("Import file cannot be read"))?;
    if bytes.len() > limit {
        return Err(too_large("Import file exceeds its size limit"));
    }
    String::from_utf8(bytes).map_err(|_| invalid("Import file is not valid UTF-8"))
}

/// The single validation and normalization function. Order: transport
/// ceilings (size, shape, JSON nesting), accepted-kind policy, core contract
/// decoding and validation, then the summary.
pub fn normalize(document: &str, options: &ImportOptions) -> Result<ImportResult> {
    anav1::check_document(document)?;
    #[derive(Deserialize)]
    struct KindPeek {
        kind: Option<String>,
    }
    // A malformed document falls through so the core decoder reports it.
    if let Ok(KindPeek { kind: Some(kind) }) = serde_json::from_str(document)
        && let Some(known) = ContractKind::ALL.into_iter().find(|k| k.as_str() == kind)
        && !options.accepted_kinds.contains(&known)
    {
        return Err(invalid("Exchange kind is not accepted by this transport"));
    }
    let exchange = decode_any(document)?;
    if !options.accepted_kinds.contains(&exchange.kind()) {
        return Err(invalid("Exchange kind is not accepted by this transport"));
    }
    Ok(summarize(exchange, options))
}

struct Facts {
    fixture: bool,
    namespace: DatasetNamespace,
    build: BuildKey,
    component: ProducerComponent,
    session_id: Option<SessionId>,
    sequence: Option<Sequence>,
    created_at_utc_ms: Option<UtcMillis>,
}
fn facts<P: Payload>(envelope: &EnvelopeData<P>) -> Facts {
    Facts {
        fixture: envelope.fixture,
        namespace: envelope.namespace,
        build: envelope.game.build_key(),
        component: envelope.producer.component,
        session_id: envelope.producer.session_id.clone(),
        sequence: envelope.producer.sequence,
        created_at_utc_ms: envelope.provenance.created_at_utc_ms,
    }
}

fn summarize(exchange: AnyExchange, options: &ImportOptions) -> ImportResult {
    let summary = |snapshot_id: &SnapshotId, capture: &ObservationMetadata| SnapshotSummary {
        snapshot_id: snapshot_id.clone(),
        freshness: freshness(capture, &options.freshness),
        observed_at_utc_ms: capture.observed_at_utc_ms,
        capture_session_id: capture.session_id.clone(),
        capture_sequence: capture.sequence,
    };
    let mut snapshots = BTreeMap::new();
    let facts = match &exchange {
        AnyExchange::CharacterSnapshot(envelope) => {
            let envelope: &Envelope<CharacterSnapshot> = envelope;
            let snapshot = &envelope.payload;
            snapshots.insert(
                snapshot.character.clone(),
                summary(&snapshot.id, &snapshot.capture),
            );
            facts(envelope)
        }
        AnyExchange::PartySnapshot(envelope) => {
            let envelope: &Envelope<PartySnapshot> = envelope;
            for participant in &envelope.payload.participants {
                snapshots.insert(
                    participant.character.clone(),
                    summary(&participant.id, &participant.capture),
                );
            }
            facts(envelope)
        }
        AnyExchange::RoutePlan(envelope) => facts(&***envelope),
        AnyExchange::EvidenceBundle(envelope) => facts(&***envelope),
        AnyExchange::ReferenceDataset(envelope) => facts(&***envelope),
        AnyExchange::GearRecommendations(envelope) => facts(&***envelope),
    };
    ImportResult {
        kind: exchange.kind(),
        fixture: facts.fixture,
        namespace: facts.namespace,
        build: facts.build,
        producer_component: facts.component,
        producer_session_id: facts.session_id,
        producer_sequence: facts.sequence,
        created_at_utc_ms: facts.created_at_utc_ms,
        snapshots,
        exchange,
    }
}
