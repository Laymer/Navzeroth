use azeroth_core::{
    coverage::{
        ContentReport, CoverageClass, CoverageFinding, ReportOutcome, content_report_from_json,
    },
    error::ErrorCode,
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::PathBuf};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/coverage/v1")
}
fn read(name: &str) -> String {
    std::fs::read_to_string(fixture_dir().join(name)).expect("fixture is readable")
}
fn read_path(path: &str) -> String {
    std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path))
        .expect("dataset is readable")
}
fn report(name: &str) -> ContentReport {
    content_report_from_json(&read(name))
}
fn codes(findings: &[CoverageFinding]) -> BTreeSet<&'static str> {
    findings.iter().map(CoverageFinding::code).collect()
}
fn code_name(code: ErrorCode) -> String {
    serde_json::to_value(code)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}
fn to_json(report: &ContentReport) -> Value {
    serde_json::to_value(report).unwrap()
}
fn quest_ids(finding: &CoverageFinding) -> Vec<&str> {
    match finding {
        CoverageFinding::QuestPrerequisiteCycle { quests }
        | CoverageFinding::PossibleQuestPrerequisiteCycle { quests } => {
            quests.iter().map(|q| q.as_str()).collect()
        }
        other => panic!("not a cycle finding: {}", other.code()),
    }
}

#[test]
fn manifest_fixtures_have_declared_outcomes() {
    let manifest: Value = serde_json::from_str(&read("manifest.json")).unwrap();
    let entries = manifest["fixtures"].as_array().unwrap();
    let mut listed: Vec<_> = entries
        .iter()
        .map(|e| e["file"].as_str().unwrap().to_owned())
        .collect();
    listed.sort();
    let mut on_disk: Vec<_> = std::fs::read_dir(fixture_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n != "manifest.json")
        .collect();
    on_disk.sort();
    assert_eq!(listed, on_disk, "every fixture file is listed exactly once");

    let names = |entry: &Value, key: &str| -> BTreeSet<String> {
        entry[key]
            .as_array()
            .map(|a| a.iter().map(|c| c.as_str().unwrap().to_owned()).collect())
            .unwrap_or_default()
    };
    for entry in entries {
        let file = entry["file"].as_str().unwrap();
        let r = report(file);
        let errors: BTreeSet<String> = codes(&r.errors).into_iter().map(String::from).collect();
        let gaps: BTreeSet<String> = codes(&r.gaps).into_iter().map(String::from).collect();
        match entry["expect"].as_str().unwrap() {
            "pass" => {
                assert!(r.passes, "{file} should pass");
                assert!(errors.is_empty(), "{file}: {errors:?}");
                assert_eq!(r.outcome, ReportOutcome::Decoded);
            }
            "fail" => {
                assert!(!r.passes, "{file} should fail");
                assert_eq!(errors, names(entry, "errors"), "{file}");
                assert_eq!(r.outcome, ReportOutcome::Decoded);
            }
            "decode_failed" => {
                assert!(!r.passes, "{file} should fail");
                let ReportOutcome::DecodeFailed { code, .. } = &r.outcome else {
                    panic!("{file} should be a decode failure");
                };
                assert_eq!(code_name(*code), entry["error_code"].as_str().unwrap());
                assert!(r.dataset.is_none() && r.coverage.is_none());
                assert!(r.errors.is_empty() && r.gaps.is_empty());
            }
            other => panic!("unknown expectation {other}"),
        }
        for expected in names(entry, "gaps") {
            assert!(gaps.contains(&expected), "{file}: missing gap {expected}");
        }
    }
}

#[test]
fn cycles_list_sorted_quest_ids_and_leave_dependents_out() {
    for (file, expected) in [
        (
            "quest-prerequisite-cycle.json",
            vec!["synth-quest-a", "synth-quest-b"],
        ),
        ("quest-self-prerequisite.json", vec!["synth-quest-a"]),
        (
            "quest-three-cycle-with-tail.json",
            vec!["synth-quest-a", "synth-quest-b", "synth-quest-c"],
        ),
        (
            "double-negated-prerequisite-cycle.json",
            vec!["synth-quest-a", "synth-quest-b"],
        ),
    ] {
        let r = report(file);
        assert!(!r.passes, "{file}");
        assert_eq!(r.errors.len(), 1, "{file}");
        assert_eq!(quest_ids(&r.errors[0]), expected, "{file}");
    }
}

