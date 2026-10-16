//! Static guarantees about the addon sources: no dynamic code loading, no restricted
//! libraries, and a pure Core/ directory.
mod common;

use azeroth_lua_harness::{
    FORBIDDEN_NAMES, addon_dir, lua_files_on_disk, read_text, scan_forbidden,
};
use common::path;

#[test]
fn addon_sources_never_reference_dynamic_loading_or_restricted_libraries() {
    let files = lua_files_on_disk().unwrap();
    assert!(
        files.len() >= 7,
        "expected the addon files, found {files:?}"
    );
    for file in files {
        let text = read_text(&addon_dir().join(&file)).unwrap();
        let hits = scan_forbidden(&text);
        assert!(hits.is_empty(), "{file}: forbidden references {hits:?}");
    }
}

#[test]
fn forbidden_list_covers_the_required_names() {
    for required in [
        "load",
        "loadstring",
        "dofile",
        "loadfile",
        "setfenv",
        "getfenv",
        "debug",
        "io",
        "os",
    ] {
        assert!(FORBIDDEN_NAMES.contains(&required), "{required}");
    }
}

#[test]
fn core_files_never_name_the_global_table() {
    for file in lua_files_on_disk().unwrap() {
        let text = read_text(&addon_dir().join(&file)).unwrap();
        let uses_global_table = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("--"))
            .any(|line| line.contains("_G") || line.contains("_ENV"));
        if file.starts_with("Core/") {
            assert!(!uses_global_table, "{file} must stay pure");
        } else {
            assert_eq!(file, "Diagnostics.lua");
        }
    }
}

#[test]
fn addon_never_writes_the_global_table_outside_the_declared_names() {
    // Only the declared SavedVariables table and the slash-command key may be created,
    // and only through the injected environment table in Diagnostics.lua.
    let text = read_text(&addon_dir().join("Diagnostics.lua")).unwrap();
    assert!(text.contains("env[Version.saved_variables] = db"));
    assert!(text.contains("env[\"SLASH_\" .. SLASH_KEY .. \"1\"]"));
    let luacheckrc = read_text(&path(".luacheckrc")).unwrap();
    assert!(luacheckrc.contains("std = \"lua51\""));
    assert!(luacheckrc.contains("\"AzerothNavigatorDB\""));
    assert!(luacheckrc.contains("max_line_length"));
}

fn code_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("--"))
        .map(|(index, line)| (index + 1, line))
}

#[test]
fn addon_lines_stay_within_the_lint_limit_and_the_declared_globals_match() {
    // luacheck is authoritative in CI; this keeps the line limit checked locally.
    for file in lua_files_on_disk().unwrap() {
        let text = read_text(&addon_dir().join(&file)).unwrap();
        for (number, line) in text.lines().enumerate() {
            assert!(
                line.len() <= 100,
                "{file}:{} is {} columns",
                number + 1,
                line.len()
            );
        }
    }
    let luacheckrc = read_text(&path(".luacheckrc")).unwrap();
    for name in [
        "AzerothNavigatorDB",
        "SLASH_ANAV1",
        "SLASH_ANAVDIAG1",
        "SLASH_ANAVPLAN1",
        "SLASH_ANAVSESSION1",
    ] {
        assert!(luacheckrc.contains(&format!("\"{name}\"")), "{name}");
    }
    assert!(luacheckrc.contains("read_globals = {}"));
}

#[test]
fn core_patterns_avoid_locale_dependent_character_classes() {
    // %w, %a, %l, %u, %p and %c follow the C library locale; a client locale could widen
    // them past ASCII. Explicit ranges keep redaction and escaping deterministic.
    for file in lua_files_on_disk().unwrap() {
        let text = read_text(&addon_dir().join(&file)).unwrap();
        for (number, line) in code_lines(&text) {
            for class in [
                "%w", "%W", "%a", "%A", "%l", "%L", "%u", "%U", "%p", "%P", "%c", "%C",
            ] {
                assert!(!line.contains(class), "{file}:{number} uses {class}");
            }
        }
    }
}

#[test]
fn the_window_never_takes_the_keyboard_the_focus_or_protected_ui() {
    // Plan section 12.4: no focus stealing, no keyboard capture, no protected UI.
    let banned = [
        "EnableKeyboard",
        "SetPropagateKeyboardInput",
        "StaticPopupDialogs",
        "StaticPopup_Show",
        "HideUIPanel",
        "ShowUIPanel",
        "SettingsPanel",
        "ReloadUI",
        "OnUpdate",
        "C_Timer",
        "SetAutoFocus(true)",
        "RunScript",
        "SecureActionButtonTemplate",
        "SetOverrideBinding",
    ];
    for file in lua_files_on_disk().unwrap() {
        let text = read_text(&addon_dir().join(&file)).unwrap();
        for (number, line) in code_lines(&text) {
            for name in banned {
                assert!(!line.contains(name), "{file}:{number} uses {name}");
            }
        }
    }
    // Only the explicit "Select text" button of the copy view moves focus; the window
    // modules never do.
    for file in ["Core/Gui.lua", "Core/GuiWidgets.lua", "Core/GuiText.lua"] {
        let text = read_text(&addon_dir().join(file)).unwrap();
        for (number, line) in code_lines(&text) {
            assert!(!line.contains("SetFocus"), "{file}:{number}");
        }
    }
}
