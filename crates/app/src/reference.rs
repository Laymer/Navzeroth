//! Bounded imported reference facts. No live provider, capabilities or game control.
use azeroth_core::{
    contract::Envelope,
    entities::CharacterSnapshot,
    error::{AppError, ErrorCode, Result},
    observation::{Freshness, FreshnessContext, Observation, freshness},
    types::*,
};
use serde::Serialize;
pub const MAX_REFERENCE_BYTES: usize = 64 * 1024;
#[derive(Clone, Debug, Serialize)]
pub struct ReferenceSnapshot {
    pub character: CharacterId,
    pub snapshot_id: SnapshotId,
    pub context: DataContext,
    pub fixture: bool,
    pub level: Observation<Level>,
    pub role: Observation<RoleId>,
    pub location: Observation<Position>,
    pub capture_freshness: Freshness,
    pub level_freshness: Freshness,
    pub role_freshness: Freshness,
    pub location_freshness: Freshness,
}
#[derive(Clone, Debug, Serialize)]
pub struct ReferenceView {
    pub schema_version: u32,
    pub app_version: &'static str,
    pub snapshot: Option<ReferenceSnapshot>,
    pub current_plan: Option<crate::today::CurrentPlanView>,
    pub accessibility: Option<azeroth_storage::today::Accessibility>,
}
pub fn build(
    snapshot: Option<&Envelope<CharacterSnapshot>>,
    clock: &FreshnessContext,
) -> Result<ReferenceView> {
    let view = ReferenceView {
        schema_version: 1,
        app_version: env!("CARGO_PKG_VERSION"),
        current_plan: None,
        accessibility: None,
        snapshot: snapshot.map(|s| ReferenceSnapshot {
            character: s.payload.character.clone(),
            snapshot_id: s.payload.id.clone(),
            context: s.data_context(),
            fixture: s.fixture,
            level: s.payload.level.clone(),
            role: s.payload.role.clone(),
            location: s.payload.location.clone(),
            capture_freshness: freshness(&s.payload.capture, clock),
            level_freshness: freshness(s.payload.level.metadata(), clock),
            role_freshness: freshness(s.payload.role.metadata(), clock),
            location_freshness: freshness(s.payload.location.metadata(), clock),
        }),
    };
    validate_bound(&view)?;
    Ok(view)
}
pub(crate) fn validate_bound(view: &ReferenceView) -> Result<()> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > MAX_REFERENCE_BYTES - self.0 {
                return Err(std::io::Error::other("Reference metadata exceeds limit"));
            }
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Counter(0), view).map_err(|_| {
        AppError::new(
            ErrorCode::ImportTooLarge,
            "Reference metadata exceeds limit",
            false,
        )
    })?;
    Ok(())
}
/// Actual native properties; frontend requested preferences are not observations.
#[derive(Clone, Debug, Serialize)]
pub struct ReferenceWindowState {
    pub schema_version: u32,
    pub exists: bool,
    pub visible: bool,
    pub always_on_top: bool,
}

/// Fixed application IPC policy; labels are supplied only from native invoke metadata.
pub const REFERENCE_LABEL: &str = "reference";
pub const REFERENCE_PATH: &str = "index.html#reference";
pub fn permits_command(caller: &str, command: &str) -> bool {
    caller == "main" || (caller == REFERENCE_LABEL && command == "get_reference_view")
}
pub fn permits_management(caller: &str) -> bool {
    caller == "main"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn serialized_reference_metadata_refuses_an_oversized_field() {
        const TOO_LONG: &str = match std::str::from_utf8(&[b'v'; MAX_REFERENCE_BYTES]) {
            Ok(value) => value,
            Err(_) => panic!("ASCII test data"),
        };
        let view = ReferenceView {
            schema_version: 1,
            app_version: TOO_LONG,
            snapshot: None,
            current_plan: None,
            accessibility: None,
        };
        let error = validate_bound(&view).unwrap_err();
        assert_eq!(error.code, ErrorCode::ImportTooLarge);
        assert!(!error.retryable);
        assert!(view.snapshot.is_none());
    }
}
