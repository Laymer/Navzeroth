//! Real private lifecycle orchestration with synthetic ABI payloads only.
use super::*;
use crate::native::boundary_tests::{open_staging, stage_with_bytes};
use azeroth_core::local_data::PerformancePreset;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    request: ExtractionRequest,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "nav-staging-abi-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let source = root.join("source");
        let destination = root.join("store");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&destination).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&destination, fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(source.join(".build.info"),format!("Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70450|{}|{}|enUS text\n", "a".repeat(32), "b".repeat(32))).unwrap();
        for key in ["a".repeat(32), "b".repeat(32)] {
            let dir = source.join(format!("Data/config/{}/{}", &key[..2], &key[2..4]));
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(key), b"synthetic configuration").unwrap();
        }
        fs::create_dir_all(source.join("Data/data")).unwrap();
        fs::write(source.join("Data/data/data.000"), b"synthetic segment").unwrap();
        Self {
            root,
            request: ExtractionRequest {
                protocol_version: 3,
                request_id: "synthetic-job".into(),
                installation: source,
                product: "wow_classic_beta".into(),
                destination,
                preset: PerformancePreset::Small,
                tables: vec!["Map".into()],
                locales: vec!["enUS".into()],
                resume: false,
            },
        }
    }
    fn run(&self, progress: impl FnMut(u32, u32) -> Result<()>) -> Result<ExtractionSummary> {
        stage_with_bytes(b"unsupported synthetic table".to_vec(), || {
            extract_using(&self.request, progress, open_staging)
        })
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn empty_map(table_hash: u32, layout: u32, fields: usize) -> Vec<u8> {
    let mut bytes = vec![0; 204 + fields * 28];
    bytes[..4].copy_from_slice(b"WDC5");
    for (at, value) in [
        (4, 5),
        (140, fields as u32),
        (152, table_hash),
        (156, layout),
        (176, fields as u32),
        (188, fields as u32 * 24),
    ] {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[test]
fn header_identity_layout_and_justified_empty_decode_have_distinct_dispositions() {
    let candidate = metadata::candidates()
        .into_iter()
        .find(|c| c.name == "Map")
        .unwrap();
    let table_hash = u32::from_str_radix(&candidate.table_hash, 16).unwrap();
    let layout = 0xd43afac3;
    let definition = metadata::definition("Map", 70450, layout).unwrap();
    let fields = definition
        .layout
        .entries
        .iter()
        .filter(|e| e.inline)
        .count();
    for (hash, layout, expected, decoded) in [
        (
            table_hash ^ 1,
            layout,
            InventoryDisposition::InvalidCorrupt,
            0,
        ),
        (table_hash, 0, InventoryDisposition::LayoutMismatch, 0),
        (table_hash, layout, InventoryDisposition::Decoded, 1),
    ] {
        let f = Fixture::new();
        let summary = stage_with_bytes(empty_map(hash, layout, fields), || {
            extract_using(&f.request, |_, _| Ok(()), open_staging)
        })
        .unwrap();
        let catalog = verify_catalog(&f.request.destination, &summary.catalog_sha256).unwrap();
        assert_eq!(catalog.tables[0].disposition, expected);
        assert_eq!(summary.tables_decoded, decoded);
        assert_eq!(catalog.tables[0].records, Some(0));
        assert!(!summary.activated);
        assert!(!summary.gameplay_projection_available);
        if decoded == 1 {
            let digest = catalog.tables[0].decoded_sha256.as_ref().unwrap();
            let document: tables::DecodedTable = serde_json::from_slice(
                &fs::read(f.request.destination.join("objects").join(digest)).unwrap(),
            )
            .unwrap();
            assert!(document.rows.is_empty());
            assert!(document.gaps.is_empty());
        }
    }
}

#[test]
fn unsupported_payload_is_immutable_research_and_never_activates_implicitly() {
    let f = Fixture::new();
    let mut events = Vec::new();
    let result = f
        .run(|done, total| {
            events.push((done, total));
            Ok(())
        })
        .unwrap();
    assert_eq!(result.tables_processed, 1);
    assert_eq!(result.tables_decoded, 0);
    assert!(!result.activated);
    assert!(!result.gameplay_projection_available);
    assert_eq!(events, vec![(0, 0), (0, 1), (1, 1)]);
    let catalog = verify_catalog(&f.request.destination, &result.catalog_sha256).unwrap();
    assert_eq!(catalog.tables.len(), 1);
    assert_eq!(
        catalog.tables[0].disposition,
        InventoryDisposition::RetainedUnsupported
    );
    assert!(catalog.tables[0].decoded_sha256.is_none());
    assert!(catalog.tables[0].retained_sha256.is_some());
    assert_eq!(
        crate::lifecycle::status(&f.request.destination, &f.request.request_id)
            .unwrap()
            .state,
        JobState::Complete
    );
    let active = crate::lifecycle::active(&f.request.destination, &result.catalog_sha256).unwrap();
    assert!(active.catalog_sha256.is_none());
    assert!(f.run(|_, _| Ok(())).is_err());
    assert_eq!(
        verify_catalog(&f.request.destination, &result.catalog_sha256)
            .unwrap()
            .tables
            .len(),
        1
    );
}

#[test]
fn cancellation_and_source_changes_refuse_sealing_then_exact_source_can_resume() {
    let mut f = Fixture::new();
    assert_eq!(
        f.run(|done, total| if total == 1 && done == 1 {
            Err("synthetic interrupted progress")
        } else {
            Ok(())
        })
        .err(),
        Some("synthetic interrupted progress")
    );
    assert_eq!(
        crate::lifecycle::status(&f.request.destination, &f.request.request_id)
            .unwrap()
            .state,
        JobState::Failed
    );
    f.request.resume = true;
    let result = f.run(|_, _| Ok(())).unwrap();
    assert_eq!(result.tables_processed, 1);
    let mut changed = Fixture::new();
    assert!(
        changed
            .run(|done, total| if total == 1 && done == 1 {
                fs::write(
                    changed.request.installation.join("Data/data/data.000"),
                    b"changed segment",
                )
                .unwrap();
                Ok(())
            } else {
                Ok(())
            })
            .is_err()
    );
    assert_eq!(
        fs::read_dir(changed.request.destination.join("catalogs"))
            .unwrap()
            .count(),
        0
    );
    changed.request.resume = true;
    assert!(changed.run(|_, _| Ok(())).is_err());
    let cancelled = Fixture::new();
    assert!(
        cancelled
            .run(|_, total| {
                if total == 1 {
                    crate::lifecycle::cancel(
                        &cancelled.request.destination,
                        &cancelled.request.request_id,
                    )?;
                }
                Ok(())
            })
            .is_err()
    );
    assert_eq!(
        crate::lifecycle::status(
            &cancelled.request.destination,
            &cancelled.request.request_id
        )
        .unwrap()
        .state,
        JobState::Cancelled
    );
}

#[test]
fn explicit_installed_locales_are_processed_separately_and_bad_selections_do_not_start_jobs() {
    let mut f = Fixture::new();
    let mut locales: Vec<String> = azeroth_core::local_data_workflow::CONTENT_LOCALES
        .iter()
        .map(|s| s.to_string())
        .collect();
    locales.sort();
    let info = f.request.installation.join(".build.info");
    let original = fs::read_to_string(&info).unwrap();
    fs::write(
        &info,
        original.replace("enUS text", &format!("{} text", locales.join(" "))),
    )
    .unwrap();
    f.request.locales = locales.clone();
    let summary = f.run(|_, _| Ok(())).unwrap();
    let catalog = verify_catalog(&f.request.destination, &summary.catalog_sha256).unwrap();
    assert_eq!(summary.tables_processed, locales.len() as u32);
    assert_eq!(
        catalog
            .tables
            .iter()
            .map(|t| t.locale.clone().unwrap())
            .collect::<Vec<_>>(),
        locales
    );
    for case in ["missing-table", "uninstalled-locale", "overlapping-store"] {
        let mut bad = Fixture::new();
        match case {
            "missing-table" => bad.request.tables = vec!["DefinitelyAbsentSyntheticTable".into()],
            "uninstalled-locale" => bad.request.locales = vec!["deDE".into()],
            _ => bad.request.destination = bad.request.installation.clone(),
        }
        assert!(bad.run(|_, _| Ok(())).is_err());
        assert!(!bad.request.destination.join("lifecycle.sqlite3").exists());
    }
}

fn media_catalog(f: &Fixture) -> String {
    let summary = f.run(|_, _| Ok(())).unwrap();
    let mut catalog = verify_catalog(&f.request.destination, &summary.catalog_sha256).unwrap();
    let table = tables::DecodedTable {
        format_version: 1,
        header: tables::TableHeader {
            format: "WDC5".into(),
            records: 1,
            fields: 1,
            record_bytes: 4,
            table_hash: 0,
            layout_hash: 0x9a2a4834,
            locale_mask: 2,
            sections: 1,
        },
        rows: vec![tables::ResearchRow {
            id: 1,
            fields: BTreeMap::from([("IconFileDataID".into(), tables::Cell::Unsigned(900))]),
            copied_from: None,
        }],
        gaps: vec![],
    };
    let mut store = Store::open(
        &f.request.destination,
        PerformancePreset::Small.limits().total_storage_bytes,
    )
    .unwrap();
    let object = store
        .put("objects", &serde_json::to_vec(&table).unwrap())
        .unwrap();
    let retained = store.put("objects", b"synthetic retained table").unwrap();
    catalog.tables.insert(
        0,
        TableInventory {
            table: "Item".into(),
            locale: Some("enUS".into()),
            file_data_id: Some(841626),
            disposition: InventoryDisposition::Decoded,
            reason: "research_decoded".into(),
            retained_sha256: Some(retained),
            decoded_sha256: Some(object),
            encoded_bytes: None,
            records: Some(1),
            layout_hash: Some("9a2a4834".into()),
        },
    );
    store
        .put("catalogs", &serde_json::to_vec(&catalog).unwrap())
        .unwrap()
}

#[test]
fn bound_media_decodes_supported_bytes_and_keeps_every_native_refusal_explicit() {
    use azeroth_core::local_data_media::{Disposition, ReadRefusal};
    let f = Fixture::new();
    let catalog = media_catalog(&f);
    let mut request = crate::media::Request {
        format_version: 1,
        installation: f.request.installation.clone(),
        destination: f.request.destination.clone(),
        catalog,
        locale: "enUS".into(),
        ids: vec![900],
        preset: PerformancePreset::Small,
    };
    let mut blp = vec![0; 152];
    blp[..4].copy_from_slice(b"BLP2");
    blp[4] = 1;
    blp[8] = 3;
    blp[9] = 8;
    blp[12] = 1;
    blp[16] = 1;
    blp[20] = 148;
    blp[84] = 4;
    blp[148..].copy_from_slice(&[1, 2, 3, 0]);
    let revision = stage_with_bytes(blp, || crate::media::extract_synthetic(&request)).unwrap();
    let manifest = crate::media::verify(&request.destination, &revision).unwrap();
    assert_eq!(manifest.assets[0].disposition, Disposition::Decoded);
    assert_eq!(manifest.assets[0].width, Some(1));
    assert_eq!(manifest.assets[0].height, Some(1));
    assert!(manifest.local_only);
    assert!(!manifest.calibrated);
    assert!(!manifest.verified_for_gameplay);
    for (code, refusal) in [
        (1, ReadRefusal::Missing),
        (2, ReadRefusal::Encrypted),
        (3, ReadRefusal::Unsupported),
        (4, ReadRefusal::Invalid),
        (5, ReadRefusal::Capacity),
        (6, ReadRefusal::Unavailable),
        (7, ReadRefusal::Truncated),
        (8, ReadRefusal::ShortRead),
    ] {
        let revision = crate::native::boundary_tests::stage_with_failure(code, || {
            crate::media::extract_synthetic(&request)
        })
        .unwrap();
        let manifest = crate::media::verify(&request.destination, &revision).unwrap();
        assert_eq!(manifest.assets[0].read_refusal, Some(refusal));
        assert!(manifest.assets[0].png_sha256.is_none());
    }
    request.ids = vec![901];
    assert!(crate::media::extract_synthetic(&request).is_err());
    request.ids = vec![900];
    fs::write(
        request.installation.join("Data/data/data.000"),
        b"changed synthetic source",
    )
    .unwrap();
    assert!(crate::media::extract_synthetic(&request).is_err());
    // The earlier usable immutable media remains readable after every refusal.
    assert!(crate::media::verify(&request.destination, &revision).is_ok());
}

#[test]
fn encrypted_metadata_and_unsupported_plain_headers_keep_partial_lineage() {
    for (header, disposition) in [
        ([1, 2, 3, 4], InventoryDisposition::RetainedUnsupported),
        (*b"WDC5", InventoryDisposition::UnavailableEncrypted),
    ] {
        let f = Fixture::new();
        let result = crate::native::boundary_tests::stage_with_partial_header(header, || {
            extract_using(&f.request, |_, _| Ok(()), open_staging)
        })
        .unwrap();
        let catalog = verify_catalog(&f.request.destination, &result.catalog_sha256).unwrap();
        let entry = &catalog.tables[0];
        assert_eq!(entry.disposition, disposition);
        assert_eq!(entry.encoded_bytes, Some(8));
        assert!(entry.decoded_sha256.is_none());
        let retained = fs::read(
            f.request
                .destination
                .join("objects")
                .join(entry.retained_sha256.as_ref().unwrap()),
        )
        .unwrap();
        assert!(retained.ends_with(&header));
        assert!(
            catalog
                .gaps
                .iter()
                .any(|gap| gap == "Map:unavailable_encrypted_frames:1-1:bytes:4+4")
        );
        if disposition == InventoryDisposition::UnavailableEncrypted {
            assert_eq!(entry.reason, UNAVAILABLE_METADATA);
        } else {
            assert!(entry.reason.ends_with(PARTIAL_SUFFIX));
        }
        assert_eq!(result.tables_decoded, 0);
        assert!(!result.activated);
        assert!(!result.gameplay_projection_available);
    }
}

#[test]
fn changed_build_artifacts_preserve_staging_and_refuse_substitution() {
    use crate::changes;
    let f = Fixture::new();
    let root = &f.request.destination;
    let current = changes::read_build_identity(&f.request.installation).unwrap();
    let previous = "0".repeat(64);
    let selection = "1".repeat(64);
    crate::lifecycle::index(root)
        .unwrap()
        .save_selection(&azeroth_storage::local_data::SavedSelection {
            selection_id: selection.clone(),
            installation: f.request.installation.clone(),
            product: f.request.product.clone(),
            preset: PerformancePreset::Small,
        })
        .unwrap();
    assert!(
        changes::stage(
            root,
            &f.request.installation,
            &f.request.product,
            &selection,
            &current,
            &current,
            PerformancePreset::Small
        )
        .is_err()
    );
    assert!(
        changes::stage(
            root,
            &f.request.installation,
            &f.request.product,
            &selection,
            &previous,
            &previous,
            PerformancePreset::Small
        )
        .is_err()
    );
    let digest = changes::stage(
        root,
        &f.request.installation,
        &f.request.product,
        &selection,
        &previous,
        &current,
        PerformancePreset::Small,
    )
    .unwrap();
    let candidate = changes::verify(root, &digest).unwrap();
    assert!(!candidate.activated);
    let dir = root.join("changes");
    fs::write(dir.join(".owned.staging"), b"incomplete").unwrap();
    let page = changes::artifacts(root, "changes", 0, 1).unwrap();
    assert_eq!(page.rows.as_slice(), std::slice::from_ref(&digest));
    assert!(page.staging_present);
    assert!(!page.integrity_checked);
    assert!(changes::artifacts(root, "changes", 2, 1).is_err());
    assert!(changes::artifacts(root, "../changes", 0, 1).is_err());
    fs::write(dir.join("unexpected"), b"untrusted").unwrap();
    assert!(changes::artifacts(root, "changes", 0, 1).is_err());
    fs::remove_file(dir.join("unexpected")).unwrap();
    fs::create_dir(dir.join("nested")).unwrap();
    assert!(changes::artifacts(root, "changes", 0, 1).is_err());
    fs::remove_dir(dir.join("nested")).unwrap();
    for field in [
        "format_version",
        "local_only",
        "activated",
        "profile_sha256",
    ] {
        let mut altered = serde_json::to_value(&candidate).unwrap();
        altered[field] = match field {
            "format_version" => serde_json::json!(2),
            "local_only" => serde_json::json!(false),
            "activated" => serde_json::json!(true),
            _ => serde_json::json!("2".repeat(64)),
        };
        let bytes = crate::bounded_json(&altered, 64 * 1024).unwrap();
        let id = crate::hash(&bytes);
        fs::write(dir.join(&id), bytes).unwrap();
        assert!(changes::verify(root, &id).is_err(), "{field}");
    }
    fs::write(dir.join(&digest), b"altered").unwrap();
    assert!(changes::verify(root, &digest).is_err());
    assert!(changes::verify(root, "invalid").is_err());
}

#[test]
fn media_discovery_preserves_unknown_zero_duplicates_and_decode_refusals() {
    use crate::media;
    use azeroth_core::local_data_media::Disposition;
    let f = Fixture::new();
    let original = media_catalog(&f);
    let root = &f.request.destination;
    let mut catalog = verify_catalog(root, &original).unwrap();
    let mut table: tables::DecodedTable = serde_json::from_slice(
        &fs::read(
            root.join("objects")
                .join(catalog.tables[0].decoded_sha256.as_ref().unwrap()),
        )
        .unwrap(),
    )
    .unwrap();
    for (id, value) in [
        (2, Some(tables::Cell::Signed(901))),
        (3, Some(tables::Cell::Signed(902))),
        (4, Some(tables::Cell::Unsigned(0))),
        (5, Some(tables::Cell::Unknown)),
        (6, None),
        (7, Some(tables::Cell::Text("unsupported expression".into()))),
        (8, Some(tables::Cell::Unsigned(u64::MAX))),
        (1, Some(tables::Cell::Unsigned(900))),
    ] {
        table.rows.push(tables::ResearchRow {
            id,
            fields: value
                .map(|v| BTreeMap::from([("IconFileDataID".into(), v)]))
                .unwrap_or_default(),
            copied_from: None,
        });
    }
    table.header.records = table.rows.len() as u32;
    table.gaps.push("synthetic_unavailable_range".into());
    let mut store =
        Store::open(root, PerformancePreset::Small.limits().total_storage_bytes).unwrap();
    catalog.tables[0].decoded_sha256 = Some(
        store
            .put("objects", &serde_json::to_vec(&table).unwrap())
            .unwrap(),
    );
    catalog.tables[0].records = Some(table.rows.len() as u32);
    let revision = store
        .put("catalogs", &serde_json::to_vec(&catalog).unwrap())
        .unwrap();
    drop(store);
    let discovery = media::discover(root, &revision, "enUS").unwrap();
    assert_eq!(
        discovery
            .candidates
            .iter()
            .map(|c| c.file_data_id)
            .collect::<Vec<_>>(),
        [900, 901, 902]
    );
    for gap in [
        "Item:duplicate_record_identity",
        "Item:IconFileDataID:zero_sentinel",
        "Item:IconFileDataID:unknown",
        "Item:IconFileDataID:missing",
        "Item:IconFileDataID:unsupported",
        "Item:synthetic_unavailable_range",
    ] {
        assert!(discovery.gaps.iter().any(|actual| actual == gap), "{gap}");
    }
    let mut capacity = vec![0; 148];
    capacity[..4].copy_from_slice(b"BLP2");
    capacity[4] = 1;
    capacity[12..16].copy_from_slice(&4096u32.to_le_bytes());
    capacity[16..20].copy_from_slice(&4096u32.to_le_bytes());
    let files = BTreeMap::from([
        (900, b"BLP1unsupported".to_vec()),
        (901, b"BLP2".to_vec()),
        (902, capacity),
    ]);
    let media_revision =
        media::stage_fixture(root, &revision, "enUS", &files, PerformancePreset::Small).unwrap();
    let manifest = media::verify(root, &media_revision).unwrap();
    assert!(manifest.fixture && !manifest.verified_for_gameplay);
    assert_eq!(
        manifest
            .assets
            .iter()
            .map(|a| a.disposition)
            .collect::<Vec<_>>(),
        [
            Disposition::Unsupported,
            Disposition::Invalid,
            Disposition::Capacity
        ]
    );
    assert!(manifest.assets.iter().all(|a| a.png_sha256.is_none()));
    assert!(
        media::page(root, &media_revision, 0, 200)
            .unwrap()
            .assets
            .is_empty()
    );
    assert!(media::page(root, &media_revision, 1, 200).is_err());
    for (layout, decoded, gap) in [
        ("00000000", true, "Item:unsupported_layout"),
        ("9a2a4834", false, "Item:undecoded:synthetic_undecoded"),
    ] {
        let mut changed = catalog.clone();
        changed.tables[0].layout_hash = Some(layout.into());
        if !decoded {
            changed.tables[0].disposition = InventoryDisposition::RetainedUnsupported;
            changed.tables[0].decoded_sha256 = None;
            changed.tables[0].reason = "synthetic_undecoded".into();
        }
        let mut store =
            Store::open(root, PerformancePreset::Small.limits().total_storage_bytes).unwrap();
        let id = store
            .put("catalogs", &serde_json::to_vec(&changed).unwrap())
            .unwrap();
        drop(store);
        let unavailable = media::discover(root, &id, "enUS").unwrap();
        assert!(unavailable.candidates.is_empty());
        assert!(unavailable.gaps.iter().any(|value| value == gap), "{gap}");
    }
    assert!(
        media::verify(root, &media_revision).is_ok(),
        "new discoveries cannot alter an earlier immutable revision"
    );
}
