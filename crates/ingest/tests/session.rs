use azeroth_core::{error::ErrorCode, sessions::SessionDocument};
use azeroth_ingest::session::{decode_text, encode_text};
fn document() -> SessionDocument {
    serde_json::from_value(serde_json::json!({"format":"anav-session-events","version":1,"fixture":true,"batch":{"character":"c1","context":{"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"demo"},"events":[{"session":"s1","sequence":1,"elapsed_ms":0,"kind":{"type":"session_start"}}]}})).unwrap()
}
#[test]
fn roundtrip_zero_and_unknown_clock() {
    let d = document();
    assert_eq!(decode_text(&encode_text(&d).unwrap()).unwrap(), d);
}
#[test]
fn malformed_framing_size_depth_and_version() {
    let text = encode_text(&document()).unwrap();
    for value in [
        format!("\u{feff}{text}"),
        format!("ANAV1:{}", &text[13..]),
        format!("{text} trailing"),
        text.replace("\"version\":1", "\"version\":2"),
        text.replace("\"fixture\":true", "\"fixture\":false"),
    ] {
        assert!(decode_text(&value).is_err());
    }
    assert_eq!(
        decode_text(&format!("ANAVSESSION1:{}", " ".repeat(40000)))
            .unwrap_err()
            .code,
        ErrorCode::ImportTooLarge
    );
    assert!(
        decode_text(&format!(
            "ANAVSESSION1:{{\"x\":{}0{}}}",
            "[".repeat(32),
            "]".repeat(32)
        ))
        .is_err()
    );
}
#[test]
fn event_bound_and_conflicts() {
    let mut d = document();
    d.batch.events = vec![d.batch.events[0].clone(); 1025];
    assert_eq!(encode_text(&d).unwrap_err().code, ErrorCode::ImportTooLarge);
    d.batch.events.truncate(2);
    d.batch.events[1].elapsed_ms = azeroth_core::types::Millis::new(1).unwrap();
    assert!(encode_text(&d).is_err());
}
#[test]
fn saved_variables_literal_only_and_required_key() {
    let path = std::env::temp_dir().join(format!("anav-session-test-{}.lua", std::process::id()));
    let text = encode_text(&document()).unwrap();
    let literal = serde_json::to_string(&text).unwrap();
    std::fs::write(
        &path,
        format!("AzerothNavigatorDB = {{ session_export = {literal} }}"),
    )
    .unwrap();
    let result =
        azeroth_ingest::session::import_file(&path, Some(("AzerothNavigatorDB", "session_export")))
            .unwrap();
    assert_eq!(result, document());
    assert!(
        azeroth_ingest::session::import_file(
            &path,
            Some(("AzerothNavigatorDB", "snapshot_export"))
        )
        .is_err()
    );
    std::fs::write(&path, "AzerothNavigatorDB = loadstring('return {}')()").unwrap();
    assert!(
        azeroth_ingest::session::import_file(&path, Some(("AzerothNavigatorDB", "session_export")))
            .is_err()
    );
    std::fs::remove_file(path).unwrap();
}
#[test]
fn exact_document_byte_boundary() {
    let text = encode_text(&document()).unwrap();
    let json = text.strip_prefix("ANAVSESSION1:").unwrap();
    let padding = " ".repeat(32768 - json.len());
    let at_limit = format!("ANAVSESSION1:{}{padding}}}", &json[..json.len() - 1]);
    assert_eq!(decode_text(&at_limit).unwrap(), document());
    let above = at_limit.replacen("}", " }", 1);
    assert_eq!(
        decode_text(&above).unwrap_err().code,
        ErrorCode::ImportTooLarge
    );
}
#[test]
fn fixture_flag_cannot_disguise_observation_sources() {
    let mut d = document();
    d.batch.events[0].kind=serde_json::from_value(serde_json::json!({"type":"level","level":{"status":"known","value":12,"source":"addon_snapshot"}})).unwrap();
    assert!(encode_text(&d).is_err());
    d.fixture = false;
    d.batch.context.namespace = azeroth_core::types::DatasetNamespace::Reference;
    d.batch.events[0].kind=serde_json::from_value(serde_json::json!({"type":"level","level":{"status":"known","value":12,"source":"synthetic_fixture"}})).unwrap();
    assert!(encode_text(&d).is_err());
}
#[test]
fn locale_extension_is_versioned_without_reinterpreting_historical_sessions() {
    let legacy = document();
    let mut localized = legacy.clone();
    localized.version = 2;
    localized.locale = Some(azeroth_core::types::Locale::new("enUS").unwrap());
    let encoded = encode_text(&localized).unwrap();
    assert_eq!(decode_text(&encoded).unwrap(), localized);
    for version in [0, 1, 3] {
        let mut d = localized.clone();
        d.version = version;
        assert!(encode_text(&d).is_err());
    }
    let mut missing = legacy.clone();
    missing.version = 2;
    assert!(encode_text(&missing).is_err());
    localized.locale = Some(azeroth_core::types::Locale::new("unknown").unwrap());
    assert!(encode_text(&localized).is_err());
    assert!(
        decode_text(&encode_text(&legacy).unwrap())
            .unwrap()
            .locale
            .is_none()
    );
}
