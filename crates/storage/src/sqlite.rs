//! `SqliteStore`: the SQLite implementation of `SnapshotStore`.
use crate::{
    error::{SqliteResult, invalid, storage},
    hash::snapshot_content_hash,
    migrations,
    model::*,
    ordering::{Recency, compare_capture},
};
use azeroth_core::{
    contract::{Envelope, decode_envelope},
    entities::{CharacterSnapshot, CharacterSnapshotData, SnapshotRef},
    error::{AppError, Result},
    observation::{ClockTrust, Observation, ObservationMetadata, ObservationStatus, StateSource},
    types::{
        BuildId, BuildKey, CharacterId, DataContext, DatasetNamespace, EditionId, InterfaceId,
        Millis, PreferenceId, Sequence, SessionId, Sha256Hex, SnapshotId, UtcMillis,
    },
};
use rusqlite::{Connection, OptionalExtension, Row, Transaction, TransactionBehavior, params};
use serde::{Serialize, de::DeserializeOwned};
use std::{collections::BTreeMap, path::Path, time::Duration};

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// A single-connection SQLite store. Foreign keys are enforced on the
/// connection and verified when it opens.
pub struct SqliteStore {
    pub(crate) conn: Connection,
}

impl SqliteStore {
    /// Opens (creating if needed) the database at `path` and migrates it.
    pub fn open(path: &Path) -> Result<Self> {
        Self::from_connection(Connection::open(path).db()?)
    }

    /// An in-memory database, migrated to the latest version.
    pub fn open_in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory().db()?)
    }

    fn from_connection(mut conn: Connection) -> Result<Self> {
        conn.busy_timeout(BUSY_TIMEOUT).db()?;
        // Must precede migrations: the pragma is a no-op inside a transaction.
        conn.pragma_update(None, "foreign_keys", true).db()?;
        let store_enabled: i64 = conn
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .db()?;
        if store_enabled != 1 {
            return Err(storage("Foreign key enforcement is unavailable"));
        }
        migrations::migrate(&mut conn)?;
        Ok(Self { conn })
    }

    /// Reads only a snapshot belonging to the exact profile context/fixture key.
    pub fn historical_snapshot(
        &self,
        key: &ProfileKey,
        selected: &SnapshotId,
    ) -> Result<Option<Envelope<CharacterSnapshot>>> {
        let Some((profile, _)) = find_profile(&self.conn, key)? else {
            return Ok(None);
        };
        let document: Option<String> = self
            .conn
            .query_row(
                "SELECT document FROM snapshots WHERE profile_id = ?1 AND snapshot_id = ?2",
                params![profile, selected.as_str()],
                |row| row.get(0),
            )
            .optional()
            .db()?;
        document.map(|s| decode_envelope(&s)).transpose()
    }

    /// Explicit compare-and-swap promotion. The immediate transaction excludes
    /// concurrent imports between validation, selection and the bounded audit.
    pub fn promote_snapshot(
        &mut self,
        key: &ProfileKey,
        selected: &SnapshotId,
        expected_current: &SnapshotId,
        reason: &str,
        now: UtcMillis,
    ) -> Result<()> {
        if reason.trim().is_empty() || reason.len() > 1024 {
            return Err(invalid("Promotion reason must contain 1–1024 bytes"));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let Some((profile, Some(current))) = find_profile(&tx, key)? else {
            return Err(invalid("Profile has no current snapshot"));
        };
        let current_document: String = tx
            .query_row(
                "SELECT document FROM snapshots WHERE id = ?1",
                [current],
                |r| r.get(0),
            )
            .db()?;
        let previous = decode_envelope::<CharacterSnapshot>(&current_document)?;
        if previous.payload.id != *expected_current {
            return Err(invalid("Current snapshot changed; preview again"));
        }
        if selected == expected_current {
            return Err(invalid("Snapshot is already current"));
        }
        let row: Option<(i64, String)> = tx
            .query_row(
                "SELECT id, document FROM snapshots WHERE profile_id = ?1 AND snapshot_id = ?2",
                params![profile, selected.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .db()?;
        let (row, document) =
            row.ok_or_else(|| invalid("Snapshot is absent from this profile history"))?;
        let chosen = decode_envelope::<CharacterSnapshot>(&document)?;
        if compare_capture(&chosen.payload.capture, &previous.payload.capture) == Recency::Older {
            return Err(invalid("A provably older snapshot cannot be promoted"));
        }
        tx.execute(
            "UPDATE profiles SET current_snapshot_id = ?1 WHERE id = ?2",
            params![row, profile],
        )
        .db()?;
        tx.execute("INSERT INTO snapshot_promotions (profile_id, previous_snapshot_id, selected_snapshot_id, reason, operated_at_utc_ms) VALUES (?1, ?2, ?3, ?4, ?5)", params![profile, expected_current.as_str(), selected.as_str(), reason, now.get()]).db()?;
        tx.execute("DELETE FROM snapshot_promotions WHERE id NOT IN (SELECT id FROM snapshot_promotions ORDER BY id DESC LIMIT 256)", []).db()?;
        tx.commit().db()
    }

    /// Whether foreign key enforcement is on for this connection.
    pub fn foreign_keys_enabled(&self) -> Result<bool> {
        let value: i64 = self
            .conn
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .db()?;
        Ok(value == 1)
    }

    /// The schema version recorded in the database.
    pub fn schema_version(&self) -> Result<u32> {
        migrations::current_version(&self.conn)
    }
}

fn enum_text<T: Serialize>(value: &T) -> Result<String> {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => Ok(text),
        _ => Err(storage("Value cannot be stored")),
    }
}
fn parse_enum<T: DeserializeOwned>(text: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(text.to_owned()))
        .map_err(|_| storage("Stored value is invalid"))
}
fn int(value: u64) -> Result<i64> {
    i64::try_from(value).map_err(|_| storage("Value is out of range"))
}
fn bad_stored<E>(_: E) -> AppError {
    storage("Stored value is invalid")
}

