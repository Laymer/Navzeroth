//! SavedVariables literal parser (M2.3): accepted constructs, exact rejection
//! reasons, bounds, non-execution and CASE-008 (parser half).
use azeroth_core::error::ErrorCode;
use azeroth_ingest::savedvars::{
    LuaKey, LuaValue, MAX_FILE_BYTES, MAX_STRING_BYTES, MAX_TABLE_DEPTH, MAX_VALUES, Rejection,
    extract_export, parse, parse_detailed,
};
use std::{collections::BTreeMap, path::PathBuf};

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/transport")
        .join(name);
    std::fs::read_to_string(path).expect("fixture is readable")
}
fn rejection(input: &str) -> Rejection {
    parse_detailed(input).expect_err("input is rejected")
}
fn table(entries: Vec<(LuaKey, LuaValue)>) -> LuaValue {
    LuaValue::Table(entries.into_iter().collect())
}
fn skey(s: &str) -> LuaKey {
    LuaKey::Str(s.to_owned())
}
fn string(s: &str) -> LuaValue {
    LuaValue::Str(s.to_owned())
}

#[test]
fn accepts_the_documented_literal_subset() {
    let file = parse(
        "-- header comment\n\
         A = { [\"k\"] = 1, name = 'x', [10] = true; \"pos\", -2.5, 1e+15, }\n\
         B = \"s\"; C = -7 D = false\n",
    )
    .unwrap();
    let expected: BTreeMap<String, LuaValue> = BTreeMap::from([
        (
            "A".to_owned(),
            table(vec![
                (skey("k"), LuaValue::Int(1)),
                (skey("name"), string("x")),
                (LuaKey::Int(10), LuaValue::Bool(true)),
                (LuaKey::Int(1), string("pos")),
                (LuaKey::Int(2), LuaValue::Float(-2.5)),
                (LuaKey::Int(3), LuaValue::Float(1e15)),
            ]),
        ),
        ("B".to_owned(), string("s")),
        ("C".to_owned(), LuaValue::Int(-7)),
        ("D".to_owned(), LuaValue::Bool(false)),
    ]);
    assert_eq!(file.globals(), &expected);
    // Real files carry per-element index comments; they are plain line comments.
    let file = parse("T = {\n\t\"a\", -- [1]\n\t\"b\", -- [2]\n}\n").unwrap();
    assert_eq!(
        file.get("T"),
        Some(&table(vec![
            (LuaKey::Int(1), string("a")),
            (LuaKey::Int(2), string("b"))
        ]))
    );
    assert_eq!(parse("").unwrap().globals().len(), 0);
    assert_eq!(parse("T = {}\r\nU = {}\r\n").unwrap().globals().len(), 2);
    // A comment may contain anything, including things that look like code.
    assert!(parse("-- os.execute('x') [[ \nT = {}\n").is_ok());
}

