//! Versioned bounded reminder preferences and history scoped to exact profiles.
use crate::{ProfileKey, SqliteStore, error::{SqliteResult, invalid, storage}, sqlite::find_profile};
use azeroth_core::{error::Result, reminders::ReminderState};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

pub const MAX_REMINDER_STATE_BYTES: usize = 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredReminderState {
    pub version: u32,
    pub enabled: bool,
    pub history: ReminderState,
}
impl Default for StoredReminderState {
    fn default() -> Self { Self { version: 1, enabled: true, history: ReminderState::new() } }
}
impl SqliteStore {
    pub fn reminder_state(&self, key: &ProfileKey) -> Result<StoredReminderState> {
        let Some((profile, _)) = find_profile(&self.conn, key)? else { return Err(invalid("No such reminder profile")); };
        let text: Option<String> = self.conn.query_row("SELECT document FROM reminder_state WHERE profile_id=?1", [profile], |r| r.get(0)).optional().db()?;
        let Some(text) = text else { return Ok(StoredReminderState::default()); };
        if text.len() > MAX_REMINDER_STATE_BYTES { return Err(storage("Stored reminder state exceeds capacity")); }
        let result: StoredReminderState = serde_json::from_str(&text).map_err(|_| storage("Stored reminder state is invalid"))?;
        if result.version != 1 || result.history.dismissals().any(|r| r.context != key.context) || result.history.shown().any(|r| r.context != key.context) { return Err(storage("Stored reminder state has incompatible identity or version")); }
        Ok(result)
    }
    pub fn save_reminder_state(&mut self, key: &ProfileKey, state: &StoredReminderState) -> Result<()> {
        if state.version != 1 || state.history.dismissals().any(|r| r.context != key.context) || state.history.shown().any(|r| r.context != key.context) { return Err(invalid("Reminder state has incompatible identity or version")); }
        let text = serde_json::to_string(state).map_err(|_| invalid("Reminder state cannot be encoded"))?;
        if text.len() > MAX_REMINDER_STATE_BYTES { return Err(invalid("Reminder state exceeds capacity")); }
        let _: StoredReminderState = serde_json::from_str(&text).map_err(|_| invalid("Reminder state failed validation"))?;
        let Some((profile, _)) = find_profile(&self.conn, key)? else { return Err(invalid("No such reminder profile")); };
        self.conn.execute("INSERT INTO reminder_state(profile_id,document) VALUES (?1,?2) ON CONFLICT(profile_id) DO UPDATE SET document=excluded.document", params![profile,text]).db()?;
        Ok(())
    }
}
