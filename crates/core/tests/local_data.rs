use azeroth_core::local_data::*;

const HEADER: &str =
    "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0";
fn row(build: u32) -> String {
    format!(
        "wow_classic_beta|1|1.60.1.{build}|{}|{}|Windows enUS text deDE speech enUS",
        "a".repeat(32),
        "b".repeat(32)
    )
}
fn input(rows: &str) -> Vec<u8> {
    format!("{HEADER}\n{rows}\n").into_bytes()
}

#[test]
fn selected_active_product_preserves_70425_and_does_not_relabel_target() {
    let text = input(&format!(
        "{}\n{}",
        row(70425),
        row(70374).replace("|1|", "|0|")
    ));
    let metadata = inspect_build_info(&text, "wow_classic_beta").unwrap();
    assert_eq!(metadata.build, 70425);
    assert_eq!(metadata.version, "1.60.1.70425");
    assert_eq!(metadata.declared_locales, ["deDE", "enUS"]);
    assert!(inspect_build_info(&text, "wow").is_err());
    let crlf = String::from_utf8(text).unwrap().replace('\n', "\r\n");
    assert_eq!(
        inspect_build_info(crlf.as_bytes(), "wow_classic_beta").unwrap(),
        metadata
    );
}

#[test]
fn refuses_ambiguous_missing_truncated_and_invalid_metadata() {
    let valid = input(&row(70425));
    for bytes in [
        input(&format!("{}\n{}", row(70425), row(70374))),
        input(&row(70425).replace("|1|", "|0|")),
        input(&row(0)),
        input(&row(70425).replace("70425", "4294967296")),
        input(&row(70425).replace("70425", "070425")),
        input(&row(70425).replace("|1|", "|2|")),
        input(&row(70425).replace("Windows", "\0")),
        input("wow_classic_beta|1"),
        valid[..20].to_vec(),
        [vec![0xff], valid.clone()].concat(),
        vec![b'x'; MAX_BUILD_INFO_BYTES + 1],
        String::from_utf8(valid.clone())
            .unwrap()
            .replace("Active!DEC:1", "Active!STRING:0")
            .into_bytes(),
        String::from_utf8(valid)
            .unwrap()
            .replace("Tags!STRING:0", "Product!STRING:0")
            .into_bytes(),
    ] {
        assert!(inspect_build_info(&bytes, "wow_classic_beta").is_err());
    }
    assert!(
        inspect_build_info(
            &input(&vec![row(70425); MAX_BUILD_INFO_ROWS + 1].join("\n")),
            "wow_classic_beta"
        )
        .is_err()
    );
}

#[test]
fn presets_are_whole_worker_and_total_storage_ceilings() {
    let cases = [
        (PerformancePreset::Small, 1, 1, 8),
        (PerformancePreset::Balanced, 2, 2, 32),
        (PerformancePreset::Large, 4, 4, 64),
        (PerformancePreset::Max, 8, 8, 128),
    ];
    for (preset, workers, memory, storage) in cases {
        assert_eq!(
            preset.limits(),
            ResourceLimits {
                workers,
                worker_memory_bytes: memory << 30,
                total_storage_bytes: storage << 30
            }
        );
    }
    assert_eq!(PerformancePreset::default(), PerformancePreset::Max);
    assert!(serde_json::from_str::<PerformancePreset>("\"unlimited\"").is_err());
}

fn bundle() -> MetadataBundle {
    MetadataBundle {
        format_version: FORMAT_VERSION,
        wowdbdefs_revision: WOWDBDEFS_REVISION.into(),
        objects: vec![
            MetadataObject {
                sha256: "a".repeat(64),
                bytes: 100,
                role: MetadataRole::Definition,
            },
            MetadataObject {
                sha256: "b".repeat(64),
                bytes: 50,
                role: MetadataRole::License,
            },
        ],
        tables: vec![TableDefinition {
            table: "Map".into(),
            file_data_id: 123,
            product: "wow_classic_beta".into(),
            build: 70425,
            layout_hash: "0123abcd".into(),
            definition_sha256: "a".repeat(64),
            license_sha256: "b".repeat(64),
            transform: "synthetic_test_only_v1".into(),
        }],
    }
}
const TRANSFORMS: &[&str] = &["synthetic_test_only_v1"];

#[test]
fn metadata_requires_exact_revision_versions_closure_and_compiled_transforms() {
    let original = bundle();
    assert!(validate_metadata(&original, TRANSFORMS).is_ok());
    assert!(validate_metadata(&original, &[]).is_err());
    let mutations: &[fn(&mut MetadataBundle)] = &[
        |b| b.format_version += 1,
        |b| b.wowdbdefs_revision = "master".into(),
        |b| b.tables[0].build = 0,
        |b| b.tables[0].file_data_id = 0,
        |b| b.tables[0].table = "../Map".into(),
        |b| b.tables[0].layout_hash = "guess".into(),
        |b| b.tables[0].transform = "eval_lua".into(),
        |b| b.tables[0].license_sha256 = "c".repeat(64),
        |b| b.objects[0].sha256 = "../bad".into(),
        |b| b.objects[0].bytes = u64::MAX,
        |b| b.objects[1].role = MetadataRole::Notice,
        |b| b.objects.push(b.objects[0].clone()),
        |b| b.tables.push(b.tables[0].clone()),
        |b| {
            let mut t = b.tables[0].clone();
            t.table = "Other".into();
            b.tables.push(t);
        },
    ];
    for mutate in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert!(validate_metadata(&changed, TRANSFORMS).is_err());
    }
}

#[test]
fn metadata_json_refuses_unknown_fields_duplicate_members_and_oversize() {
    let text = serde_json::to_string(&bundle()).unwrap();
    assert_eq!(
        decode_metadata(text.as_bytes(), TRANSFORMS).unwrap(),
        bundle()
    );
    for changed in [
        text.replacen('{', "{\"format_version\":1,", 1),
        text.replace("\"table\":\"Map\"", "\"table\":\"Map\",\"table\":\"Other\""),
        text.replacen('{', "{\"execute\":true,", 1),
        text[..text.len() - 1].into(),
    ] {
        assert!(decode_metadata(changed.as_bytes(), TRANSFORMS).is_err());
    }
    assert!(decode_metadata(&vec![b' '; MAX_METADATA_BYTES + 1], TRANSFORMS).is_err());
}
