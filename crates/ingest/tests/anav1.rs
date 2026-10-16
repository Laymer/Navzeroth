//! ANAV1 text transport grammar (M2.2).
mod common;
use azeroth_core::{
    contract::{AnyExchange, Extensions, MAX_ENVELOPE_BYTES, decode_any},
    error::{ErrorCode, Validated},
};
use azeroth_ingest::anav1::{
    MAX_DOCUMENT_BYTES, MAX_JSON_DEPTH, MAX_TEXT_BYTES, PREFIX, decode_text, encode_text,
    json_depth_within,
};
use common::*;

fn code(text: &str) -> ErrorCode {
    decode_text(text).expect_err("rejected").code
}

#[test]
fn encode_and_decode_round_trip_every_contract_kind() {
    let manifest: serde_json::Value = serde_json::from_str(&contract("manifest.json")).unwrap();
    let mut kinds = std::collections::BTreeSet::new();
    for entry in manifest["fixtures"].as_array().unwrap() {
        if entry["expect"] != "valid" {
            continue;
        }
        let exchange = decode_any(&contract(entry["file"].as_str().unwrap())).unwrap();
        kinds.insert(exchange.kind());
        let Ok(text) = encode_text(&exchange) else {
            // Only a document over the 4 MiB transport ceiling may be refused.
            panic!("{} cannot be encoded", entry["file"]);
        };
        assert!(text.starts_with(PREFIX));
        assert!(!text.ends_with('\n'));
        let again = decode_any(decode_text(&text).unwrap()).unwrap();
        assert_eq!(again, exchange);
        assert_eq!(encode_text(&again).unwrap(), text, "encoding is stable");
    }
    assert_eq!(kinds.len(), 6);
}

#[test]
fn grammar_tolerates_ascii_whitespace_around_the_document_only() {
    let json = decode_any(&contract("character-snapshot.json")).unwrap();
    let text = encode_text(&json).unwrap();
    let document = &text[PREFIX.len()..];
    for framed in [
        text.clone(),
        format!("{text}\n"),
        format!("{text}\r\n"),
        format!("\r\n \t{text}\r\n\r\n"),
        format!("\n\n{text}\n\n"),
    ] {
        assert_eq!(decode_text(&framed).unwrap(), document, "{framed:?}");
    }
    let bad = [
        ("lowercase prefix", format!("anav1:{document}")),
        ("mixed case", format!("Anav1:{document}")),
        ("other version", format!("ANAV2:{document}")),
        ("no colon", format!("ANAV1{document}")),
        ("no prefix", document.to_owned()),
        ("duplicated prefix", format!("{PREFIX}{text}")),
        ("space after prefix", format!("{PREFIX} {document}")),
        ("newline after prefix", format!("{PREFIX}\n{document}")),
        ("text before prefix", format!("x{text}")),
        ("BOM before prefix", format!("\u{feff}{text}")),
        ("NBSP padding", format!("\u{a0}{text}")),
        ("form feed padding", format!("\u{c}{text}")),
        ("empty", String::new()),
        ("prefix only", PREFIX.to_owned()),
        ("not an object", format!("{PREFIX}[]")),
        ("string payload", format!("{PREFIX}\"{document}\"")),
    ];
    for (name, input) in bad {
        assert_eq!(code(&input), ErrorCode::InvalidInput, "{name}");
    }
}

#[test]
fn document_after_the_framing_is_still_validated_by_the_contract() {
    // The framing accepts these shapes; the contract decoder (via the import
    // service) is what rejects them.
    let empty = format!("{PREFIX}{{}}");
    let document = decode_text(&empty).unwrap();
    assert_eq!(document, "{}");
    assert!(decode_any(document).is_err());
    let tail = format!("{PREFIX}{{}} tail");
    let trailing = decode_text(&tail).unwrap();
    assert!(decode_any(trailing).is_err());
}