fn namespace_text(namespace: DatasetNamespace) -> &'static str {
    match namespace {
        DatasetNamespace::Demo => "demo",
        DatasetNamespace::Reference => "reference",
    }
}

/// Column values identifying a profile, in the order the SQL binds them.
struct KeyColumns<'a> {
    character: &'a str,
    edition: &'a str,
    build: &'a str,
    interface: &'a str,
    namespace: &'static str,
    fixture: i64,
}
impl<'a> KeyColumns<'a> {
    fn new(key: &'a ProfileKey) -> Self {
        Self {
            character: key.character.as_str(),
            edition: key.context.build.edition.as_str(),
            build: key.context.build.build.as_str(),
            interface: key.context.build.interface.as_str(),
            namespace: namespace_text(key.context.namespace),
            fixture: i64::from(key.fixture),
        }
    }
}

const PROFILE_LOOKUP: &str = "SELECT id, current_snapshot_id FROM profiles
    WHERE character_id = ?1 AND edition = ?2 AND build = ?3
      AND interface = ?4 AND namespace = ?5 AND is_fixture = ?6";

pub(crate) fn find_profile(
    conn: &Connection,
    key: &ProfileKey,
) -> Result<Option<(i64, Option<i64>)>> {
    let c = KeyColumns::new(key);
    conn.prepare_cached(PROFILE_LOOKUP)
        .db()?
        .query_row(
            params![
                c.character,
                c.edition,
                c.build,
                c.interface,
                c.namespace,
                c.fixture
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .db()
}

fn ensure_profile(conn: &Connection, key: &ProfileKey) -> Result<(i64, Option<i64>)> {
    if let Some(found) = find_profile(conn, key)? {
        return Ok(found);
    }
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM profiles", [], |row| row.get(0))
        .db()?;
    if usize::try_from(count).unwrap_or(usize::MAX) >= MAX_PROFILES {
        return Err(invalid("Profile limit reached"));
    }
    let c = KeyColumns::new(key);
    conn.execute(
        "INSERT INTO profiles (character_id, edition, build, interface, namespace, is_fixture)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            c.character,
            c.edition,
            c.build,
            c.interface,
            c.namespace,
            c.fixture
        ],
    )
    .db()?;
    Ok((conn.last_insert_rowid(), None))
}

/// Capture columns as stored; converted back through the core constructors.
struct CaptureColumns {
    source: String,
    clock_trust: String,
    observed_at: Option<i64>,
    session_id: Option<String>,
    elapsed_ms: Option<i64>,
    sequence: Option<i64>,
}
const CAPTURE_SELECT: &str = "capture_source, capture_clock_trust, capture_observed_at_utc_ms,
    capture_session_id, capture_session_elapsed_ms, capture_sequence";
impl CaptureColumns {
    fn read(row: &Row, first: usize) -> rusqlite::Result<Self> {
        Ok(Self {
            source: row.get(first)?,
            clock_trust: row.get(first + 1)?,
            observed_at: row.get(first + 2)?,
            session_id: row.get(first + 3)?,
            elapsed_ms: row.get(first + 4)?,
            sequence: row.get(first + 5)?,
        })
    }
    fn into_metadata(self) -> Result<ObservationMetadata> {
        Ok(ObservationMetadata {
            source: parse_enum::<StateSource>(&self.source)?,
            clock_trust: parse_enum::<ClockTrust>(&self.clock_trust)?,
            observed_at_utc_ms: self.observed_at.map(UtcMillis::new).transpose()?,
            received_at_utc_ms: None,
            session_id: self.session_id.map(SessionId::new).transpose()?,
            session_elapsed_ms: self
                .elapsed_ms
                .map(|v| Millis::new(u64::try_from(v).map_err(bad_stored)?))
                .transpose()?,
            sequence: self
                .sequence
                .map(|v| Sequence::new(u64::try_from(v).map_err(bad_stored)?))
                .transpose()?,
        })
    }
}

/// One queryable observation status.
struct ObservationRow {
    field: &'static str,
    entry_key: String,
    sub_key: String,
    status: ObservationStatus,
    metadata: ObservationMetadata,
}
fn observation_rows(snapshot: &CharacterSnapshotData) -> Result<Vec<ObservationRow>> {
    let mut rows = Vec::new();
    fn push<T>(
        rows: &mut Vec<ObservationRow>,
        field: &'static str,
        entry_key: &str,
        sub_key: &str,
        observation: &Observation<T>,
    ) -> Result<()> {
        if rows.len() >= MAX_OBSERVATION_ROWS {
            return Err(invalid("Snapshot has too many observations"));
        }
        rows.push(ObservationRow {
            field,
            entry_key: entry_key.to_owned(),
            sub_key: sub_key.to_owned(),
            status: observation.status(),
            metadata: observation.metadata().clone(),
        });
        Ok(())
    }
    push(&mut rows, "level", "", "", &snapshot.level)?;
    push(&mut rows, "role", "", "", &snapshot.role)?;
    push(&mut rows, "location", "", "", &snapshot.location)?;
    for (id, observation) in snapshot.preferences.iter() {
        push(&mut rows, "preferences", id.as_str(), "", observation)?;
    }
    for (id, observation) in snapshot.equipment.iter() {
        push(&mut rows, "equipment", id.as_str(), "", observation)?;
    }
    for (id, observation) in snapshot.objectives.iter() {
        push(&mut rows, "objectives", id.as_str(), "", observation)?;
    }
    for (id, quest) in snapshot.quests.iter() {
        let quest_id = id.as_str();
        push(&mut rows, "quest_accepted", quest_id, "", &quest.accepted)?;
        push(&mut rows, "quest_completed", quest_id, "", &quest.completed)?;
        push(&mut rows, "quest_turned_in", quest_id, "", &quest.turned_in)?;
        for (objective, observation) in quest.objectives.iter() {
            push(
                &mut rows,
                "quest_objectives",
                quest_id,
                objective.as_str(),
                observation,
            )?;
        }
    }
    Ok(rows)
}

/// Everything an import needs, computed before the write transaction opens.
struct PreparedImport {
    key: ProfileKey,
    hash: Sha256Hex,
    document: String,
    capture: ObservationMetadata,
    observations: Vec<ObservationRow>,
}
impl PreparedImport {
    fn new(envelope: &Envelope<CharacterSnapshot>) -> Result<Self> {
        Ok(Self {
            key: ProfileKey::of(envelope),
            hash: snapshot_content_hash(envelope)?,
            document: serde_json::to_string(envelope)
                .map_err(|_| storage("Snapshot cannot be serialized"))?,
            capture: envelope.payload.capture.clone(),
            observations: observation_rows(&envelope.payload)?,
        })
    }
}

fn insert_import(
    tx: &Transaction,
    source: ImportSource,
    status: &str,
    error: Option<&AppError>,
    snapshot_row: Option<i64>,
    received_at: Option<UtcMillis>,
) -> Result<()> {
    let code = error.map(|e| enum_text(&e.code)).transpose()?;
    let message = error.map(ToString::to_string);
    tx.execute(
        "INSERT INTO imports (source_kind, status, error_code, message, snapshot_id, received_at_utc_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            source.as_str(),
            status,
            code,
            message,
            snapshot_row,
            received_at.map(UtcMillis::get)
        ],
    )
    .db()?;
    tx.execute(
        "DELETE FROM imports WHERE id NOT IN (SELECT id FROM imports ORDER BY id DESC LIMIT ?1)",
        [MAX_IMPORT_ROWS as i64],
    )
    .db()?;
    Ok(())
}