#[test]
fn decodes_every_documented_escape() {
    let value = |literal: &str| match parse(&format!("T = {literal}")).unwrap().get("T") {
        Some(LuaValue::Str(s)) => s.clone(),
        other => panic!("not a string: {other:?}"),
    };
    assert_eq!(
        value(r#""a\n\r\t\a\b\f\v\\\"\'z""#),
        "a\n\r\t\u{7}\u{8}\u{c}\u{b}\\\"'z"
    );
    assert_eq!(value(r"'\65\066\0067'"), "AB\u{6}7");
    assert_eq!(value(r"'\x41\x7a'"), "Az");
    assert_eq!(value("'a\\z \n\t b'"), "ab");
    assert_eq!(value(r"'\u{41}\u{e9}\u{1F600}'"), "A\u{e9}\u{1F600}");
    assert_eq!(value("'caf\u{e9}'"), "caf\u{e9}");
    assert_eq!(value(r"'\0'"), "\0");
    // A multi-byte character written as decimal escapes is valid UTF-8 after decoding.
    assert_eq!(value(r"'\195\169'"), "\u{e9}");
}

#[test]
fn rejects_each_construct_for_its_own_reason() {
    let cases: &[(&str, Rejection)] = &[
        ("T = function() end", Rejection::Function),
        (
            "T = { f = function() os.execute('x') end }",
            Rejection::Function,
        ),
        ("T = os.execute('x')", Rejection::NameReference),
        ("T = loadstring('x')()", Rejection::Call),
        ("T = { load('x') }", Rejection::Call),
        ("T = { dofile 'x' }", Rejection::Call),
        ("T = { x = f{} }", Rejection::Call),
        ("T = Other", Rejection::NameReference),
        ("T = { Other }", Rejection::NameReference),
        ("T = { x = Other }", Rejection::NameReference),
        ("T = { [Other] = 1 }", Rejection::NameReference),
        ("T = nil", Rejection::Nil),
        ("T = { nil }", Rejection::Nil),
        ("T = inf", Rejection::NameReference),
        ("T = 'a' .. 'b'", Rejection::Concatenation),
        ("T = { 'a' .. 'b' }", Rejection::Concatenation),
        ("T = { 1 .. 2 }", Rejection::Concatenation),
        ("T = 1 + 2", Rejection::Operator),
        ("T = { 1 + 2 }", Rejection::Operator),
        ("T = { 1 - 2 }", Rejection::Operator),
        ("T = { - 5 }", Rejection::Operator),
        ("T = { -x }", Rejection::Operator),
        ("T = { #x }", Rejection::Operator),
        ("T = { not true }", Rejection::NameReference),
        ("T = { a == 1 }", Rejection::NameReference),
        ("T = [[long]]", Rejection::LongBracket),
        ("T = [==[long]==]", Rejection::LongBracket),
        ("T = { [[long]] = 1 }", Rejection::LongBracket),
        ("--[[ long\ncomment ]] T = {}", Rejection::LongBracket),
        ("--[==[ long ]==] T = {}", Rejection::LongBracket),
        ("T = 0x10", Rejection::BadNumber),
        ("T = 12abc", Rejection::BadNumber),
        ("T = 1e", Rejection::BadNumber),
        ("T = 1e999", Rejection::BadNumber),
        ("T = { [1.5] = 1 }", Rejection::BadKey),
        ("T = { [true] = 1 }", Rejection::NameReference),
        ("T = { [{}] = 1 }", Rejection::BadKey),
        ("T = { [1e2] = 1 }", Rejection::BadKey),
        ("T = { a = 1, a = 2 }", Rejection::DuplicateKey),
        ("T = { [1] = 1, 2 }", Rejection::DuplicateKey),
        ("T = { x = 1, [\"x\"] = 2 }", Rejection::DuplicateKey),
        ("T = {} T = {}", Rejection::DuplicateGlobal),
        ("local T = {}", Rejection::ReservedName),
        ("T = { function = 1 }", Rejection::ReservedName),
        ("T = { end = 1 }", Rejection::ReservedName),
        ("return {}", Rejection::ReservedName),
        ("T = '\\q'", Rejection::BadEscape),
        ("T = '\\256'", Rejection::BadEscape),
        ("T = '\\xZZ'", Rejection::BadEscape),
        ("T = '\\u{110000}'", Rejection::BadEscape),
        ("T = '\\u{D800}'", Rejection::BadEscape),
        ("T = '\\u{}'", Rejection::BadEscape),
        ("T = '\\u41'", Rejection::BadEscape),
        ("T = 'a\\\nb'", Rejection::BadEscape),
        ("T = 'a\nb'", Rejection::RawLineBreak),
        ("T = 'a\rb'", Rejection::RawLineBreak),
        ("T = '\\255'", Rejection::NotUtf8),
        ("T = '\\xC3'", Rejection::NotUtf8),
        ("T = 'a", Rejection::Truncated),
        ("T = {", Rejection::Truncated),
        ("T = { 'a'", Rejection::Truncated),
        ("T = { 'a',", Rejection::Truncated),
        ("T =", Rejection::Truncated),
        ("T", Rejection::Truncated),
        ("T = '\\", Rejection::Truncated),
        ("T = -", Rejection::Truncated),
        ("T = {} x", Rejection::Truncated),
        ("T = {}}", Rejection::Syntax),
        ("T = {} 'x'", Rejection::Call),
        ("T = {,}", Rejection::Syntax),
        ("T = {1,,2}", Rejection::Syntax),
        ("'x' = 1", Rejection::Syntax),
        ("T.x = 1", Rejection::Syntax),
        ("T == 1", Rejection::Operator),
        ("T = {} ) ", Rejection::Syntax),
        ("\u{feff}T = {}", Rejection::Syntax),
        ("T = {} \u{e9}", Rejection::Syntax),
    ];
    for (input, expected) in cases {
        assert_eq!(
            parse_detailed(input).err(),
            Some(*expected),
            "input {input:?} should be rejected as {expected:?}"
        );
    }
}

#[test]
fn rejection_codes_and_messages_are_closed() {
    let err = parse("T = function() end").unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert_eq!(err.to_string(), Rejection::Function.message());
    let err = parse(&format!("T = '{}'", "a".repeat(MAX_STRING_BYTES + 1))).unwrap_err();
    assert_eq!(err.code, ErrorCode::ImportTooLarge);
}

#[test]
fn nesting_is_bounded() {
    let nested = |depth: usize| format!("T = {}{}\n", "{".repeat(depth), "}".repeat(depth));
    assert!(parse(&nested(MAX_TABLE_DEPTH)).is_ok());
    assert_eq!(rejection(&nested(MAX_TABLE_DEPTH + 1)), Rejection::TooDeep);
    // Depth is checked before recursing: extreme nesting cannot overflow the stack.
    assert_eq!(rejection(&nested(200_000)), Rejection::TooDeep);
    let keyed = |depth: usize| format!("T = {}1{}", "{ x = ".repeat(depth), " }".repeat(depth));
    assert!(parse(&keyed(MAX_TABLE_DEPTH)).is_ok());
    assert_eq!(rejection(&keyed(MAX_TABLE_DEPTH + 1)), Rejection::TooDeep);
    // Unclosed deep input is also rejected for depth, not for its missing braces.
    assert_eq!(rejection(&"{".repeat(100_000)), Rejection::Syntax);
    assert_eq!(
        rejection(&format!("T = {}", "{".repeat(100_000))),
        Rejection::TooDeep
    );
}

#[test]
fn sizes_and_counts_are_bounded() {
    let longest = format!("T = '{}'\n", "a".repeat(MAX_STRING_BYTES));
    assert!(parse(&longest).is_ok());
    assert_eq!(
        rejection(&format!("T = '{}'", "a".repeat(MAX_STRING_BYTES + 1))),
        Rejection::StringTooLong
    );
    let padding = " ".repeat(MAX_FILE_BYTES - "T = {}".len());
    assert!(parse(&format!("T = {{}}{padding}")).is_ok());
    assert_eq!(
        rejection(&format!("T = {{}} {padding}")),
        Rejection::FileTooLarge
    );
    let values = |n: usize| format!("T = {{{}}}", "1,".repeat(n));
    assert!(parse(&values(MAX_VALUES - 1)).is_ok());
    assert_eq!(rejection(&values(MAX_VALUES)), Rejection::TooManyValues);
    let globals = (0..65)
        .map(|i| format!("G{i} = {{}}\n"))
        .collect::<String>();
    assert_eq!(rejection(&globals), Rejection::TooManyGlobals);
}

#[test]
fn pathological_inputs_terminate_deterministically() {
    // 10k elements of every shape terminate, accepted or rejected, without panicking.
    let elements = (0..10_000).map(|i| format!("{i},")).collect::<String>();
    let parsed = parse(&format!("T = {{{elements}}}")).unwrap();
    let Some(LuaValue::Table(t)) = parsed.get("T") else {
        panic!("table expected")
    };
    assert_eq!(t.len(), 10_000);
    for unit in [
        "[", "{", "-", "--", "'", "\\", "..", "x", "\"\\", "[=", "--[", "\n", ",", "= ",
    ] {
        let _ = parse(&unit.repeat(10_000));
        let _ = parse(&format!("T = {}", unit.repeat(10_000)));
        let _ = parse(&format!("T = {{{}", unit.repeat(10_000)));
    }
    // A deterministic pseudo-random walk over a Lua-ish alphabet.
    const ALPHABET: &[u8] = b"{}[]=,;\"' \n-.\\0123456789eExnz_aT()+*/#";
    let mut state = 0x2545_f491_4f6c_dd1du64;
    for _ in 0..300 {
        let mut text = String::from("T = ");
        for _ in 0..2_000 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            text.push(char::from(
                ALPHABET[usize::try_from(state >> 40).unwrap() % ALPHABET.len()],
            ));
        }
        let _ = parse(&text);
    }
}

