//! Synthetic source fixtures only. No upstream gameplay data is committed.
use azeroth_ingest::{
    questiedb::{
        self, AUDITED_REVISION, CORRECTION_PATHS, Entity, MAX_FILE_BYTES, MAX_RECORDS,
        MAX_STRING_BYTES, MAX_TABLE_DEPTH, Rejection, SourceOrigin, inspect_correction, stage_raw,
        summarize,
    },
    savedvars::{LuaKey, LuaValue, Rejection as LiteralRejection},
};

fn origin() -> SourceOrigin {
    SourceOrigin {
        revision: Some(AUDITED_REVISION.to_owned()),
        edition: Some("wow_forever".to_owned()),
        // A synthetic declaration is not evidence about a client build.
        build: Some("SYNTHETIC".to_owned()),
    }
}
fn source(entity: Entity, rows: &str) -> String {
    let prefix = match entity {
        Entity::Quest => "quest",
        Entity::Npc => "npc",
        Entity::Item => "item",
        Entity::Object => "object",
    };
    let declarations = entity
        .fields()
        .iter()
        .enumerate()
        .map(|(i, name)| format!("['{name}']={},", i + 1))
        .collect::<String>();
    format!(
        "-- synthetic header\nlocal QuestieDB = QuestieLoader:ImportModule(\"QuestieDB\");\n\
         QuestieDB.{prefix}Keys = {{{declarations}}}\n\
         QuestieDB.{prefix}Data = [[return {{{rows}}}]]\n"
    )
}
fn quest(rows: &str) -> String {
    source(Entity::Quest, rows)
}
fn stage(text: &str) -> Result<questiedb::StagedTable, Rejection> {
    stage_raw(text.as_bytes(), Entity::Quest, origin())
}
fn rejection(text: &str) -> Rejection {
    stage(text).unwrap_err()
}

#[test]
fn preserves_nil_false_zero_empty_absent_and_wide_masks() {
    let staged = stage(&quest(
        "[42]={'Synthetic',nil,false,0,{},8589934770,[9]={nil,{false,0}},},",
    ))
    .unwrap();
    let row = &staged.records()[&42];
    assert_eq!(row.get(&LuaKey::Int(2)), Some(&LuaValue::Nil));
    assert_eq!(row.get(&LuaKey::Int(3)), Some(&LuaValue::Bool(false)));
    assert_eq!(row.get(&LuaKey::Int(4)), Some(&LuaValue::Int(0)));
    assert_eq!(
        row.get(&LuaKey::Int(5)),
        Some(&LuaValue::Table(Default::default()))
    );
    assert_eq!(row.get(&LuaKey::Int(6)), Some(&LuaValue::Int(8589934770)));
    assert_eq!(row.get(&LuaKey::Int(7)), None);
    let Some(LuaValue::Table(nested)) = row.get(&LuaKey::Int(9)) else {
        panic!("table");
    };
    assert_eq!(nested.get(&LuaKey::Int(1)), Some(&LuaValue::Nil));
    assert!(matches!(
        nested.get(&LuaKey::Int(2)),
        Some(LuaValue::Table(_))
    ));
    // Reusing the reader does not widen the existing SavedVariables entry point.
    assert_eq!(
        azeroth_ingest::savedvars::parse_detailed("T={nil}").unwrap_err(),
        LiteralRejection::Nil
    );
}

#[test]
fn validates_each_wrapper_and_exact_field_schema() {
    assert_eq!(Entity::Quest.fields().len(), 36);
    assert_eq!(Entity::Npc.fields()[6], "spawns");
    assert_eq!(Entity::Item.fields()[13], "vendors");
    assert_eq!(Entity::Item.fields().len(), 15);
    assert_eq!(Entity::Object.fields()[6], "waypoints");
    for entity in Entity::ALL {
        let text = source(entity, "[9]={'Synthetic'}").replace('\n', "\r\n");
        let parsed = stage_raw(text.as_bytes(), entity, origin()).unwrap();
        assert_eq!(parsed.entity(), entity);
        assert_eq!(parsed.records().len(), 1);
        assert_eq!(parsed.fingerprint().bytes, text.len());
    }
    let good = quest("[1]={'Synthetic'}");
    for bad in [
        good.replace("['name']=1,", "['name']=2,"),
        good.replace("['name']=1,", ""),
        good.replace("['name']=1,", "['futureField']=1,"),
        good.replace("['name']=1,", "['name']=1,['unknown']=37,"),
    ] {
        assert_eq!(rejection(&bad), Rejection::Schema);
    }
    assert_eq!(
        rejection(&source(Entity::Item, "[1]={}")),
        Rejection::Wrapper
    );
    assert_eq!(rejection(&quest("[1]={[37]=nil}")), Rejection::FieldIndex);
    assert_eq!(rejection(&quest("[1]={future=0}")), Rejection::FieldIndex);
    assert_eq!(rejection(&quest("[1]={[0]=0}")), Rejection::FieldIndex);
}

