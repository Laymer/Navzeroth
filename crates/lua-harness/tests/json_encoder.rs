//! The bounded JSON encoder, checked against serde_json as an independent parser.
mod common;

use azeroth_lua_harness::{EnvMode, Sandbox};
use common::*;
use mlua::{UserData, Value as LuaValue};
use serde_json::Value;

struct Opaque;
impl UserData for Opaque {}

fn core() -> Sandbox {
    strict_core()
}

fn encode(sandbox: &Sandbox, expr: &str) -> (Option<String>, Option<String>) {
    encode_pair(sandbox, &format!("return ns.Json.encode({expr})"))
}

fn ok(sandbox: &Sandbox, expr: &str) -> String {
    match encode(sandbox, expr) {
        (Some(text), None) => text,
        other => panic!("{expr} failed: {other:?}"),
    }
}

fn code(sandbox: &Sandbox, expr: &str) -> String {
    match encode(sandbox, expr) {
        (None, Some(code)) => code,
        other => panic!("{expr} should fail, got {other:?}"),
    }
}

#[test]
fn limits_match_the_documented_bounds() {
    let sandbox = core();
    let (depth, output, string): (i64, i64, i64) = sandbox
        .run(
            "limits",
            "local l = ns.Json.limits return l.max_depth, l.max_output_bytes, l.max_string_bytes",
        )
        .unwrap();
    assert_eq!(depth, 32, "plan section 7.1 and MAX_ADDON_JSON_DEPTH");
    assert!(output < 32 * 1024, "addon text bound of section 7.1");
    assert_eq!(string, 512);
}

