//! Property tests (plan section 18.4). Seeds are fixed and case counts are bounded,
//! so a failure reproduces identically on every machine and in CI.
mod common;
use azeroth_core::contract::decode_any;
use azeroth_ingest::{
    anav1::{PREFIX, decode_text, encode_text},
    savedvars::{self, LuaKey, LuaValue},
    session,
};
use common::contract;
use proptest::{
    prelude::*,
    test_runner::{Config, RngAlgorithm, TestRng, TestRunner},
};

const CASES: u32 = 128;

fn runner() -> TestRunner {
    let config = Config {
        cases: CASES,
        failure_persistence: None,
        max_shrink_iters: 256,
        ..Config::default()
    };
    TestRunner::new_with_rng(config, TestRng::from_seed(RngAlgorithm::ChaCha, &[7u8; 32]))
}

fn framing() -> impl Strategy<Value = String> {
    proptest::collection::vec(prop::sample::select(vec![' ', '\t', '\r', '\n']), 0..8)
        .prop_map(|v| v.into_iter().collect())
}

#[test]
fn anav1_round_trip_survives_any_ascii_whitespace_framing() {
    let exchange = decode_any(&contract("character-snapshot.json")).unwrap();
    let text = encode_text(&exchange).unwrap();
    let document = text[PREFIX.len()..].to_owned();
    runner()
        .run(&(framing(), framing()), |(before, after)| {
            let framed = format!("{before}{text}{after}");
            prop_assert_eq!(decode_text(&framed).unwrap(), document.as_str());
            prop_assert_eq!(
                decode_any(decode_text(&framed).unwrap()).unwrap(),
                exchange.clone()
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn anav1_decode_is_total_on_arbitrary_text() {
    // Never panics; either a document slice that starts with an object, or an error.
    runner()
        .run(&any::<String>(), |s| {
            if let Ok(document) = decode_text(&s) {
                prop_assert!(is_object(document));
            }
            if let Ok(document) = decode_text(&format!("{PREFIX}{s}")) {
                prop_assert!(is_object(document));
            }
            Ok(())
        })
        .unwrap();
}

fn lua_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c if (c as u32) < 32 || c as u32 == 127 => out.push_str(&format!("\\{:03}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn render(value: &LuaValue) -> String {
    match value {
        LuaValue::Nil => "nil".into(),
        LuaValue::Bool(b) => b.to_string(),
        LuaValue::Int(i) => i.to_string(),
        LuaValue::Float(f) => format!("{f:e}"),
        LuaValue::Str(s) => lua_string(s),
        LuaValue::Table(t) => {
            let mut out = String::from("{");
            for (key, v) in t {
                match key {
                    LuaKey::Int(i) => out.push_str(&format!("[{i}]={},", render(v))),
                    LuaKey::Str(k) => out.push_str(&format!("[{}]={},", lua_string(k), render(v))),
                }
            }
            out.push('}');
            out
        }
    }
}

fn leaf() -> impl Strategy<Value = LuaValue> {
    prop_oneof![
        any::<bool>().prop_map(LuaValue::Bool),
        any::<i64>().prop_map(LuaValue::Int),
        (-1.0e9f64..1.0e9f64).prop_map(LuaValue::Float),
        ".{0,24}".prop_map(LuaValue::Str),
        // Zero and the empty string must stay distinct from absent.
        Just(LuaValue::Int(0)),
        Just(LuaValue::Str(String::new())),
    ]
}

fn value() -> impl Strategy<Value = LuaValue> {
    leaf().prop_recursive(3, 24, 4, |inner| {
        proptest::collection::btree_map(
            prop_oneof![
                any::<i32>().prop_map(|i| LuaKey::Int(i64::from(i))),
                "[a-zA-Z_][a-zA-Z0-9_]{0,8}".prop_map(LuaKey::Str),
            ],
            inner,
            0..4,
        )
        .prop_map(LuaValue::Table)
    })
}

#[test]
fn savedvariables_literals_round_trip_through_the_parser() {
    runner()
        .run(&value(), |v| {
            let source = format!("AzerothNavigatorDB = {}\n", render(&v));
            let parsed = savedvars::parse(&source).unwrap();
            prop_assert_eq!(parsed.get("AzerothNavigatorDB"), Some(&v));
            Ok(())
        })
        .unwrap();
}

#[test]
fn savedvariables_parser_is_total_and_never_executes() {
    runner()
        .run(&any::<String>(), |s| {
            let _ = savedvars::parse(&s);
            let _ = savedvars::parse(&format!("X = {s}"));
            let _ = savedvars::parse(&format!("X = {{ [\"a\"] = \"{s}\" }}"));
            Ok(())
        })
        .unwrap();
}

#[test]
fn session_transport_round_trips_for_generated_documents() {
    runner()
        .run(
            &(
                "[a-z0-9]{1,16}",
                "[a-z0-9]{1,16}",
                0u64..1_000_000,
                framing(),
                framing(),
            ),
            |(character, session_id, elapsed, before, after)| {
                let document: azeroth_core::sessions::SessionDocument =
                    serde_json::from_value(serde_json::json!({
                        "format": "anav-session-events", "version": 1, "fixture": true,
                        "batch": {
                            "character": character,
                            "context": {"build": {"edition": "wow_forever",
                                "build": "SYNTHETIC", "interface": "SYNTHETIC"},
                                "namespace": "demo"},
                            "events": [{"session": session_id, "sequence": 1,
                                "elapsed_ms": elapsed, "kind": {"type": "session_start"}}]
                        }
                    }))
                    .unwrap();
                let text = session::encode_text(&document).unwrap();
                let framed = format!("{before}{text}{after}");
                prop_assert_eq!(session::decode_text(&framed).unwrap(), document);
                Ok(())
            },
        )
        .unwrap();
}

fn is_object(document: &str) -> bool {
    document.starts_with('{')
}
