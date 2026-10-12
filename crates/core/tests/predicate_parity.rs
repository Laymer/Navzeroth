//! Data-driven evaluator parity corpus (M2.5, Rust side).
//!
//! The corpus under `fixtures/predicates/v1` is anchored to this crate's
//! evaluator: every `expected` value was produced by running `evaluate` and
//! then reviewed against the intended three-valued meaning. A later Lua
//! evaluator must reproduce the same normalized outputs, including `steps` and
//! the order of `reasons` and `warnings`. See the corpus README for the format.
//!
//! Cases marked `lua_only` exercise decoder behavior that only the addon's
//! stricter JSON decoder can reproduce. They are counted and skipped here.
use azeroth_core::{
    entities::{CharacterSnapshot, PartySnapshot},
    evaluation::{EvaluationContext, StalePolicy, evaluate},
    observation::{ClockTrust, FreshnessContext},
    predicate::{Predicate, RuleLimits},
    types::{Millis, SessionId, UtcMillis},
};
use serde::Deserialize;
use serde_json::{Map, Value, from_str, from_value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

/// The closed operator vocabulary of `PredicateNode`. The corpus must use all of it.
const OPERATORS: [&str; 14] = [
    "all",
    "any",
    "not",
    "known",
    "equals",
    "greater_than_or_equal",
    "quest_state",
    "objective_state",
    "level_at_least",
    "equipment_matches",
    "role_is",
    "location_in_supported_area",
    "user_preference",
    "participant_state",
];
const UNAVAILABLE_STATUSES: [&str; 4] = ["unknown", "restricted", "unsupported", "not_applicable"];
const NAMED_CASES: [&str; 6] = [
    "CASE-001", "CASE-002", "CASE-005", "CASE-010", "CASE-011", "CASE-015",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    #[allow(dead_code)]
    description: String,
    schema_version: u32,
    total_cases: usize,
    total_lua_only: usize,
    files: Vec<ManifestFile>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestFile {
    file: String,
    cases: usize,
    lua_only: usize,
    #[allow(dead_code)]
    note: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseFile {
    #[allow(dead_code)]
    description: String,
    schema_version: u32,
    snapshots: BTreeMap<String, Value>,
    parties: BTreeMap<String, Value>,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    #[allow(dead_code)]
    note: Option<String>,
    case_ref: Option<String>,
    predicate: Option<Value>,
    predicate_json: Option<String>,
    /// Only the Lua decoder's byte budget reads this; Rust cases never set it.
    #[allow(dead_code)]
    predicate_json_trailing_spaces: Option<usize>,
    snapshot: String,
    party: Option<String>,
    freshness: FreshnessSpec,
    stale_policy: StalePolicySpec,
    limits: LimitsSpec,
    #[serde(default)]
    lua_only: bool,
    expected: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FreshnessSpec {
    now_utc_ms: Option<i64>,
    clock_trust: ClockTrust,
    session: Option<(String, u64)>,
    max_age_ms: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum StalePolicySpec {
    Reject,
    AllowStale,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LimitsSpec {
    depth: usize,
    nodes: usize,
    steps: usize,
}

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/predicates/v1")
}
fn read_json<T: for<'de> Deserialize<'de>>(name: &str) -> T {
    let text = std::fs::read_to_string(corpus_dir().join(name))
        .unwrap_or_else(|e| panic!("corpus file {name} is readable: {e}"));
    from_str(&text).unwrap_or_else(|e| panic!("corpus file {name} is well formed: {e}"))
}

/// Declarative generators keep depth and width cases small and independent of
/// the addon decoder's 32-level JSON nesting limit. Only a whole predicate may
/// be a generator:
///   {"generate": {"kind": "nest", "wrapper": "not", "wrappers": N, "leaf": P}}
///   {"generate": {"kind": "wide", "op": "all"|"any", "count": N, "leaf": P}}
fn expand(id: &str, node: &Value) -> Value {
    let Some(spec) = node.as_object().and_then(|o| o.get("generate")) else {
        return node.clone();
    };
    assert_eq!(node.as_object().map(Map::len), Some(1), "{id}: generate is the only member");
    let leaf = spec["leaf"].clone();
    match spec["kind"].as_str() {
        Some("nest") => {
            assert_eq!(spec["wrapper"], "not", "{id}: only the not wrapper is generated");
            let mut tree = leaf;
            for _ in 0..spec["wrappers"].as_u64().expect("wrappers is a count") {
                tree = json!({"op": "not", "predicate": tree});
            }
            tree
        }
        Some("wide") => {
            let count = spec["count"].as_u64().expect("count is a number") as usize;
            json!({"op": spec["op"], "predicates": vec![leaf; count]})
        }
        other => panic!("{id}: unknown generator kind {other:?}"),
    }
}

fn collect_ops(node: &Value, seen: &mut BTreeSet<String>) {
    match node {
        Value::Object(map) => {
            if let Some(op) = map.get("op").and_then(Value::as_str) {
                seen.insert(op.to_owned());
            }
            map.values().for_each(|v| collect_ops(v, seen));
        }
        Value::Array(items) => items.iter().for_each(|v| collect_ops(v, seen)),
        _ => {}
    }
}
fn collect_unavailable(node: &Value, seen: &mut BTreeSet<String>) {
    match node {
        Value::Object(map) => {
            if map.get("reason").and_then(Value::as_str) == Some("unavailable")
                && let Some(status) = map.get("status").and_then(Value::as_str)
            {
                seen.insert(status.to_owned());
            }
            map.values().for_each(|v| collect_unavailable(v, seen));
        }
        Value::Array(items) => items.iter().for_each(|v| collect_unavailable(v, seen)),
        _ => {}
    }
}

/// The normalized outcome of one case: the serialized `EvaluationResult`,
/// `{"error": CODE}` for an evaluation error, or `{"rejected": true}` when any
/// input fails to deserialize and validate. Error messages are never compared.
fn outcome(file: &CaseFile, case: &Case) -> Value {
    let rejected = json!({"rejected": true});
    let predicate = match (&case.predicate, &case.predicate_json) {
        (Some(value), None) => from_value::<Predicate>(expand(&case.id, value)),
        (None, Some(text)) => from_str::<Predicate>(text),
        _ => panic!("{}: exactly one of predicate and predicate_json is required", case.id),
    };
    let Ok(predicate) = predicate else {
        return rejected;
    };
    let snapshot = file
        .snapshots
        .get(&case.snapshot)
        .unwrap_or_else(|| panic!("{}: snapshot {} is declared", case.id, case.snapshot));
    let Ok(snapshot) = from_value::<CharacterSnapshot>(snapshot.clone()) else {
        return rejected;
    };
    let party = match &case.party {
        Some(name) => {
            let value = file
                .parties
                .get(name)
                .unwrap_or_else(|| panic!("{}: party {name} is declared", case.id));
            match from_value::<PartySnapshot>(value.clone()) {
                Ok(party) => Some(party),
                Err(_) => return rejected,
            }
        }
        None => None,
    };
    let spec = &case.freshness;
    let freshness = FreshnessContext {
        now_utc_ms: spec.now_utc_ms.map(|n| {
            UtcMillis::new(n).unwrap_or_else(|_| panic!("{}: now_utc_ms is in range", case.id))
        }),
        clock_trust: spec.clock_trust,
        session: spec.session.as_ref().map(|(id, elapsed)| {
            (
                SessionId::new(id.clone()).expect("session id is valid"),
                Millis::new(*elapsed).expect("session elapsed is in range"),
            )
        }),
        max_age_ms: Millis::new(spec.max_age_ms).expect("max age is in range"),
    };
    let limits = RuleLimits::new(case.limits.depth, case.limits.nodes, case.limits.steps)
        .unwrap_or_else(|_| panic!("{}: limits are within the defaults", case.id));
    let context = EvaluationContext {
        snapshot: &snapshot,
        party: party.as_ref(),
        freshness: &freshness,
        stale_policy: match case.stale_policy {
            StalePolicySpec::Reject => StalePolicy::Reject,
            StalePolicySpec::AllowStale => StalePolicy::AllowStale,
        },
        limits,
    };
    match evaluate(&predicate, &context) {
        Ok(result) => serde_json::to_value(&result).expect("result serializes"),
        Err(error) => json!({"error": serde_json::to_value(error.code).expect("code serializes")}),
    }
}

fn expected_is_well_formed(case: &Case) -> bool {
    let Some(object) = case.expected.as_object() else {
        return false;
    };
    let keys: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    keys == BTreeSet::from(["truth", "steps", "reasons", "warnings"])
        || (keys == BTreeSet::from(["error"]) && object["error"].is_string())
        || (keys == BTreeSet::from(["rejected"]) && object["rejected"] == true)
}

#[test]
fn corpus_matches_the_rust_evaluator() {
    let manifest: Manifest = read_json("manifest.json");
    assert_eq!(manifest.schema_version, 1);

    // Every JSON file in the directory must be declared; nothing is skipped silently.
    let declared: BTreeSet<String> = manifest.files.iter().map(|f| f.file.clone()).collect();
    assert_eq!(declared.len(), manifest.files.len(), "manifest lists a file twice");
    let on_disk: BTreeSet<String> = std::fs::read_dir(corpus_dir())
        .expect("corpus directory is readable")
        .map(|entry| entry.expect("directory entry").file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".json") && name != "manifest.json")
        .collect();
    assert_eq!(declared, on_disk, "manifest and corpus directory disagree");

    let dump_path = std::env::var_os("PREDICATE_PARITY_DUMP");
    let mut dump = Map::new();
    let (mut run, mut skipped) = (0usize, 0usize);
    let mut ids = BTreeSet::new();
    let (mut ops, mut statuses, mut refs) = (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
    let (mut truths, mut errors) = (BTreeSet::new(), BTreeSet::new());
    let mut rejected_seen = false;
    let mut failures = Vec::new();

    for entry in &manifest.files {
        let file: CaseFile = read_json(&entry.file);
        assert_eq!(file.schema_version, 1, "{}", entry.file);
        assert_eq!(file.cases.len(), entry.cases, "{}: declared case count", entry.file);
        let lua_only = file.cases.iter().filter(|c| c.lua_only).count();
        assert_eq!(lua_only, entry.lua_only, "{}: declared lua_only count", entry.file);
        for case in &file.cases {
            assert!(ids.insert(case.id.clone()), "duplicate case id {}", case.id);
            assert!(expected_is_well_formed(case), "{}: malformed expected", case.id);
            if let Some(reference) = &case.case_ref {
                refs.insert(reference.clone());
            }
            if case.lua_only {
                skipped += 1;
                assert!(
                    case.expected == json!({"rejected": true}),
                    "{}: lua_only cases document decoder rejections",
                    case.id
                );
                continue;
            }
            assert!(
                case.predicate_json_trailing_spaces.is_none(),
                "{}: padding applies to lua_only cases",
                case.id
            );
            run += 1;
            let actual = outcome(&file, case);
            if dump_path.is_some() {
                dump.insert(case.id.clone(), actual.clone());
            }
            if actual != case.expected {
                failures.push(format!(
                    "{} ({}): expected {} but got {actual}",
                    case.id, entry.file, case.expected
                ));
            }
            if let Some(object) = actual.as_object() {
                if let Some(truth) = object.get("truth").and_then(Value::as_str) {
                    truths.insert(truth.to_owned());
                    if let Some(source) = &case.predicate {
                        collect_ops(&expand(&case.id, source), &mut ops);
                    }
                    collect_unavailable(&actual, &mut statuses);
                } else if let Some(code) = object.get("error").and_then(Value::as_str) {
                    errors.insert(code.to_owned());
                } else {
                    rejected_seen = true;
                }
            }
        }
    }
    if let Some(path) = dump_path {
        let text = serde_json::to_string_pretty(&Value::Object(dump)).expect("dump serializes");
        std::fs::write(path, text).expect("dump path is writable");
    }
    assert!(failures.is_empty(), "parity mismatches:\n{}", failures.join("\n"));

    // The manifest totals describe exactly what was consumed.
    assert_eq!(run + skipped, manifest.total_cases, "total case count");
    assert_eq!(skipped, manifest.total_lua_only, "total lua_only count");
    assert!(run > 0 && skipped > 0, "both executed and skipped cases exist");
    println!("predicate parity: {run} executed, {skipped} lua_only skipped");

    // Coverage the corpus must keep.
    let want = |names: &[&str]| names.iter().map(|s| (*s).to_owned()).collect::<BTreeSet<_>>();
    assert_eq!(ops, want(&OPERATORS), "every operator is evaluated at least once");
    assert_eq!(statuses, want(&UNAVAILABLE_STATUSES), "all four unavailable statuses");
    assert!(refs.is_superset(&want(&NAMED_CASES)), "named CASE references: {refs:?}");
    assert_eq!(truths, want(&["true", "false", "unknown"]), "all three truth values");
    assert!(errors.is_superset(&want(&["BUILD_MISMATCH", "INVALID_INPUT"])), "{errors:?}");
    assert!(rejected_seen, "rejected inputs are covered");
}
