//! Development evidence inspection, never client operation or acceptance.
//! Parse SavedVariables as inert data and emit only fixed, bounded fields.
use azeroth_ingest::savedvars::{self, LuaKey, LuaValue};
use serde::Serialize;
use serde_json::Value;
use std::{fs::File, io::Read, path::Path};

#[derive(Debug, Serialize)]
pub struct Report {
    pub report_version: u32,
    pub diagnostic: Option<Diagnostic>,
    pub snapshot: Option<Snapshot>,
    pub session: Option<Session>,
    pub snapshot_export_present: bool,
    pub session_export_present: bool,
    pub live_acceptance_verified: bool,
    pub installed_revision_verified: bool,
}

#[derive(Debug, Serialize)]
pub struct Diagnostic {
    pub addon_version: String,
    pub client_version: String,
    pub client_build: String,
    pub client_interface: String,
    pub target_tuple_matches: bool,
    pub untested_capabilities: usize,
    pub failed_capabilities: usize,
}

#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub fixture: bool,
    pub target_tuple_matches: bool,
    pub level_status: azeroth_core::observation::ObservationStatus,
    pub level: Option<u16>,
    pub clock_trust: azeroth_core::observation::ClockTrust,
    pub quest_records: usize,
    pub quests_with_readable_completion: usize,
    pub completed_quests: usize,
    pub incomplete_quests: usize,
    pub equipment_slots: usize,
    pub known_equipment_slots: usize,
    pub objective_records: usize,
}

#[derive(Debug, Serialize)]
pub struct Session {
    pub fixture: bool,
    pub target_tuple_matches: bool,
    pub events: usize,
    pub quest_turn_in_events: usize,
    pub maximum_elapsed_ms: Option<u64>,
}

fn session(text: &str) -> Result<Session, &'static str> {
    let document = azeroth_ingest::session::decode_text(text)
        .map_err(|_| "Invalid session export contract")?;
    let json = text
        .trim_matches([' ', '\t', '\r', '\n'])
        .strip_prefix(azeroth_ingest::session::PREFIX)
        .ok_or("Invalid session export transport")?;
    azeroth_core::strict_json::value(json.as_bytes(), 32768)
        .map_err(|_| "Invalid bounded session JSON")?;
    let build = &document.batch.context.build;
    Ok(Session {
        fixture: document.fixture,
        target_tuple_matches: build.edition.as_str() == "wow_forever"
            && build.build.as_str() == "1.60.1.70374"
            && build.interface.as_str() == "16001",
        events: document.batch.events.len(),
        quest_turn_in_events: document
            .batch
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    azeroth_core::sessions::SessionEventKind::QuestTurnIn { .. }
                )
            })
            .count(),
        maximum_elapsed_ms: document
            .batch
            .events
            .iter()
            .map(|e| e.elapsed_ms.get())
            .max(),
    })
}

fn snapshot(text: &str) -> Result<Snapshot, &'static str> {
    let json =
        azeroth_ingest::anav1::decode_text(text).map_err(|_| "Invalid snapshot transport")?;
    azeroth_core::strict_json::value(json.as_bytes(), 32768)
        .map_err(|_| "Invalid bounded snapshot JSON")?;
    let azeroth_core::contract::AnyExchange::CharacterSnapshot(envelope) =
        azeroth_core::contract::decode_any(json).map_err(|_| "Invalid snapshot contract")?
    else {
        return Err("Expected character snapshot export");
    };
    let data = &envelope.payload;
    Ok(Snapshot {
        fixture: envelope.fixture,
        target_tuple_matches: data.context.build.edition.as_str() == "wow_forever"
            && data.context.build.build.as_str() == "1.60.1.70374"
            && data.context.build.interface.as_str() == "16001",
        level_status: data.level.status(),
        level: data.level.value().map(|level| level.get()),
        clock_trust: data.capture.clock_trust,
        quest_records: data.quests.len(),
        quests_with_readable_completion: data
            .quests
            .values()
            .filter(|q| q.completed.value().is_some())
            .count(),
        completed_quests: data
            .quests
            .values()
            .filter(|q| q.completed.value() == Some(&true))
            .count(),
        incomplete_quests: data
            .quests
            .values()
            .filter(|q| q.completed.value() == Some(&false))
            .count(),
        equipment_slots: data.equipment.len(),
        known_equipment_slots: data
            .equipment
            .values()
            .filter(|slot| slot.value().is_some())
            .count(),
        objective_records: data.objectives.len(),
    })
}

