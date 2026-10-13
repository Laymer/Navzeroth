//! Explicit session import/history behind the blocking application boundary.
use crate::snapshots::{FileInputFormat, SnapshotApplication};
use azeroth_core::{
    analytics::SessionAnalytics,
    error::{AppError, ErrorCode, Result},
    types::SessionId,
};
use azeroth_storage::sessions::StoredSession;
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
pub struct SessionImportReceipt {
    pub profile_id: String,
    pub inserted: u32,
    pub duplicates: u32,
    pub session_ids: Vec<SessionId>,
}
#[derive(Clone, Debug, Serialize)]
pub struct SessionListEntry {
    pub session_id: SessionId,
    pub analytics: SessionAnalytics,
}
pub type SessionDetail = StoredSession;
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
impl SnapshotApplication {
    pub fn import_session_text(
        &self,
        profile_id: &str,
        text: &str,
    ) -> Result<SessionImportReceipt> {
        let document = azeroth_ingest::session::decode_text(text)?;
        self.with_profile_store_mut(profile_id, |store, key| {
            let result = store.import_sessions(key, &document)?;
            Ok(SessionImportReceipt {
                profile_id: profile_id.to_owned(),
                inserted: result.inserted,
                duplicates: result.duplicates,
                session_ids: result.session_ids,
            })
        })
    }
    pub fn import_session_file(
        &self,
        profile_id: &str,
        path: &Path,
        format: FileInputFormat,
    ) -> Result<SessionImportReceipt> {
        let saved = match &format {
            FileInputFormat::Anav1Text => None,
            FileInputFormat::SavedVariables { global, key } => {
                Some((global.as_str(), key.as_str()))
            }
        };
        let document = azeroth_ingest::session::import_file(path, saved)?;
        self.with_profile_store_mut(profile_id, |store, key| {
            let result = store.import_sessions(key, &document)?;
            Ok(SessionImportReceipt {
                profile_id: profile_id.to_owned(),
                inserted: result.inserted,
                duplicates: result.duplicates,
                session_ids: result.session_ids,
            })
        })
    }
    pub fn list_sessions(&self, profile_id: &str) -> Result<Vec<SessionListEntry>> {
        self.with_profile_store_mut(profile_id, |store, key| {
            store
                .session_ids(key)?
                .into_iter()
                .map(|session_id| {
                    let detail = store
                        .session(key, &session_id)?
                        .ok_or_else(|| invalid("Session no longer exists"))?;
                    Ok(SessionListEntry {
                        session_id,
                        analytics: detail.analytics,
                    })
                })
                .collect()
        })
    }
    pub fn get_session_detail(
        &self,
        profile_id: &str,
        session_id: &SessionId,
    ) -> Result<SessionDetail> {
        self.with_profile_store_mut(profile_id, |store, key| {
            store
                .session(key, session_id)?
                .ok_or_else(|| invalid("No such session"))
        })
    }
    pub fn delete_session(&self, profile_id: &str, session_id: &SessionId) -> Result<bool> {
        self.with_profile_store_mut(profile_id, |store, key| {
            store.delete_session(key, session_id)
        })
    }
}
