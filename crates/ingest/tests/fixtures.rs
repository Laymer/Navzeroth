//! Fixture importer stub (M1.5) and the transport fixture manifest.
mod common;
use azeroth_core::error::ErrorCode;
use azeroth_ingest::{
    fixtures::{
        MANIFEST_FILE, Mismatch, Problem, check_contract_fixtures, check_transport_fixtures,
    },
    import::{FileFormat, import_file},
};
use common::*;
use serde_json::{Value, json};

#[test]
fn every_transport_fixture_is_listed_exactly_once() {
    // Mirrors manifest_fixtures_have_declared_rust_outcomes in crates/core/tests/contracts.rs.
    let manifest: Value = serde_json::from_str(&transport(MANIFEST_FILE)).unwrap();
    assert_eq!(manifest["schema_version"], 1);
    assert!(
        manifest["description"]
            .as_str()
            .unwrap()
            .contains("Synthetic")
    );
    let entries = manifest["fixtures"].as_array().unwrap();
    let mut listed: Vec<_> = entries
        .iter()
        .map(|e| e["file"].as_str().unwrap())
        .collect();
    listed.sort();
    let mut on_disk: Vec<_> = std::fs::read_dir(fixture_dir("transport"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n != MANIFEST_FILE)
        .collect();
    on_disk.sort();
    assert_eq!(listed, on_disk, "every fixture file is listed exactly once");
    for entry in entries {
        assert!(matches!(
            entry["format"].as_str(),
            Some("anav1_text" | "saved_variables")
        ));
        match entry["expect"].as_str().unwrap() {
            "valid" => assert!(entry["kind"].is_string() && entry["error_code"].is_null()),
            "invalid" => assert_eq!(entry["error_code"], "INVALID_INPUT"),
            other => panic!("unknown expectation {other}"),
        }
        assert!(entry["note"].is_string());
    }
    // Synthetic only: every valid fixture imports as a fixture (the flag itself is
    // exercised in import.rs).
    for entry in entries.iter().filter(|e| e["expect"] == "valid") {
        let format = match entry["format"].as_str().unwrap() {
            "anav1_text" => FileFormat::Anav1Text,
            _ => FileFormat::SavedVariables(location()),
        };
        let path = fixture_dir("transport").join(entry["file"].as_str().unwrap());
        let result = import_file(&path, &format, &no_clock()).unwrap();
        assert!(result.fixture, "{}", entry["file"]);
    }
    for entry in entries {
        // fixtures/** is eol=lf; CRLF variants are built in memory by the tests.
        let text = transport(entry["file"].as_str().unwrap());
        assert!(!text.contains('\r'), "{}", entry["file"]);
    }
}

#[test]
fn repository_fixtures_match_their_manifests() {
    let contracts = check_contract_fixtures(&fixture_dir("contracts/v1")).unwrap();
    let manifest: Value = serde_json::from_str(&contract(MANIFEST_FILE)).unwrap();
    assert_eq!(
        contracts.checked,
        manifest["fixtures"].as_array().unwrap().len()
    );
    assert_eq!(contracts.valid + contracts.rejected, contracts.checked);
    assert!(contracts.valid >= 6 && contracts.rejected > 0);

    let transport = check_transport_fixtures(&fixture_dir("transport")).unwrap();
    assert_eq!(transport.valid + transport.rejected, transport.checked);
    assert!(transport.valid >= 5 && transport.rejected >= 20);
}

#[test]
fn invalid_transport_fixtures_fail_for_their_documented_reasons() {
    let cases = [
        (
            "anav1-wrong-prefix.txt",
            "Transport text does not start with the ANAV1 prefix",
        ),
        (
            "anav1-lowercase-prefix.txt",
            "Transport text does not start with the ANAV1 prefix",
        ),
        (
            "anav1-missing-prefix.txt",
            "Transport text does not start with the ANAV1 prefix",
        ),
        (
            "anav1-bom.txt",
            "Transport text does not start with the ANAV1 prefix",
        ),
        (
            "anav1-duplicated-prefix.txt",
            "ANAV1 payload must be a JSON object",
        ),
        (
            "anav1-space-after-prefix.txt",
            "ANAV1 payload must be a JSON object",
        ),
        (
            "anav1-json-depth-33.txt",
            "Exchange JSON nesting depth exceeded",
        ),
        ("anav1-trailing-garbage.txt", "Invalid exchange JSON header"),
        ("anav1-two-documents.txt", "Invalid exchange JSON header"),
        ("anav1-truncated.txt", "Invalid exchange JSON header"),
        (
            "anav1-route-plan-not-accepted.txt",
            "Exchange kind is not accepted by this transport",
        ),
    ];
    for (file, message) in cases {
        let err = import_file(
            &fixture_dir("transport").join(file),
            &FileFormat::Anav1Text,
            &no_clock(),
        )
        .expect_err(file);
        assert_eq!(err.code, ErrorCode::InvalidInput, "{file}");
        assert_eq!(err.to_string(), message, "{file}");
    }
    for file in [
        "savedvars-missing-export.lua",
        "savedvars-export-not-anav1.lua",
        "savedvars-export-truncated.lua",
    ] {
        let err = import_file(
            &fixture_dir("transport").join(file),
            &FileFormat::SavedVariables(location()),
            &no_clock(),
        )
        .expect_err(file);
        assert_eq!(err.code, ErrorCode::InvalidInput, "{file}");
    }
}

fn problems(
    result: Result<azeroth_ingest::fixtures::ManifestReport, Vec<Mismatch>>,
) -> Vec<(String, Problem)> {
    result
        .expect_err("check fails")
        .into_iter()
        .map(|m| (m.file, m.problem))
        .collect()
}
fn entry(file: &str, format: &str, expect: &str, code: Option<&str>) -> Value {
    let mut e = json!({"file": file, "format": format, "expect": expect});
    if let Some(code) = code {
        e["error_code"] = json!(code);
    }
    e
}
fn transport_dir(scratch: &Scratch, entries: Vec<Value>, files: &[(&str, String)]) {
    let manifest = json!({
        "schema_version": 1,
        "saved_variables": {"global": "AzerothNavigatorDB", "key": "export"},
        "fixtures": entries,
    });
    scratch.write(MANIFEST_FILE, manifest.to_string());
    for (name, content) in files {
        scratch.write(name, content);
    }
}

#[test]
fn transport_check_reports_each_kind_of_disagreement() {
    let good = transport("anav1-character-snapshot.txt");
    let bad = transport("anav1-wrong-prefix.txt");
    let sv = transport("savedvars-valid.lua");
    let files = [
        ("good.txt", good.clone()),
        ("bad.txt", bad.clone()),
        ("ok.lua", sv.clone()),
    ];
    let run = |name: &str, entries: Vec<Value>, files: &[(&str, String)]| {
        let scratch = Scratch::new(name);
        transport_dir(&scratch, entries, files);
        check_transport_fixtures(&scratch.0)
    };
    let all_ok = vec![
        entry("good.txt", "anav1_text", "valid", None),
        entry("bad.txt", "anav1_text", "invalid", Some("INVALID_INPUT")),
        entry("ok.lua", "saved_variables", "valid", None),
    ];
    let report = run("tm-ok", all_ok.clone(), &files).unwrap();
    assert_eq!((report.checked, report.valid, report.rejected), (3, 2, 1));

    let mut declared_wrong = all_ok.clone();
    declared_wrong[0] = entry("good.txt", "anav1_text", "invalid", Some("INVALID_INPUT"));
    declared_wrong[1] = entry("bad.txt", "anav1_text", "valid", None);
    assert_eq!(
        problems(run("tm-wrong", declared_wrong, &files)),
        vec![
            ("good.txt".into(), Problem::AcceptedButDeclaredInvalid),
            ("bad.txt".into(), Problem::RejectedButDeclaredValid),
        ]
    );
    let mut code = all_ok.clone();
    code[1] = entry("bad.txt", "anav1_text", "invalid", Some("IMPORT_TOO_LARGE"));
    assert_eq!(
        problems(run("tm-code", code, &files)),
        vec![("bad.txt".into(), Problem::WrongErrorCode)]
    );
    let mut no_code = all_ok.clone();
    no_code[1] = entry("bad.txt", "anav1_text", "invalid", None);
    assert_eq!(
        problems(run("tm-nocode", no_code, &files)),
        vec![("bad.txt".into(), Problem::MissingErrorCode)]
    );
    let mut kind = all_ok.clone();
    kind[0]["kind"] = json!("party_snapshot");
    assert_eq!(
        problems(run("tm-kind", kind, &files)),
        vec![("good.txt".into(), Problem::WrongKind)]
    );
    let mut unknown = all_ok.clone();
    unknown[0] = entry("good.txt", "anav1_text", "maybe", None);
    unknown[1] = entry("bad.txt", "base64", "valid", None);
    assert_eq!(
        problems(run("tm-unknown", unknown, &files)),
        vec![
            ("good.txt".into(), Problem::UnknownExpectation),
            ("bad.txt".into(), Problem::UnknownExpectation)
        ]
    );
    // Listing problems.
    let unlisted = all_ok[..2].to_vec();
    assert_eq!(
        problems(run("tm-unlisted", unlisted, &files)),
        vec![("ok.lua".into(), Problem::Unlisted)]
    );
    let mut twice = all_ok.clone();
    twice.push(all_ok[0].clone());
    assert_eq!(
        problems(run("tm-twice", twice, &files)),
        vec![("good.txt".into(), Problem::ListedTwice)]
    );
    let mut missing = all_ok.clone();
    missing.push(entry("gone.txt", "anav1_text", "valid", None));
    assert_eq!(
        problems(run("tm-missing", missing, &files)),
        vec![("gone.txt".into(), Problem::Missing)]
    );
    for unsafe_name in [
        "../good.txt",
        "sub/good.txt",
        "/etc/passwd",
        ".hidden",
        "a b.txt",
        "manifest.json",
        "",
    ] {
        let mut escape = all_ok.clone();
        escape.push(entry(unsafe_name, "anav1_text", "valid", None));
        let found = problems(run("tm-unsafe", escape, &files));
        assert!(
            found.contains(&(String::new(), Problem::UnsafeFileName)),
            "{unsafe_name:?}: {found:?}"
        );
    }
    // A subdirectory on disk is an unlisted entry, not recursed into.
    let scratch = Scratch::new("tm-subdir");
    transport_dir(&scratch, all_ok.clone(), &files);
    std::fs::create_dir(scratch.0.join("nested")).unwrap();
    assert_eq!(
        problems(check_transport_fixtures(&scratch.0)),
        vec![("nested".into(), Problem::Unlisted)]
    );
}

#[test]
fn manifest_level_failures_and_no_content_echo() {
    let scratch = Scratch::new("tm-manifest");
    assert_eq!(
        problems(check_transport_fixtures(&scratch.0)),
        vec![(String::new(), Problem::ManifestUnreadable)]
    );
    assert_eq!(
        problems(check_transport_fixtures(&scratch.0.join("absent"))),
        vec![(String::new(), Problem::ManifestUnreadable)]
    );
    let secret = "SENTINEL-private-fixture-content";
    scratch.write(MANIFEST_FILE, format!("{{\"fixtures\": [{secret}"));
    assert_eq!(
        problems(check_transport_fixtures(&scratch.0)),
        vec![(String::new(), Problem::ManifestInvalid)]
    );
    // Content of listed files and of manifest fields is never part of a mismatch.
    let scratch = Scratch::new("tm-echo");
    transport_dir(
        &scratch,
        vec![
            entry("secret.txt", "anav1_text", "valid", None),
            entry("secret.lua", "saved_variables", "valid", None),
        ],
        &[
            ("secret.txt", secret.to_owned()),
            ("secret.lua", format!("T = '{secret}' .. 'x'")),
        ],
    );
    let shown = format!("{:?}", check_transport_fixtures(&scratch.0).unwrap_err());
    assert!(!shown.contains(secret), "{shown}");
    // A SavedVariables entry without a declared location is a manifest problem.
    let scratch = Scratch::new("tm-nolocation");
    scratch.write(
        MANIFEST_FILE,
        json!({"fixtures": [entry("x.lua", "saved_variables", "valid", None)]}).to_string(),
    );
    scratch.write("x.lua", "T = {}");
    assert_eq!(
        problems(check_transport_fixtures(&scratch.0)),
        vec![("x.lua".into(), Problem::UnknownExpectation)]
    );
}

#[test]
fn contract_check_applies_the_core_manifest_rules() {
    let valid = contract("character-snapshot.json");
    let invalid = contract("level-zero.json");
    let run = |name: &str, entries: Value, listed_files: &[(&str, &str)]| {
        let scratch = Scratch::new(name);
        scratch.write(
            MANIFEST_FILE,
            json!({"schema_version": 1, "fixtures": entries}).to_string(),
        );
        for (file, content) in listed_files {
            scratch.write(file, content);
        }
        check_contract_fixtures(&scratch.0)
    };
    let files = [("a.json", valid.as_str()), ("b.json", invalid.as_str())];
    let ok = json!([
        {"file": "a.json", "kind": "character_snapshot", "expect": "valid"},
        {"file": "b.json", "expect": "semantic_invalid", "error_code": "INVALID_INPUT"}
    ]);
    let report = run("cm-ok", ok, &files).unwrap();
    assert_eq!((report.checked, report.valid, report.rejected), (2, 1, 1));
    let wrong_kind = json!([
        {"file": "a.json", "kind": "route_plan", "expect": "valid"},
        {"file": "b.json", "expect": "schema_invalid", "error_code": "UNSUPPORTED_SCHEMA"}
    ]);
    assert_eq!(
        problems(run("cm-wrong", wrong_kind, &files)),
        vec![
            ("a.json".into(), Problem::WrongKind),
            ("b.json".into(), Problem::WrongErrorCode)
        ]
    );
    let flipped = json!([
        {"file": "a.json", "expect": "semantic_invalid", "error_code": "INVALID_INPUT"},
        {"file": "b.json", "expect": "valid"}
    ]);
    assert_eq!(
        problems(run("cm-flip", flipped, &files)),
        vec![
            ("a.json".into(), Problem::AcceptedButDeclaredInvalid),
            ("b.json".into(), Problem::RejectedButDeclaredValid)
        ]
    );
    let listing =
        json!([{"file": "a.json", "expect": "valid"}, {"file": "a.json", "expect": "valid"}]);
    assert_eq!(
        problems(run("cm-list", listing, &files)),
        vec![
            ("a.json".into(), Problem::ListedTwice),
            ("b.json".into(), Problem::Unlisted)
        ]
    );
}

#[test]
fn fixture_diagnostics_are_distinct_closed_descriptions() {
    use std::collections::BTreeSet;
    let problems = [
        Problem::ManifestUnreadable,
        Problem::ManifestInvalid,
        Problem::TooManyEntries,
        Problem::UnsafeFileName,
        Problem::ListedTwice,
        Problem::Unlisted,
        Problem::Missing,
        Problem::Unreadable,
        Problem::UnknownExpectation,
        Problem::MissingErrorCode,
        Problem::AcceptedButDeclaredInvalid,
        Problem::RejectedButDeclaredValid,
        Problem::WrongErrorCode,
        Problem::WrongKind,
        Problem::Internal,
    ];
    let messages: BTreeSet<_> = problems.into_iter().map(Problem::message).collect();
    assert_eq!(
        messages.len(),
        problems.len(),
        "distinct failure classes need distinct diagnostics"
    );
    for message in messages {
        assert!(!message.is_empty());
        assert!(message.len() <= 80);
        assert!(!message.contains(['\n', '\r', '\0']));
    }
    assert_eq!(
        Problem::WrongErrorCode.message(),
        "rejected with a different error code"
    );
    assert_eq!(
        Problem::AcceptedButDeclaredInvalid.message(),
        "accepted but declared invalid"
    );
}
