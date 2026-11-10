//! Immutable companion persistence. Selection is always tied to a validated
//! persisted character snapshot; manual records occupy a separate origin.
use crate::{
    ProfileKey, SqliteStore,
    error::{SqliteResult, invalid, storage},
    hash::snapshot_content_hash,
    sqlite::find_profile,
};
use azeroth_core::{
    contract::{Envelope, MAX_ENVELOPE_BYTES, decode_envelope},
    entities::CharacterSnapshot,
    error::Result,
    observation::StateSource,
    progression::ProgressionSnapshot,
};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MAX_PROGRESSION_RECORDS: usize = 128;
pub const MAX_PROGRESSION_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressionOrigin {
    Imported,
    Manual,
}
impl ProgressionOrigin {
    fn name(self) -> &'static str {
        match self {
            Self::Imported => "imported",
            Self::Manual => "manual",
        }
    }
}

fn verify_origin(
    envelope: &Envelope<ProgressionSnapshot>,
    origin: ProgressionOrigin,
) -> Result<()> {
    let source = if envelope.fixture {
        StateSource::SyntheticFixture
    } else {
        match origin {
            ProgressionOrigin::Imported => StateSource::AddonSnapshot,
            ProgressionOrigin::Manual => StateSource::ManualEntry,
        }
    };
    if envelope.provenance.source != source || envelope.payload.capture.source != source {
        return Err(invalid(
            "Progression origin disagrees with capture provenance",
        ));
    }
    // Predecessor capture is not a manual edit and retains its original source.
    let mut payload = serde_json::to_value(&envelope.payload)
        .map_err(|_| invalid("Invalid progression payload"))?;
    payload
        .as_object_mut()
        .ok_or_else(|| invalid("Invalid progression payload"))?
        .remove("binding");
    let source = serde_json::to_value(source).map_err(|_| invalid("Invalid progression source"))?;
    fn sources(value: &serde_json::Value, expected: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Object(map) => {
                let source = map.get("source");
                let known = map.get("status").and_then(|v| v.as_str()) == Some("known");
                (!map.contains_key("clock_trust")
                    || source == Some(expected)
                    || (!known && source.and_then(|v| v.as_str()) == Some("unrecorded")))
                    && map.values().all(|v| sources(v, expected))
            }
            serde_json::Value::Array(values) => values.iter().all(|v| sources(v, expected)),
            _ => true,
        }
    }
    if !sources(&payload, &source) {
        return Err(invalid("Progression observations mix capture origins"));
    }
    Ok(())
}

fn validate_binding(
    envelope: &Envelope<ProgressionSnapshot>,
    parent: &str,
    key: &ProfileKey,
) -> Result<()> {
    let parent = decode_envelope::<CharacterSnapshot>(parent)?;
    if &ProfileKey::of(&parent) != key || envelope.fixture != key.fixture {
        return Err(invalid("Progression profile differs from predecessor"));
    }
    envelope
        .payload
        .binding
        .ensure_matches(&parent.payload, &snapshot_content_hash(&parent)?)
}