#[test]
fn keys_are_sorted_and_output_is_deterministic_whatever_the_insertion_order() {
    let sandbox = core();
    let a = ok(
        &sandbox,
        "{ zebra = 1, apple = 2, mango = { y = true, x = false } }",
    );
    let b = ok(
        &sandbox,
        "{ mango = { x = false, y = true }, apple = 2, zebra = 1 }",
    );
    assert_eq!(a, r#"{"apple":2,"mango":{"x":false,"y":true},"zebra":1}"#);
    assert_eq!(a, b);
    let parsed: Value = serde_json::from_str(&a).unwrap();
    assert_eq!(parsed["mango"]["y"], true);
    assert_eq!(ok(&sandbox, "{ 'a', 'b', { 1, 2 } }"), r#"["a","b",[1,2]]"#);
    assert_eq!(ok(&sandbox, "{}"), "[]");
    assert_eq!(ok(&sandbox, "{ [10] = nil, k = 'v' }"), r#"{"k":"v"}"#);
}

#[test]
fn strings_round_trip_through_serde_json() {
    let sandbox = core();
    let samples = [
        "plain",
        "quote \" and backslash \\ and slash /",
        "controls \u{0} \u{1} \u{8} \u{c} \n \r \t \u{1f} \u{7f}",
        "unicode \u{e9} \u{4e2d}\u{6587} \u{1f600} \u{10ffff} \u{2028}",
        "",
    ];
    for sample in samples {
        sandbox
            .ns()
            .set("sample", sandbox.lua().create_string(sample).unwrap())
            .unwrap();
        let text = ok(&sandbox, "ns.sample");
        let decoded: String = serde_json::from_str(&text).unwrap();
        assert_eq!(decoded, sample);
        assert!(
            !text.contains(|c: char| c.is_control()),
            "raw control byte in {text:?}"
        );
    }
}

#[test]
fn invalid_utf8_and_oversize_strings_are_rejected() {
    let sandbox = core();
    let bad: [&[u8]; 6] = [
        &[0xff],
        &[0xc0, 0xaf],
        &[0xed, 0xa0, 0x80],
        &[0xf4, 0x90, 0x80, 0x80],
        &[0xe2, 0x82],
        &[b'a', 0x80],
    ];
    for bytes in bad {
        sandbox
            .ns()
            .set("bad", sandbox.lua().create_string(bytes).unwrap())
            .unwrap();
        assert_eq!(code(&sandbox, "ns.bad"), "json_invalid_utf8", "{bytes:?}");
        assert_eq!(code(&sandbox, "{ [ns.bad] = 1 }"), "json_invalid_utf8");
    }
    assert!(ok(&sandbox, "string.rep('a', 512)").len() == 514);
    assert_eq!(
        code(&sandbox, "string.rep('a', 513)"),
        "json_string_too_long"
    );
    assert_eq!(
        code(&sandbox, "{ [string.rep('k', 513)] = 1 }"),
        "json_string_too_long"
    );
}

#[test]
fn numbers_are_integers_within_the_safe_range() {
    let sandbox = core();
    assert_eq!(
        ok(&sandbox, "{ 0, -0, 1, -1, 42, 2147483648, -2147483649 }"),
        "[0,0,1,-1,42,2147483648,-2147483649]"
    );
    assert_eq!(ok(&sandbox, "9007199254740991"), "9007199254740991");
    assert_eq!(ok(&sandbox, "-9007199254740991"), "-9007199254740991");
    let parsed: Value = serde_json::from_str(&ok(&sandbox, "9007199254740991")).unwrap();
    assert_eq!(parsed.as_i64(), Some(9_007_199_254_740_991));
    assert_eq!(
        code(&sandbox, "9007199254740992"),
        "json_number_out_of_range"
    );
    assert_eq!(
        code(&sandbox, "-9007199254740992"),
        "json_number_out_of_range"
    );
    assert_eq!(code(&sandbox, "1.5"), "json_number_not_integer");
    assert_eq!(code(&sandbox, "0.1"), "json_number_not_integer");
    assert_eq!(code(&sandbox, "0/0"), "json_number_nan");
    assert_eq!(code(&sandbox, "math.huge"), "json_number_infinite");
    assert_eq!(code(&sandbox, "-math.huge"), "json_number_infinite");
    assert_eq!(code(&sandbox, "{ 1, 0/0 }"), "json_number_nan");
}

#[test]
fn unsupported_values_are_rejected_and_never_serialized() {
    let sandbox = core();
    assert_eq!(code(&sandbox, "nil"), "json_type");
    assert_eq!(code(&sandbox, "function() end"), "json_type");
    assert_eq!(code(&sandbox, "{ a = function() end }"), "json_type");
    assert_eq!(code(&sandbox, "{ 1, tostring }"), "json_type");
    sandbox
        .ns()
        .set("opaque", sandbox.lua().create_userdata(Opaque).unwrap())
        .unwrap();
    assert_eq!(code(&sandbox, "ns.opaque"), "json_type");
    assert_eq!(code(&sandbox, "{ nested = { ns.opaque } }"), "json_type");
    let thread = sandbox
        .lua()
        .create_thread(sandbox.lua().create_function(|_, ()| Ok(())).unwrap())
        .unwrap();
    sandbox
        .ns()
        .set("thread", LuaValue::Thread(thread))
        .unwrap();
    assert_eq!(code(&sandbox, "ns.thread"), "json_type");
    assert_eq!(code(&sandbox, "setmetatable({}, {})"), "json_type");
    assert_eq!(code(&sandbox, "{ [true] = 1 }"), "json_key");
    assert_eq!(code(&sandbox, "{ [{}] = 1 }"), "json_key");
    assert_eq!(code(&sandbox, "{ [1.5] = 1 }"), "json_key");
    assert_eq!(code(&sandbox, "{ [0] = 'a' }"), "json_key");
    assert_eq!(code(&sandbox, "{ [-1] = 'a' }"), "json_key");
    assert_eq!(
        code(&sandbox, "{ 1, 2, [4] = 4 }"),
        "json_key",
        "sparse array"
    );
    assert_eq!(code(&sandbox, "{ 1, 2, name = 'mixed' }"), "json_key");
}

#[test]
fn failure_returns_exactly_nil_and_a_code() {
    let sandbox = core();
    let (count, first, second): (i64, Option<String>, Option<String>) = sandbox
        .run(
            "pair",
            "local a, b = ns.Json.encode(0/0) return select('#', ns.Json.encode(0/0)), a, b",
        )
        .unwrap();
    assert_eq!(count, 2);
    assert!(first.is_none());
    assert_eq!(second.as_deref(), Some("json_number_nan"));
    let success: i64 = sandbox
        .run("single", "return select('#', ns.Json.encode({}))")
        .unwrap();
    assert_eq!(success, 1);
}

#[test]
fn cycles_are_detected_and_shared_references_are_not_cycles() {
    let sandbox = core();
    let direct = "(function() local t = {} t.self = t return t end)()";
    assert_eq!(code(&sandbox, direct), "json_cycle");
    let mutual = "(function() local a, b = {}, {} a.b = b b.a = a return a end)()";
    assert_eq!(code(&sandbox, mutual), "json_cycle");
    let array = "(function() local t = {} t[1] = t return t end)()";
    assert_eq!(code(&sandbox, array), "json_cycle");
    let shared = "(function() local s = { v = 1 } return { first = s, second = s } end)()";
    assert_eq!(
        ok(&sandbox, shared),
        r#"{"first":{"v":1},"second":{"v":1}}"#
    );
}

fn nested(depth: usize) -> String {
    format!("(function() local t = 1 for _ = 1, {depth} do t = {{ t }} end return t end)()")
}

#[test]
fn depth_is_bounded_at_32_matching_the_rust_side() {
    let sandbox = core();
    let at_limit = ok(&sandbox, &nested(32));
    let parsed: Value = serde_json::from_str(&at_limit).unwrap();
    assert_eq!(json_depth(&parsed), 32);
    assert_eq!(code(&sandbox, &nested(33)), "json_depth");
    assert_eq!(code(&sandbox, &nested(200)), "json_depth");
    let objects = "(function() local t = 1 for _ = 1, 33 do t = { k = t } end return t end)()";
    assert_eq!(code(&sandbox, objects), "json_depth");
}

#[test]
fn element_node_and_output_bounds_are_enforced() {
    let sandbox = core();
    let array = |n: usize| {
        format!("(function() local t = {{}} for i = 1, {n} do t[i] = i end return t end)()")
    };
    assert!(ok(&sandbox, &array(256)).starts_with("[1,2,3"));
    assert_eq!(code(&sandbox, &array(257)), "json_too_many_elements");
    let object = |n: usize| {
        format!("(function() local t = {{}} for i = 1, {n} do t['k' .. i] = i end return t end)()")
    };
    let sixty_four: Value = serde_json::from_str(&ok(&sandbox, &object(64))).unwrap();
    assert_eq!(sixty_four.as_object().unwrap().len(), 64);
    assert_eq!(code(&sandbox, &object(65)), "json_too_many_elements");
    // 9 arrays of 256 elements = 2313 values > 2048 nodes, yet each array is legal.
    let wide = "(function() local r = {} for i = 1, 9 do local t = {} for j = 1, 256 do t[j] = 1 end r[i] = t end return r end)()";
    assert_eq!(code(&sandbox, wide), "json_too_many_elements");
    // Output cap: strings are individually legal but together exceed 16 KiB.
    let heavy =
        "(function() local t = {} for i = 1, 40 do t[i] = string.rep('x', 512) end return t end)()";
    assert_eq!(code(&sandbox, heavy), "json_output_too_large");
    let fits =
        "(function() local t = {} for i = 1, 30 do t[i] = string.rep('x', 512) end return t end)()";
    let text = ok(&sandbox, fits);
    assert!(text.len() <= 16384);
}

#[test]
fn a_caller_may_lower_the_output_cap_but_never_raise_it() {
    let sandbox = core();
    assert_eq!(ok(&sandbox, "{ 1, 2, 3 }, 7"), "[1,2,3]");
    assert_eq!(code(&sandbox, "{ 1, 2, 3 }, 6"), "json_output_too_large");
    assert_eq!(code(&sandbox, "{ 1, 2, 3 }, 0"), "json_output_too_large");
    let heavy = "(function() local t = {} for i = 1, 40 do t[i] = string.rep('x', 512) end return t end)(), 1e9";
    assert_eq!(code(&sandbox, heavy), "json_output_too_large");
}

/// A small deterministic generator (an LCG written in Lua) builds nested values from a
/// restricted alphabet. The canonical output must equal serde_json's compact
/// re-serialisation of what it parsed: same key order, no whitespace, same escapes.
const GENERATOR: &str = r#"
    local function generate(seed)
      local state = seed
      local function rnd(n)
        state = state * 16807 % 2147483647
        return state % n + 1
      end
      local alphabet = { "a", "b", "z", "0", "9", " ", "_", "-", '"', "\\", "\n", "\t" }
      local function word()
        local parts = {}
        for i = 1, rnd(6) - 1 do parts[i] = alphabet[rnd(#alphabet)] end
        return table.concat(parts)
      end
      local function build(depth)
        local kind = rnd(depth >= 5 and 4 or 6)
        if kind == 1 then return rnd(2000000) - 1000000
        elseif kind == 2 then return word()
        elseif kind == 3 then return rnd(2) == 1
        elseif kind == 4 then return 0
        elseif kind == 5 then
          local t = {}
          for i = 1, rnd(5) - 1 do t[i] = build(depth + 1) end
          return t
        end
        local t = {}
        for _ = 1, rnd(5) - 1 do t["k" .. word()] = build(depth + 1) end
        return t
      end
      return build(0)
    end
"#;

/// A small deterministic generator (a Park-Miller LCG written in Lua) builds nested
/// values from a restricted alphabet. The canonical output must equal serde_json's
/// compact re-serialisation of what it parsed: same key order, no whitespace, same
/// escapes, integers only.
#[test]
fn generated_documents_match_serde_json_canonical_form() {
    let sandbox = core();
    let mut accepted = 0;
    for seed in 1..=300 {
        let code = format!("{GENERATOR} return ns.Json.encode(generate({seed}))");
        let first = encode_pair(&sandbox, &code);
        let second = encode_pair(&sandbox, &code);
        assert_eq!(first, second, "seed {seed} must be deterministic");
        match first {
            (Some(text), None) => {
                let parsed: Value = serde_json::from_str(&text).unwrap();
                assert_eq!(serde_json::to_string(&parsed).unwrap(), text, "seed {seed}");
                assert!(json_depth(&parsed) <= 32);
                accepted += 1;
            }
            (None, Some(err)) => assert!(
                err == "json_too_many_elements" || err == "json_output_too_large",
                "seed {seed}: {err}"
            ),
            other => panic!("seed {seed}: {other:?}"),
        }
    }
    assert!(
        accepted >= 250,
        "only {accepted} of 300 generated documents were accepted"
    );
}

#[test]
fn encoder_uses_no_client_global() {
    // Strict mode already proves this for every call above; this one states it directly.
    let sandbox = Sandbox::new(EnvMode::Strict).unwrap();
    sandbox.load_addon_file("Core/Bootstrap.lua").unwrap();
    sandbox.load_addon_file("Core/Json.lua").unwrap();
    let text: String = sandbox
        .run("encode", "return ns.Json.encode({ a = 1 })")
        .unwrap();
    assert_eq!(text, r#"{"a":1}"#);
}
