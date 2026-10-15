//! Imported Lua is data. These tests show that structurally (no Lua engine, no
//! process spawning, no dynamic loading in the crate) and behaviorally (a
//! sentinel side effect never happens).
mod common;
use azeroth_ingest::{
    import::{FileFormat, import_file, import_saved_variables},
    savedvars::parse,
};
use common::*;
use std::path::{Path, PathBuf};

fn workspace_file(relative: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative),
    )
    .unwrap()
}

#[test]
fn the_crate_has_no_lua_engine_dependency() {
    let manifest =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .unwrap();
    let deps: Vec<_> = manifest
        .lines()
        .skip_while(|l| !l.starts_with("[dependencies]"))
        .skip(1)
        .take_while(|l| !l.starts_with('['))
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .map(|l| l.split('=').next().unwrap().trim().to_owned())
        .collect();
    assert_eq!(
        deps,
        ["azeroth-core", "serde", "serde_json", "sha2", "image"]
    );

    let lock = workspace_file("Cargo.lock");
    let block = lock
        .split("[[package]]")
        .find(|b| b.contains("name = \"azeroth-ingest\""))
        .expect("azeroth-ingest is locked");
    let locked: Vec<_> = block
        .lines()
        .skip_while(|l| !l.starts_with("dependencies"))
        .skip(1)
        .take_while(|l| l.starts_with(' '))
        .map(|l| l.trim().trim_matches(|c| c == '"' || c == ',').to_owned())
        .collect();
    assert_eq!(
        locked,
        ["azeroth-core", "image", "serde", "serde_json", "sha2"]
    );
    // No Lua engine is reachable from this crate, directly or transitively.
    // The dev-only `azeroth-lua-harness` crate (ADR 0003) may lock a Lua
    // engine for addon tests; it must never enter the import path.
    let reachable = lock_closure(&lock, "azeroth-ingest");
    assert!(
        reachable.contains("serde_json"),
        "closure walks dependencies"
    );
    for name in &reachable {
        assert!(
            !name.to_ascii_lowercase().contains("lua"),
            "azeroth-ingest reaches {name}"
        );
    }
}

/// Package names reachable from `root` through the lock file's dependency
/// lists. Entries may carry a version (`name 1.2.3`) when several versions
/// are locked; reachability is tracked by name, which over-approximates.
fn lock_closure(lock: &str, root: &str) -> std::collections::BTreeSet<String> {
    let mut deps: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for block in lock.split("[[package]]") {
        let Some(name) = block
            .lines()
            .find_map(|l| l.strip_prefix("name = \""))
            .map(|n| n.trim_end_matches('"').to_owned())
        else {
            continue;
        };
        let listed = block
            .lines()
            .skip_while(|l| !l.starts_with("dependencies"))
            .skip(1)
            .take_while(|l| l.starts_with(' '))
            .filter_map(|l| {
                l.trim()
                    .trim_matches(|c| c == '"' || c == ',')
                    .split(' ')
                    .next()
                    .map(str::to_owned)
            });
        deps.entry(name).or_default().extend(listed);
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![root.to_owned()];
    while let Some(name) = stack.pop() {
        if seen.insert(name.clone()) {
            stack.extend(deps.get(&name).into_iter().flatten().cloned());
        }
    }
    seen
}

fn code_lines(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut lines = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            lines.extend(code_lines(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            for line in std::fs::read_to_string(&path).unwrap().lines() {
                if !line.trim_start().starts_with("//") {
                    lines.push((path.clone(), line.to_owned()));
                }
            }
        }
    }
    lines
}

#[test]
fn library_source_has_no_execution_or_dynamic_loading() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let forbidden = [
        "mlua",
        "rlua",
        "hlua",
        "piccolo",
        "libloading",
        "dlopen",
        "Command::new",
        "std::process",
        "loadstring",
        "dofile",
        "luaL_",
        "eval(",
        "unsafe ",
        "extern \"C\"",
    ];
    let lines = code_lines(&src);
    assert!(lines.len() > 500);
    for (path, line) in lines {
        for token in forbidden {
            // Reading the current process ID only names create-new staging directories.
            let executable = line.replace("std::process::id()", "current_process_id");
            assert!(
                !executable.contains(token),
                "{} contains {token}",
                path.display()
            );
        }
    }
}