#[test]
fn duplicate_ids_and_fields_never_overwrite_even_nil() {
    for rows in [
        "[1]={},[1]={}",
        "[1]={[1]=nil,'duplicate'}",
        "[1]={[2]=0,[2]=false}",
    ] {
        assert_eq!(
            rejection(&quest(rows)),
            Rejection::Literal(LiteralRejection::DuplicateKey)
        );
    }
    assert_eq!(
        rejection(&quest("[1]={}").replace("['name']=1,", "['name']=1,['name']=1,")),
        Rejection::Literal(LiteralRejection::DuplicateKey)
    );
}

#[test]
fn executable_or_symbolic_constructs_and_wrapper_garbage_fail_closed() {
    for value in [
        "function() end",
        "os.execute('x')",
        "loadstring('return {}')()",
        "QuestieDB.questKeys",
        "raceIDs.ALL_ALLIANCE",
        "1+2",
        "'a'..'b'",
        "setmetatable({}, {})",
        "[[text]]",
        "{[questKeys.name]='x'}",
        "0x10",
    ] {
        assert!(
            stage(&quest(&format!("[1]={{{value}}}"))).is_err(),
            "{value}"
        );
    }
    let good = quest("[1]={}");
    for bad in [
        format!("os.execute('x')\n{good}"),
        format!("{good}\nos.execute('x')"),
        good.replace("QuestieDB.questKeys", "other(); QuestieDB.questKeys"),
        good.replace("return {", "return (function() return {"),
        good.replace("}]]", "}; arbitrary()]]"),
        good.replace("[[return", "[=[return"),
        format!("--[[ comment ]]\n{good}"),
        good.replace("ImportModule(\"QuestieDB\")", "ImportModule(\"Other\")"),
    ] {
        assert!(stage(&bad).is_err());
    }
    // First long-string terminator wins; a later terminator cannot hide code.
    assert!(stage(&quest("[1]={'x]] arbitrary() --'}")).is_err());
}

#[test]
fn inert_text_remains_data_and_errors_do_not_echo_it() {
    let row = "[1]={\"os.execute('SENSITIVE_SENTINEL')\",[8]={'function() end'}}";
    let parsed = stage(&quest(row)).unwrap();
    assert_eq!(
        parsed.records()[&1][&LuaKey::Int(1)],
        LuaValue::Str("os.execute('SENSITIVE_SENTINEL')".to_owned())
    );
    let error = rejection(&quest("[1]={SENSITIVE_SENTINEL()}"));
    assert!(!error.to_string().contains("SENSITIVE_SENTINEL"));
    assert!(!format!("{error:?}").contains("SENSITIVE_SENTINEL"));
}

#[test]
fn numbers_do_not_round_overflowing_integer_tokens_or_accept_nonfinite_values() {
    let parsed = stage(&quest(
        "[1]={[6]=9223372036854775807,[7]=-9223372036854775808,[8]={1.5,1e2,-0.25}}",
    ))
    .unwrap();
    assert_eq!(
        parsed.records()[&1][&LuaKey::Int(6)],
        LuaValue::Int(i64::MAX)
    );
    assert_eq!(
        parsed.records()[&1][&LuaKey::Int(7)],
        LuaValue::Int(i64::MIN)
    );
    for value in [
        "9223372036854775808",
        "-9223372036854775809",
        "1e999",
        "1e",
        "12bad",
    ] {
        assert_eq!(
            rejection(&quest(&format!("[1]={{{value}}}"))),
            Rejection::Literal(LiteralRejection::BadNumber)
        );
    }
    for id in ["0", "-1", "4294967296", "'1'"] {
        assert_eq!(
            rejection(&quest(&format!("[{id}]={{}}"))),
            Rejection::RecordId
        );
    }
    assert_eq!(rejection(&quest("[1]=false")), Rejection::RecordShape);
    assert_eq!(
        rejection(&quest("[1.0]={}")),
        Rejection::Literal(LiteralRejection::BadKey)
    );
}

