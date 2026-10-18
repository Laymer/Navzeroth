use azeroth_core::strict_json::value;
use serde_json::{Value, json};

#[test]
fn scalar_states_numbers_and_inert_strings_round_trip_without_collapsing() {
    let bytes =
        br#"[false,true,0,-7,18446744073709551615,1.25,null,"",[],{},"os.execute('never run')"]"#;
    let decoded = value(bytes, bytes.len()).unwrap();
    assert_eq!(decoded[0], Value::Bool(false));
    assert_eq!(decoded[1], Value::Bool(true));
    assert_eq!(decoded[2].as_u64(), Some(0));
    assert_eq!(decoded[3].as_i64(), Some(-7));
    assert_eq!(decoded[4].as_u64(), Some(u64::MAX));
    assert_eq!(decoded[5].as_f64(), Some(1.25));
    assert!(decoded[6].is_null());
    assert_eq!(decoded[7], "");
    assert_eq!(decoded[8], json!([]));
    assert_eq!(decoded[9], json!({}));
    assert_eq!(decoded[10], "os.execute('never run')");
}

#[test]
fn duplicate_keys_include_escaped_equivalents_and_nested_array_members() {
    for bytes in [
        br#"{"a":false,"a":true}"#.as_slice(),
        br#"{"a":0,"\u0061":1}"#,
        br#"[{"safe":1},{"nested":{"key":0,"key":false}}]"#,
    ] {
        assert_eq!(
            value(bytes, bytes.len()),
            Err("Invalid or duplicate-key pack JSON")
        );
    }
    assert_eq!(
        value(br#"[{"a":0},{"a":false}]"#, 100).unwrap(),
        json!([{ "a": 0 }, { "a": false }])
    );
}

#[test]
fn quotas_truncation_nonfinite_and_excessive_nesting_fail_without_input_echo() {
    let valid = br#"{"secret":"synthetic marker"}"#;
    assert!(value(valid, valid.len()).is_ok());
    assert_eq!(
        value(valid, valid.len() - 1),
        Err("JSON input exceeds byte bound")
    );
    for length in 0..valid.len() {
        assert_eq!(
            value(&valid[..length], valid.len()),
            Err("Invalid or duplicate-key pack JSON")
        );
    }
    for malformed in [
        b"NaN".as_slice(),
        b"Infinity",
        b"1e10000",
        b"{}{}",
        b"[1,]",
        b"\xff",
    ] {
        assert_eq!(
            value(malformed, 100),
            Err("Invalid or duplicate-key pack JSON")
        );
    }
    let deep = format!("{}0{}", "[".repeat(129), "]".repeat(129));
    assert_eq!(
        value(deep.as_bytes(), deep.len()),
        Err("Invalid or duplicate-key pack JSON")
    );
}