#[test]
fn size_ceiling_is_enforced_before_any_json_parsing() {
    // The payload is not JSON at all: only the size rule can reject it.
    let at_limit = format!("{PREFIX}{{{}", " ".repeat(MAX_DOCUMENT_BYTES - 1));
    // Trailing spaces are framing whitespace; the document itself is the lone brace.
    assert_eq!(decode_text(&at_limit).unwrap(), "{");
    let at_limit = format!("{PREFIX}{{{}x", " ".repeat(MAX_DOCUMENT_BYTES - 2));
    assert_eq!(decode_text(&at_limit).map(str::len), Ok(MAX_DOCUMENT_BYTES));
    let over = format!("{PREFIX}{{{}x", " ".repeat(MAX_DOCUMENT_BYTES - 1));
    assert_eq!(code(&over), ErrorCode::ImportTooLarge);
    // Raw text beyond the framing slack is refused outright.
    assert_eq!(
        code(&" ".repeat(MAX_TEXT_BYTES + 1)),
        ErrorCode::ImportTooLarge
    );
    assert_eq!(MAX_DOCUMENT_BYTES, MAX_ENVELOPE_BYTES);
    // The transport ceiling is 4 MiB even for kinds the contract lets grow to 16 MiB.
    const { assert!(MAX_TEXT_BYTES < 16 * 1024 * 1024) };
}

#[test]
fn json_nesting_is_bounded_before_parsing() {
    let nested = |depth: usize| {
        format!(
            "{PREFIX}{{\"x\":{}{}}}",
            "[".repeat(depth - 1),
            "]".repeat(depth - 1)
        )
    };
    // `{` counts as depth 1.
    assert!(decode_text(&nested(MAX_JSON_DEPTH)).is_ok());
    assert_eq!(code(&nested(MAX_JSON_DEPTH + 1)), ErrorCode::InvalidInput);
    // Brackets inside strings, including escaped quotes, do not count.
    let in_string = format!(
        "{PREFIX}{{\"x\":\"{}\\\"{}\"}}",
        "[".repeat(100),
        "{".repeat(100)
    );
    assert!(decode_text(&in_string).is_ok());
    assert!(json_depth_within("{\"a\":[[1]]}", 3));
    assert!(!json_depth_within("{\"a\":[[1]]}", 2));
    // Unbalanced and absurd inputs terminate without recursion.
    assert!(!json_depth_within(&"[".repeat(1_000_000), 32));
    assert!(json_depth_within(&"]".repeat(1_000_000), 32));
}

#[test]
fn truncated_text_is_a_structured_error_for_every_prefix() {
    // CASE-008, parser half: no panic, no partial result.
    for file in ["anav1-character-snapshot.txt", "anav1-party-snapshot.txt"] {
        let text = transport(file);
        let full = text.trim_end().len();
        let options = no_clock();
        for cut in 0..full {
            if !text.is_char_boundary(cut) {
                continue;
            }
            let error = azeroth_ingest::import::import_text(&text[..cut], &options)
                .expect_err("a strict prefix never imports");
            assert!(matches!(
                error.code,
                ErrorCode::InvalidInput | ErrorCode::ImportTooLarge
            ));
        }
        // A following complete document imports.
        assert!(azeroth_ingest::import::import_text(&text, &options).is_ok());
    }
}

#[test]
fn encode_refuses_exchange_nested_beyond_the_decode_limit() {
    // `decode_any` bounds depth, but `Extensions::new` only checks size, so a typed
    // exchange built in memory can serialise deeper than the transport accepts. Encode
    // must refuse it rather than emit text that decode would reject.
    let manifest: serde_json::Value = serde_json::from_str(&contract("manifest.json")).unwrap();
    let plan = manifest["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["expect"] == "valid")
        .find_map(
            |entry| match decode_any(&contract(entry["file"].as_str().unwrap())) {
                Ok(AnyExchange::RoutePlan(plan)) => Some(plan),
                _ => None,
            },
        )
        .expect("a valid route plan fixture exists");
    let with_depth = |levels: usize| {
        let mut nested = serde_json::json!(1);
        for _ in 0..levels {
            nested = serde_json::json!([nested]);
        }
        let mut data = plan.clone().into_inner();
        data.extensions = Extensions::new([("x_deep".to_owned(), nested)]).unwrap();
        AnyExchange::RoutePlan(Box::new(Validated::try_new(data).unwrap()))
    };
    // Control: a shallow extension encodes, so the refusal below is caused by depth.
    assert!(encode_text(&with_depth(3)).is_ok());
    let error = encode_text(&with_depth(MAX_JSON_DEPTH + 8)).expect_err("too deep to encode");
    assert_eq!(error.code, ErrorCode::InvalidInput);
}