#[test]
fn negation_and_any_do_not_create_definite_cycles() {
    let negated = report("negated-prerequisite-not-cycle.json");
    assert!(negated.passes);
    assert!(negated.errors.is_empty());
    assert!(negated.gaps.iter().all(|g| !g.code().contains("cycle")));
    assert_eq!(negated.coverage.unwrap().quest_graph.unconditional_edges, 2);

    let any = report("any-prerequisite-possible-cycle.json");
    assert!(
        any.passes,
        "a cycle that needs a conditional branch is a gap"
    );
    let possible: Vec<_> = any
        .gaps
        .iter()
        .filter(|g| g.code() == "possible_quest_prerequisite_cycle")
        .collect();
    assert_eq!(possible.len(), 1);
    assert_eq!(quest_ids(possible[0]), ["synth-quest-a", "synth-quest-b"]);
    let graph = any.coverage.unwrap().quest_graph;
    assert_eq!((graph.unconditional_edges, graph.conditional_edges), (1, 2));
}

#[test]
fn dangling_references_name_the_target_and_the_referrer() {
    let r = report("predicate-dangling-quest.json");
    assert!(!r.passes);
    assert_eq!(
        to_json(&r)["errors"],
        json!([{
            "code": "dangling_predicate_quest",
            "referrer": {"kind": "quest", "id": "synth-quest-a"},
            "site": "prerequisites",
            "quest": "synth-quest-missing"
        }])
    );
    let r = report("predicate-dangling-objective.json");
    assert_eq!(
        to_json(&r)["errors"],
        json!([{
            "code": "dangling_predicate_objective",
            "referrer": {"kind": "objective", "id": "synth-objective-1"},
            "site": "requirements",
            "objective": "synth-objective-missing"
        }])
    );
    let r = report("predicate-dangling-item.json");
    let errors = to_json(&r)["errors"].clone();
    assert_eq!(errors[0]["code"], "dangling_predicate_item");
    assert_eq!(
        errors[0]["referrer"],
        json!({"kind": "rule", "id": "synth-rule-1"})
    );
    assert_eq!(errors[0]["site"], "predicate");
    assert_eq!(errors[0]["item"]["item_id"], "synth-item-missing");
    assert_eq!(errors[0]["item"]["context"]["namespace"], "demo");
    let r = report("predicate-undeclared-area-map.json");
    let errors = to_json(&r)["errors"].clone();
    assert_eq!(errors[0]["code"], "undeclared_predicate_area_map");
    assert_eq!(errors[0]["map"]["map_id"], "synth-map-missing");
}

#[test]
fn objective_references_resolve_against_stages_and_objective_records() {
    let r = report("predicate-objective-resolves-stage-and-id.json");
    assert!(r.passes && r.errors.is_empty());
}

#[test]
fn decode_failures_keep_only_code_and_static_message() {
    for (file, code, absent) in [
        ("invalid-coordinate.json", "INVALID_INPUT", "1.5"),
        (
            "broken-reference.json",
            "DATASET_INCOMPLETE",
            "synth-item-missing",
        ),
    ] {
        let r = report(file);
        let ReportOutcome::DecodeFailed { code: got, message } = &r.outcome else {
            panic!("{file} should fail to decode");
        };
        assert_eq!(code_name(*got), code);
        assert!(!message.contains(absent), "{file}: {message}");
        assert!(!serde_json::to_string(&r).unwrap().contains(absent));
        assert!(!r.passes);
    }
    for input in ["", "not json", "{}", "[1, 2"] {
        let r = content_report_from_json(input);
        assert!(!r.passes);
        assert!(matches!(r.outcome, ReportOutcome::DecodeFailed { .. }));
        assert!(!serde_json::to_string(&r).unwrap().contains(input) || input.is_empty());
    }
}