#[test]
fn malformed_strings_utf8_and_truncations_fail_without_partial_results() {
    for value in ["'\\255'", "'\\q'", "'line\nbreak'", "'unfinished"] {
        assert!(stage(&quest(&format!("[1]={{{value}}}"))).is_err());
    }
    assert_eq!(
        stage_raw(&[0xff], Entity::Quest, origin()).unwrap_err(),
        Rejection::NotUtf8
    );
    let text = quest("[1]={'Synthetic'}");
    let close = text.find("]]").unwrap() + 2;
    for cut in 0..close {
        assert!(stage(&text[..cut]).is_err(), "cut {cut}");
    }
    assert!(stage(&text).is_ok());
    assert!(stage(&quest("[1]={'valid'},[2]={malicious()}")).is_err());
}

#[test]
fn file_string_depth_and_record_limits_are_enforced() {
    let small = quest("");
    let at_limit = format!("{small}{}", " ".repeat(MAX_FILE_BYTES - small.len()));
    assert!(stage(&at_limit).is_ok());
    assert_eq!(rejection(&format!("{at_limit} ")), Rejection::FileTooLarge);
    assert!(
        stage(&quest(&format!(
            "[1]={{'{}'}}",
            "a".repeat(MAX_STRING_BYTES)
        )))
        .is_ok()
    );
    assert_eq!(
        rejection(&quest(&format!(
            "[1]={{'{}'}}",
            "a".repeat(MAX_STRING_BYTES + 1)
        ))),
        Rejection::Literal(LiteralRejection::StringTooLong)
    );
    let nested = |n| quest(&format!("[1]={{[1]={}0{}}}", "{".repeat(n), "}".repeat(n)));
    assert!(stage(&nested(MAX_TABLE_DEPTH - 2)).is_ok());
    assert_eq!(
        rejection(&nested(MAX_TABLE_DEPTH - 1)),
        Rejection::Literal(LiteralRejection::TooDeep)
    );
    assert_eq!(
        rejection(&nested(10_000)),
        Rejection::Literal(LiteralRejection::TooDeep)
    );
    let rows = (1..=MAX_RECORDS)
        .map(|id| format!("[{id}]={{}},"))
        .collect::<String>();
    assert_eq!(stage(&quest(&rows)).unwrap().records().len(), MAX_RECORDS);
    assert_eq!(
        rejection(&quest(&format!("{rows}[{}]={{}}", MAX_RECORDS + 1))),
        Rejection::RecordLimit
    );
}

#[test]
fn bare_and_quoted_keys_share_the_source_string_bound() {
    for quoted in [false, true] {
        let make_source = |length| {
            let name = "a".repeat(length);
            let key = if quoted { format!("['{name}']") } else { name };
            quest(&format!("[1]={{[8]={{{key}=false}}}}"))
        };
        assert!(stage(&make_source(MAX_STRING_BYTES)).is_ok());
        assert_eq!(
            rejection(&make_source(MAX_STRING_BYTES + 1)),
            Rejection::Literal(LiteralRejection::StringTooLong)
        );
    }
}

#[test]
fn unknown_origin_stays_unknown_and_mixed_origins_are_rejected() {
    let text = quest("[1]={}");
    let unknown = stage_raw(text.as_bytes(), Entity::Quest, SourceOrigin::default()).unwrap();
    let report = summarize(&[unknown], &[]).unwrap();
    assert_eq!(report.caller_declared_origin, SourceOrigin::default());
    assert!(!report.declared_revision_matches_audit);
    let mut other = origin();
    other.edition = Some("wow_classic".to_owned());
    assert_eq!(
        stage_raw(text.as_bytes(), Entity::Quest, other).unwrap_err(),
        Rejection::EditionMismatch
    );
    let mut unverified = origin();
    unverified.build = None;
    let tables = [
        stage(&text).unwrap(),
        stage_raw(
            source(Entity::Item, "").as_bytes(),
            Entity::Item,
            unverified,
        )
        .unwrap(),
    ];
    assert_eq!(
        summarize(&tables, &[]).unwrap_err(),
        Rejection::MixedOrigins
    );
    for bad in ["", "not-a-revision", &"f".repeat(41)] {
        let mut invalid = origin();
        invalid.revision = Some(bad.to_owned());
        assert_eq!(
            stage_raw(text.as_bytes(), Entity::Quest, invalid).unwrap_err(),
            Rejection::Origin
        );
    }
    for bad in ["", "build\nwith-secret", &"x".repeat(65)] {
        let mut invalid = origin();
        invalid.build = Some(bad.to_owned());
        assert_eq!(
            stage_raw(text.as_bytes(), Entity::Quest, invalid).unwrap_err(),
            Rejection::Origin
        );
    }
}

