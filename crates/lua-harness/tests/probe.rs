//! Capability probe behaviour against synthetic environments (CASE-010 analogue).
//! Nothing here observes a client: a stub only shows what the probe does with the
//! shape it is given.
mod common;

use azeroth_lua_harness::{EnvMode, Sandbox, read_text};
use common::*;
use serde_json::Value;
use std::collections::BTreeSet;

const ALLOWED_STATUSES: [&str; 2] = ["untested", "failed"];

fn probe_json(sandbox: &Sandbox, env_expr: &str) -> Value {
    let text: String = sandbox
        .run(
            "probe",
            &format!("return assert(ns.Export.encode(ns.Capabilities.probe({env_expr})))"),
        )
        .unwrap();
    serde_json::from_str(&text).unwrap()
}

fn capabilities(export: &Value) -> &Vec<Value> {
    export["capabilities"]
        .as_array()
        .expect("capabilities array")
}

fn record<'a>(export: &'a Value, id: &str) -> &'a Value {
    capabilities(export)
        .iter()
        .find(|record| record["id"] == id)
        .unwrap_or_else(|| panic!("capability {id} missing"))
}

#[test]
fn empty_environment_reports_every_capability_without_error() {
    let sandbox = strict_core();
    let export = probe_json(&sandbox, "{}");
    let records = capabilities(&export);
    assert_eq!(records.len(), 16);
    for record in records {
        assert_eq!(record["status"], "untested", "{record}");
        let code = record["detail_code"].as_str().unwrap();
        let has_apis = !record["api_names"].as_array().unwrap().is_empty();
        if has_apis {
            assert_eq!(code, "api_missing");
            assert_eq!(record["missing"], record["api_names"]);
        } else {
            assert!(
                ["manual_check_required", "needs_relog_verification"].contains(&code),
                "{record}"
            );
        }
        assert!(record.get("detail").is_none());
    }
    assert_eq!(
        export["build"],
        serde_json::json!({
            "edition": "wow_forever", "edition_basis": "assumed_target",
            "version": "unknown", "build": "unknown", "interface": "unknown"
        })
    );
    // A non-table environment is treated like an empty one.
    assert_eq!(probe_json(&sandbox, "nil"), export);
    assert_eq!(probe_json(&sandbox, "42"), export);
}

#[test]
fn full_synthetic_environment_calls_only_the_build_getter() {
    let sandbox = loaded_client();
    sandbox.run::<()>("reset", "calls = {}").unwrap();
    let export = probe_json(&sandbox, "_G");
    let calls: String = sandbox
        .run("calls", "return table.concat(calls, ',')")
        .unwrap();
    assert_eq!(
        calls, "GetBuildInfo",
        "existence checks must not call anything else"
    );

    for record in capabilities(&export) {
        let status = record["status"].as_str().unwrap();
        assert!(ALLOWED_STATUSES.contains(&status), "{record}");
        assert_eq!(status, "untested");
        let code = record["detail_code"].as_str().unwrap();
        match record["id"].as_str().unwrap() {
            "build_identification" => assert_eq!(code, "call_returned_values"),
            "saved_variables_persistence" => assert_eq!(code, "needs_relog_verification"),
            "talents" | "text_import_capacity" => assert_eq!(code, "manual_check_required"),
            _ => assert_eq!(code, "api_present", "{record}"),
        }
        assert!(record["missing"].as_array().unwrap().is_empty());
    }
    assert_eq!(export["build"]["version"], "0.0.0");
    assert_eq!(export["build"]["build"], "SYNTHETIC");
    assert_eq!(export["build"]["interface"], "99999");
}

#[test]
fn partial_environment_marks_only_the_affected_capability() {
    let sandbox = loaded_client();
    sandbox
        .run::<()>("partial", "C_Map.GetPlayerMapPosition = nil")
        .unwrap();
    let export = probe_json(&sandbox, "_G");
    let map = record(&export, "map_position");
    assert_eq!(map["detail_code"], "api_partial");
    assert_eq!(
        map["missing"],
        serde_json::json!(["C_Map.GetPlayerMapPosition"])
    );
    assert_eq!(
        record(&export, "map_zone_rectangle")["detail_code"],
        "api_present"
    );
    // A non-function value does not count as a present API.
    sandbox
        .run::<()>("shadow", "GetMoney = 'not a function'")
        .unwrap();
    let export = probe_json(&sandbox, "_G");
    assert_eq!(
        record(&export, "money_bags_durability")["detail_code"],
        "api_partial"
    );
}

