//! Export format, size cap, no-partial-write rule and the settings merge.
mod common;

use azeroth_lua_harness::Toc;
use common::*;
use serde_json::{Value, json};
use std::collections::BTreeSet;

const MAX_BYTES: usize = 12288;

#[test]
fn export_is_a_package_local_object_not_an_anav1_envelope() {
    let sandbox = synthetic_client();
    sandbox.load_toc_files().unwrap();
    let text: String = sandbox
        .run(
            "export",
            "return assert(ns.Export.encode(ns.Capabilities.probe(_G)))",
        )
        .unwrap();
    assert!(text.starts_with('{'), "no ANAV1 prefix");
    assert!(!text.starts_with("ANAV1"));
    assert!(text.len() <= MAX_BYTES && MAX_BYTES < 32 * 1024);
    let export: Value = serde_json::from_str(&text).unwrap();
    let object = export.as_object().unwrap();
    let keys: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        BTreeSet::from([
            "addon_version",
            "build",
            "capabilities",
            "format",
            "toc_interface",
            "version"
        ]),
        "not a contract envelope: no kind, schema_version or producer fields"
    );
    assert_eq!(export["format"], "anav-diagnostic");
    assert_eq!(export["version"], 1);
    assert_eq!(export["toc_interface"], 16001);
    assert_eq!(
        export["addon_version"].as_str(),
        Toc::read()
            .unwrap()
            .directives
            .get("Version")
            .map(String::as_str)
    );
    let ids: Vec<&str> = export["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["id"].as_str().unwrap())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "capabilities are sorted by id");
    for record in export["capabilities"].as_array().unwrap() {
        let record_keys: BTreeSet<&str> = record
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let expected: BTreeSet<&str> =
            BTreeSet::from(["api_names", "detail_code", "id", "missing", "status"]);
        assert_eq!(record_keys, expected, "{record}");
    }
}

#[test]
fn store_writes_the_export_and_clears_a_previous_error() {
    let sandbox = strict_core();
    let (stored, export, error): (bool, String, Option<String>) = sandbox
        .run(
            "store",
            "local db = { diagnostic_export_error = 'old', other = 7 } local ok = ns.Export.store(db, ns.Capabilities.probe({})) return ok, db.diagnostic_export, db.diagnostic_export_error",
        )
        .unwrap();
    assert!(stored && error.is_none());
    serde_json::from_str::<Value>(&export).unwrap();
}

/// A report whose strings are individually legal but together exceed the export cap.
const OVERSIZE_REPORT: &str = r#"
    local report = ns.Capabilities.probe({})
    for _, record in ipairs(report.capabilities) do
      record.detail = string.rep("x", 512)
    end
    return report
"#;

#[test]
fn oversize_exports_are_rejected_and_never_leave_a_partial_string() {
    let sandbox = strict_core();
    // 16 records * ~560 bytes is below 12288, so grow the payload until it cannot fit.
    let (stored, code, export, error): (bool, String, Option<String>, String) = sandbox
        .run(
            "oversize",
            &format!(
                r#"
                local function make()
                  {OVERSIZE_REPORT}
                end
                local report = make()
                for i = 1, 8 do report.capabilities[#report.capabilities + 1] = {{
                  id = "extra" .. i, status = "untested", api_names = {{}}, missing = {{}},
                  detail_code = "x", detail = string.rep("y", 512) }} end
                local db = {{}}
                local ok, code = ns.Export.store(db, report)
                return ok, code, db.diagnostic_export, db.diagnostic_export_error
                "#
            ),
        )
        .unwrap();
    assert!(!stored);
    assert_eq!(code, "json_output_too_large");
    assert_eq!(error, "json_output_too_large");
    assert!(export.is_none(), "no partial JSON may be written");
}

#[test]
fn a_failed_store_keeps_the_previous_valid_export_untouched() {
    let sandbox = strict_core();
    let (stored, export, error): (bool, String, String) = sandbox
        .run(
            "keep",
            r#"
            local db = {}
            assert(ns.Export.store(db, ns.Capabilities.probe({})))
            local previous = db.diagnostic_export
            local bad = ns.Capabilities.probe({})
            bad.capabilities[1].detail = 0/0
            local ok = ns.Export.store(db, bad)
            assert(db.diagnostic_export == previous)
            return ok, db.diagnostic_export, db.diagnostic_export_error
            "#,
        )
        .unwrap();
    assert!(!stored);
    assert_eq!(error, "json_number_nan");
    serde_json::from_str::<Value>(&export).unwrap();
}

