//! Exact synthetic gear scoring in repository-owned Lua 5.1 modules.
mod common;

use mlua::{Table, Value as LuaValue};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn json_value(value: LuaValue) -> Value {
    match value {
        LuaValue::Nil => Value::Null,
        LuaValue::Boolean(b) => Value::Bool(b),
        LuaValue::Integer(n) => json!(n),
        LuaValue::Number(n) => {
            if n.fract() == 0.0 {
                json!(n as i64)
            } else {
                json!(n)
            }
        }
        LuaValue::String(s) => json!(s.to_str().expect("UTF-8").to_string()),
        LuaValue::Table(t) => {
            let pairs: Vec<_> = t
                .pairs::<LuaValue, LuaValue>()
                .map(Result::unwrap)
                .collect();
            if pairs.is_empty()
                || pairs
                    .iter()
                    .all(|(k, _)| matches!(k, LuaValue::Integer(_) | LuaValue::Number(_)))
            {
                let mut values: Vec<(usize, Value)> = pairs
                    .into_iter()
                    .map(|(k, v)| {
                        let index = match k {
                            LuaValue::Integer(n) => n as usize,
                            LuaValue::Number(n) => n as usize,
                            _ => unreachable!(),
                        };
                        (index, json_value(v))
                    })
                    .collect();
                values.sort_by_key(|(i, _)| *i);
                Value::Array(values.into_iter().map(|(_, v)| v).collect())
            } else {
                Value::Object(
                    pairs
                        .into_iter()
                        .map(|(k, v)| {
                            let LuaValue::String(key) = k else {
                                panic!("string key")
                            };
                            (key.to_str().unwrap().to_string(), json_value(v))
                        })
                        .collect(),
                )
            }
        }
        other => panic!("unexpected output {other:?}"),
    }
}

fn read(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(common::path(&format!("fixtures/gear-lua/v1/{name}"))).unwrap(),
    )
    .unwrap()
}

#[test]
fn compiled_synthetic_scores_components_certainty_and_ties_match_rust() {
    let sandbox = common::strict_core();
    let manifest = read("manifest.json");
    let file = read("cases.json");
    assert_eq!(
        file["cases"].as_array().unwrap().len(),
        manifest["files"][0]["cases"].as_u64().unwrap() as usize
    );
    let mut ids = BTreeSet::new();
    let mut comparisons = 0;
    for case in file["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        assert!(ids.insert(id));
        assert_eq!(case["synthetic"], true);
        sandbox
            .ns()
            .set(
                "gear_text",
                serde_json::to_string(&case["compiled"]).unwrap(),
            )
            .unwrap();
        sandbox
            .run::<()>(id, "ns.scorer = assert(ns.Gear.new(ns.gear_text))")
            .unwrap();
        for (indices, expected) in case["selections"]
            .as_array()
            .unwrap()
            .iter()
            .zip(case["scores"].as_array().unwrap())
        {
            let list: Vec<i64> = indices
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap())
                .collect();
            sandbox
                .ns()
                .set("indices", sandbox.lua().create_sequence_from(list).unwrap())
                .unwrap();
            let actual: Table = sandbox.run(id, "local result, code = ns.scorer:score(ns.indices); return result or {error=code}").unwrap();
            assert_eq!(json_value(LuaValue::Table(actual)), *expected, "{id}");
            comparisons += 1;
        }
        sandbox
            .ns()
            .set(
                "selections_text",
                serde_json::to_string(&case["selections"]).unwrap(),
            )
            .unwrap();
        let ranked: Table = sandbox.run(id, "local result, code = ns.scorer:rank(assert(ns.Decode.decode(ns.selections_text))); return result or {error=code}").unwrap();
        assert_eq!(
            json_value(LuaValue::Table(ranked)),
            case["ranking"],
            "{id}: tie ordering"
        );
        // Mutating a returned score cannot affect the private loaded profile/key state.
        sandbox
            .run::<()>(
                id,
                r#"
          local score = ns.scorer:score({})
          score.profile_unmodeled[1] = 'tampered'
          score.weight_source.type = 'tampered'
          assert(ns.scorer:score({}).profile_unmodeled[1] == 'synthetic set bonuses')
          assert(ns.scorer:score({}).weight_source.type == 'synthetic_test')
        "#,
            )
            .unwrap();
    }
    assert!(comparisons >= 30);
    println!(
        "Lua gear: {} synthetic catalogs, {comparisons} score comparisons",
        ids.len()
    );
}