impl SqliteStore {
    fn import_in_transaction(
        &mut self,
        prepared: &PreparedImport,
        envelope: &Envelope<CharacterSnapshot>,
        source: ImportSource,
        received_at: Option<UtcMillis>,
    ) -> Result<ImportOutcome> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let (profile, current) = ensure_profile(&tx, &prepared.key)?;
        let hash = prepared.hash.as_str();

        let duplicate: Option<i64> = tx
            .query_row(
                "SELECT id FROM snapshots WHERE profile_id = ?1 AND content_sha256 = ?2",
                params![profile, hash],
                |row| row.get(0),
            )
            .optional()
            .db()?;
        if let Some(existing) = duplicate {
            insert_import(&tx, source, "duplicate", None, Some(existing), received_at)?;
            tx.commit().db()?;
            return Ok(ImportOutcome::Duplicate);
        }

        // The same snapshot ID with different content is never last-writer-wins.
        let id_taken: Option<i64> = tx
            .query_row(
                "SELECT id FROM snapshots WHERE profile_id = ?1 AND snapshot_id = ?2",
                params![profile, envelope.payload.id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .db()?;
        if id_taken.is_some() {
            return Err(invalid(
                "Snapshot ID is already stored with different content",
            ));
        }

        let row = insert_snapshot(&tx, profile, prepared, envelope, received_at)?;

        let outcome = match current {
            None => ImportOutcome::Stored {
                previous_current: None,
            },
            Some(current_row) => {
                let (current_id, current_capture) = tx
                    .query_row(
                        &format!(
                            "SELECT snapshot_id, {CAPTURE_SELECT} FROM snapshots WHERE id = ?1"
                        ),
                        [current_row],
                        |r| Ok((r.get::<_, String>(0)?, CaptureColumns::read(r, 1)?)),
                    )
                    .db()?;
                match compare_capture(&prepared.capture, &current_capture.into_metadata()?) {
                    Recency::Newer => ImportOutcome::Stored {
                        previous_current: Some(SnapshotId::new(current_id)?),
                    },
                    Recency::Older => ImportOutcome::StoredHistoryOnly {
                        reason: HistoryReason::Older,
                    },
                    Recency::Same => ImportOutcome::StoredHistoryOnly {
                        reason: HistoryReason::SamePosition,
                    },
                    Recency::Incomparable => ImportOutcome::StoredHistoryOnly {
                        reason: HistoryReason::Incomparable,
                    },
                }
            }
        };
        let status = match outcome {
            ImportOutcome::Stored { .. } => {
                tx.execute(
                    "UPDATE profiles SET current_snapshot_id = ?1 WHERE id = ?2",
                    params![row, profile],
                )
                .db()?;
                "stored"
            }
            _ => "history_only",
        };
        prune_history(&tx, profile)?;
        insert_import(&tx, source, status, None, Some(row), received_at)?;
        tx.commit().db()?;
        Ok(outcome)
    }
}

fn insert_snapshot(
    tx: &Transaction,
    profile: i64,
    prepared: &PreparedImport,
    envelope: &Envelope<CharacterSnapshot>,
    received_at: Option<UtcMillis>,
) -> Result<i64> {
    let producer = &envelope.producer;
    let capture = &prepared.capture;
    tx.execute(
        "INSERT INTO snapshots (
            profile_id, snapshot_id, content_sha256,
            producer_component, producer_version, producer_session_id, producer_sequence,
            game_locale, source_revision,
            capture_source, capture_clock_trust, capture_observed_at_utc_ms,
            capture_session_id, capture_session_elapsed_ms, capture_sequence,
            received_at_utc_ms, document)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            profile,
            envelope.payload.id.as_str(),
            prepared.hash.as_str(),
            enum_text(&producer.component)?,
            producer.version.as_str(),
            producer.session_id.as_ref().map(SessionId::as_str),
            producer.sequence.map(|s| int(s.get())).transpose()?,
            envelope.game.locale.as_str(),
            envelope.payload.source_revision.as_str(),
            enum_text(&capture.source)?,
            enum_text(&capture.clock_trust)?,
            capture.observed_at_utc_ms.map(UtcMillis::get),
            capture.session_id.as_ref().map(SessionId::as_str),
            capture
                .session_elapsed_ms
                .map(|m| int(m.get()))
                .transpose()?,
            capture.sequence.map(|s| int(s.get())).transpose()?,
            received_at.map(UtcMillis::get),
            prepared.document,
        ],
    )
    .db()?;
    let row = tx.last_insert_rowid();
    let mut statement = tx
        .prepare_cached(
            "INSERT INTO snapshot_observations
                (snapshot_id, field, entry_key, sub_key, status, clock_trust, observed_at_utc_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .db()?;
    for o in &prepared.observations {
        statement
            .execute(params![
                row,
                o.field,
                o.entry_key,
                o.sub_key,
                enum_text(&o.status)?,
                enum_text(&o.metadata.clock_trust)?,
                o.metadata.observed_at_utc_ms.map(UtcMillis::get),
            ])
            .db()?;
    }
    Ok(row)
}

/// Keeps the newest `MAX_SNAPSHOTS_PER_PROFILE` snapshots (by insertion) and
/// never deletes the current one. Observation rows go with them (cascade).
fn prune_history(tx: &Transaction, profile: i64) -> Result<()> {
    tx.execute(
        "DELETE FROM snapshots
         WHERE profile_id = ?1
           AND id NOT IN (SELECT id FROM snapshots WHERE profile_id = ?1 ORDER BY id DESC LIMIT ?2)
           AND id IS NOT (SELECT current_snapshot_id FROM profiles WHERE id = ?1)",
        params![profile, MAX_SNAPSHOTS_PER_PROFILE as i64],
    )
    .db()?;
    Ok(())
}

fn key_from_row(row: &Row, first: usize) -> rusqlite::Result<[String; 5]> {
    Ok([
        row.get(first)?,
        row.get(first + 1)?,
        row.get(first + 2)?,
        row.get(first + 3)?,
        row.get(first + 4)?,
    ])
}
fn build_key(columns: [String; 5], fixture: i64) -> Result<ProfileKey> {
    let [character, edition, build, interface, namespace] = columns;
    Ok(ProfileKey {
        character: CharacterId::new(character)?,
        context: DataContext {
            build: BuildKey {
                edition: EditionId::new(edition)?,
                build: BuildId::new(build)?,
                interface: InterfaceId::new(interface)?,
            },
            namespace: parse_enum(&namespace)?,
        },
        fixture: fixture != 0,
    })
}

impl SnapshotStore for SqliteStore {
    fn import_character_snapshot(
        &mut self,
        envelope: &Envelope<CharacterSnapshot>,
        source: ImportSource,
        received_at: Option<UtcMillis>,
    ) -> Result<ImportOutcome> {
        let result = PreparedImport::new(envelope).and_then(|prepared| {
            self.import_in_transaction(&prepared, envelope, source, received_at)
        });
        if let Err(error) = &result {
            // The import transaction was rolled back; the audit row is written
            // separately and is best effort (the original error is returned).
            let _ = self.record_failed_import(source, error, received_at);
        }
        result
    }

    fn record_failed_import(
        &mut self,
        source: ImportSource,
        error: &AppError,
        received_at: Option<UtcMillis>,
    ) -> Result<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        insert_import(&tx, source, "failed", Some(error), None, received_at)?;
        tx.commit().db()
    }

    fn current_snapshot(&self, key: &ProfileKey) -> Result<Option<Envelope<CharacterSnapshot>>> {
        let Some((_, Some(current))) = find_profile(&self.conn, key)? else {
            return Ok(None);
        };
        let document: String = self
            .conn
            .query_row(
                "SELECT document FROM snapshots WHERE id = ?1",
                [current],
                |row| row.get(0),
            )
            .db()?;
        // Stored data is validated again; the contract validator is never bypassed.
        decode_envelope::<CharacterSnapshot>(&document)
            .map(Some)
            .map_err(|_| storage("Stored snapshot failed validation"))
    }

    fn snapshot_history(&self, key: &ProfileKey, limit: usize) -> Result<Vec<SnapshotSummary>> {
        if limit == 0 || limit > MAX_HISTORY_LIMIT {
            return Err(invalid("History limit is out of range"));
        }
        let Some((profile, current)) = find_profile(&self.conn, key)? else {
            return Ok(Vec::new());
        };
        let mut statement = self
            .conn
            .prepare_cached(&format!(
                "SELECT id, snapshot_id, content_sha256, received_at_utc_ms, {CAPTURE_SELECT}
                 FROM snapshots WHERE profile_id = ?1 ORDER BY id DESC LIMIT ?2"
            ))
            .db()?;
        let rows = statement
            .query_map(params![profile, limit as i64], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    CaptureColumns::read(row, 4)?,
                ))
            })
            .db()?;
        let mut summaries = Vec::new();
        for row in rows {
            let (id, snapshot, hash, received, capture) = row.db()?;
            summaries.push(SnapshotSummary {
                snapshot: SnapshotId::new(snapshot)?,
                content_sha256: Sha256Hex::new(hash)?,
                is_current: current == Some(id),
                capture: capture.into_metadata()?,
                received_at_utc_ms: received.map(UtcMillis::new).transpose()?,
            });
        }
        Ok(summaries)
    }

    fn list_profiles(&self) -> Result<Vec<ProfileSummary>> {
        let mut statement = self
            .conn
            .prepare_cached(
                "SELECT p.character_id, p.edition, p.build, p.interface, p.namespace, p.is_fixture,
                        c.snapshot_id,
                        (SELECT COUNT(*) FROM snapshots s WHERE s.profile_id = p.id)
                 FROM profiles p LEFT JOIN snapshots c ON c.id = p.current_snapshot_id
                 ORDER BY p.character_id, p.edition, p.build, p.interface, p.namespace, p.is_fixture
                 LIMIT ?1",
            )
            .db()?;
        let rows = statement
            .query_map([MAX_PROFILES as i64], |row| {
                Ok((
                    key_from_row(row, 0)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            })
            .db()?;
        let mut profiles = Vec::new();
        for row in rows {
            let (columns, fixture, current, count) = row.db()?;
            let key = build_key(columns, fixture)?;
            let current = current
                .map(|id| {
                    Ok::<_, AppError>(SnapshotRef {
                        id: SnapshotId::new(id)?,
                        character: key.character.clone(),
                        context: key.context.clone(),
                    })
                })
                .transpose()?;
            profiles.push(ProfileSummary {
                key,
                current,
                snapshot_count: u32::try_from(count).map_err(bad_stored)?,
            });
        }
        Ok(profiles)
    }

    fn set_preference(
        &mut self,
        key: &ProfileKey,
        id: &PreferenceId,
        value: &Observation<bool>,
    ) -> Result<()> {
        let document =
            serde_json::to_string(value).map_err(|_| storage("Value cannot be stored"))?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let (profile, _) = ensure_profile(&tx, key)?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM profile_preferences WHERE profile_id = ?1 AND preference_id = ?2)",
                params![profile, id.as_str()],
                |row| row.get(0),
            )
            .db()?;
        if !exists {
            let count: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM profile_preferences WHERE profile_id = ?1",
                    [profile],
                    |row| row.get(0),
                )
                .db()?;
            if usize::try_from(count).unwrap_or(usize::MAX) >= MAX_PREFERENCES_PER_PROFILE {
                return Err(invalid("Preference limit reached"));
            }
        }
        tx.execute(
            "INSERT INTO profile_preferences (profile_id, preference_id, status, value, document)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (profile_id, preference_id)
             DO UPDATE SET status = excluded.status, value = excluded.value, document = excluded.document",
            params![
                profile,
                id.as_str(),
                enum_text(&value.status())?,
                value.value().map(|v| i64::from(*v)),
                document
            ],
        )
        .db()?;
        tx.commit().db()
    }

    fn preferences(&self, key: &ProfileKey) -> Result<BTreeMap<PreferenceId, Observation<bool>>> {
        let Some((profile, _)) = find_profile(&self.conn, key)? else {
            return Ok(BTreeMap::new());
        };
        let mut statement = self
            .conn
            .prepare_cached(
                "SELECT preference_id, document FROM profile_preferences
                 WHERE profile_id = ?1 ORDER BY preference_id LIMIT ?2",
            )
            .db()?;
        let rows = statement
            .query_map(
                params![profile, MAX_PREFERENCES_PER_PROFILE as i64],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .db()?;
        let mut result = BTreeMap::new();
        for row in rows {
            let (id, document) = row.db()?;
            let observation = serde_json::from_str(&document).map_err(bad_stored)?;
            result.insert(PreferenceId::new(id)?, observation);
        }
        Ok(result)
    }

    fn delete_profile(&mut self, key: &ProfileKey) -> Result<bool> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let Some((profile, _)) = find_profile(&tx, key)? else {
            return Ok(false);
        };
        // Snapshots, observations and preferences cascade; audit rows survive
        // with their snapshot reference cleared.
        tx.execute("DELETE FROM profiles WHERE id = ?1", [profile])
            .db()?;
        tx.commit().db()?;
        Ok(true)
    }
}

