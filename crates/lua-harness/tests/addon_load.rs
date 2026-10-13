//! TOC consistency, load order, purity of Core/, reload behaviour and event wiring.
//! All of it runs against synthetic stubs; nothing here observes a Forever client.
mod common;

use azeroth_lua_harness::{
    ADDON_NAME, EnvMode, SAVED_VARIABLES_NAME, Sandbox, Toc, lua_files_on_disk, read_text,
};
use common::*;
use serde_json::Value;
use std::collections::BTreeSet;

#[test]
fn toc_matches_files_on_disk_and_marks_the_interface_unverified() {
    let toc = Toc::read().unwrap();
    assert_eq!(toc.directives["Interface"], "16001");
    assert_eq!(toc.directives["X-Interface-Status"], "UNVERIFIED");
    assert!(
        toc.comments
            .iter()
            .any(|comment| comment.starts_with("UNVERIFIED: 16001")),
        "a comment must say the interface number is unverified"
    );
    assert_eq!(toc.directives["SavedVariables"], SAVED_VARIABLES_NAME);
    for key in ["Title", "Notes", "Version"] {
        assert!(!toc.directives[key].is_empty(), "{key} is required");
    }

    let listed: BTreeSet<&String> = toc.files.iter().collect();
    assert_eq!(listed.len(), toc.files.len(), "no file is listed twice");
    let on_disk = lua_files_on_disk().unwrap();
    let on_disk_set: BTreeSet<&String> = on_disk.iter().collect();
    assert_eq!(
        listed, on_disk_set,
        "TOC file list must equal the Lua files on disk"
    );
    assert!(
        toc.files
            .iter()
            .all(|file| file == "Diagnostics.lua" || file.starts_with("Core/")),
        "canonical casing is Core/"
    );
    assert_eq!(
        toc.files.last().map(String::as_str),
        Some("Diagnostics.lua")
    );
    assert_eq!(
        toc.files.first().map(String::as_str),
        Some("Core/Bootstrap.lua")
    );
}

#[test]
fn toc_version_and_interface_agree_with_version_lua() {
    let sandbox = strict_core();
    let (addon, interface, unverified, saved): (String, i64, bool, String) = sandbox
        .run(
            "version",
            "local v = ns.Version; return v.addon, v.toc_interface, v.toc_interface_unverified, v.saved_variables",
        )
        .unwrap();
    let toc = Toc::read().unwrap();
    assert_eq!(toc.directives["Version"], addon);
    assert_eq!(toc.directives["Interface"], interface.to_string());
    assert!(unverified);
    assert_eq!(toc.directives["SavedVariables"], saved);
}

#[test]
fn every_toc_file_loads_in_order_into_a_fresh_state() {
    let sandbox = synthetic_client();
    let before: BTreeSet<String> = global_names(&sandbox);
    let order = sandbox.load_toc_files().unwrap();
    assert_eq!(order, Toc::read().unwrap().files);
    let modules: Vec<String> = sandbox
        .run("modules", "return table.concat(ns.modules, ',')")
        .map(|joined: String| joined.split(',').map(str::to_owned).collect())
        .unwrap();
    assert_eq!(
        modules,
        [
            "Version",
            "Json",
            "Decode",
            "Predicate",
            "Snapshot",
            "Clock",
            "Plan",
            "Hud",
            "Gear",
            "Capabilities",
            "Export",
            "Settings",
            "Diagnostics"
        ]
    );
    let added: Vec<String> = global_names(&sandbox)
        .difference(&before)
        .cloned()
        .collect();
    assert_eq!(
        added,
        ["SLASH_ANAVDIAG1"],
        "loading may define only the slash command global; the SavedVariables table appears at ADDON_LOADED"
    );
}

fn global_names(sandbox: &Sandbox) -> BTreeSet<String> {
    let joined: String = sandbox
        .run(
            "globals",
            "local names = {} for k in pairs(_G) do names[#names + 1] = tostring(k) end table.sort(names) return table.concat(names, ',')",
        )
        .unwrap();
    joined.split(',').map(str::to_owned).collect()
}

#[test]
fn core_modules_touch_no_client_global_even_when_called() {
    // Strict mode raises on any undeclared global read and on any global write.
    let sandbox = strict_core();
    let json: String = sandbox
        .run(
            "core-calls",
            r#"
            local report = ns.Capabilities.probe({})
            local settings = ns.Settings.merge({ auto_probe = false })
            local db = {}
            local ok = ns.Export.store(db, report)
            assert(ok and settings.auto_probe == false)
            assert(ns.Settings.apply({}) ~= nil)
            return db.diagnostic_export
            "#,
        )
        .unwrap();
    serde_json::from_str::<Value>(&json).unwrap();
}

