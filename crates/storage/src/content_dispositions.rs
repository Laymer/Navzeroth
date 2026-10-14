//! Durable local eligibility. Operator decisions never rewrite immutable content
//! or establish confidence, authenticity or redistribution permission.
use crate::{content::ContentRevisionKey, error::invalid};
use azeroth_core::{
    error::Result,
    types::{MAX_SAFE_INTEGER, Sha256Hex, UtcMillis},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispositionState {
    Eligible,
    Quarantined,
    Revoked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispositionReason {
    InitialEligibility,
    OperatorReview,
    InvalidContent,
    SourcePermission,
    ContextMismatch,
    WithdrawnContribution,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentDisposition {
    pub key: ContentRevisionKey,
    pub content_sha256: Sha256Hex,
    pub state: DispositionState,
    pub state_revision: u64,
    pub reason_code: DispositionReason,
    pub changed_at_utc_ms: Option<UtcMillis>,
    pub last_activated_order: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispositionGuard {
    pub key: ContentRevisionKey,
    pub content_sha256: Sha256Hex,
    pub state: DispositionState,
    pub state_revision: u64,
}
impl ContentDisposition {
    pub fn guard(&self) -> DispositionGuard {
        DispositionGuard {
            key: self.key.clone(),
            content_sha256: self.content_sha256.clone(),
            state: self.state,
            state_revision: self.state_revision,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispositionMutation {
    pub guard: DispositionGuard,
    pub state: DispositionState,
    pub reason_code: DispositionReason,
}
#[derive(Clone, Debug, Serialize)]
pub struct DispositionOutcome {
    pub disposition: ContentDisposition,
    pub active: Option<ContentRevisionKey>,
    pub fallback_applied: bool,
}
fn validate(d: &ContentDisposition) -> Result<()> {
    if (d.state != DispositionState::Eligible
        && d.reason_code == DispositionReason::InitialEligibility)
        || d.state_revision == 0
        || d.state_revision > MAX_SAFE_INTEGER
        || d.last_activated_order
            .is_some_and(|n| n == 0 || n > MAX_SAFE_INTEGER)
    {
        return Err(invalid("Content disposition counter is invalid"));
    }
    Ok(())
}
fn transition(current: &ContentDisposition, m: &DispositionMutation) -> Result<u64> {
    validate(current)?;
    if current.guard() != m.guard {
        return Err(invalid(
            "Content disposition changed; refresh before deciding",
        ));
    }
    if current.state == DispositionState::Revoked {
        return Err(invalid("Revoked content cannot be released or changed"));
    }
    if current.state == m.state {
        return Err(invalid(
            "Content disposition is already in the requested state",
        ));
    }
    if m.reason_code == DispositionReason::InitialEligibility
        || (m.state == DispositionState::Eligible
            && m.reason_code != DispositionReason::OperatorReview)
    {
        return Err(invalid(
            "Content disposition reason is invalid for this action",
        ));
    }
    current
        .state_revision
        .checked_add(1)
        .filter(|n| *n <= MAX_SAFE_INTEGER)
        .ok_or_else(|| invalid("Content disposition counter is exhausted"))
}
use crate::content::{self, ContentRevision, ContentScope, MAX_CONTENT_REVISIONS};
use crate::{
    SqliteStore,
    error::{SqliteResult, storage},
};
use rusqlite::{Connection, TransactionBehavior, params};
fn decode<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    serde_json::from_str(s).map_err(|_| storage("Stored content disposition is invalid"))
}
fn state_string(s: DispositionState) -> &'static str {
    match s {
        DispositionState::Eligible => "eligible",
        DispositionState::Quarantined => "quarantined",
        DispositionState::Revoked => "revoked",
    }
}
fn reason_string(s: DispositionReason) -> &'static str {
    match s {
        DispositionReason::InitialEligibility => "initial_eligibility",
        DispositionReason::OperatorReview => "operator_review",
        DispositionReason::InvalidContent => "invalid_content",
        DispositionReason::SourcePermission => "source_permission",
        DispositionReason::ContextMismatch => "context_mismatch",
        DispositionReason::WithdrawnContribution => "withdrawn_contribution",
    }
}
pub(crate) fn read(conn: &Connection, id: i64) -> Result<ContentDisposition> {
    read_checked(conn, id).map_err(content::LoadFailure::error)
}
type RawDispositionRow = (
    String,
    String,
    String,
    String,
    bool,
    String,
    String,
    String,
    i64,
    String,
    Option<i64>,
    Option<i64>,
);
fn read_checked(
    conn: &Connection,
    id: i64,
) -> std::result::Result<ContentDisposition, content::LoadFailure> {
    let (edition,build,interface,namespace,fixture,revision,hash,state,counter,reason,time,order): RawDispositionRow=conn.query_row(
 "SELECT CASE WHEN length(CAST(r.edition AS BLOB))<=256 THEN r.edition END, CASE WHEN length(CAST(r.build AS BLOB))<=256 THEN r.build END, CASE WHEN length(CAST(r.interface AS BLOB))<=256 THEN r.interface END, CASE WHEN length(r.namespace)<=16 THEN r.namespace END, r.is_fixture, CASE WHEN length(CAST(r.revision AS BLOB))<=256 THEN r.revision END, CASE WHEN length(m.content_sha256)=64 THEN m.content_sha256 END, CASE WHEN length(d.state)<=16 THEN d.state END, d.state_revision, CASE WHEN length(d.reason_code)<=32 THEN d.reason_code END,d.changed_at_utc_ms,d.last_activated_order FROM dataset_revisions r JOIN dataset_manifests m ON m.revision_id=r.id JOIN content_dispositions d ON d.revision_id=r.id WHERE r.id=?1",
 [id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?,row.get(8)?,row.get(9)?,row.get(10)?,row.get(11)?))).map_err(content::load_sql_error)?;
    let key:ContentRevisionKey=serde_json::from_value(serde_json::json!({"scope":{"context":{"build":{"edition":edition,"build":build,"interface":interface},"namespace":namespace},"fixture":fixture},"revision":revision})).map_err(|_|storage("Stored content disposition identity is invalid"))?;
    let d = ContentDisposition {
        key,
        content_sha256: Sha256Hex::new(hash)?,
        state: decode(&serde_json::to_string(&state).unwrap())?,
        state_revision: u64::try_from(counter)
            .map_err(|_| storage("Stored content disposition counter is invalid"))?,
        reason_code: decode(&serde_json::to_string(&reason).unwrap())?,
        changed_at_utc_ms: time.map(UtcMillis::new).transpose()?,
        last_activated_order: order
            .map(|n| {
                u64::try_from(n).map_err(|_| storage("Stored content activation order is invalid"))
            })
            .transpose()?,
    };
    validate(&d)?;
    Ok(d)
}
fn unique_order(conn: &Connection, d: &ContentDisposition) -> Result<bool> {
    let Some(order) = d.last_activated_order else {
        return Ok(true);
    };
    let scope = &d.key.scope;
    let count:i64=conn.query_row("SELECT count(*) FROM content_dispositions d JOIN dataset_revisions r ON r.id=d.revision_id WHERE r.edition=?1 AND r.build=?2 AND r.interface=?3 AND r.namespace=?4 AND r.is_fixture=?5 AND d.last_activated_order=?6",params![scope.context.build.edition.as_str(),scope.context.build.build.as_str(),scope.context.build.interface.as_str(),content::namespace(scope),scope.fixture,order as i64],|row|row.get(0)).db()?;
    Ok(count == 1)
}
pub(crate) fn ensure_eligible(conn: &Connection, id: i64) -> Result<ContentDisposition> {
    let d = read(conn, id)?;
    if d.state != DispositionState::Eligible {
        return Err(invalid("Content revision is restricted and cannot be used"));
    }
    Ok(d)
}
impl SqliteStore {
    pub fn content_disposition(
        &self,
        key: &ContentRevisionKey,
    ) -> Result<Option<ContentDisposition>> {
        let Some(id) = content::find(&self.conn, key)? else {
            return Ok(None);
        };
        let d = read(&self.conn, id)?;
        if d.key != *key {
            return Err(storage("Content disposition identity disagrees"));
        }
        Ok(Some(d))
    }
    pub fn list_content_dispositions(
        &self,
        scope: &ContentScope,
    ) -> Result<Vec<ContentDisposition>> {
        let mut stmt=self.conn.prepare("SELECT id FROM dataset_revisions WHERE edition=?1 AND build=?2 AND interface=?3 AND namespace=?4 AND is_fixture=?5 ORDER BY id DESC LIMIT 1025").db()?;
        let ids = stmt
            .query_map(
                params![
                    scope.context.build.edition.as_str(),
                    scope.context.build.build.as_str(),
                    scope.context.build.interface.as_str(),
                    content::namespace(scope),
                    scope.fixture
                ],
                |row| row.get::<_, i64>(0),
            )
            .db()?
            .collect::<rusqlite::Result<Vec<_>>>()
            .db()?;
        if ids.len() > MAX_CONTENT_REVISIONS {
            return Err(storage("Content disposition count exceeds limit"));
        }
        ids.into_iter().map(|id| read(&self.conn, id)).collect()
    }
    /// Use this for fresh eligibility+identity guards. No confidence/rights are inferred.
    pub fn usable_content_revision(
        &self,
        key: &ContentRevisionKey,
    ) -> Result<Option<(ContentRevision, ContentDisposition)>> {
        let get = |s: &Self| {
            let Some(id) = content::find(&s.conn, key)? else {
                return Ok(None);
            };
            let d = ensure_eligible(&s.conn, id)?;
            let revision = content::load(&s.conn, id)?;
            if d.key != *key || revision.key() != *key {
                return Err(storage("Content disposition identity disagrees"));
            }
            Ok(Some((revision, d)))
        };
        if self.conn.is_autocommit() {
            self.with_immediate_read(get)
        } else {
            get(self)
        }
    }
    pub fn validate_usable_content_guard(
        &self,
        guard: &DispositionGuard,
    ) -> Result<ContentRevision> {
        let get = |store: &Self| {
            let (revision, d) = store
                .usable_content_revision(&guard.key)?
                .ok_or_else(|| invalid("Content revision is not stored"))?;
            if d.guard() != *guard {
                return Err(invalid("Content eligibility changed; refresh inputs"));
            }
            Ok(revision)
        };
        if self.conn.is_autocommit() {
            self.with_immediate_read(get)
        } else {
            get(self)
        }
    }
    pub fn change_content_disposition(
        &mut self,
        m: &DispositionMutation,
        now: Option<UtcMillis>,
    ) -> Result<DispositionOutcome> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let id = content::find(&tx, &m.guard.key)?
            .ok_or_else(|| invalid("Content revision is not stored"))?;
        let current = read(&tx, id)?;
        let next = transition(&current, m)?;
        if tx.execute("UPDATE content_dispositions SET state=?1,state_revision=?2,reason_code=?3,changed_at_utc_ms=?4 WHERE revision_id=?5 AND state_revision=?6 AND state=?7",params![state_string(m.state),next as i64,reason_string(m.reason_code),now.map(|t|t.get()),id,current.state_revision as i64,state_string(current.state)]).db()?!=1{return Err(invalid("Content disposition changed; refresh before deciding"));}
        let scope = &m.guard.key.scope;
        let pointed = content::pointers(&tx, scope)?;
        let mut active = None;
        let mut fallback_applied = false;
        if pointed.is_some_and(|(active, _)| active == id) && m.state != DispositionState::Eligible
        {
            let mut stmt=tx.prepare("SELECT r.id FROM dataset_revisions r JOIN content_dispositions d ON d.revision_id=r.id WHERE r.edition=?1 AND r.build=?2 AND r.interface=?3 AND r.namespace=?4 AND r.is_fixture=?5 AND d.state='eligible' AND d.last_activated_order IS NOT NULL ORDER BY d.last_activated_order DESC,r.id DESC LIMIT 1025").db()?;
            let ids = stmt
                .query_map(
                    params![
                        scope.context.build.edition.as_str(),
                        scope.context.build.build.as_str(),
                        scope.context.build.interface.as_str(),
                        content::namespace(scope),
                        scope.fixture
                    ],
                    |row| row.get::<_, i64>(0),
                )
                .db()?
                .collect::<rusqlite::Result<Vec<_>>>()
                .db()?;
            drop(stmt);
            if ids.len() > MAX_CONTENT_REVISIONS {
                return Err(storage("Content disposition count exceeds limit"));
            }
            let ordinal:i64=tx.query_row("SELECT coalesce(max(CASE WHEN d.last_activated_order IS NULL THEN 0 WHEN typeof(d.last_activated_order)='integer' AND d.last_activated_order BETWEEN 1 AND 9007199254740991 THEN d.last_activated_order ELSE 9007199254740991 END),0) FROM content_dispositions d JOIN dataset_revisions r ON r.id=d.revision_id WHERE r.edition=?1 AND r.build=?2 AND r.interface=?3 AND r.namespace=?4 AND r.is_fixture=?5",params![scope.context.build.edition.as_str(),scope.context.build.build.as_str(),scope.context.build.interface.as_str(),content::namespace(scope),scope.fixture],|row|row.get(0)).db()?;
            for candidate in ids {
                if ordinal >= MAX_SAFE_INTEGER as i64 {
                    break;
                }

                // Malformed retained data is unavailable; genuine SQLite failures propagate.
                let d = match read_checked(&tx, candidate) {
                    Ok(d) => d,
                    Err(content::LoadFailure::Invalid(_)) => continue,
                    Err(content::LoadFailure::Database(e)) => return Err(e),
                };
                let revision = match content::load_checked(&tx, candidate) {
                    Ok(r) => r,
                    Err(content::LoadFailure::Invalid(_)) => continue,
                    Err(content::LoadFailure::Database(e)) => return Err(e),
                };
                if !unique_order(&tx, &d)? {
                    continue;
                }
                if d.state != DispositionState::Eligible
                    || d.key.scope != *scope
                    || revision.key() != d.key
                {
                    continue;
                }
                content::activate(&tx, candidate, d.key.clone())?;
                active = Some(d.key);
                fallback_applied = true;
                break;
            }
            if active.is_none() {
                tx.execute("DELETE FROM active_datasets WHERE edition=?1 AND build=?2 AND interface=?3 AND namespace=?4 AND is_fixture=?5",params![scope.context.build.edition.as_str(),scope.context.build.build.as_str(),scope.context.build.interface.as_str(),content::namespace(scope),scope.fixture]).db()?;
            }
        } else if let Some((pointed, _)) = pointed {
            active = Some(ensure_eligible(&tx, pointed)?.key);
        }
        let disposition = read(&tx, id)?;
        tx.commit().db()?;
        Ok(DispositionOutcome {
            disposition,
            active,
            fallback_applied,
        })
    }
}
/// Strict recovery registration hook. Imported schema must already be validated.
pub fn validate_content_dispositions(conn: &Connection) -> Result<()> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM content_dispositions", [], |row| {
            row.get(0)
        })
        .db()?;
    let revisions: i64 = conn
        .query_row("SELECT COUNT(*) FROM dataset_revisions", [], |row| {
            row.get(0)
        })
        .db()?;
    if count != revisions || count > MAX_CONTENT_REVISIONS as i64 {
        return Err(storage("Content disposition coverage is invalid"));
    }
    let mut stmt = conn
        .prepare("SELECT revision_id FROM content_dispositions ORDER BY revision_id LIMIT 1025")
        .db()?;
    let ids = stmt
        .query_map([], |row| row.get::<_, i64>(0))
        .db()?
        .collect::<rusqlite::Result<Vec<_>>>()
        .db()?;
    for id in ids {
        let d = read(conn, id)?;
        if !unique_order(conn, &d)? {
            return Err(storage("Content activation history is ambiguous"));
        }
        if content::load(conn, id)?.key() != d.key {
            return Err(storage("Content disposition identity disagrees"));
        }
    }
    let mut stmt = conn
        .prepare("SELECT revision_id FROM active_datasets LIMIT 1025")
        .db()?;
    let active = stmt
        .query_map([], |row| row.get::<_, i64>(0))
        .db()?
        .collect::<rusqlite::Result<Vec<_>>>()
        .db()?;
    if active.len() > MAX_CONTENT_REVISIONS {
        return Err(storage("Active disposition count exceeds limit"));
    }
    for id in active {
        if ensure_eligible(conn, id)?.last_activated_order.is_none() {
            return Err(storage("Active content was never activated"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_policy_and_safe_counter_boundaries() {
        let key=serde_json::from_value(serde_json::json!({"scope":{"context":{"build":{"edition":"wow_forever","build":"UNVERIFIED_DEMO","interface":"UNVERIFIED_DEMO"},"namespace":"demo"},"fixture":true},"revision":"demo-r1"})).unwrap();
        let mut d = ContentDisposition {
            key,
            content_sha256: Sha256Hex::new("a".repeat(64)).unwrap(),
            state: DispositionState::Eligible,
            state_revision: 1,
            reason_code: DispositionReason::InitialEligibility,
            changed_at_utc_ms: None,
            last_activated_order: None,
        };
        let m = DispositionMutation {
            guard: d.guard(),
            state: DispositionState::Quarantined,
            reason_code: DispositionReason::InvalidContent,
        };
        assert_eq!(transition(&d, &m).unwrap(), 2);
        d.state_revision = MAX_SAFE_INTEGER;
        let m = DispositionMutation {
            guard: d.guard(),
            ..m
        };
        assert!(transition(&d, &m).is_err());
        d.state = DispositionState::Revoked;
        d.state_revision = 2;
        let m = DispositionMutation {
            guard: d.guard(),
            state: DispositionState::Eligible,
            reason_code: DispositionReason::OperatorReview,
        };
        assert!(transition(&d, &m).is_err());
        assert!(serde_json::from_str::<DispositionState>("\"future\"").is_err());
        assert!(serde_json::from_str::<DispositionReason>("\"free_text\"").is_err());
    }
}