fn malicious_samples(sentinel: &str) -> Vec<String> {
    vec![
        format!(
            "AzerothNavigatorDB = {{ export = function() os.execute('touch {sentinel}') end }}"
        ),
        format!("AzerothNavigatorDB = {{ export = os.execute('touch {sentinel}') }}"),
        format!("AzerothNavigatorDB = {{ export = io.open('{sentinel}', 'w') }}"),
        format!("AzerothNavigatorDB = {{ export = loadstring(\"io.open('{sentinel}','w')\")() }}"),
        "AzerothNavigatorDB = { export = load('x')() }".to_owned(),
        format!("AzerothNavigatorDB = {{ export = dofile('{sentinel}') }}"),
        format!("AzerothNavigatorDB = {{ export = require('{sentinel}') }}"),
        "AzerothNavigatorDB = { export = 'ANAV1:' .. '{}' }".to_owned(),
        "AzerothNavigatorDB = { export = 1 + 2 }".to_owned(),
        "AzerothNavigatorDB = { export = SomeGlobal }".to_owned(),
        format!("os.execute('touch {sentinel}')\nAzerothNavigatorDB = {{ export = 'x' }}"),
        format!("AzerothNavigatorDB = {{ export = [==[io.open('{sentinel}')]==] }}"),
        format!("--[[ io.open('{sentinel}') ]] AzerothNavigatorDB = {{}}"),
        format!("AzerothNavigatorDB = {{}} ; os.execute('touch {sentinel}')"),
        format!("AzerothNavigatorDB = {{ [(function() io.open('{sentinel}') end)()] = 1 }}"),
    ]
}

#[test]
fn malicious_samples_are_rejected_and_never_run() {
    let scratch = Scratch::new("sentinel");
    let sentinel = scratch.0.join("sentinel-must-never-exist");
    let sentinel_text = sentinel.display().to_string().replace('\\', "/");
    for sample in malicious_samples(&sentinel_text) {
        let err = parse(&sample).expect_err(&sample);
        assert_eq!(
            err.code,
            azeroth_core::error::ErrorCode::InvalidInput,
            "{sample}"
        );
        let imported =
            import_saved_variables(&sample, &location(), &no_clock()).expect_err(&sample);
        assert_eq!(imported.code, azeroth_core::error::ErrorCode::InvalidInput);
        let file = scratch.write("sample.lua", &sample);
        assert!(import_file(&file, &FileFormat::SavedVariables(location()), &no_clock()).is_err());
    }
    // The committed malicious fixtures are rejected without side effects as well.
    for entry in std::fs::read_dir(fixture_dir("transport")).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if name.starts_with("savedvars-malicious-") {
            let err = import_file(
                &fixture_dir("transport").join(&name),
                &FileFormat::SavedVariables(location()),
                &no_clock(),
            )
            .expect_err(&name);
            assert_eq!(
                err.code,
                azeroth_core::error::ErrorCode::InvalidInput,
                "{name}"
            );
        }
    }
    assert!(!sentinel.exists(), "a sentinel side effect happened");
    assert!(!Path::new("/nonexistent-azeroth-sentinel/never-created").exists());
    // Code-looking text inside a string or a comment is inert data and not rejected.
    let inert = format!(
        "-- os.execute('{sentinel_text}')\nT = {{ s = \"os.execute('{sentinel_text}') .. function() end\" }}"
    );
    let file = parse(&inert).unwrap();
    assert!(file.get("T").is_some());
    assert!(!sentinel.exists());
}