#[test]
fn extract_export_takes_the_names_as_parameters() {
    let file = parse(&fixture("savedvars-valid.lua")).unwrap();
    let export = extract_export(&file, "AzerothNavigatorDB", "export").unwrap();
    assert!(export.starts_with("ANAV1:{"));
    // Unrelated tables are parsed but ignored.
    assert!(matches!(file.get("OtherAddonDB"), Some(LuaValue::Table(_))));
    for (global, key) in [
        ("AzerothNavigatorDB", "missing"),
        ("AzerothNavigatorDB", "schema"),
        ("OtherAddonDB", "export"),
        ("MissingDB", "export"),
        ("azerothnavigatordb", "export"),
    ] {
        let err = extract_export(&file, global, key).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidInput);
    }
    // A different layout works when the caller says so.
    let other = parse("MyDB = { payload = 'ANAV1:{}' }").unwrap();
    assert_eq!(
        extract_export(&other, "MyDB", "payload").unwrap(),
        "ANAV1:{}"
    );
}

#[test]
fn transport_fixtures_are_rejected_for_the_intended_reason() {
    let cases: &[(&str, Option<Rejection>)] = &[
        ("savedvars-valid.lua", None),
        ("savedvars-valid-escapes.lua", None),
        ("savedvars-export-truncated.lua", None),
        ("savedvars-missing-export.lua", None),
        ("savedvars-export-not-anav1.lua", None),
        (
            "savedvars-malicious-function.lua",
            Some(Rejection::Function),
        ),
        ("savedvars-malicious-loadstring.lua", Some(Rejection::Call)),
        ("savedvars-malicious-statement.lua", Some(Rejection::Syntax)),
        (
            "savedvars-malicious-concat.lua",
            Some(Rejection::Concatenation),
        ),
        (
            "savedvars-malicious-arithmetic.lua",
            Some(Rejection::Operator),
        ),
        (
            "savedvars-malicious-global.lua",
            Some(Rejection::NameReference),
        ),
        (
            "savedvars-malicious-long-string.lua",
            Some(Rejection::LongBracket),
        ),
        (
            "savedvars-malicious-long-comment.lua",
            Some(Rejection::LongBracket),
        ),
        ("savedvars-truncated.lua", Some(Rejection::Truncated)),
        ("savedvars-deep-nesting.lua", Some(Rejection::TooDeep)),
        ("savedvars-bad-escape.lua", Some(Rejection::BadEscape)),
        ("savedvars-invalid-utf8.lua", Some(Rejection::NotUtf8)),
        ("savedvars-duplicate-key.lua", Some(Rejection::DuplicateKey)),
        ("savedvars-bom.lua", Some(Rejection::Syntax)),
    ];
    for (file, expected) in cases {
        assert_eq!(parse_detailed(&fixture(file)).err(), *expected, "{file}");
    }
}

#[test]
fn truncated_file_is_a_structured_error_and_the_next_file_imports() {
    // CASE-008, parser half: no panic, no partial result, and the parser keeps no state.
    let valid = fixture("savedvars-valid.lua");
    let mut boundary_count = 0;
    for cut in 0..valid.len() - 1 {
        if !valid.is_char_boundary(cut) {
            continue;
        }
        boundary_count += 1;
        let prefix = &valid[..cut];
        match parse(prefix) {
            // A prefix ending after the first complete assignment is itself a valid
            // file; it just does not contain the export.
            Ok(file) => assert!(
                extract_export(&file, "AzerothNavigatorDB", "export").is_err(),
                "prefix {cut} yields an export"
            ),
            Err(err) => assert!(matches!(
                err.code,
                ErrorCode::InvalidInput | ErrorCode::ImportTooLarge
            )),
        }
    }
    assert!(boundary_count > 1000);
    let file = parse(&valid).unwrap();
    assert!(extract_export(&file, "AzerothNavigatorDB", "export").is_ok());
}