fn numeric(value: &Value, dotted: bool) -> Result<String, &'static str> {
    let text = match value {
        Value::String(s) => s.clone(),
        Value::Number(n) if n.is_u64() => n.to_string(),
        _ => return Err("Invalid diagnostic version field"),
    };
    if text == "unknown" {
        return Ok(text);
    }
    if text.is_empty()
        || text.len() > 32
        || !text
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
        || (!dotted && text.contains('.'))
    {
        return Err("Invalid diagnostic version field");
    }
    Ok(text)
}

pub fn inspect(text: &str) -> Result<Report, &'static str> {
    let file = savedvars::parse(text).map_err(|_| "Invalid bounded SavedVariables data")?;
    let Some(LuaValue::Table(db)) = file.get("AzerothNavigatorDB") else {
        return Err("Missing addon SavedVariables table");
    };
    let export = |key: &str| -> Result<Option<&str>, &'static str> {
        match db.get(&LuaKey::Str(key.into())) {
            None => Ok(None),
            Some(LuaValue::Str(s)) => Ok(Some(s)),
            _ => Err("Invalid addon export field type"),
        }
    };
    let diagnostic = export("diagnostic_export")?
        .map(|text| {
            let value = azeroth_core::strict_json::value(text.as_bytes(), 12288)
                .map_err(|_| "Invalid bounded diagnostic JSON")?;
            if value["format"] != "anav-diagnostic" || value["version"] != 1 {
                return Err("Unsupported diagnostic format or version");
            }
            let addon_version = numeric(&value["addon_version"], true)?;
            let client_version = numeric(&value["build"]["version"], true)?;
            let client_build = numeric(&value["build"]["build"], false)?;
            let client_interface = numeric(&value["build"]["interface"], false)?;
            let capabilities = value["capabilities"]
                .as_array()
                .filter(|rows| rows.len() <= 256)
                .ok_or("Invalid diagnostic capabilities")?;
            let (mut untested_capabilities, mut failed_capabilities) = (0, 0);
            for row in capabilities {
                match row["status"].as_str() {
                    Some("untested") => untested_capabilities += 1,
                    Some("failed") => failed_capabilities += 1,
                    _ => return Err("Invalid diagnostic capability status"),
                }
            }
            Ok(Diagnostic {
                addon_version,
                target_tuple_matches: client_version == "1.60.1"
                    && client_build == "70374"
                    && client_interface == "16001",
                client_version,
                client_build,
                client_interface,
                untested_capabilities,
                failed_capabilities,
            })
        })
        .transpose()?;
    Ok(Report {
        report_version: 1,
        diagnostic,
        snapshot: export("snapshot_export")?.map(snapshot).transpose()?,
        session: export("session_export")?.map(session).transpose()?,
        snapshot_export_present: export("snapshot_export")?.is_some_and(|s| !s.is_empty()),
        session_export_present: export("session_export")?.is_some_and(|s| !s.is_empty()),
        live_acceptance_verified: false,
        installed_revision_verified: false,
    })
}

pub fn inspect_file(path: &Path) -> Result<Report, &'static str> {
    // Reject links in the selected path, including parent components. File
    // handles and a capped read retain the byte bound if a file grows.
    for ancestor in path.ancestors().filter(|p| !p.as_os_str().is_empty()) {
        let metadata =
            std::fs::symlink_metadata(ancestor).map_err(|_| "Cannot inspect capture path")?;
        #[cfg(windows)]
        let linked = {
            use std::os::windows::fs::MetadataExt;
            metadata.file_attributes() & 0x400 != 0
        };
        #[cfg(not(windows))]
        let linked = metadata.file_type().is_symlink();
        if linked {
            return Err("Linked capture paths are unsupported");
        }
        if ancestor == path
            && (!metadata.is_file() || metadata.len() > savedvars::MAX_FILE_BYTES as u64)
        {
            return Err("Capture must be a bounded regular file");
        }
    }
    let file = File::open(path).map_err(|_| "Cannot open capture file")?;
    let metadata = file.metadata().map_err(|_| "Cannot inspect capture file")?;
    if !metadata.is_file() || metadata.len() > savedvars::MAX_FILE_BYTES as u64 {
        return Err("Capture must be a bounded regular file");
    }
    let mut bytes = Vec::new();
    file.take(savedvars::MAX_FILE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read capture file")?;
    if bytes.len() > savedvars::MAX_FILE_BYTES {
        return Err("Capture exceeds byte bound");
    }
    inspect(std::str::from_utf8(&bytes).map_err(|_| "Capture is not UTF-8")?)
}
