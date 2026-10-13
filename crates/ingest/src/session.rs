//! Separate bounded local session transport. Imported Lua is parsed as literals.
use crate::{
    anav1::json_depth_within,
    error::{invalid, too_large, unreadable},
    savedvars,
};
use azeroth_core::{
    error::Result,
    sessions::{SessionBuildConfig, SessionDocument, build_sessions},
    types::Millis,
};
use std::{fs::File, io::Read, path::Path};

pub const PREFIX: &str = "ANAVSESSION1:";
pub const MAX_DOCUMENT_BYTES: usize = 32768;
pub const MAX_TEXT_BYTES: usize = MAX_DOCUMENT_BYTES + PREFIX.len() + 1024;
/// Whole SavedVariables file may also contain snapshots and diagnostic exports.
pub const MAX_FILE_BYTES: usize = savedvars::MAX_FILE_BYTES;

pub fn build_config() -> SessionBuildConfig {
    SessionBuildConfig {
        max_observed_span_ms: Millis::new(300_000).expect("valid constant"),
        clock_tolerance_ms: Millis::new(5_000).expect("valid constant"),
    }
}
pub fn validate(document: &SessionDocument) -> Result<()> {
    document.validate_identity()?;
    build_sessions(&document.batch, &build_config())?;
    Ok(())
}
pub fn decode_text(text: &str) -> Result<SessionDocument> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(too_large("Session transport exceeds its size limit"));
    }
    let json = text
        .trim_matches(|c| matches!(c, ' ' | '\t' | '\r' | '\n'))
        .strip_prefix(PREFIX)
        .ok_or_else(|| invalid("Session transport requires ANAVSESSION1 framing"))?;
    if json.len() > MAX_DOCUMENT_BYTES {
        return Err(too_large("Session document exceeds 32 KiB"));
    }
    if !json.starts_with('{') || !json_depth_within(json, 32) {
        return Err(invalid("Invalid session document shape or depth"));
    }
    let document: SessionDocument =
        serde_json::from_str(json).map_err(|_| invalid("Invalid session JSON"))?;
    validate(&document)?;
    Ok(document)
}
pub fn encode_text(document: &SessionDocument) -> Result<String> {
    validate(document)?;
    let json = serde_json::to_string(document).map_err(|_| invalid("Session encoding failed"))?;
    if json.len() > MAX_DOCUMENT_BYTES {
        return Err(too_large("Session document exceeds 32 KiB"));
    }
    Ok(format!("{PREFIX}{json}"))
}
pub fn import_file(path: &Path, saved_variables: Option<(&str, &str)>) -> Result<SessionDocument> {
    let limit = if saved_variables.is_some() {
        MAX_FILE_BYTES
    } else {
        MAX_TEXT_BYTES
    };
    let file = File::open(path).map_err(|_| unreadable("Session file cannot be read"))?;
    let mut bytes = Vec::new();
    file.take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| unreadable("Session file cannot be read"))?;
    if bytes.len() > limit {
        return Err(too_large("Session file exceeds its size limit"));
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| invalid("Session file is not UTF-8"))?;
    if let Some((global, key)) = saved_variables {
        if key != "session_export" {
            return Err(invalid("Session SavedVariables key must be session_export"));
        }
        let parsed = savedvars::parse(text)?;
        decode_text(savedvars::extract_export(&parsed, global, key)?)
    } else {
        decode_text(text)
    }
}
