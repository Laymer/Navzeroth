//! Atomic full-context content selection, revision travel choices and bundle defaults.
use crate::{
    SqliteStore,
    content::{self, ActivationOutcome, ContentRevisionKey, ContentScope},
    error::{SqliteResult, invalid, storage},
};
use azeroth_core::error::Result;
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct BundleDefaultState {
    pub app_version: String,
    pub previous_app_version: Option<String>,
    pub revision: azeroth_core::types::DatasetRevision,
}
fn scope_json(scope: &ContentScope) -> Result<String> {
    let text = serde_json::to_string(scope).map_err(|_| invalid("Invalid content scope"))?;
    if text.len() > 4096 {
        return Err(invalid("Content scope exceeds bounds"));
    }
    Ok(text)
}
impl SqliteStore {
    pub fn content_accepts_inferred(&self, key: &ContentRevisionKey) -> Result<bool> {
        let Some(id) = content::find(&self.conn, key)? else {
            return Ok(false);
        };
        self.conn
            .query_row(
                "SELECT accept_inferred FROM content_travel_choices WHERE revision_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .db()
            .map(|v| v.unwrap_or(false))
    }
    /// Selection and preference commit together; failure changes neither.
    pub fn activate_content_with_travel(
        &mut self,
        key: &ContentRevisionKey,
        accept: bool,
    ) -> Result<ActivationOutcome> {
        // Own the writer reservation before consulting the validated catalog cache.
        // This managed read ends before any activation/preference write; arbitrary
        // caller transactions still cannot nest this operation or publish a cache.
        let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate).db()?;
        let cache_read = self.catalog_generation.begin_read(&self.conn)?;
        let id =
            content::find(&tx, key)?.ok_or_else(|| invalid("Content revision is not stored"))?;
        if self.cached_catalog(id)?.key() != *key {
            return Err(storage("Stored content identity disagrees"));
        }
        drop(cache_read);
        let result = content::activate(&tx, id, key.clone())?;
        tx.execute("INSERT INTO content_travel_choices VALUES(?1,?2) ON CONFLICT(revision_id) DO UPDATE SET accept_inferred=excluded.accept_inferred", params![id, accept]).db()?;
        tx.commit().db()?;
        Ok(result)
    }
    pub fn bundle_default_state(&self, scope: &ContentScope) -> Result<Option<BundleDefaultState>> {
        let row: Option<(String, Option<String>, i64)> = self.conn.query_row(
            "SELECT app_version,previous_app_version,revision_id FROM bundled_content_defaults WHERE scope_json=?1",
            [scope_json(scope)?], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().db()?;
        row.map(|(app_version, previous_app_version, id)| {
            let key = crate::content_dispositions::read(&self.conn, id)?.key;
            if key.scope != *scope {
                return Err(storage("Bundle default scope disagrees"));
            }
            Ok(BundleDefaultState {
                app_version,
                previous_app_version,
                revision: key.revision,
            })
        })
        .transpose()
    }
    /// Same-version calls preserve user selection. Every distinct application version
    /// resets defaults, and activation, travel choice and marker share one transaction.
    pub fn apply_bundle_default(
        &mut self,
        version: &str,
        key: &ContentRevisionKey,
    ) -> Result<bool> {
        if version.len() > 128 || azeroth_core::release::is_stable(version).is_err() {
            return Err(invalid("Invalid default application version"));
        }
        let scope = scope_json(&key.scope)?;
        let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate).db()?;
        let cache_read = self.catalog_generation.begin_read(&self.conn)?;
        let previous: Option<(String, i64)> = tx
            .query_row(
                "SELECT app_version,revision_id FROM bundled_content_defaults WHERE scope_json=?1",
                [&scope],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .db()?;
        let id =
            content::find(&tx, key)?.ok_or_else(|| invalid("Bundled revision is not stored"))?;
        if let Some((previous_version, previous_id)) = &previous
            && previous_version == version
        {
            if *previous_id != id {
                return Err(invalid(
                    "Bundled defaults changed within an application version",
                ));
            }
            return Ok(false);
        }
        if self.cached_catalog(id)?.key() != *key {
            return Err(storage("Stored content identity disagrees"));
        }
        drop(cache_read);
        content::activate(&tx, id, key.clone())?;
        tx.execute("INSERT INTO content_travel_choices VALUES(?1,1) ON CONFLICT(revision_id) DO UPDATE SET accept_inferred=1", [id]).db()?;
        tx.execute("INSERT INTO bundled_content_defaults VALUES(?1,?2,?3,?4) ON CONFLICT(scope_json) DO UPDATE SET app_version=excluded.app_version,previous_app_version=excluded.previous_app_version,revision_id=excluded.revision_id", params![scope,version,previous.map(|(version,_)| version),id]).db()?;
        tx.commit().db()?;
        Ok(true)
    }
}