#[test]
fn exact_fixed_point_matches_rust_at_halves_safe_limits_and_large_products() {
    let sandbox = common::strict_core();
    let file = read("arithmetic.json");
    let manifest = read("manifest.json");
    for field in ["to_fixed", "fixed_mul"] {
        assert_eq!(
            file[field].as_array().unwrap().len(),
            manifest["files"][1][field].as_u64().unwrap() as usize
        );
    }
    let mut count = 0;
    for row in file["to_fixed"].as_array().unwrap() {
        sandbox
            .ns()
            .set("value", row["value"].as_f64().unwrap())
            .unwrap();
        let (actual, code): (Option<i64>, Option<String>) = sandbox
            .run("to-fixed", "return ns.Gear.toFixed(ns.value)")
            .unwrap();
        let actual = actual.map_or_else(|| json!({"error":code.unwrap()}), |n| json!(n));
        assert_eq!(actual, row["expected"], "value {}", row["value"]);
        count += 1;
    }
    for row in file["fixed_mul"].as_array().unwrap() {
        sandbox.ns().set("a", row["a"].as_i64().unwrap()).unwrap();
        sandbox.ns().set("b", row["b"].as_i64().unwrap()).unwrap();
        let (actual, code): (Option<i64>, Option<String>) = sandbox
            .run("multiply", "return ns.Gear.fixedMul(ns.a, ns.b)")
            .unwrap();
        let actual = actual.map_or_else(|| json!({"error":code.unwrap()}), |n| json!(n));
        assert_eq!(actual, row["expected"], "{} * {}", row["a"], row["b"]);
        count += 1;
    }
    println!("Lua gear arithmetic: {count} exact Rust vectors");
}

#[test]
fn gear_import_and_selection_reject_invalid_boundaries_without_a_live_claim() {
    let sandbox = common::strict_core();
    let base = read("cases.json")["cases"][0]["compiled"].clone();
    let reject = |input: Value| {
        sandbox
            .ns()
            .set("gear_text", serde_json::to_string(&input).unwrap())
            .unwrap();
        assert!(
            sandbox
                .run::<bool>("reject", "return ns.Gear.new(ns.gear_text) == nil")
                .unwrap()
        );
    };
    let mut input = base.clone();
    input["fixture"] = json!(false);
    reject(input);
    let mut input = base.clone();
    input["context"]["namespace"] = json!("reference");
    reject(input);
    let mut input = base.clone();
    input["profile"]["version"] = json!("unknown-model");
    reject(input);
    let mut input = base.clone();
    input["profile"]["weight_source"]["type"] = json!("user_preference");
    reject(input);
    let mut input = base.clone();
    input["profile"]["weights"]["synthetic-power"] = json!(1500.0);
    reject(input);
    let mut input = base.clone();
    input["candidates"][0]["attributes_milli"]["source"] = json!("addon_snapshot");
    reject(input);
    let mut input = base.clone();
    input["candidates"][0]["key"]["context"]["build"]["edition"] = json!("different");
    reject(input);
    let mut input = base.clone();
    input["candidates"] = json!(vec![base["candidates"][0].clone(); 65]);
    reject(input);
    sandbox
        .ns()
        .set("gear_text", serde_json::to_string(&base).unwrap())
        .unwrap();
    sandbox
        .run::<()>(
            "selection",
            r#"
      local scorer = assert(ns.Gear.new(ns.gear_text))
      assert(scorer:score({1,1}) == nil and scorer:score({2}) == nil and scorer:score({0}) == nil)
      assert(scorer:rank({{1},{1}}) == nil)
      assert(scorer:rank(string.rep('x', 500)) == nil)
      assert(ns.Gear.fixedMul(0/0, 1) == nil and ns.Gear.toFixed(math.huge) == nil)
      local complete = scorer:score({1})
      local empty = scorer:score({})
      assert(ns.Gear.definitivelyExceeds(complete, empty))
      complete.certainty = 'reduced'
      assert(not ns.Gear.definitivelyExceeds(complete, empty))
    "#,
        )
        .unwrap();
}
