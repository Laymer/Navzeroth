//! Rust anchors the synthetic compiled-input and exact arithmetic Lua fixtures.
use azeroth_core::{
    gear::{lua::*, *},
    types::*,
};
use serde_json::{Value, from_value, json};
use std::{collections::BTreeSet, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/gear-lua/v1")
}
fn read(name: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(root().join(name)).unwrap()).unwrap()
}
fn parse(case: &Value) -> (DataContext, RoleProfile, Vec<GearCandidate>) {
    (
        from_value(case["context"].clone()).unwrap(),
        from_value(case["profile"].clone()).unwrap(),
        from_value(case["candidates"].clone()).unwrap(),
    )
}
fn check_or_update(file: &mut Value, key: &str, actual: Value, update: bool) {
    if update {
        file[key] = actual;
    } else {
        assert_eq!(file[key], actual, "{key}");
    }
}
fn normalized<T: serde::Serialize>(result: azeroth_core::error::Result<T>) -> Value {
    match result {
        Ok(value) => json!(value),
        Err(error) => json!({"error":error.code}),
    }
}

#[test]
fn compiled_inputs_components_and_ordering_match_committed_rust_anchors() {
    let update = std::env::var_os("UPDATE_LUA_GEAR_FIXTURES").is_some();
    let manifest = read("manifest.json");
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["synthetic"], true);
    let declared: BTreeSet<_> = manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["file"].as_str().unwrap().to_owned())
        .collect();
    let on_disk: BTreeSet<_> = std::fs::read_dir(root())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".json") && name != "manifest.json")
        .collect();
    assert_eq!(declared, on_disk);
    let mut file = read("cases.json");
    let cases = file["cases"].as_array_mut().unwrap();
    assert_eq!(
        cases.len(),
        manifest["files"][0]["cases"].as_u64().unwrap() as usize
    );
    let mut ids = BTreeSet::new();
    for case in cases {
        assert_eq!(case["synthetic"], true);
        assert!(ids.insert(case["id"].as_str().unwrap().to_owned()));
        let (context, profile, candidates) = parse(case);
        let compiled = compile_synthetic(&context, &profile, &candidates).unwrap();
        assert!(encode(&compiled).unwrap().len() <= MAX_LUA_GEAR_BYTES);
        let mut selections = Vec::new();
        let mut scores = Vec::new();
        let mut entries = Vec::new();
        let mut rank_error = None;
        for selection in case["configurations"].as_array().unwrap() {
            let selected: Vec<_> = selection
                .as_array()
                .unwrap()
                .iter()
                .map(|i| &candidates[i.as_u64().unwrap() as usize])
                .collect();
            let indices: Vec<usize> = selected
                .iter()
                .map(|c| {
                    compiled
                        .candidates
                        .iter()
                        .position(|item| item.key == *c.key())
                        .unwrap()
                        + 1
                })
                .collect();
            selections.push(indices);
            let score = score_candidates(&profile, &selected, vec![]);
            scores.push(normalized(score.clone()));
            match score {
                Ok(score) => {
                    let mut keys: Vec<_> = selected.iter().map(|c| c.key().clone()).collect();
                    keys.sort();
                    entries.push((score, keys));
                }
                Err(error) => rank_error = Some(error.code),
            }
        }
        entries.sort_by(|a, b| {
            b.0.total
                .cmp(&a.0.total)
                .then(a.0.certainty.cmp(&b.0.certainty))
                .then(a.1.cmp(&b.1))
        });
        let ranking = rank_error.map_or_else(
            || {
                json!(
                    entries
                        .iter()
                        .map(|(score, keys)| json!({"score":score,"keys":keys}))
                        .collect::<Vec<_>>()
                )
            },
            |error| json!({"error":error}),
        );
        check_or_update(case, "compiled", json!(compiled), update);
        check_or_update(case, "selections", json!(selections), update);
        check_or_update(case, "scores", json!(scores), update);
        check_or_update(case, "ranking", ranking, update);
        // Pin the same ties against the existing configuration ranking, not just
        // the test's independent comparator. Every item occupies one shared slot.
        if matches!(
            case["id"].as_str(),
            Some("full-key-variant-ties" | "certainty-before-key-tie")
        ) {
            let layout: SlotLayout =
                from_value(json!({"slots":["synthetic-slot"],"hands":null})).unwrap();
            let ranked = rank_theoretical(&GearScenario {
                context: &context,
                profile: &profile,
                layout: &layout,
                candidates: &candidates,
            })
            .unwrap();
            let ranked_keys: Vec<_> = ranked
                .valid
                .iter()
                .map(|entry| {
                    let mut keys: Vec<_> = entry
                        .configuration
                        .items
                        .values()
                        .map(|item| item.item.clone())
                        .collect();
                    keys.sort();
                    keys
                })
                .collect();
            let fixture_keys: Vec<_> = entries
                .iter()
                .filter(|(_, keys)| !keys.is_empty())
                .map(|(_, keys)| keys.clone())
                .collect();
            assert_eq!(ranked_keys, fixture_keys);
        }
    }
    if update {
        std::fs::write(
            root().join("cases.json"),
            serde_json::to_string_pretty(&file).unwrap() + "\n",
        )
        .unwrap();
    }
}

