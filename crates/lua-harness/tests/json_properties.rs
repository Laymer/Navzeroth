//! Property: whatever the addon's bounded `Json.encode` accepts, an independent
//! parser (serde_json, the desktop's decoder) reads back as the same value, and
//! the encoding is deterministic. Seeds and case counts are fixed.
mod common;

use proptest::{
    prelude::*,
    test_runner::{Config, RngAlgorithm, TestRng, TestRunner},
};
use serde_json::{Map, Value};

fn lua_string(s: &str) -> String {
    // Every byte as a decimal escape: independent of the source encoding.
    let mut out = String::from("\"");
    for b in s.bytes() {
        out.push_str(&format!("\\{b:03}"));
    }
    out.push('"');
    out
}

fn lua_literal(value: &Value) -> String {
    match value {
        Value::Null => unreachable!("generator never emits null"),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => lua_string(s),
        Value::Array(items) => {
            let inner: Vec<_> = items.iter().map(lua_literal).collect();
            format!("{{{}}}", inner.join(","))
        }
        Value::Object(map) => {
            let inner: Vec<_> = map
                .iter()
                .map(|(k, v)| format!("[{}]={}", lua_string(k), lua_literal(v)))
                .collect();
            format!("{{{}}}", inner.join(","))
        }
    }
}

fn leaf() -> impl Strategy<Value = Value> {
    prop_oneof![
        any::<bool>().prop_map(Value::Bool),
        (-9_007_199_254_740_991i64..9_007_199_254_740_991i64).prop_map(Value::from),
        ".{1,24}".prop_map(Value::String),
        Just(Value::from(0)),
    ]
}

fn json_value() -> impl Strategy<Value = Value> {
    leaf().prop_recursive(3, 24, 4, |inner| {
        prop_oneof![
            proptest::collection::vec(inner.clone(), 1..4).prop_map(Value::Array),
            proptest::collection::btree_map("[a-z_][a-z0-9_]{0,6}", inner, 1..4)
                .prop_map(|m| Value::Object(m.into_iter().collect::<Map<_, _>>())),
        ]
    })
}

#[test]
fn encoded_json_parses_back_to_the_same_value_and_is_deterministic() {
    let sandbox = common::strict_core();
    let config = Config {
        cases: 128,
        failure_persistence: None,
        max_shrink_iters: 128,
        ..Config::default()
    };
    let mut runner = TestRunner::new_with_rng(
        config,
        TestRng::from_seed(RngAlgorithm::ChaCha, &[11u8; 32]),
    );
    runner
        .run(&json_value(), |value| {
            let literal = lua_literal(&value);
            let (text, code) = common::encode_pair(
                &sandbox,
                &format!("local v = {literal}; local a = ns.Json.encode(v); return a, nil"),
            );
            let text = text.unwrap_or_else(|| panic!("encode refused {literal}: {code:?}"));
            let parsed: Value = serde_json::from_str(&text).unwrap();
            // Whole-number floats may come back as integers; compare numerically.
            prop_assert!(equal(&parsed, &value), "{text} != {value}");
            let (again, _) =
                common::encode_pair(&sandbox, &format!("return ns.Json.encode({literal}), nil"));
            prop_assert_eq!(again.as_deref(), Some(text.as_str()));
            Ok(())
        })
        .unwrap();
}

#[test]
fn non_integers_are_refused_with_a_distinct_code_never_rounded() {
    let sandbox = common::strict_core();
    let config = Config {
        cases: 64,
        failure_persistence: None,
        ..Config::default()
    };
    let mut runner = TestRunner::new_with_rng(
        config,
        TestRng::from_seed(RngAlgorithm::ChaCha, &[13u8; 32]),
    );
    // Quarter steps are exactly representable; odd multiples always have a fraction.
    runner
        .run(&(-1_000_000i64..1_000_000i64), |n| {
            let fractional = n * 2 + 1;
            let (text, code) = common::encode_pair(
                &sandbox,
                &format!("return ns.Json.encode({{x = {}}})", fractional as f64 / 4.0),
            );
            prop_assert_eq!(text, None);
            prop_assert_eq!(code.as_deref(), Some("json_number_not_integer"));
            Ok(())
        })
        .unwrap();
}

fn equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| equal(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| equal(v, w)))
        }
        _ => a == b,
    }
}
