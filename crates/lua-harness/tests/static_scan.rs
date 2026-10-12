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