#[test]
fn arithmetic_vectors_match_the_established_rust_rule() {
    let update = std::env::var_os("UPDATE_LUA_GEAR_FIXTURES").is_some();
    let mut file = read("arithmetic.json");
    let manifest = read("manifest.json");
    for field in ["to_fixed", "fixed_mul"] {
        assert_eq!(
            file[field].as_array().unwrap().len(),
            manifest["files"][1][field].as_u64().unwrap() as usize
        );
    }
    for row in file["to_fixed"].as_array_mut().unwrap() {
        let actual = normalized(to_fixed_point(
            FiniteNumber::new(row["value"].as_f64().unwrap()).unwrap(),
        ));
        check_or_update(row, "expected", actual, update);
    }
    for row in file["fixed_mul"].as_array_mut().unwrap() {
        let actual = normalized(fixed_mul(
            row["a"].as_i64().unwrap(),
            row["b"].as_i64().unwrap(),
        ));
        check_or_update(row, "expected", actual, update);
    }
    if update {
        std::fs::write(
            root().join("arithmetic.json"),
            serde_json::to_string_pretty(&file).unwrap() + "\n",
        )
        .unwrap();
    }
}

#[test]
fn compiler_rejects_live_mixed_oversized_duplicate_and_out_of_range_inputs() {
    let file = read("cases.json");
    let base = &file["cases"][0];
    let (context, profile, candidates) = parse(base);
    let encoded = encode(&compile_synthetic(&context, &profile, &candidates).unwrap()).unwrap();
    let decoded: LuaGearInput = serde_json::from_str(&encoded).unwrap();
    assert_eq!(encode(&decoded).unwrap(), encoded);
    let mut foreign = candidates.clone();
    let mut value = json!(foreign[0]);
    value["item"]["key"]["context"]["build"]["interface"] = json!("foreign");
    foreign[0] = from_value(value).unwrap();
    assert!(compile_synthetic(&context, &profile, &foreign).is_err());
    let mut live = json!(profile);
    live["weight_source"] = json!({"type":"user_preference"});
    assert!(compile_synthetic(&context, &from_value(live).unwrap(), &candidates).is_err());
    assert!(compile_synthetic(&context, &profile, &vec![candidates[0].clone(); 65]).is_err());
    assert!(
        compile_synthetic(
            &context,
            &profile,
            &[candidates[0].clone(), candidates[0].clone()]
        )
        .is_err()
    );
    let mut huge = json!(candidates[0]);
    huge["item"]["attributes"]["value"] = json!({"synthetic-power":9_000_000_000_000_000_i64});
    assert!(compile_synthetic(&context, &profile, &[from_value(huge).unwrap()]).is_err());
    let mut source = json!(candidates[0]);
    source["item"]["attributes"]["source"] = json!("addon_snapshot");
    assert!(compile_synthetic(&context, &profile, &[from_value(source).unwrap()]).is_err());
    let mut oversized = json!(candidates[0]);
    oversized["item"]["effects"]["value"] = json!(vec!["x".repeat(4096); 8]);
    assert!(compile_synthetic(&context, &profile, &[from_value(oversized).unwrap()]).is_err());
}
