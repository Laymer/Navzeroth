use azeroth_ingest::{
    foreverdb::{self, Rejection, Stratum},
    savedvars::{LuaKey, LuaValue},
};
const F: &[&str] = &[
    "qfQuests",
    "qfPoi",
    "qfSeen",
    "qfNpcNames",
    "qfNpcSpots",
    "qfItems",
    "qfArea",
    "qfByMap",
];
const C: &[&str] = &[
    "qcQuests",
    "qcNpc",
    "qcObj",
    "qcItem",
    "qcByMap",
    "qcExcl",
    "dgClassic",
];
fn meta() -> Vec<u8> {
    br#"local _, ns = ...
ns.meta={version="fixture-r1",built=0,build="1.60.1.70338",cap=0,site="https://foreverdb.net",addon="0.34.0"}"#.to_vec()
}
fn file(names: &[&str], rows: &str) -> Vec<u8> {
    let mut s = "-- synthetic namespace data\nlocal _, ns = ...\n".to_owned();
    for (i, name) in names.iter().enumerate() {
        s.push_str(&format!(
            "ns.{name}={{{}}}\n",
            if i == 0 { rows } else { "" }
        ));
    }
    s.into_bytes()
}
fn forever() -> Vec<u8> {
    file(F, r#"[7]="SYNTHETIC PRIVATE|0|0||||0|0""#)
}
fn classic() -> Vec<u8> {
    file(
        C,
        &format!(
            "[7]=\"{}\",[9]=\"{}\"",
            ["SYNTHETIC PRIVATE"; 14].join("|"),
            ["fixture"; 14].join("|")
        ),
    )
}
#[test]
fn separates_source_inventories_without_claiming_target_coverage_or_rights() {
    let staged = foreverdb::stage(&meta(), &forever(), &classic()).unwrap();
    let r = staged.report();
    assert_eq!(r.metadata.bundle_origin_build, "1.60.1.70338");
    assert_eq!(r.metadata.built_epoch, 0);
    assert_eq!(r.metadata.level_cap, 0);
    assert_eq!(r.quest_inventories[0].source_quest_ids, [7]);
    assert_eq!(r.quest_inventories[1].source_quest_ids, [7, 9]);
    assert_eq!(r.overlapping_quest_ids, [7]);
    assert_eq!(r.quest_inventories[0].declared_edition, "wow_forever");
    assert_eq!(r.quest_inventories[1].declared_edition, "wow_classic");
    assert!(
        r.quest_inventories
            .iter()
            .all(|i| i.declared_record_build.is_none())
    );
    assert!(r.independently_required_quest_ids.is_none());
    assert!(!r.publication_allowed);
    let text = serde_json::to_string(&r).unwrap();
    assert!(!text.contains("SYNTHETIC PRIVATE"));
    assert_eq!(text, serde_json::to_string(&staged.report()).unwrap());
}
#[test]
fn preserves_nil_false_zero_empty_and_absent_source_literals() {
    let f = String::from_utf8(forever())
        .unwrap()
        .replace("ns.qfPoi={}", "ns.qfPoi={nil,false,0,\"\"}");
    let staged = foreverdb::stage(&meta(), f.as_bytes(), &classic()).unwrap();
    let t = staged.table(Stratum::Forever, "qfPoi").unwrap();
    assert_eq!(t.get(&LuaKey::Int(1)), Some(&LuaValue::Nil));
    assert_eq!(t.get(&LuaKey::Int(2)), Some(&LuaValue::Bool(false)));
    assert_eq!(t.get(&LuaKey::Int(3)), Some(&LuaValue::Int(0)));
    assert_eq!(t.get(&LuaKey::Int(4)), Some(&LuaValue::Str(String::new())));
    assert_eq!(t.get(&LuaKey::Int(5)), None);
    assert!(staged.table(Stratum::Classic, "qfPoi").is_none());
}
#[test]
fn rejects_executable_suffixes_wrappers_and_schema_changes() {
    let f = String::from_utf8(forever()).unwrap();
    for bad in [
        format!("{f}\nos.execute('side effect')"),
        f.replace("ns.qfPoi={}", "ns.qfPoi=loadstring('return 1')()"),
        f.replace("ns.qfPoi={}", "ns.qfPoi=function() end"),
        f.replace("ns.qfPoi={}", "ns.qfPoi={x=1,x=2}"),
        f.replace("ns.qfPoi={}", "ns.qfPoi={} ns.qfPoi={}"),
        f.replace("ns.qfPoi={}", "ns.unknown={}"),
        f.replace("ns.qfPoi={}", ""),
        f.replace("local _, ns", "local_ , ns"),
        format!("{f}return ns"),
    ] {
        assert!(foreverdb::stage(&meta(), bad.as_bytes(), &classic()).is_err());
    }
}
#[test]
fn validates_versions_metadata_and_record_shapes() {
    let m = String::from_utf8(meta()).unwrap();
    assert_eq!(
        foreverdb::stage(
            m.replace("0.34.0", "0.35.0").as_bytes(),
            &forever(),
            &classic()
        )
        .unwrap_err(),
        Rejection::Version
    );
    for bad in [
        m.replace("built=0", "built=false"),
        m.replace("cap=0", "cap=-1"),
        m.replace("70338", "unknown"),
        m.replace("cap=0", "cap=0,extra=true"),
    ] {
        assert!(foreverdb::stage(bad.as_bytes(), &forever(), &classic()).is_err());
    }
    let f = String::from_utf8(forever()).unwrap();
    for bad in [
        f.replace("[7]", "[0]"),
        f.replace("[7]", "[-1]"),
        f.replace("[7]", "[4294967296]"),
        f.replace("[7]", "[\"7\"]"),
        f.replace("|0|0||||0|0", "|0|0"),
    ] {
        assert_eq!(
            foreverdb::stage(&meta(), bad.as_bytes(), &classic()).unwrap_err(),
            Rejection::Quest
        );
    }
    let changed = m.replace("70338", "70374");
    let r = foreverdb::stage(changed.as_bytes(), &forever(), &classic())
        .unwrap()
        .report();
    assert_eq!(r.metadata.bundle_origin_build, "1.60.1.70374");
    assert_eq!(r.target_validation, "unverified");
    let empty = foreverdb::stage(&meta(), &file(F, ""), &file(C, ""))
        .unwrap()
        .report();
    assert!(empty.quest_inventories[0].source_quest_ids.is_empty());
    assert!(empty.independently_required_quest_ids.is_none());
}
#[test]
fn enforces_file_depth_and_aggregate_value_bounds() {
    assert_eq!(
        foreverdb::stage(
            &vec![b' '; foreverdb::MAX_FILE_BYTES + 1],
            &forever(),
            &classic()
        )
        .unwrap_err(),
        Rejection::Capacity
    );
    let f = String::from_utf8(forever()).unwrap();
    let deep = f.replace(
        "ns.qfPoi={}",
        &format!("ns.qfPoi={}0{}", "{".repeat(34), "}".repeat(34)),
    );
    assert!(foreverdb::stage(&meta(), deep.as_bytes(), &classic()).is_err());
    let many = f
        .replace(
            "ns.qfPoi={}",
            &format!("ns.qfPoi={{{}}}", "0,".repeat(500_001)),
        )
        .replace(
            "ns.qfSeen={}",
            &format!("ns.qfSeen={{{}}}", "0,".repeat(500_001)),
        );
    assert_eq!(
        foreverdb::stage(&meta(), many.as_bytes(), &classic()).unwrap_err(),
        Rejection::Capacity
    );
}
