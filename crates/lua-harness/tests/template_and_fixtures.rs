//! The machine-readable capability template and the synthetic addon fixtures must stay
//! consistent with the probe, the TOC and the existing BUILD_CAPABILITIES record.
mod common;

use azeroth_lua_harness::read_text;
use common::*;
use serde_json::Value;
use std::collections::BTreeSet;

const STATUS_VOCABULARY: [&str; 5] = [
    "verified available",
    "verified unavailable",
    "restricted",
    "failed",
    "untested",
];

const ROW_FIELDS: [&str; 13] = [
    "id",
    "capability",
    "api_names",
    "documentation_source",
    "conditions_tested",
    "observed_result",
    "readable_and_serializable",
    "restricted_contexts",
    "reproduction_steps",
    "sanitized_evidence",
    "status",
    "affected_tasks",
    "fallback",
];

fn empty_probe() -> Value {
    let text: String = strict_core()
        .run(
            "probe",
            "return assert(ns.Export.encode(ns.Capabilities.probe({})))",
        )
        .unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn template_ids_and_api_names_equal_the_probe_catalogue() {
    let template = template();
    let probe = empty_probe();
    let template_rows = template["capabilities"].as_array().unwrap();
    let probe_rows = probe["capabilities"].as_array().unwrap();
    let template_ids: Vec<&str> = template_rows
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    let probe_ids: Vec<&str> = probe_rows
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        template_ids, probe_ids,
        "capability ids must match, in order"
    );
    for (row, record) in template_rows.iter().zip(probe_rows) {
        assert_eq!(row["api_names"], record["api_names"], "{}", row["id"]);
    }
}

#[test]
fn template_rows_use_the_section_22_4_fields_and_are_all_untested() {
    let template = template();
    assert_eq!(template["template_version"], 1);
    let vocabulary: Vec<&str> = template["status_vocabulary"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert_eq!(vocabulary, STATUS_VOCABULARY);
    let header = template["header"].as_object().unwrap();
    for field in [
        "date",
        "operator",
        "game",
        "os_locale",
        "addon_revision",
        "application_revision",
    ] {
        assert!(header.contains_key(field), "header.{field}");
    }
    let expected: BTreeSet<&str> = ROW_FIELDS.into_iter().collect();
    for row in template["capabilities"].as_array().unwrap() {
        let keys: BTreeSet<&str> = row
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, expected, "{}", row["id"]);
        assert_eq!(
            row["status"], "untested",
            "{}: nothing was observed",
            row["id"]
        );
        assert!(row["observed_result"].is_null());
        assert!(row["readable_and_serializable"].is_null());
        assert!(!row["fallback"].as_str().unwrap().is_empty());
        assert!(!row["affected_tasks"].as_array().unwrap().is_empty());
        assert!(
            row["documentation_source"]
                .as_str()
                .unwrap()
                .starts_with("none"),
            "no authoritative documentation was consulted"
        );
    }
}

#[test]
fn template_vocabulary_and_build_tuple_agree_with_build_capabilities_md() {
    let doc = read_text(&path("docs/compatibility/BUILD_CAPABILITIES.md")).unwrap();
    let flattened = doc.split_whitespace().collect::<Vec<_>>().join(" ");
    let status_line = format!("Status values: {}.", STATUS_VOCABULARY.join(", "));
    assert!(flattened.contains(&status_line), "{status_line}");
    let template = template();
    let game = &template["header"]["game"];
    // The existing row reads "wow_forever / 1.60.1.70205 (poster footer) / 16001 (...)".
    let edition_and_build = format!(
        "{} / {}",
        game["edition"].as_str().unwrap(),
        game["build"].as_str().unwrap()
    );
    assert!(
        flattened.contains(&edition_and_build),
        "{edition_and_build}"
    );
    assert!(flattened.contains(&format!("/ {} (", game["interface"].as_str().unwrap())));
    assert!(game["basis"].as_str().unwrap().contains("not verified"));
}

fn golden(name: &str) -> String {
    read_text(&path(&format!("fixtures/addon/{name}")))
        .unwrap()
        .trim_end()
        .to_owned()
}

fn failing_client() -> azeroth_lua_harness::Sandbox {
    let sandbox = synthetic_client();
    sandbox.load_toc_files().unwrap();
    sandbox
        .run::<()>(
            "synthetic-failure",
            r#"function GetBuildInfo() error("Synthetic-Failure at C:\\Synthetic\\path.lua:1: boom 'quoted'") end"#,
        )
        .unwrap();
    sandbox
}

/// `UPDATE_ADDON_FIXTURES=1 cargo test -p azeroth-lua-harness --test template_and_fixtures`
/// regenerates the three golden exports and the export line of the SavedVariables
/// sample from the addon sources. Without it the files are only compared.
fn update_requested() -> bool {
    std::env::var_os("UPDATE_ADDON_FIXTURES").is_some()
}