#[test]
fn strict_mode_really_rejects_global_access() {
    let sandbox = Sandbox::new(EnvMode::Strict).unwrap();
    assert!(sandbox.run::<()>("read", "return undeclared_name").is_err());
    assert!(sandbox.run::<()>("write", "leaked = 1").is_err());
}

#[test]
fn sandbox_exposes_no_dynamic_loading_or_forbidden_libraries() {
    let sandbox = Sandbox::new(EnvMode::Client).unwrap();
    let names = [
        "load",
        "loadstring",
        "loadfile",
        "dofile",
        "setfenv",
        "getfenv",
        "require",
        "package",
        "io",
        "os",
        "debug",
        "collectgarbage",
        "coroutine",
    ];
    for name in names {
        let present: bool = sandbox
            .run("absent", &format!("return _G[{name:?}] ~= nil"))
            .unwrap();
        assert!(!present, "{name} must not exist in the sandbox");
    }
    let dump_absent: bool = sandbox
        .run(
            "dump",
            "return string.dump == nil and getmetatable('').__index.dump == nil",
        )
        .unwrap();
    assert!(dump_absent);
}

fn event(sandbox: &Sandbox, name: &str, arg: &str) {
    sandbox
        .run::<()>(
            "event",
            &format!("frame.scripts.OnEvent(frame, {name:?}, {arg:?})"),
        )
        .unwrap();
}

/// Starts a synthetic client, loads the addon, and delivers ADDON_LOADED and PLAYER_LOGIN.
fn run_session(sandbox: &Sandbox) {
    sandbox.load_toc_files().unwrap();
    event(sandbox, "ADDON_LOADED", ADDON_NAME);
    event(sandbox, "PLAYER_LOGIN", "");
}

fn export_of(sandbox: &Sandbox) -> Option<String> {
    sandbox
        .run(
            "export",
            &format!("return {SAVED_VARIABLES_NAME}.diagnostic_export"),
        )
        .unwrap()
}

#[test]
fn addon_registers_events_and_a_slash_command_and_stores_one_export() {
    let sandbox = synthetic_client();
    sandbox.load_toc_files().unwrap();
    let (events, kind, slash, handler): (String, String, String, String) = sandbox
        .run(
            "wiring",
            "return table.concat(frame.events, ','), frame.kind, SLASH_ANAVDIAG1, type(SlashCmdList.ANAVDIAG)",
        )
        .unwrap();
    assert_eq!(events, "ADDON_LOADED,PLAYER_LOGIN");
    assert_eq!(kind, "Frame");
    assert_eq!(slash, "/anavdiag");
    assert_eq!(handler, "function");

    event(&sandbox, "ADDON_LOADED", "SomeOtherAddon");
    let untouched: bool = sandbox
        .run("db", &format!("return {SAVED_VARIABLES_NAME} == nil"))
        .unwrap();
    assert!(untouched, "events for other addons are ignored");

    event(&sandbox, "ADDON_LOADED", ADDON_NAME);
    event(&sandbox, "PLAYER_LOGIN", "");
    let export = export_of(&sandbox).expect("auto probe stores an export");
    let parsed: Value = serde_json::from_str(&export).unwrap();
    assert_eq!(parsed["format"], "anav-diagnostic");
    let printed = sandbox.printed();
    assert_eq!(printed.len(), 1);
    assert!(
        printed[0].starts_with("AzerothNavigator: 16 capabilities ("),
        "{printed:?}"
    );
}

#[test]
fn reload_raises_no_error_and_yields_identical_output() {
    // First session, then a "reload": a brand-new Lua state that receives the previous
    // SavedVariables content, exactly as the client would hand it back.
    let first = synthetic_client();
    run_session(&first);
    let first_export = export_of(&first).expect("export after first session");
    let first_chat = first.printed();

    let second = synthetic_client();
    second
        .run::<()>(
            "restore",
            &format!(
                "{SAVED_VARIABLES_NAME} = {{ db_version = 1, settings = {{ auto_probe = true, chat_summary = true }}, diagnostic_export = {first_export:?} }}"
            ),
        )
        .unwrap();
    run_session(&second);
    assert_eq!(export_of(&second).as_deref(), Some(first_export.as_str()));
    assert_eq!(second.printed(), first_chat);

    // Repeating the probe inside one state is also stable and error free.
    let (a, b): (String, String) = first
        .run(
            "twice",
            "local one = ns.Export.encode(ns.Capabilities.probe(_G)) local two = ns.Export.encode(ns.Capabilities.probe(_G)) return one, two",
        )
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(a, first_export);
    first
        .run::<()>("slash", "SlashCmdList.ANAVDIAG('probe')")
        .unwrap();
    assert_eq!(export_of(&first).as_deref(), Some(first_export.as_str()));
}