#[test]
fn unknown_transform_is_a_gap_not_an_error() {
    let r = report("unknown-transform.json");
    assert!(r.passes);
    assert!(
        r.gaps
            .iter()
            .any(|g| matches!(g, CoverageFinding::UnknownMapTransform { .. }))
    );
}

#[test]
fn baseline_counts_keep_every_status_and_confidence_visible() {
    let r = report("coverage-baseline.json");
    assert!(r.passes);
    let report = to_json(&r);
    let coverage = &report["coverage"];
    let collections = coverage["collections"].as_object().unwrap();
    let names: BTreeSet<_> = collections.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        BTreeSet::from([
            "evidence",
            "items",
            "acquisitions",
            "quests",
            "map_spaces",
            "nodes",
            "edges",
            "objectives",
            "rules",
            "map_images",
            "map_embeddings"
        ])
    );
    assert_eq!(collections["items"]["total"], 2);
    assert_eq!(
        collections["items"]["evidence"],
        json!({
            "records_with_evidence": 1,
            "records_without_evidence": 1,
            "links_by_confidence": {
                "verified_on_build": 0, "publisher_documented": 0, "limited_sample": 1,
                "cross_edition_unverified": 0, "inferred": 0, "disputed_or_stale": 0
            }
        })
    );
    assert_eq!(collections["nodes"]["evidence"], Value::Null);
    assert_eq!(
        collections["acquisitions"]["fields"]["conditions"],
        json!({"known": 0, "unknown": 0, "unsupported": 1, "restricted": 0, "not_applicable": 0})
    );
    assert_eq!(
        collections["acquisitions"]["fields"]["probability"]["restricted"],
        1
    );
    assert_eq!(
        collections["quests"]["fields"]["prerequisites"],
        json!({"known": 1, "unknown": 1, "unsupported": 0, "restricted": 0, "not_applicable": 1})
    );
    assert_eq!(collections["quests"]["fields"]["rewards"]["restricted"], 1);
    assert_eq!(collections["evidence"]["fields"]["license"]["known"], 1);
    assert_eq!(collections["evidence"]["fields"]["license"]["unknown"], 2);
    assert_eq!(
        coverage["evidence_records"],
        json!({
            "by_confidence": {
                "verified_on_build": 0, "publisher_documented": 0, "limited_sample": 1,
                "cross_edition_unverified": 0, "inferred": 1, "disputed_or_stale": 1
            },
            "records_with_conflicts": 2,
            "conflict_links": 2
        })
    );
    assert_eq!(
        coverage["edge_status"],
        json!({"verified": 0, "estimated": 1, "temporarily_unavailable": 0, "unverified": 1})
    );
    // Zero collections still list their fields with zero counts.
    assert_eq!(collections["map_embeddings"]["total"], 0);
    assert_eq!(
        collections["map_embeddings"]["evidence"]["records_with_evidence"],
        0
    );

    let gap_codes = codes(&r.gaps);
    for expected in [
        "record_without_evidence",
        "unverified_evidence",
        "unknown_map_transform",
        "uncalibrated_image",
        "unprojectable_position",
    ] {
        assert!(gap_codes.contains(expected), "{expected}");
    }
    // Records without evidence and with only unverified evidence are listed by ID.
    assert!(r.gaps.iter().any(|g| matches!(
        g,
        CoverageFinding::RecordWithoutEvidence { record }
            if serde_json::to_value(record).unwrap() == json!({"kind": "item", "key": {
                "context": {"build": {"edition": "wow_forever", "build": "SYNTHETIC", "interface": "SYNTHETIC"}, "namespace": "demo"},
                "item_id": "synth-item-2", "variant": "base"}})
    )));
    assert!(r.gaps.iter().any(|g| matches!(
        g,
        CoverageFinding::UnverifiedEvidence { record, evidence }
            if serde_json::to_value(record).unwrap() == json!({"kind": "quest", "id": "synth-quest-b"})
                && evidence.len() == 1
    )));
    // Positions: the node and quest location on map 1 meet an uncalibrated
    // image, and the two on map 2 have no image.
    let atlas = &coverage["atlas"];
    assert_eq!(atlas["positions_checked"], 4);
    assert_eq!(atlas["projected"], 0);
    assert_eq!(
        atlas["gaps"],
        json!({"no_image_for_map": 2, "uncalibrated_image": 2})
    );
}