impl SqliteStore {
    /// Transaction-consistent SQLite backup, including committed WAL pages.
    /// The destination must be new; existing files are never overwritten.
    /// Run on a blocking worker. Busy backups return retryable StorageError.
    pub fn backup_to(&self, path: &Path) -> Result<()> {
        use rusqlite::backup::{Backup, StepResult};
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|_| storage("Backup destination cannot be created"))?;
        drop(file);
        let result = (|| {
            let mut destination = Connection::open(path).db()?;
            let backup = Backup::new(&self.conn, &mut destination).db()?;
            match backup.step(-1).db()? {
                StepResult::Done => Ok(()),
                StepResult::Busy | StepResult::Locked => Err(AppError::new(
                    azeroth_core::error::ErrorCode::StorageError,
                    "Database is busy",
                    true,
                )),
                _ => Err(storage("Backup did not complete")),
            }
        })();
        if result.is_err() {
            // We created this file exclusively; an incomplete backup is not a
            // usable artifact. The source database was never written.
            let _ = std::fs::remove_file(path);
        }
        result
    }
}

pub(crate) fn recovery_profile_key(row: &Row<'_>) -> rusqlite::Result<ProfileKey> {
    build_key(key_from_row(row, 1)?, row.get(6)?).map_err(|_| rusqlite::Error::InvalidQuery)
}
/// Compare every queryable derived snapshot column/observation to a fresh
/// trusted derivation. IDs, profile FK and desktop receipt remain historical.
pub(crate) fn validate_recovery_snapshot(
    conn: &Connection,
    id: i64,
    envelope: &Envelope<CharacterSnapshot>,
) -> Result<()> {
    let prepared = PreparedImport::new(envelope)?;
    let mut expected = Connection::open_in_memory().db()?;
    migrations::migrate_with(&mut expected, &migrations::MIGRATIONS[..1])?;
    expected.pragma_update(None, "foreign_keys", false).db()?;
    let tx = expected.transaction().db()?;
    let row = insert_snapshot(&tx, 1, &prepared, envelope, None)?;
    fn values(conn: &Connection, sql: &str, id: i64) -> Result<Vec<Vec<rusqlite::types::Value>>> {
        let mut q = conn.prepare(sql).db()?;
        let columns = q.column_count();
        q.query_map([id], |r| (0..columns).map(|i| r.get(i)).collect())
            .db()?
            .collect::<rusqlite::Result<_>>()
            .db()
    }
    let sql = "SELECT snapshot_id,content_sha256,producer_component,producer_version,producer_session_id,producer_sequence,game_locale,source_revision,capture_source,capture_clock_trust,capture_observed_at_utc_ms,capture_session_id,capture_session_elapsed_ms,capture_sequence FROM snapshots WHERE id=?1";
    let observations = "SELECT field,entry_key,sub_key,status,clock_trust,observed_at_utc_ms FROM snapshot_observations WHERE snapshot_id=?1 ORDER BY field,entry_key,sub_key";
    if values(conn, sql, id)? != values(&tx, sql, row)?
        || values(conn, observations, id)? != values(&tx, observations, row)?
    {
        return Err(storage("Backup derived snapshot columns disagree"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use azeroth_core::error::ErrorCode;

    #[test]
    fn foreign_keys_are_on_and_reject_orphan_observations() {
        let store = SqliteStore::open_in_memory().unwrap();
        assert!(store.foreign_keys_enabled().unwrap());
        let orphan = store.conn.execute(
            "INSERT INTO snapshot_observations (snapshot_id, field, status, clock_trust)
             VALUES (999, 'level', 'known', 'trusted')",
            [],
        );
        assert!(
            orphan.is_err(),
            "orphan observation must violate the foreign key"
        );
    }

    #[test]
    fn sqlite_failures_map_to_safe_storage_errors() {
        let store = SqliteStore::open_in_memory().unwrap();
        let error = store
            .conn
            .execute("INSERT INTO no_such_table VALUES (1)", [])
            .map_err(crate::error::from_sqlite)
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::StorageError);
        assert!(!error.retryable);
        assert!(!error.to_string().contains("no_such_table"));
    }
}