impl SqliteStore {
    /// Returns false for identical re-import, refuses same-ID replacement.
    /// The immediate transaction excludes a concurrent predecessor selection.
    pub fn save_progression(
        &mut self,
        key: &ProfileKey,
        envelope: &Envelope<ProgressionSnapshot>,
        origin: ProgressionOrigin,
    ) -> Result<bool> {
        let document = serde_json::to_string(envelope)
            .map_err(|_| invalid("Progression cannot be serialized"))?;
        if document.len() > MAX_ENVELOPE_BYTES {
            return Err(invalid("Progression exceeds document capacity"));
        }
        let envelope = decode_envelope::<ProgressionSnapshot>(&document)?;
        verify_origin(&envelope, origin)?;
        let hash = format!("{:x}", Sha256::digest(document.as_bytes()));
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let Some((profile, Some(parent))) = find_profile(&tx, key)? else {
            return Err(invalid(
                "Progression requires a selected character snapshot",
            ));
        };
        let predecessor: String = tx
            .query_row(
                "SELECT document FROM snapshots WHERE id=?1 AND profile_id=?2",
                params![parent, profile],
                |r| r.get(0),
            )
            .db()?;
        validate_binding(&envelope, &predecessor, key)?;
        let previous: Option<(String,String)> = tx.query_row("SELECT content_sha256,document FROM progression_snapshots WHERE profile_id=?1 AND origin=?2 AND companion_id=?3", params![profile,origin.name(),envelope.payload.id.as_str()], |r|Ok((r.get(0)?,r.get(1)?))).optional().db()?;
        if let Some((old_hash, old_document)) = previous {
            if old_hash != hash || old_document != document {
                return Err(invalid(
                    "Progression identity already has different content",
                ));
            }
            return Ok(false);
        }
        let (count, bytes): (i64,i64) = tx.query_row("SELECT count(*),coalesce(sum(length(CAST(document AS BLOB))),0) FROM progression_snapshots WHERE profile_id=?1", [profile], |r|Ok((r.get(0)?,r.get(1)?))).db()?;
        if count >= MAX_PROGRESSION_RECORDS as i64
            || bytes + document.len() as i64 > MAX_PROGRESSION_BYTES as i64
        {
            return Err(invalid("Progression profile capacity exceeded"));
        }
        tx.execute("INSERT INTO progression_snapshots(profile_id,snapshot_row_id,origin,companion_id,content_sha256,document) VALUES(?1,?2,?3,?4,?5,?6)",params![profile,parent,origin.name(),envelope.payload.id.as_str(),hash,document]).db()?;
        tx.commit().db()?;
        Ok(true)
    }

    /// All captures for the current predecessor, ordered by ID, not presumed
    /// recency. Callers must evaluate freshness before selecting observations.
    pub fn current_progression(
        &self,
        key: &ProfileKey,
        origin: ProgressionOrigin,
    ) -> Result<Vec<Envelope<ProgressionSnapshot>>> {
        // Reuse a caller-owned consistent read (for example with_immediate_read)
        // so snapshot and companion queries can share one database view.
        if !self.conn.is_autocommit() {
            return self.read_progression(key, Some(origin));
        }
        let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Deferred).db()?;
        let result = self.read_progression(key, Some(origin));
        tx.commit().db()?;
        result
    }

    /// With no origin filter, validate ALL history for recovery, not just current.
    pub(crate) fn read_progression(
        &self,
        key: &ProfileKey,
        origin: Option<ProgressionOrigin>,
    ) -> Result<Vec<Envelope<ProgressionSnapshot>>> {
        let Some((profile, current)) = find_profile(&self.conn, key)? else {
            return Ok(Vec::new());
        };
        let mut statement = self.conn.prepare("SELECT p.origin,p.companion_id,p.content_sha256,p.document,s.document FROM progression_snapshots p JOIN snapshots s ON s.id=p.snapshot_row_id WHERE p.profile_id=?1 AND (?2 IS NULL OR (p.origin=?2 AND p.snapshot_row_id=?3)) ORDER BY p.origin,p.companion_id LIMIT ?4").db()?;
        let mut rows = statement
            .query(params![
                profile,
                origin.map(ProgressionOrigin::name),
                current,
                MAX_PROGRESSION_RECORDS as i64 + 1
            ])
            .db()?;
        let mut result = Vec::new();
        let mut bytes = 0;
        while let Some(row) = rows.next().db()? {
            let name: String = row.get(0).db()?;
            let stored_origin = match name.as_str() {
                "manual" => ProgressionOrigin::Manual,
                "imported" => ProgressionOrigin::Imported,
                _ => return Err(storage("Invalid stored progression origin")),
            };
            let id: String = row.get(1).db()?;
            let hash: String = row.get(2).db()?;
            let document: String = row.get(3).db()?;
            bytes += document.len();
            if result.len() >= MAX_PROGRESSION_RECORDS
                || bytes > MAX_PROGRESSION_BYTES
                || document.len() > MAX_ENVELOPE_BYTES
            {
                return Err(storage("Stored progression exceeds capacity"));
            }
            if format!("{:x}", Sha256::digest(document.as_bytes())) != hash {
                return Err(storage("Stored progression hash disagrees"));
            }
            let envelope = decode_envelope::<ProgressionSnapshot>(&document)?;
            if envelope.payload.id.as_str() != id {
                return Err(storage("Stored progression identity disagrees"));
            }
            verify_origin(&envelope, stored_origin)?;
            validate_binding(&envelope, &row.get::<_, String>(4).db()?, key)?;
            result.push(envelope);
        }
        Ok(result)
    }
}