#[test]
fn invalid_inputs_are_rejected_without_error() {
    let sandbox = strict_core();
    let (a, b, c, d): (Option<String>, String, bool, String) = sandbox
        .run(
            "invalid",
            r#"
            local text, code = ns.Export.encode("not a report")
            local ok, store_code = ns.Export.store("not a db", {})
            local _, cyc = ns.Export.encode((function() local r = {} r.build = r return r end)())
            return text, code, ok, cyc
            "#,
        )
        .unwrap();
    assert!(a.is_none());
    assert_eq!(b, "export_invalid_report");
    assert!(!c);
    assert_eq!(d, "json_cycle");
}

#[test]
fn export_bound_is_below_the_addon_text_limit() {
    let sandbox = strict_core();
    let bytes: i64 = sandbox.run("max", "return ns.Export.MAX_BYTES").unwrap();
    assert_eq!(bytes as usize, MAX_BYTES);
    let json_cap: i64 = sandbox
        .run("cap", "return ns.Json.limits.max_output_bytes")
        .unwrap();
    assert!(bytes <= json_cap && json_cap < 32 * 1024);
}

#[test]
fn settings_merge_applies_defaults_and_keeps_unknown_keys_out() {
    let sandbox = strict_core();
    let merged: String = sandbox
        .run(
            "merge",
            r#"
            local active = ns.Settings.merge({
              auto_probe = false,
              chat_summary = "yes",
              unknown_future_key = true,
              diagnostic_export = "must not appear",
            })
            return assert(ns.Json.encode(active))
            "#,
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&merged).unwrap(),
        json!({ "auto_probe": false, "chat_summary": true })
    );
    for saved in ["nil", "'text'", "42", "true", "{}"] {
        let defaults: String = sandbox
            .run(
                "defaults",
                &format!("return assert(ns.Json.encode(ns.Settings.merge({saved})))"),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&defaults).unwrap(),
            json!({ "auto_probe": true, "chat_summary": true })
        );
    }
    let fresh: bool = sandbox
        .run(
            "fresh",
            "local a = ns.Settings.defaults() a.auto_probe = false return ns.Settings.defaults().auto_probe",
        )
        .unwrap();
    assert!(fresh, "defaults() must return a new table each time");
}

#[test]
fn settings_apply_never_overwrites_the_export_or_unrelated_fields() {
    let sandbox = strict_core();
    let result: String = sandbox
        .run(
            "apply",
            r#"
            local db = {
              diagnostic_export = "{\"kept\":true}",
              diagnostic_export_error = "json_cycle",
              settings = { auto_probe = false, stale = 1 },
              untouched = { 1, 2, 3 },
            }
            local settings = assert(ns.Settings.apply(db))
            assert(settings == db.settings)
            assert(db.settings.stale == nil and db.settings.auto_probe == false)
            assert(db.db_version == 1)
            return assert(ns.Json.encode(db))
            "#,
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&result).unwrap(),
        json!({
            "db_version": 1,
            "diagnostic_export": "{\"kept\":true}",
            "diagnostic_export_error": "json_cycle",
            "settings": { "auto_probe": false, "chat_summary": true },
            "untouched": [1, 2, 3]
        })
    );
}

#[test]
fn settings_apply_refuses_unknown_layouts_and_leaves_them_untouched() {
    let sandbox = strict_core();
    let (none_version, none_code, db_json, no_db): (bool, String, String, String) = sandbox
        .run(
            "refuse",
            r#"
            local future = { db_version = 2, settings = { auto_probe = false }, extra = "x" }
            local settings, code = ns.Settings.apply(future)
            local _, nodb = ns.Settings.apply("nope")
            local text = assert(ns.Json.encode(future))
            return settings == nil, code, text, nodb
            "#,
        )
        .unwrap();
    assert!(none_version);
    assert_eq!(none_code, "settings_db_version");
    assert_eq!(no_db, "settings_no_db");
    assert_eq!(
        serde_json::from_str::<Value>(&db_json).unwrap(),
        json!({ "db_version": 2, "settings": { "auto_probe": false }, "extra": "x" })
    );
}
