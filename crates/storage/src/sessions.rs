//! Immutable session batches. Every insert and capacity/conflict check is atomic.
use crate::{
    ProfileKey, SqliteStore,
    error::{SqliteResult, invalid, storage},
    sqlite::find_profile,
};
use azeroth_core::{
    analytics::{SessionAnalytics, analyze_session},
    error::Result,
    sessions::{SessionBatch, SessionBuildConfig, SessionDocument, build_sessions},
    types::{Millis, SessionId},
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const MAX_SESSIONS_PER_PROFILE: usize = 128;
pub const MAX_SESSION_DOCUMENT_BYTES: usize = 32768;
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SessionImportOutcome {
    pub inserted: u32,
    pub duplicates: u32,
    pub session_ids: Vec<SessionId>,
}
#[derive(Clone, Debug, Serialize)]
pub struct StoredSession {
    pub fixture: bool,
    pub batch: SessionBatch,
    pub analytics: SessionAnalytics,
}
fn config() -> SessionBuildConfig {
    SessionBuildConfig {
        max_observed_span_ms: Millis::new(300_000).expect("constant"),
        clock_tolerance_ms: Millis::new(5_000).expect("constant"),
    }
}
fn encode<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(|_| storage("Session document cannot be encoded"))
}
fn hash(document: &str) -> String {
    format!("{:x}", Sha256::digest(document.as_bytes()))
}
fn validate(
    document: &SessionDocument,
    key: &ProfileKey,
) -> Result<BTreeMap<SessionId, (String, String)>> {
    document.validate_identity()?;
    if document.fixture != key.fixture
        || document.batch.character != key.character
        || document.batch.context != key.context
    {
        return Err(invalid("Session identity does not match selected profile"));
    }
    if encode(document)?.len() > MAX_SESSION_DOCUMENT_BYTES {
        return Err(invalid("Session document exceeds 32 KiB"));
    }
    let builds = build_sessions(&document.batch, &config())?;
    builds
        .into_iter()
        .map(|(id, build)| {
            let mut events = document
                .batch
                .events
                .iter()
                .filter(|e| e.session == id)
                .cloned()
                .collect::<Vec<_>>();
            events.sort_by_key(|e| e.sequence);
            events.dedup();
            let one = SessionDocument {
                batch: SessionBatch {
                    events,
                    ..document.batch.clone()
                },
                ..document.clone()
            };
            Ok((id, (encode(&one)?, encode(&analyze_session(&build)?)?)))
        })
        .collect()
}
impl SqliteStore {
    pub fn import_sessions(
        &mut self,
        key: &ProfileKey,
        document: &SessionDocument,
    ) -> Result<SessionImportOutcome> {
        let records = validate(document, key)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let (profile, _) = find_profile(&tx, key)?
            .ok_or_else(|| invalid("Import a matching character profile first"))?;
        let mut fresh = Vec::new();
        let mut outcome = SessionImportOutcome {
            inserted: 0,
            duplicates: 0,
            session_ids: records.keys().cloned().collect(),
        };
        for (id, (text, summary)) in &records {
            let prior: Option<String> = tx
                .query_row(
                    "SELECT document FROM sessions WHERE profile_id=?1 AND session_id=?2",
                    params![profile, id.as_str()],
                    |r| r.get(0),
                )
                .optional()
                .db()?;
            match prior {
                Some(prior) if prior == *text => outcome.duplicates += 1,
                Some(_) => {
                    return Err(invalid(
                        "Session identity already contains different immutable events",
                    ));
                }
                None => fresh.push((id, text, summary)),
            }
        }
        let count: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE profile_id=?1",
                [profile],
                |r| r.get(0),
            )
            .db()?;
        if count + fresh.len() as i64 > MAX_SESSIONS_PER_PROFILE as i64 {
            return Err(invalid(
                "Session history has reached 128 entries; explicitly delete history before importing",
            ));
        }
        for (id, text, summary) in fresh {
            tx.execute("INSERT INTO sessions(profile_id,session_id,content_sha256,document,summary_document) VALUES (?1,?2,?3,?4,?5)",params![profile,id.as_str(),hash(text),text,summary]).db()?;
            outcome.inserted += 1;
        }
        tx.commit().db()?;
        Ok(outcome)
    }
    pub fn session_ids(&self, key: &ProfileKey) -> Result<Vec<SessionId>> {
        let Some((profile, _)) = find_profile(&self.conn, key)? else {
            return Ok(vec![]);
        };
        let mut query = self
            .conn
            .prepare(
                "SELECT session_id FROM sessions WHERE profile_id=?1 ORDER BY session_id LIMIT ?2",
            )
            .db()?;
        let rows = query
            .query_map(
                params![profile, (MAX_SESSIONS_PER_PROFILE + 1) as i64],
                |r| r.get::<_, String>(0),
            )
            .db()?;
        let ids = rows
            .map(|r| {
                SessionId::new(r.db()?).map_err(|_| storage("Stored session identity is invalid"))
            })
            .collect::<Result<Vec<_>>>()?;
        if ids.len() > MAX_SESSIONS_PER_PROFILE {
            return Err(storage("Stored session history exceeds capacity"));
        }
        Ok(ids)
    }
    pub fn session(&self, key: &ProfileKey, id: &SessionId) -> Result<Option<StoredSession>> {
        let Some((profile, _)) = find_profile(&self.conn, key)? else {
            return Ok(None);
        };
        let row: Option<(String,String,String)> = self.conn.query_row("SELECT document,summary_document,content_sha256 FROM sessions WHERE profile_id=?1 AND session_id=?2",params![profile,id.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().db()?;
        let Some((text, summary, digest)) = row else {
            return Ok(None);
        };
        if text.len() > MAX_SESSION_DOCUMENT_BYTES || hash(&text) != digest {
            return Err(storage("Stored session integrity check failed"));
        }
        let document: SessionDocument = serde_json::from_str(&text)
            .map_err(|_| storage("Stored session document is invalid"))?;
        let records =
            validate(&document, key).map_err(|_| storage("Stored session failed validation"))?;
        if records.len() != 1
            || records
                .get(id)
                .is_none_or(|(canonical, derived)| *canonical != text || *derived != summary)
        {
            return Err(storage(
                "Stored session summary or identity disagrees with events",
            ));
        }
        let build = build_sessions(&document.batch, &config())?
            .remove(id)
            .ok_or_else(|| storage("Stored session build is missing"))?;
        Ok(Some(StoredSession {
            fixture: document.fixture,
            batch: document.batch,
            analytics: analyze_session(&build)?,
        }))
    }
    pub fn delete_session(&mut self, key: &ProfileKey, id: &SessionId) -> Result<bool> {
        let Some((profile, _)) = find_profile(&self.conn, key)? else {
            return Ok(false);
        };
        Ok(self
            .conn
            .execute(
                "DELETE FROM sessions WHERE profile_id=?1 AND session_id=?2",
                params![profile, id.as_str()],
            )
            .db()?
            != 0)
    }
}
