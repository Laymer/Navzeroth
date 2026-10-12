//! The Rust-anchored corpus is consumed unchanged, including Lua-only decoder traps.
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

const EVALUATE: &str = r#"
local pack = ns.Decode.decode(ns.case_text, {bytes=262144, depth=64, entries=2048, nodes=32768})
if not pack then return {rejected=true} end
local case, predicate = pack.case, nil
if case.predicate_json ~= nil then
  predicate = ns.Decode.decode(case.predicate_json .. string.rep(' ', case.predicate_json_trailing_spaces or 0))
else
  predicate = case.predicate
  if predicate.generate then
    local spec = predicate.generate
    predicate = spec.leaf
    if spec.kind == 'nest' then
      for _ = 1, spec.wrappers do predicate = {op='not', predicate=predicate} end
    else
      local children = {}
      for i = 1, spec.count do children[i] = predicate end
      predicate = {op=spec.op, predicates=children}
    end
  end
end
if not predicate or not ns.Predicate.validateInputs(predicate, pack.snapshot, pack.party) then
  return {rejected=true}
end
return ns.Predicate.evaluate(predicate, {snapshot=pack.snapshot, party=pack.party,
  freshness=case.freshness, stale_policy=case.stale_policy, limits=case.limits})
"#;

#[test]
fn all_rust_anchored_cases_and_lua_decoder_rejections_agree() {
    let sandbox = common::strict_core();
    let root = common::path("fixtures/predicates/v1");
    let read = |name: &str| -> Value {
        serde_json::from_str(&std::fs::read_to_string(root.join(name)).unwrap()).unwrap()
    };
    let manifest = read("manifest.json");
    let mut ids = BTreeSet::new();
    let (mut total, mut lua_only) = (0, 0);
    let mut failures = Vec::new();
    for entry in manifest["files"].as_array().unwrap() {
        let file = read(entry["file"].as_str().unwrap());
        assert_eq!(
            file["cases"].as_array().unwrap().len(),
            entry["cases"].as_u64().unwrap() as usize
        );
        for case in file["cases"].as_array().unwrap() {
            let id = case["id"].as_str().unwrap();
            assert!(ids.insert(id.to_owned()), "duplicate {id}");
            total += 1;
            lua_only += usize::from(case["lua_only"] == true);
            let party = case["party"]
                .as_str()
                .map(|name| file["parties"][name].clone())
                .unwrap_or(Value::Null);
            let pack = json!({"case":case,"snapshot":file["snapshots"][case["snapshot"].as_str().unwrap()],"party":party});
            sandbox
                .ns()
                .set("case_text", serde_json::to_string(&pack).unwrap())
                .unwrap();
            let actual: Table = sandbox.run(id, EVALUATE).unwrap();
            let actual = json_value(LuaValue::Table(actual));
            if actual != case["expected"] {
                failures.push(format!("{id}: expected {} got {actual}", case["expected"]));
            }
        }
    }
    assert_eq!(total, manifest["total_cases"].as_u64().unwrap() as usize);
    assert_eq!(
        lua_only,
        manifest["total_lua_only"].as_u64().unwrap() as usize
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    println!("Lua parity: {total} cases including {lua_only} Lua-only rejections");
}