fn write_fixture(name: &str, text: &str) {
    std::fs::write(path(&format!("fixtures/addon/{name}")), format!("{text}\n")).unwrap();
}

/// The Lua string literal the SavedVariables writer would emit for `text`.
fn lua_literal(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Replaces the `["diagnostic_export"]` line of the sample with the given export.
fn sample_with_export(sample: &str, export: &str) -> String {
    let mut replaced = false;
    let lines: Vec<String> = sample
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("[\"diagnostic_export\"] = ") {
                replaced = true;
                format!("\t[\"diagnostic_export\"] = {},", lua_literal(export))
            } else {
                line.to_owned()
            }
        })
        .collect();
    assert!(replaced, "the sample has an export line");
    format!("{}\n", lines.join("\n"))
}

#[test]
fn golden_exports_match_the_probe_byte_for_byte() {
    let sandbox = synthetic_client();
    sandbox.load_toc_files().unwrap();
    let probe = "return assert(ns.Export.encode(ns.Capabilities.probe(_G)))";
    let present: String = sandbox.run("present", probe).unwrap();
    let empty: String = strict_core()
        .run(
            "empty",
            "return assert(ns.Export.encode(ns.Capabilities.probe({})))",
        )
        .unwrap();
    let failed: String = failing_client().run("failed", probe).unwrap();
    if update_requested() {
        write_fixture("export-synthetic-all-present.json", &present);
        write_fixture("export-synthetic-empty-environment.json", &empty);
        write_fixture("export-synthetic-build-call-failed.json", &failed);
        let sample_path = path("fixtures/addon/savedvariables-sample.synthetic.lua");
        let sample = read_text(&sample_path).unwrap();
        std::fs::write(&sample_path, sample_with_export(&sample, &present)).unwrap();
    }
    assert_eq!(present, golden("export-synthetic-all-present.json"));
    assert_eq!(empty, golden("export-synthetic-empty-environment.json"));
    assert_eq!(failed, golden("export-synthetic-build-call-failed.json"));
}

#[test]
fn the_fixture_writer_is_deterministic_and_reproduces_the_committed_sample() {
    // Regenerating from the current sources must be a no-op on a clean tree.
    let sandbox = synthetic_client();
    sandbox.load_toc_files().unwrap();
    let present: String = sandbox
        .run(
            "present",
            "return assert(ns.Export.encode(ns.Capabilities.probe(_G)))",
        )
        .unwrap();
    let sample = read_text(&path("fixtures/addon/savedvariables-sample.synthetic.lua")).unwrap();
    assert_eq!(sample_with_export(&sample, &present), sample);
    assert_eq!(
        sample_export_string(&sample_with_export(&sample, &present)),
        present
    );
}

#[test]
fn fixtures_are_listed_synthetic_and_parse() {
    let manifest: Value =
        serde_json::from_str(&read_text(&path("fixtures/addon/manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["synthetic"], true);
    assert_eq!(manifest["fixture"], true);
    let listed: BTreeSet<String> = manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["file"].as_str().unwrap().to_owned())
        .collect();
    let on_disk: BTreeSet<String> = std::fs::read_dir(path("fixtures/addon"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name != "manifest.json")
        .collect();
    assert_eq!(
        listed, on_disk,
        "manifest must list exactly the fixture files"
    );
    for name in listed.iter().filter(|name| name.ends_with(".json")) {
        let text = golden(name);
        let export: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(export["format"], "anav-diagnostic");
        assert!(text.len() <= 12288);
        assert!(!text.contains("Sentinel") && !text.contains("Igor"));
    }
}

/// Decodes the Lua string literal after `["diagnostic_export"] = ` (only `\"` and `\\`
/// escapes occur in the sample). Rust parses the sample as text; nothing executes it.
fn sample_export_string(sample: &str) -> String {
    let line = sample
        .lines()
        .find(|line| line.trim_start().starts_with("[\"diagnostic_export\"] = "))
        .expect("export line");
    let literal = line
        .split_once(" = ")
        .map(|(_, rest)| rest.trim_end_matches(','))
        .unwrap();
    let inner = &literal[1..literal.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            out.push(chars.next().expect("escape target"));
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn savedvariables_sample_embeds_the_all_present_export_and_is_marked_unverified() {
    let sample = read_text(&path("fixtures/addon/savedvariables-sample.synthetic.lua")).unwrap();
    assert!(sample.contains("SYNTHETIC") && sample.contains("UNVERIFIED"));
    assert!(sample.contains("never execute"));
    assert_eq!(
        sample_export_string(&sample),
        golden("export-synthetic-all-present.json")
    );
    for expected in [
        "AzerothNavigatorDB = {",
        "[\"db_version\"] = 1,",
        "[\"auto_probe\"] = true,",
        "[\"chat_summary\"] = true,",
    ] {
        assert!(sample.contains(expected), "{expected}");
    }
}
