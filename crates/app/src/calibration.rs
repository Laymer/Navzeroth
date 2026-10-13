//! Profile-resolved deterministic comparison; no caller-supplied observed duration.
use crate::snapshots::SnapshotApplication;
use azeroth_core::{
    calibration::{
        CalibrationReport, CalibrationRequest, CalibrationSample, SessionCalibrationAnnotation,
        calibrate,
    },
    error::{AppError, ErrorCode, Result},
    types::SessionId,
};
impl SnapshotApplication {
    pub fn set_session_calibration(
        &self,
        profile_id: &str,
        session_id: &SessionId,
        annotation: &SessionCalibrationAnnotation,
    ) -> Result<()> {
        self.with_profile_store_mut(profile_id, |store, key| {
            store.set_session_calibration(key, session_id, annotation)
        })
    }
    pub fn get_calibration(
        &self,
        profile_id: &str,
        request: &CalibrationRequest,
    ) -> Result<CalibrationReport> {
        request.validate()?;
        self.with_profile_store_mut(profile_id, |store, key| {
            let samples = store
                .session_ids(key)?
                .into_iter()
                .map(|id| {
                    let session = store.session(key, &id)?.ok_or_else(|| {
                        AppError::new(ErrorCode::InvalidInput, "Session disappeared", false)
                    })?;
                    Ok(CalibrationSample {
                        fixture: session.fixture,
                        batch: session.batch,
                        analytics: session.analytics,
                        annotation: store.session_calibration(key, &id)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            calibrate(&key.character, &key.context, key.fixture, request, &samples)
        })
    }
}
