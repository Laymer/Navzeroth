//! Contract drift guard (G2): the TypeScript `CommandMap` and the Rust
//! `generate_handler!` list must name exactly the same commands, and every
//! command with a multi-word argument must opt into `rename_all = "snake_case"`,
//! because the TypeScript client sends snake_case keys and Tauri expects
//! camelCase by default. Both sides are read as text: the desktop crate cannot
//! be built on every CI host.
use std::{collections::BTreeSet, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(relative: &str) -> String {
    fs::read_to_string(root().join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"))
}

fn is_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn typescript_commands() -> BTreeSet<String> {
    let source = read("apps/desktop/src/commands/contract.ts");
    let start = source
        .find("export interface CommandMap {")
        .expect("CommandMap is declared");
    let body = &source[start..];
    let body = &body[..body.find("\n}").expect("CommandMap is closed")];
    let mut found = BTreeSet::new();
    for line in body.lines().skip(1) {
        // Entries sit at two-space indent: `name: {input:...` or `name:{input:...`.
        if line.starts_with("  ") && !line.starts_with("   ") {
            let trimmed = line.trim_start();
            if let Some((name, rest)) = trimmed.split_once(':')
                && is_ident(name.trim())
                && rest.trim_start().starts_with('{')
            {
                assert!(found.insert(name.trim().to_owned()), "duplicate {name}");
            }
        }
    }
    found
}

fn rust_handlers() -> BTreeSet<String> {
    let source = read("apps/desktop/src-tauri/src/lib.rs");
    let start = source
        .find("tauri::generate_handler![")
        .expect("generate_handler! is present")
        + "tauri::generate_handler![".len();
    let body = &source[start..];
    let body = &body[..body.find(']').expect("handler list is closed")];
    let mut found = BTreeSet::new();
    for name in body.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        assert!(is_ident(name), "unexpected handler entry {name:?}");
        assert!(found.insert(name.to_owned()), "duplicate handler {name}");
    }
    found
}

#[test]
fn typescript_command_map_equals_the_rust_handler_list() {
    let ts = typescript_commands();
    let rust = rust_handlers();
    assert!(
        ts.len() >= 80,
        "parsed only {} TypeScript commands",
        ts.len()
    );
    let missing_in_rust: Vec<_> = ts.difference(&rust).collect();
    let missing_in_ts: Vec<_> = rust.difference(&ts).collect();
    assert!(
        missing_in_rust.is_empty() && missing_in_ts.is_empty(),
        "CommandMap without handler: {missing_in_rust:?}; handler without CommandMap: {missing_in_ts:?}"
    );
}

/// Parameters of `name`: the attribute text above the function and the argument
/// names, with Tauri-injected arguments (`State`, `AppHandle`, windows) removed.
fn command_signature(source: &str, name: &str) -> (String, Vec<String>) {
    let needle = format!("fn {name}(");
    let at = source
        .find(&needle)
        .or_else(|| source.find(&format!("fn {name}<")))
        .unwrap_or_else(|| panic!("no function {name}"));
    let before = &source[..at];
    let attribute_start = before.rfind("#[tauri::command").expect("command attribute");
    let attribute = before[attribute_start..].lines().next().unwrap().to_owned();
    let open = source[at..].find('(').unwrap() + at;
    let mut depth = 0usize;
    let mut close = open;
    for (i, c) in source[open..].char_indices() {
        match c {
            '(' | '<' | '[' => depth += 1,
            ')' | '>' | ']' => {
                depth -= 1;
                if depth == 0 && c == ')' {
                    close = open + i;
                    break;
                }
            }
            _ => {}
        }
    }
    // Split on top-level commas only.
    let params = &source[open + 1..close];
    let (mut level, mut start, mut parts) = (0i32, 0usize, Vec::new());
    for (i, c) in params.char_indices() {
        match c {
            '<' | '(' | '[' => level += 1,
            '>' | ')' | ']' => level -= 1,
            ',' if level == 0 => {
                parts.push(&params[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&params[start..]);
    let args = parts
        .into_iter()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .filter_map(|p| {
            let (arg, ty) = p.split_once(':')?;
            let ty = ty.trim();
            let injected = ty.starts_with("State<")
                || ty.starts_with("tauri::State<")
                || ty.contains("AppHandle")
                || ty.contains("Window")
                || ty.contains("Webview");
            (!injected).then(|| arg.trim().trim_start_matches("mut ").to_owned())
        })
        .collect();
    (attribute, args)
}

#[test]
fn multi_word_arguments_use_snake_case_renaming() {
    let source = read("apps/desktop/src-tauri/src/lib.rs");
    let mut multi_word = 0;
    for name in rust_handlers() {
        let (attribute, args) = command_signature(&source, &name);
        if args.iter().any(|a| a.contains('_')) {
            multi_word += 1;
            assert!(
                attribute.contains("rename_all = \"snake_case\""),
                "{name} has multi-word arguments {args:?} but its attribute is {attribute:?}"
            );
        }
    }
    assert!(
        multi_word > 20,
        "only {multi_word} multi-word commands were found"
    );
}
