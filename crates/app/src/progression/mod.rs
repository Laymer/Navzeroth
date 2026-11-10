//! Explicit profile-bound companion actions. No game I/O or state automation.
mod dto;
mod manual;
use crate::snapshots::{SnapshotApplication, context};
use azeroth_core::{
    contract::*,
    entities::{CharacterSnapshot, SnapshotRef},
    error::{AppError, ErrorCode, Result},
    observation::{ClockTrust, ObservationMetadata, StateSource, freshness},
    progression::SnapshotBinding,
    types::*,
};
use azeroth_ingest::import::{ImportOptions, import_text};
use azeroth_storage::{
    ProfileKey, SnapshotStore, SqliteStore, progression::ProgressionOrigin, snapshot_content_hash,
};
pub use dto::*;
use serde::de::DeserializeOwned;
use std::collections::BTreeSet;

pub const MAX_PROGRESSION_REQUEST: usize = 8 * 1024 * 1024;
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
fn parse<T: DeserializeOwned>(text: &str) -> Result<T> {
    if text.len() > MAX_PROGRESSION_REQUEST {
        return Err(AppError::new(
            ErrorCode::ImportTooLarge,
            "Progression request exceeds capacity",
            false,
        ));
    }
    if !azeroth_ingest::anav1::json_depth_within(text, 32) {
        return Err(invalid("Progression request exceeds nesting limit"));
    }
    let value = azeroth_core::strict_json::value(text.as_bytes(), MAX_PROGRESSION_REQUEST)
        .map_err(|_| invalid("Invalid progression request"))?;
    serde_json::from_value(value).map_err(|_| invalid("Invalid progression request"))
}
fn parent(store: &SqliteStore, key: &ProfileKey) -> Result<Envelope<CharacterSnapshot>> {
    store
        .current_snapshot(key)?
        .ok_or_else(|| invalid("Progression requires a current character snapshot"))
}
fn expected(parent: &Envelope<CharacterSnapshot>, expected: &SnapshotBinding) -> Result<()> {
    expected
        .ensure_matches(&parent.payload, &snapshot_content_hash(parent)?)
        .map_err(|_| {
            AppError::new(
                ErrorCode::SnapshotOutdated,
                "Selected character snapshot changed; reload progression inputs",
                false,
            )
        })
}
fn receipt(
    profile_id: &str,
    parent: &Envelope<CharacterSnapshot>,
    id: SnapshotId,
    stored: bool,
) -> ProgressionReceipt {
    ProgressionReceipt {
        profile_id: profile_id.into(),
        snapshot_id: parent.payload.id.clone(),
        companion_id: id,
        stored,
        fixture: parent.fixture,
    }
}
impl SnapshotApplication {
    pub fn progression_view(&self, id: &str, now: Option<UtcMillis>) -> Result<ProgressionView> {
        self.with_profile_store_mut(id, |store, key| {
            store.with_immediate_read(|store| {
                let parent = parent(store, key)?;
                let records = |origin| {
                    store
                        .current_progression(key, origin)?
                        .into_iter()
                        .map(|e| {
                            Ok(ProgressionRecord {
                                freshness: freshness(&e.payload.capture, &context(now)),
                                snapshot: e.payload.clone(),
                            })
                        })
                        .collect::<Result<Vec<_>>>()
                };
                Ok(ProgressionView {
                    profile_id: id.into(),
                    fixture: key.fixture,
                    binding: SnapshotBinding::from_snapshot(
                        &parent.payload,
                        snapshot_content_hash(&parent)?,
                    ),
                    imported: records(ProgressionOrigin::Imported)?,
                    manual: records(ProgressionOrigin::Manual)?,
                })
            })
        })
    }
    pub fn import_progression_text(
        &self,
        text: &str,
        now: Option<UtcMillis>,
    ) -> Result<ProgressionReceipt> {
        let request: ImportProgressionRequest = parse(text)?;
        let imported = import_text(
            &request.text,
            &ImportOptions {
                accepted_kinds: BTreeSet::from([ContractKind::ProgressionSnapshot]),
                freshness: context(now),
            },
        )?;
        let AnyExchange::ProgressionSnapshot(envelope) = imported.exchange else {
            return Err(invalid("Expected a progression companion"));
        };
        self.with_profile_store_mut(&request.profile_id, |store, key| {
            let parent = parent(store, key)?;
            expected(&parent, &request.expected)?;
            // Require both UI expectation and companion binding to select the same predecessor.
            expected(&parent, &envelope.payload.binding)?;
            let stored = store.save_progression(key, &envelope, ProgressionOrigin::Imported)?;
            Ok(receipt(
                &request.profile_id,
                &parent,
                envelope.payload.id.clone(),
                stored,
            ))
        })
    }
    pub fn save_manual_progression(
        &self,
        text: &str,
        now: Option<UtcMillis>,
    ) -> Result<ProgressionReceipt> {
        let request: ManualProgressionRequest = parse(text)?;
        self.with_profile_store_mut(&request.profile_id, |store, key| {
            let parent = parent(store, key)?;
            expected(&parent, &request.expected)?;
            let source = if parent.fixture {
                StateSource::SyntheticFixture
            } else {
                StateSource::ManualEntry
            };
            let capture = ObservationMetadata {
                source,
                clock_trust: if now.is_some() {
                    ClockTrust::Trusted
                } else {
                    ClockTrust::Uncertain
                },
                observed_at_utc_ms: now,
                ..Default::default()
            };
            let payload = manual::payload(&request, capture.clone())?;
            let envelope = Envelope::try_new(EnvelopeData {
                format: FormatTag,
                kind: KindTag::new(),
                schema_version: SchemaVersion,
                fixture: parent.fixture,
                producer: Producer {
                    component: ProducerComponent::Desktop,
                    version: Text::new(env!("CARGO_PKG_VERSION"))?,
                    session_id: None,
                    sequence: None,
                },
                game: parent.game.clone(),
                namespace: parent.namespace,
                provenance: Provenance {
                    source,
                    clock_trust: capture.clock_trust,
                    created_at_utc_ms: now,
                },
                dependencies: Dependencies {
                    dataset_revision: Some(parent.payload.source_revision.clone()),
                    snapshots: vec![SnapshotRef {
                        id: parent.payload.id.clone(),
                        character: parent.payload.character.clone(),
                        context: parent.payload.context.clone(),
                    }],
                    ..Default::default()
                },
                payload,
                extensions: Default::default(),
            })?;
            let stored = store.save_progression(key, &envelope, ProgressionOrigin::Manual)?;
            Ok(receipt(
                &request.profile_id,
                &parent,
                request.id.clone(),
                stored,
            ))
        })
    }
}
pub fn api_schema_text() -> String {
    let schema = schemars::generate::SchemaSettings::draft2020_12()
        .into_generator()
        .into_root_schema_for::<ProgressionApi>();
    let mut value = serde_json::to_value(schema).expect("schema serializes");
    value["$id"] = serde_json::json!(
        "https://github.com/Laymer/Navzeroth/schemas/v1/progression-api.schema.json"
    );
    azeroth_core::contract::schema::strip_numeric_formats(&mut value);
    format!(
        "{}\n",
        serde_json::to_string_pretty(&value).expect("schema serializes")
    )
}