#[test]
fn source_reports_remain_ineligible_even_when_all_files_are_supplied() {
    let tables: Vec<_> = Entity::ALL
        .into_iter()
        .map(|e| stage_raw(source(e, "[1]={}").as_bytes(), e, origin()).unwrap())
        .collect();
    let corrections: Vec<_> = CORRECTION_PATHS
        .into_iter()
        .map(|p| {
            inspect_correction(
                b"function Provider:Load() return {[keys.name]=call()} end",
                p,
                origin(),
            )
            .unwrap()
        })
        .collect();
    let report = summarize(&tables, &corrections).unwrap();
    assert!(report.missing_raw_entities.is_empty());
    assert!(report.missing_corrections.is_empty());
    assert_eq!(report.unsupported_corrections.len(), 18);
    assert_eq!(report.corrections_applied, 0);
    assert!(!report.correction_coverage_complete);
    assert!(!report.derived_passes_applied);
    assert!(!report.source_build_verified);
    assert!(!report.target_inventory_complete);
    assert!(!report.distribution_eligible);
    assert!(!report.playable_dataset);
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("function Provider"));
    assert_eq!(
        summarize(&tables[..1], &[]).unwrap().missing_corrections,
        CORRECTION_PATHS
    );
    assert_eq!(summarize(&[], &[]).unwrap_err(), Rejection::EmptyInput);
    assert_eq!(
        inspect_correction(b"return {}", "../outside.lua", origin()).unwrap_err(),
        Rejection::UnknownCorrection
    );
}

#[test]
fn report_order_is_deterministic_and_duplicate_files_never_hide_conflicts() {
    let mut tables: Vec<_> = Entity::ALL
        .into_iter()
        .map(|e| stage_raw(source(e, "[2]={},[1]={}").as_bytes(), e, origin()).unwrap())
        .collect();
    let report = summarize(&tables, &[]).unwrap();
    tables.reverse();
    assert_eq!(report, summarize(&tables, &[]).unwrap());
    assert_eq!(
        serde_json::to_string(&report).unwrap(),
        serde_json::to_string(&summarize(&tables, &[]).unwrap()).unwrap()
    );
    assert_eq!(
        tables[0].records().keys().copied().collect::<Vec<_>>(),
        vec![1, 2]
    );
    let repeated = [
        stage(&quest("[1]={}")).unwrap(),
        stage(&quest("[2]={}")).unwrap(),
    ];
    assert_eq!(
        summarize(&repeated, &[]).unwrap_err(),
        Rejection::DuplicateInput
    );
    let corrections = [
        inspect_correction(b"first", CORRECTION_PATHS[0], origin()).unwrap(),
        inspect_correction(b"different", CORRECTION_PATHS[0], origin()).unwrap(),
    ];
    assert_eq!(
        summarize(&tables, &corrections).unwrap_err(),
        Rejection::DuplicateInput
    );
    let mut other = origin();
    other.revision = Some("f".repeat(40));
    let correction = inspect_correction(b"return {}", CORRECTION_PATHS[0], other).unwrap();
    assert_eq!(
        summarize(&tables, &[correction]).unwrap_err(),
        Rejection::MixedOrigins
    );
}

#[test]
fn fingerprints_bind_exact_bytes_but_never_establish_source_acceptance() {
    let first = stage(&quest("[1]={}")).unwrap();
    let second = stage(&format!("{}-- harmless comment\n", quest("[1]={}"))).unwrap();
    assert_eq!(first.records(), second.records());
    assert_ne!(first.fingerprint(), second.fingerprint());
    let correction = inspect_correction(b"", CORRECTION_PATHS[0], origin()).unwrap();
    let report = summarize(&[first], &[correction]).unwrap();
    assert_eq!(
        report.unsupported_corrections[0].fingerprint.sha256,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert!(!report.correction_coverage_complete);
}

#[test]
fn pathological_data_and_source_comments_never_become_execution() {
    let text = quest("[1]={'Synthetic'}");
    assert!(stage(&format!("-- os.execute('x')\n{text}-- trailing\n")).is_ok());
    for token in ["{", "[", "'", "\\", "--", "[=", "x", "..", "("] {
        let _ = stage(&quest(&format!("[1]={{{}}}", token.repeat(4000))));
    }
    let mut state = 81u64;
    let alphabet = b"{}[]=,'\"-+0123nilf()\\ \n";
    for _ in 0..200 {
        let body = (0..500)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                char::from(alphabet[(state as usize) % alphabet.len()])
            })
            .collect::<String>();
        let _ = stage(&quest(&body));
    }
}