#[test]
fn slash_commands_print_bounded_usage_and_never_echo_input() {
    let sandbox = synthetic_client();
    run_session(&sandbox);
    let before = sandbox.printed().len();
    sandbox
        .run::<()>(
            "slash",
            "SlashCmdList.ANAVDIAG('status') SlashCmdList.ANAVDIAG('Thrall-Area52 secret') SlashCmdList.ANAVDIAG(nil) SlashCmdList.ANAVDIAG(42)",
        )
        .unwrap();
    let printed = sandbox.printed();
    assert_eq!(printed.len(), before + 4);
    assert!(printed[before].contains("capabilities"));
    for line in &printed[before + 1..] {
        assert!(line.contains("usage: /anavdiag probe | status"), "{line}");
        assert!(!line.contains("Thrall") && !line.contains("Area52"));
    }
}

#[test]
fn a_failing_handler_reports_once_and_leaves_the_addon_functional() {
    let sandbox = synthetic_client();
    sandbox.load_toc_files().unwrap();
    event(&sandbox, "ADDON_LOADED", ADDON_NAME);
    sandbox
        .run::<()>(
            "break",
            "ns.Capabilities.probe = function() error('synthetic failure') end",
        )
        .unwrap();
    event(&sandbox, "PLAYER_LOGIN", "");
    event(&sandbox, "PLAYER_LOGIN", "");
    let printed = sandbox.printed();
    assert_eq!(
        printed.len(),
        1,
        "an error loop must not spam chat: {printed:?}"
    );
    assert!(printed[0].contains("handler_failed"));
    assert!(!printed[0].contains("synthetic failure"));
    // The slash command path still answers.
    sandbox
        .run::<()>("slash", "SlashCmdList.ANAVDIAG('help')")
        .unwrap();
}

#[test]
fn saved_variables_of_an_unexpected_shape_are_never_overwritten() {
    for (setup, label) in [
        ("AzerothNavigatorDB = 'junk'", "string"),
        (
            "AzerothNavigatorDB = { db_version = 2, settings = { auto_probe = false } }",
            "future version",
        ),
    ] {
        let sandbox = synthetic_client();
        sandbox.run::<()>("restore", setup).unwrap();
        run_session(&sandbox);
        let (kind, version, has_export): (String, Option<i64>, bool) = sandbox
            .run(
                "inspect",
                "local db = AzerothNavigatorDB return type(db), type(db) == 'table' and db.db_version or nil, type(db) == 'table' and db.diagnostic_export ~= nil",
            )
            .unwrap();
        match label {
            "string" => assert_eq!(kind, "string"),
            _ => {
                assert_eq!(kind, "table");
                assert_eq!(version, Some(2));
                assert!(
                    !has_export,
                    "{label}: nothing is written into an unknown layout"
                );
            }
        }
        let printed = sandbox.printed();
        assert!(
            printed
                .iter()
                .any(|line| line.contains("export not stored")),
            "{label}: {printed:?}"
        );
    }
}

#[test]
fn missing_frame_api_leaves_the_slash_command_working() {
    let sandbox = Sandbox::new(EnvMode::Client).unwrap();
    sandbox.run::<()>("stub", "SlashCmdList = {}").unwrap();
    sandbox.load_toc_files().unwrap();
    sandbox
        .run::<()>("slash", "SlashCmdList.ANAVDIAG('probe')")
        .unwrap();
    let printed = sandbox.printed();
    assert_eq!(printed.len(), 1);
    assert!(printed[0].contains("api_missing"), "{printed:?}");
    assert!(printed[0].contains("export_not_persisted"), "{printed:?}");
}

#[test]
fn addon_files_are_text_without_a_byte_order_mark() {
    for file in lua_files_on_disk().unwrap() {
        let text = read_text(&azeroth_lua_harness::addon_dir().join(&file)).unwrap();
        assert!(!text.starts_with('\u{feff}'), "{file}");
        assert!(
            text.is_ascii(),
            "{file} should stay ASCII until localization exists"
        );
    }
}
