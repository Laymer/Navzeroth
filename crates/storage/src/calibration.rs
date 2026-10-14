//! Bounded explicit annotations attached to exact immutable session identity.
use crate::{
    ProfileKey, SqliteStore,
    error::{SqliteResult, invalid, storage},
    sqlite::find_profile,
};
use azeroth_core::{calibration::SessionCalibrationAnnotation, error::Result, types::SessionId};
use rusqlite::{OptionalExtension, params};
impl SqliteStore {
    pub fn set_session_calibration(
        &mut self,
        key: &ProfileKey,
        session: &SessionId,
        annotation: &SessionCalibrationAnnotation,
    ) -> Result<()> {
        annotation.validate()?;
        if self.session(key, session)?.is_none() {
            return Err(invalid("No matching retained session"));
        }
        let (profile, _) =
            find_profile(&self.conn, key)?.ok_or_else(|| invalid("No matching profile"))?;
        let document = serde_json::to_string(annotation)
            .map_err(|_| invalid("Cannot encode calibration annotation"))?;
        if document.len() > 2048 {
            return Err(invalid("Calibration annotation exceeds capacity"));
        }
        self.conn.execute("INSERT INTO session_calibration_annotations(profile_id,session_id,document) VALUES (?1,?2,?3) ON CONFLICT(profile_id,session_id) DO UPDATE SET document=excluded.document", params![profile,session.as_str(),document]).db()?;
        Ok(())
    }
    pub fn session_calibration(
        &self,
        key: &ProfileKey,
        session: &SessionId,
    ) -> Result<Option<SessionCalibrationAnnotation>> {
        let Some((profile, _)) = find_profile(&self.conn, key)? else {
            return Ok(None);
        };
        let document: Option<String> = self.conn.query_row("SELECT document FROM session_calibration_annotations WHERE profile_id=?1 AND session_id=?2",params![profile,session.as_str()],|r|r.get(0)).optional().db()?;
        document
            .map(|text| {
                if text.len() > 2048 {
                    return Err(storage("Stored calibration annotation exceeds capacity"));
                }
                let annotation: SessionCalibrationAnnotation = serde_json::from_str(&text)
                    .map_err(|_| storage("Stored calibration annotation is invalid"))?;
                annotation
                    .validate()
                    .map_err(|_| storage("Stored calibration conditions are invalid"))?;
                Ok(annotation)
            })
            .transpose()
    }
}