#[test]
fn a_raising_getter_is_recorded_as_failed_with_a_redacted_bounded_message() {
    let sandbox = loaded_client();
    sandbox
        .run::<()>(
            "raise",
            r#"function GetBuildInfo()
              error("Thrall-Area52 at C:\\Users\\Igor\\Interface\\AddOns\\x.lua:12: Player-1234-ABCDEF01 'Secret Name' said \"hi\" " .. string.rep("long ", 100))
            end"#,
        )
        .unwrap();
    let export = probe_json(&sandbox, "_G");
    let build = record(&export, "build_identification");
    assert_eq!(build["status"], "failed");
    assert_eq!(build["detail_code"], "call_failed");
    let detail = build["detail"].as_str().unwrap();
    assert!(detail.len() <= 96, "{detail}");
    for leaked in [
        "Thrall",
        "Area52",
        "Igor",
        "C:",
        "Player-1234",
        "ABCDEF01",
        "Secret",
        "AddOns",
    ] {
        assert!(!detail.contains(leaked), "{leaked} leaked into {detail:?}");
    }
    assert!(detail.is_ascii());
    // CASE-010 analogue: the other capabilities are unaffected and the export is stored.
    let others = capabilities(&export)
        .iter()
        .filter(|record| record["id"] != "build_identification")
        .all(|record| record["status"] == "untested" && record.get("detail").is_none());
    assert!(others);
    assert_eq!(export["build"]["version"], "unknown");

    let stored: bool = sandbox
        .run(
            "store",
            "local db = {} return ns.Export.store(db, ns.Capabilities.probe(_G))",
        )
        .unwrap();
    assert!(stored);
}

#[test]
fn non_string_errors_and_indexing_failures_stay_bounded() {
    let sandbox = loaded_client();
    sandbox
        .run::<()>(
            "raise",
            "function GetBuildInfo() error({ secret = 'Thrall-Area52' }) end",
        )
        .unwrap();
    let export = probe_json(&sandbox, "_G");
    assert_eq!(record(&export, "build_identification")["detail"], "<table>");

    sandbox
        .run::<()>(
            "trap",
            r#"C_Item = setmetatable({}, { __index = function() error("blocked Name-Realm") end })"#,
        )
        .unwrap();
    let export = probe_json(&sandbox, "_G");
    let item = record(&export, "item_data");
    assert_eq!(item["status"], "failed");
    assert_eq!(item["detail_code"], "lookup_failed");
    let detail = item["detail"].as_str().unwrap();
    assert!(
        detail.len() <= 96 && !detail.contains("Name-Realm"),
        "{detail}"
    );
}

#[test]
fn unexpected_build_values_become_unknown_never_guessed() {
    let sandbox = loaded_client();
    for (returns, label) in [
        ("42, {}, nil, 'x'", "wrong types"),
        (
            "'a b', 'c d', nil, 16001.5",
            "spaces and fractional interface",
        ),
        ("string.rep('v', 33), '1', nil, 1", "too long"),
        ("'1.0.0', '1', nil, 0", "interface out of range"),
        ("'1.0.0', '1', nil, 1e9", "interface too large"),
    ] {
        sandbox
            .run::<()>(
                "getter",
                &format!("function GetBuildInfo() return {returns} end"),
            )
            .unwrap();
        let export = probe_json(&sandbox, "_G");
        assert_eq!(
            record(&export, "build_identification")["detail_code"],
            "call_unexpected_return",
            "{label}"
        );
        assert_eq!(
            record(&export, "build_identification")["status"],
            "untested"
        );
    }
    sandbox
        .run::<()>(
            "getter",
            "function GetBuildInfo() return '1.2.3', '4', nil, 16001 end",
        )
        .unwrap();
    let export = probe_json(&sandbox, "_G");
    assert_eq!(
        export["build"],
        serde_json::json!({
            "edition": "wow_forever", "edition_basis": "assumed_target",
            "version": "1.2.3", "build": "4", "interface": "16001"
        })
    );
}