#[test]
fn empty_dataset_has_zero_counts_everywhere() {
    let report = to_json(&report("empty-dataset.json"));
    assert_eq!(report["passes"], true);
    for collection in report["coverage"]["collections"]
        .as_object()
        .unwrap()
        .values()
    {
        assert_eq!(collection["total"], 0);
    }
    assert_eq!(report["gaps"], json!([]));
}

#[test]
fn demo_fixture_is_never_forever_coverage() {
    let r = report("coverage-baseline.json");
    let header = r.dataset.unwrap();
    assert_eq!(header.coverage_class, CoverageClass::Demo);
    assert!(header.fixture && !header.forever_reference_content);
    let json = serde_json::to_value(&header).unwrap();
    assert_eq!(json["namespace"], "demo");
    assert_eq!(json["edition"], "wow_forever");
    assert_eq!(json["build"], "SYNTHETIC");
    assert_eq!(json["interface"], "SYNTHETIC");
    assert_eq!(json["revision"], "synth-coverage-r1");
    let coverage = r.coverage.unwrap();
    assert_eq!(
        serde_json::to_value(&coverage.evidence_records.by_confidence).unwrap()["verified_on_build"],
        0
    );
}

/// Turn a demo fixture into a non-fixture reference dataset with one verified
/// evidence record, in memory only.
fn as_reference(name: &str) -> String {
    fn retarget(value: &mut Value) {
        match value {
            Value::Object(o) => {
                if o.get("namespace") == Some(&json!("demo")) {
                    o.insert("namespace".into(), json!("reference"));
                }
                o.values_mut().for_each(retarget);
            }
            Value::Array(a) => a.iter_mut().for_each(retarget),
            _ => {}
        }
    }
    let mut doc: Value = serde_json::from_str(&read(name)).unwrap();
    retarget(&mut doc);
    doc["fixture"] = json!(false);
    doc["producer"]["component"] = json!("cli");
    doc["provenance"]["source"] = json!("manual_entry");
    doc["payload"]["evidence"][0]["confidence"] = json!("verified_on_build");
    serde_json::to_string(&doc).unwrap()
}

#[test]
fn reference_namespace_counts_verified_evidence() {
    let r = content_report_from_json(&as_reference("coverage-baseline.json"));
    assert_eq!(r.outcome, ReportOutcome::Decoded, "{:?}", r.outcome);
    assert!(r.passes);
    let header = r.dataset.as_ref().unwrap();
    assert_eq!(header.coverage_class, CoverageClass::Reference);
    assert!(header.forever_reference_content);
    let json = to_json(&r);
    assert_eq!(
        json["coverage"]["evidence_records"]["by_confidence"]["verified_on_build"],
        1
    );
    assert_eq!(
        json["coverage"]["collections"]["items"]["evidence"]["links_by_confidence"]["verified_on_build"],
        1
    );
    // Item 1 has verified evidence now, so only the item without evidence is listed.
    assert!(!r.gaps.iter().any(|g| matches!(
        g,
        CoverageFinding::UnverifiedEvidence { record, .. }
            if serde_json::to_value(record).unwrap()["kind"] == "item"
    )));
}

#[test]
fn existing_datasets_decode_and_report_deterministically() {
    let inputs = [
        read_path("../../fixtures/contracts/v1/reference-dataset.json"),
        read_path("../../fixtures/contracts/v1/reference-dataset-imagery.json"),
        read_path("../../content/forever/datasets/forever-atlas-r1.json"),
    ];
    for input in &inputs {
        let first = content_report_from_json(input);
        let second = content_report_from_json(input);
        assert_eq!(first.outcome, ReportOutcome::Decoded);
        let a = serde_json::to_string(&first).unwrap();
        let b = serde_json::to_string(&second).unwrap();
        assert_eq!(a, b, "byte-identical across runs");
        let value: Value = serde_json::from_str(&a).unwrap();
        assert_eq!(value, serde_json::to_value(&first).unwrap());
        assert_eq!(serde_json::to_string(&value).unwrap().len(), a.len());
    }
    let demo = content_report_from_json(&inputs[0]);
    assert_eq!(demo.dataset.unwrap().coverage_class, CoverageClass::Demo);
}

#[test]
fn forever_atlas_dataset_passes_as_unverified_reference_content() {
    let r = content_report_from_json(&read_path(
        "../../content/forever/datasets/forever-atlas-r1.json",
    ));
    assert!(r.passes, "{:?}", r.errors);
    let header = r.dataset.as_ref().unwrap();
    assert_eq!(header.coverage_class, CoverageClass::Reference);
    assert!(header.forever_reference_content);
    let json = to_json(&r);
    let collections = &json["coverage"]["collections"];
    assert_eq!(collections["map_images"]["total"], 1);
    assert_eq!(collections["map_spaces"]["total"], 1);
    assert_eq!(collections["quests"]["total"], 0);
    // The atlas is calibrated provisionally; nothing is claimed as verified.
    assert_eq!(
        json["coverage"]["evidence_records"]["by_confidence"]["verified_on_build"],
        0
    );
}

#[test]
fn reports_carry_identifiers_never_imported_free_text() {
    let free_text = [
        "Synthetic evidence",
        "Synthetic tip",
        "Synthetic licence text",
        "Synthetic image",
    ];
    for name in ["coverage-baseline.json", "quest-prerequisite-cycle.json"] {
        let text = serde_json::to_string(&report(name)).unwrap();
        for needle in free_text {
            assert!(!text.contains(needle), "{name} leaks {needle}");
        }
    }
}

/// Appending a record with an unresolved reference can add errors, never remove
/// them, and always fails a passing dataset.
#[test]
fn adding_unresolved_reference_never_improves_a_report() {
    let manifest: Value = serde_json::from_str(&read("manifest.json")).unwrap();
    for entry in manifest["fixtures"].as_array().unwrap() {
        let file = entry["file"].as_str().unwrap();
        if entry["expect"] == "decode_failed" {
            continue;
        }
        let mut doc: Value = serde_json::from_str(&read(file)).unwrap();
        let ctx = doc["payload"]["evidence"]
            .get(0)
            .map(|e| e["context"].clone())
            .unwrap_or_else(|| {
                json!({"build": {"edition": "wow_forever", "build": "SYNTHETIC", "interface": "SYNTHETIC"}, "namespace": "demo"})
            });
        let before = report(file);
        doc["payload"]["rules"].as_array_mut().unwrap().push(json!({
            "id": "synth-rule-added", "context": ctx, "trigger": "level_changed",
            "predicate": {"op": "quest_state", "quest": "synth-quest-nowhere", "state": "accepted", "value": true},
            "result_template": "Synthetic tip", "priority": 0, "deduplication_key": "synth-rule-added",
            "cooldown_ms": 0, "dismissal": "session", "expires_at_utc_ms": null, "evidence": []
        }));
        let after = content_report_from_json(&serde_json::to_string(&doc).unwrap());
        assert_eq!(after.outcome, ReportOutcome::Decoded, "{file}");
        assert!(!after.passes, "{file}");
        for error in &before.errors {
            assert!(after.errors.contains(error), "{file} lost {}", error.code());
        }
        assert!(after.errors.len() > before.errors.len(), "{file}");
    }
}

#[test]
fn error_and_gap_lists_are_sorted() {
    for name in ["coverage-baseline.json", "quest-three-cycle-with-tail.json"] {
        let r = report(name);
        assert!(r.errors.windows(2).all(|w| w[0] < w[1]), "{name}");
        assert!(r.gaps.windows(2).all(|w| w[0] < w[1]), "{name}");
    }
}