#[test]
fn probe_never_reads_or_exports_personal_data() {
    let sandbox = synthetic_client();
    sandbox
        .run::<()>(
            "personal",
            r#"
            for _, name in ipairs({ "UnitName", "GetRealmName", "UnitGUID", "GetUnitName",
                                    "GetPlayerInfoByGUID", "GetGuildInfo" }) do
              _G[name] = function() calls[#calls + 1] = name return "SentinelPersonalValue-Realm" end
            end
            CHAT_FRAME_TEXT = "SentinelChatText"
            "#,
        )
        .unwrap();
    sandbox.run::<()>("reset", "calls = {}").unwrap();
    sandbox.load_toc_files().unwrap();
    sandbox
        .run::<()>(
            "session",
            "frame.scripts.OnEvent(frame, 'ADDON_LOADED', 'AzerothNavigator') frame.scripts.OnEvent(frame, 'PLAYER_LOGIN') SlashCmdList.ANAVDIAG('probe')",
        )
        .unwrap();
    let calls: String = sandbox
        .run("calls", "return table.concat(calls, ',')")
        .unwrap();
    assert!(
        calls
            .split(',')
            .all(|call| call == "GetBuildInfo" || call == "CreateFrame"),
        "{calls}"
    );
    let export: String = sandbox
        .run("export", "return AzerothNavigatorDB.diagnostic_export")
        .unwrap();
    let everything = format!("{export}{}", sandbox.printed().join("\n"));
    assert!(!everything.contains("Sentinel"));

    // The sources do not even name the personal-data APIs (comments excluded by wording).
    let forbidden = [
        "UnitName",
        "GetRealmName",
        "UnitGUID",
        "GetUnitName",
        "GetPlayerInfoByGUID",
        "GetNormalizedRealmName",
        "CHAT_MSG",
        "GetGuildInfo",
        "BNGetInfo",
        "COMBAT_LOG_EVENT",
    ];
    for file in azeroth_lua_harness::lua_files_on_disk().unwrap() {
        let text = read_text(&azeroth_lua_harness::addon_dir().join(&file)).unwrap();
        for name in forbidden {
            assert!(!text.contains(name), "{file} mentions {name}");
        }
    }
}

#[test]
fn catalogue_is_sorted_unique_and_uses_only_the_documented_vocabulary() {
    let sandbox = strict_core();
    let (ids, apis): (String, String) = sandbox
        .run(
            "catalogue",
            r#"
            local ids, apis = {}, {}
            for _, entry in ipairs(ns.Capabilities.catalogue) do
              ids[#ids + 1] = entry.id
              for _, api in ipairs(entry.apis) do apis[#apis + 1] = api end
            end
            return table.concat(ids, ','), table.concat(apis, ',')
            "#,
        )
        .unwrap();
    let ids: Vec<&str> = ids.split(',').collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "catalogue must be sorted by id");
    assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), ids.len());
    let apis: Vec<&str> = apis.split(',').collect();
    assert_eq!(apis.iter().collect::<BTreeSet<_>>().len(), apis.len());
    assert!(
        ids.iter()
            .all(|id| id.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
    );

    // The probe may only emit statuses that exist in BUILD_CAPABILITIES.md.
    let doc = read_text(&path("docs/compatibility/BUILD_CAPABILITIES.md")).unwrap();
    let flattened = doc.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flattened.contains(
        "Status values: verified available, verified unavailable, restricted, failed, untested."
    ));
    for status in ALLOWED_STATUSES {
        assert!(doc.contains(status));
    }
}

#[test]
fn excerpt_redacts_paths_ids_quotes_and_non_ascii() {
    let sandbox = Sandbox::new(EnvMode::Strict).unwrap();
    for file in [
        "Core/Bootstrap.lua",
        "Core/Version.lua",
        "Core/Capabilities.lua",
    ] {
        sandbox.load_addon_file(file).unwrap();
    }
    let cases = [
        ("Interface/AddOns/Foo/x.lua:3: boom", "<path> boom"),
        ("bad Name-Realm here", "bad <id> here"),
        ("call 'secret' failed", "call '?' failed"),
        ("caf\u{e9} ok", "caf? ok"),
        ("plain", "plain"),
    ];
    for (input, expected) in cases {
        let out: String = sandbox
            .run(
                "excerpt",
                &format!("return ns.Capabilities.excerpt({input:?})"),
            )
            .unwrap();
        assert_eq!(out, expected, "input {input:?}");
    }
}
