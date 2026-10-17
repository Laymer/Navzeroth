//! Synthetic catalogs only: no client tables, private paths or real records.
use azeroth_core::{
    local_data::{CASCLIB_REVISION, InstallationMetadata, InventoryDisposition},
    local_data_catalog::{DistributionClass, ResearchCatalog, TableInventory},
    local_data_projection::{EquipmentSlot, Fact, LocalProjection, SourceState},
};
use azeroth_local_data::{
    metadata::METADATA_SHA256,
    projection::{self, Reproduction, SOURCES},
    tables::{Cell, DecodedTable, ResearchRow, TableHeader},
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};

const CATALOG: &str = "1111111111111111111111111111111111111111111111111111111111111111";

fn u(v: u64) -> Cell {
    Cell::Unsigned(v)
}
fn s(v: i64) -> Cell {
    Cell::Signed(v)
}
fn t(v: &str) -> Cell {
    Cell::Text(v.into())
}
fn f(values: &[f64]) -> Cell {
    Cell::Array(values.iter().map(|v| Cell::Float(*v)).collect())
}
fn row(id: u32, fields: &[(&str, Cell)]) -> ResearchRow {
    ResearchRow {
        id,
        fields: fields
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect(),
        copied_from: None,
    }
}
fn table(rows: Vec<ResearchRow>, gaps: &[&str]) -> DecodedTable {
    DecodedTable {
        format_version: 1,
        header: TableHeader {
            format: "WDC5".into(),
            records: rows.len() as u32,
            fields: 1,
            record_bytes: 4,
            table_hash: 0,
            layout_hash: 0,
            locale_mask: 2,
            sections: 1,
        },
        rows,
        gaps: gaps.iter().map(|g| (*g).to_owned()).collect(),
    }
}

fn tables() -> BTreeMap<&'static str, DecodedTable> {
    let stat_types = Cell::Array((0..10).map(|i| s(if i == 0 { 7 } else { -1 })).collect());
    let stat_amounts = Cell::Array((0..10).map(|i| s(if i == 0 { 5000 } else { 0 })).collect());
    let sparse = |id, name: &str, level, inv| {
        row(
            id,
            &[
                ("Display_lang", t(name)),
                ("OverallQualityID", s(1)),
                ("ItemLevel", u(level)),
                ("RequiredLevel", s(0)),
                ("InventoryType", s(inv)),
                ("AllowableClass", s(-1)),
                ("AllowableRace", Cell::Array(vec![s(-1), s(0)])),
                ("RequiredSkill", u(0)),
                ("RequiredSkillRank", u(0)),
                ("Bonding", u(1)),
                ("StatModifier_bonusStat", stat_types.clone()),
                ("StatPercentEditor", stat_amounts.clone()),
            ],
        )
    };
    BTreeMap::from([
        (
            "ItemSparse",
            table(
                vec![
                    sparse(10, "Synthetic Cap", 5, 1),
                    sparse(11, "Synthetic Ring", 7, 11),
                    sparse(12, "Synthetic Oddity", 3, 99),
                    // Missing quality field: projected as missing, not zero.
                    row(13, &[("Display_lang", t("Synthetic Stone"))]),
                ],
                &[],
            ),
        ),
        (
            "Item",
            table(
                vec![
                    row(
                        10,
                        &[
                            ("ClassID", s(4)),
                            ("SubclassID", u(1)),
                            ("InventoryType", s(1)),
                        ],
                    ),
                    row(
                        11,
                        &[
                            ("ClassID", s(4)),
                            ("SubclassID", u(0)),
                            ("InventoryType", s(2)),
                        ],
                    ),
                    row(
                        14,
                        &[
                            ("ClassID", s(15)),
                            ("SubclassID", u(0)),
                            ("InventoryType", s(0)),
                        ],
                    ),
                ],
                &[],
            ),
        ),
        (
            "Map",
            table(
                vec![row(
                    0,
                    &[
                        ("MapName_lang", t("Synthetic Continent")),
                        ("Directory", t("synthetic")),
                        ("MapType", u(0)),
                        ("InstanceType", s(0)),
                        ("ParentMapID", s(-1)),
                    ],
                )],
                &[],
            ),
        ),
        (
            "UiMap",
            table(
                vec![
                    row(
                        1,
                        &[("Name_lang", t("Root")), ("ParentUiMapID", Cell::Unknown)],
                    ),
                    row(2, &[("Name_lang", t("Child")), ("ParentUiMapID", u(1))]),
                ],
                &[],
            ),
        ),
        (
            "UiMapAssignment",
            table(
                vec![row(
                    5,
                    &[
                        ("UiMapID", u(2)),
                        ("MapID", s(0)),
                        ("UiMin", f(&[0.0, 0.0])),
                        ("UiMax", f(&[1.0, 1.0])),
                        ("Region", f(&[-10.0, -10.0, 0.0, 10.0, 10.0, 5.0])),
                    ],
                )],
                &[],
            ),
        ),
        (
            "TaxiNodes",
            table(
                vec![
                    row(
                        1,
                        &[
                            ("Name_lang", t("North")),
                            ("ContinentID", u(0)),
                            ("Pos", f(&[1.0, 2.0, 3.0])),
                        ],
                    ),
                    row(
                        2,
                        &[
                            ("Name_lang", t("South")),
                            ("ContinentID", u(0)),
                            ("Pos", f(&[4.0, 5.0, 6.0])),
                        ],
                    ),
                ],
                &[],
            ),
        ),
        (
            "TaxiPath",
            table(
                vec![
                    row(
                        100,
                        &[("FromTaxiNode", u(1)), ("ToTaxiNode", u(2)), ("Cost", u(0))],
                    ),
                    row(
                        101,
                        &[
                            ("FromTaxiNode", Cell::Unknown),
                            ("ToTaxiNode", u(1)),
                            ("Cost", u(50)),
                        ],
                    ),
                ],
                &[],
            ),
        ),
        (
            "TaxiPathNode",
            table(
                vec![
                    row(
                        1000,
                        &[
                            ("PathID", u(100)),
                            ("NodeIndex", s(1)),
                            ("Loc", f(&[2.0, 2.0, 2.0])),
                        ],
                    ),
                    row(
                        1001,
                        &[
                            ("PathID", u(100)),
                            ("NodeIndex", s(0)),
                            ("Loc", f(&[1.0, 1.0, 1.0])),
                        ],
                    ),
                    row(1002, &[("PathID", u(101)), ("NodeIndex", s(0))]),
                    row(1003, &[("PathID", u(101)), ("NodeIndex", s(0))]),
                    row(1004, &[("PathID", u(999)), ("NodeIndex", s(0))]),
                ],
                &[],
            ),
        ),
        (
            "ChrClasses",
            table(
                vec![row(
                    1,
                    &[
                        ("Name_lang", t("Synthetic Class")),
                        ("Filename", t("SYNTH")),
                        ("RolesMask", u(0)),
                    ],
                )],
                &["encrypted_section_records:2"],
            ),
        ),
    ])
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn catalog(decoded: &BTreeMap<&str, DecodedTable>) -> (ResearchCatalog, BTreeMap<String, Vec<u8>>) {
    let mut objects = BTreeMap::new();
    let mut inventory = Vec::new();
    for (_, name, layouts) in SOURCES {
        let mut entry = TableInventory {
            table: (*name).into(),
            locale: Some("enUS".into()),
            file_data_id: Some(1),
            disposition: InventoryDisposition::NotLocallyPresent,
            reason: "not_locally_present".into(),
            retained_sha256: None,
            decoded_sha256: None,
            encoded_bytes: None,
            records: None,
            layout_hash: None,
        };
        if let Some(table) = decoded.get(name) {
            let raw = format!("synthetic raw {name}").into_bytes();
            let json = serde_json::to_vec(table).unwrap();
            entry.retained_sha256 = Some(hash(&raw));
            entry.decoded_sha256 = Some(hash(&json));
            entry.disposition = InventoryDisposition::Decoded;
            entry.reason = "research_decoded".into();
            entry.layout_hash = Some(layouts[0].into());
            objects.insert(hash(&raw), raw);
            objects.insert(hash(&json), json);
        }
        inventory.push(entry);
    }
    let catalog = ResearchCatalog {
        format_version: 1,
        classification: DistributionClass::LocalOnly,
        installation: InstallationMetadata {
            product: "wow_classic_beta".into(),
            version: "1.60.1.70425".into(),
            build: 70425,
            build_key: "a".repeat(32),
            cdn_key: "b".repeat(32),
            declared_locales: vec!["enUS".into()],
        },
        build_info_sha256: "c".repeat(64),
        extractor_version: "synthetic".into(),
        native_revision: CASCLIB_REVISION.into(),
        metadata_sha256: METADATA_SHA256.into(),
        parser_profile: "synthetic".into(),
        extractor_profile_sha256: None,
        archive_statistics: None,
        input_hashes: BTreeMap::new(),
        tables: inventory,
        gaps: vec![],
        gameplay_projection_sha256: None,
    };
    (catalog, objects)
}

fn run(catalog: &ResearchCatalog, objects: &BTreeMap<String, Vec<u8>>) -> LocalProjection {
    projection::project(catalog, CATALOG, "enUS", |entry| {
        let bytes = &objects[entry.decoded_sha256.as_ref().unwrap()];
        Ok(serde_json::from_slice(bytes).unwrap())
    })
    .unwrap()
}

#[test]
fn equipment_keeps_zero_unknown_missing_and_unsupported_distinct() {
    let (catalog, objects) = catalog(&tables());
    let p = run(&catalog, &objects);
    assert!(!p.activated && !p.verified_for_planning);
    assert_eq!((p.build, p.locale.as_str()), (70425, "enUS"));
    assert_eq!(p.catalog_sha256, CATALOG);
    let items: BTreeMap<_, _> = p.equipment.items.iter().map(|i| (i.item_id, i)).collect();
    let cap = items[&10];
    assert_eq!(cap.name, Fact::Known("Synthetic Cap".into()));
    assert_eq!(cap.required_level, Fact::Known(0));
    assert_eq!(cap.slots, Fact::Known(vec![EquipmentSlot::Head]));
    assert_eq!(
        (cap.class_id.clone(), cap.subclass_id.clone()),
        (Fact::Known(4), Fact::Known(1))
    );
    let Fact::Known(stats) = &cap.stat_allocations else {
        panic!()
    };
    assert_eq!(
        (stats.len(), &stats[0].stat_type, &stats[1].allocation),
        (10, &Fact::Known(7), &Fact::Known(0))
    );
    // ItemSparse says finger, Item says neck: refused, not guessed.
    assert_eq!(items[&11].inventory_type, Fact::Unsupported);
    assert_eq!(items[&11].slots, Fact::Unsupported);
    // Unmapped inventory type is unsupported, not "not equippable".
    assert_eq!(items[&12].inventory_type, Fact::Known(99));
    assert_eq!(items[&12].slots, Fact::Unsupported);
    assert_eq!(items[&13].quality, Fact::Missing);
    assert_eq!(items[&13].class_id, Fact::Missing);
    // Item row without ItemSparse: identity kept, sparse facts missing.
    assert_eq!(items[&14].name, Fact::Missing);
    assert_eq!(items[&14].slots, Fact::Known(vec![]));
    for gap in [
        "equipment:inventory_type_disagreements:1",
        "equipment:inventory_types_without_slot_mapping:1",
        "equipment:item_rows_without_item_sparse:1",
        "equipment:item_sparse_rows_without_item:2",
        "research_only_not_activated",
    ] {
        assert!(p.gaps.iter().any(|g| g == gap), "{gap}");
    }
    assert!(p.equipment.sources_complete);
}

#[test]
fn travel_geography_and_classes_project_with_explicit_gaps() {
    let (catalog, objects) = catalog(&tables());
    let p = run(&catalog, &objects);
    let path = &p.travel.paths[0];
    assert!(path.endpoints_resolved && path.waypoints_ordered);
    assert_eq!(path.cost, Fact::Known(0));
    let order: Vec<_> = path.waypoints.iter().map(|w| w.waypoint_id).collect();
    assert_eq!(order, [1001, 1000]);
    let other = &p.travel.paths[1];
    assert_eq!(other.from_node, Fact::Unknown);
    assert!(!other.endpoints_resolved && !other.waypoints_ordered);
    assert_eq!(p.travel.nodes[0].position, Fact::Known(vec![1.0, 2.0, 3.0]));
    assert_eq!(p.geography.ui_maps[0].parent_ui_map_id, Fact::Unknown);
    assert_eq!(p.geography.ui_maps[1].parent_ui_map_id, Fact::Known(1));
    assert_eq!(p.geography.maps[0].parent_map_id, Fact::Known(-1));
    assert_eq!(p.geography.maps[0].max_players, Fact::Missing);
    assert_eq!(
        p.geography.ui_map_assignments[0].region,
        Fact::Known(vec![-10.0, -10.0, 0.0, 10.0, 10.0, 5.0])
    );
    assert_eq!(p.class_roles.classes[0].roles_mask, Fact::Known(0));
    assert!(p.travel.sources_complete && p.geography.sources_complete);
    // ChrRaces absent, ChrClasses had an encrypted section.
    assert!(!p.class_roles.sources_complete);
    assert!(p.class_roles.races.is_empty());
    let races = p.sources.iter().find(|s| s.table == "ChrRaces").unwrap();
    assert_eq!(races.state, SourceState::Gap);
    for gap in [
        "class_roles:ChrClasses:encrypted_section_records:2",
        "class_roles:ChrRaces:not_locally_present:not_locally_present",
        "travel:paths_with_unordered_waypoints:1",
        "travel:paths_with_unresolved_endpoints:1",
        "travel:waypoints_for_unprojected_paths:1",
    ] {
        assert!(p.gaps.iter().any(|g| g == gap), "{gap}");
    }
}

#[test]
fn unsupported_layouts_and_undeclared_locales_refuse_projection_input() {
    let (mut catalog, objects) = catalog(&tables());
    let entry = catalog
        .tables
        .iter_mut()
        .find(|e| e.table == "Map")
        .unwrap();
    entry.layout_hash = Some("00000000".into());
    let p = run(&catalog, &objects);
    assert!(p.geography.maps.is_empty() && !p.geography.sources_complete);
    assert!(
        p.gaps
            .iter()
            .any(|g| g == "geography:Map:projection_layout_unsupported")
    );
    let refused = projection::project(&catalog, CATALOG, "deDE", |_| unreachable!());
    assert_eq!(
        refused.unwrap_err(),
        "Locale is not declared by the catalog"
    );
    assert!(projection::project(&catalog, "not-a-digest", "enUS", |_| unreachable!()).is_err());
}

#[test]
fn projection_is_deterministic_regardless_of_row_order() {
    let (catalog, objects) = catalog(&tables());
    let first = serde_json::to_vec(&run(&catalog, &objects)).unwrap();
    let second = projection::project(&catalog, CATALOG, "enUS", |entry| {
        let mut table: DecodedTable =
            serde_json::from_slice(&objects[entry.decoded_sha256.as_ref().unwrap()]).unwrap();
        table.rows.reverse();
        Ok(table)
    })
    .unwrap();
    assert_eq!(first, serde_json::to_vec(&second).unwrap());
}

fn private_store(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "navzeroth-projection-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let mut builder = fs::DirBuilder::new();
    builder.recursive(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(&root).unwrap();
    root
}

fn seeded_store(name: &str) -> (PathBuf, String, ResearchCatalog) {
    let root = private_store(name);
    let (catalog, objects) = catalog(&tables());
    fs::create_dir(root.join("objects")).unwrap();
    fs::create_dir(root.join("catalogs")).unwrap();
    for (digest, bytes) in &objects {
        fs::write(root.join("objects").join(digest), bytes).unwrap();
    }
    let manifest = serde_json::to_vec(&catalog).unwrap();
    let catalog_sha256 = hash(&manifest);
    fs::write(root.join("catalogs").join(&catalog_sha256), &manifest).unwrap();
    (root, catalog_sha256, catalog)
}

/// Re-seal an edited projection under its new canonical digest.
fn reseal(root: &std::path::Path, digest: &str, edit: impl FnOnce(&mut LocalProjection)) -> String {
    let bytes = fs::read(root.join("projections").join(digest)).unwrap();
    let mut projection: LocalProjection = serde_json::from_slice(&bytes).unwrap();
    edit(&mut projection);
    let bytes = serde_json::to_vec(&projection).unwrap();
    let digest = hash(&bytes);
    fs::write(root.join("projections").join(&digest), bytes).unwrap();
    digest
}

#[test]
fn sealed_projection_verifies_reproduces_and_detects_tampering() {
    let (root, catalog_sha256, _) = seeded_store("seal");
    let summary = projection::project_catalog(&root, &catalog_sha256, "enUS").unwrap();
    assert_eq!(summary.catalog_sha256, catalog_sha256);
    assert!(!summary.activated && !summary.verified_for_planning);
    assert_eq!(summary.counts["equipment_items"], 5);
    assert_eq!(summary.counts["travel_waypoints"], 4);
    assert!(!summary.domains_complete["class_roles"]);
    let again = projection::project_catalog(&root, &catalog_sha256, "enUS").unwrap();
    assert_eq!(again.projection_sha256, summary.projection_sha256);

    let verified = projection::verify_projection(&root, &summary.projection_sha256).unwrap();
    assert!(verified.verified && verified.integrity_verified);
    assert_eq!(verified.reproduction, Reproduction::Reproduced);
    assert_eq!(
        verified.projector_profile_sha256,
        projection::PROJECTOR_PROFILE_SHA256
    );
    assert!(projection::project_catalog(&root, &catalog_sha256, "frFR").is_err());

    let sealed = root.join("projections").join(&summary.projection_sha256);
    let mut bytes = fs::read(&sealed).unwrap();
    let last = bytes.len() - 2;
    bytes[last] ^= 1;
    fs::write(&sealed, &bytes).unwrap();
    assert_eq!(
        projection::verify_projection(&root, &summary.projection_sha256).unwrap_err(),
        "Projection hash mismatch"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn other_projector_profiles_are_integrity_checked_but_not_verified() {
    let (root, catalog_sha256, _) = seeded_store("profile");
    let summary = projection::project_catalog(&root, &catalog_sha256, "enUS").unwrap();
    let foreign = reseal(&root, &summary.projection_sha256, |p| {
        p.projector_profile_sha256 = "e".repeat(64);
    });
    let report = projection::verify_projection(&root, &foreign).unwrap();
    assert!(report.integrity_verified && !report.verified);
    assert_eq!(report.reproduction, Reproduction::SkippedProfileMismatch);
    // Same profile, different records: re-derivation refuses rather than skips.
    let altered = reseal(&root, &summary.projection_sha256, |p| {
        p.equipment.items[0].item_level = Fact::Known(999);
    });
    assert_eq!(
        projection::verify_projection(&root, &altered).unwrap_err(),
        "Projection is not reproducible from its catalog"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn verification_refuses_tampered_or_rebound_decoded_objects() {
    let (root, catalog_sha256, catalog) = seeded_store("rebind");
    let summary = projection::project_catalog(&root, &catalog_sha256, "enUS").unwrap();
    // Point the Map source at another decoded object that is in the catalog.
    let other = catalog
        .tables
        .iter()
        .find(|e| e.table == "UiMap")
        .and_then(|e| e.decoded_sha256.clone())
        .unwrap();
    let rebound = reseal(&root, &summary.projection_sha256, |p| {
        let source = p.sources.iter_mut().find(|s| s.table == "Map").unwrap();
        source.decoded_sha256 = Some(other);
    });
    assert_eq!(
        projection::verify_projection(&root, &rebound).unwrap_err(),
        "Projection source is not bound to its catalog"
    );
    let decoded = catalog
        .tables
        .iter()
        .find(|e| e.table == "Map")
        .and_then(|e| e.decoded_sha256.clone())
        .unwrap();
    let object = root.join("objects").join(&decoded);
    let mut bytes = fs::read(&object).unwrap();
    bytes[0] ^= 1;
    fs::write(&object, &bytes).unwrap();
    assert_eq!(
        projection::verify_projection(&root, &summary.projection_sha256).unwrap_err(),
        "Retained object hash mismatch"
    );
    assert!(projection::project_catalog(&root, &catalog_sha256, "enUS").is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_held_writer_lock_refuses_a_second_projection() {
    let (root, catalog_sha256, _) = seeded_store("lock");
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("writer.lock"))
        .unwrap();
    lock.lock().unwrap();
    assert_eq!(
        projection::project_catalog(&root, &catalog_sha256, "enUS").unwrap_err(),
        "Another extraction owns the destination"
    );
    assert!(!root.join("projections").exists());
    lock.unlock().unwrap();
    assert!(projection::project_catalog(&root, &catalog_sha256, "enUS").is_ok());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn duplicate_identities_and_record_budget_refuse_projection() {
    let (catalog, objects) = catalog(&tables());
    let load = |entry: &TableInventory| -> Result<DecodedTable, &'static str> {
        Ok(serde_json::from_slice(&objects[entry.decoded_sha256.as_ref().unwrap()]).unwrap())
    };
    let duplicated = projection::project(&catalog, CATALOG, "enUS", |entry| {
        let mut table = load(entry)?;
        if let Some(first) = table.rows.first().cloned() {
            table.rows.push(first);
        }
        Ok(table)
    });
    assert_eq!(duplicated.unwrap_err(), "Duplicate decoded record identity");
    // The synthetic catalog has 21 decoded rows in projected sources.
    assert!(projection::project_within(&catalog, CATALOG, "enUS", 21, load).is_ok());
    assert_eq!(
        projection::project_within(&catalog, CATALOG, "enUS", 20, load).unwrap_err(),
        "Projection exceeds record capacity"
    );
    // A larger request never raises the production cap.
    assert!(projection::project_within(&catalog, CATALOG, "enUS", usize::MAX, load).is_ok());
}

#[test]
fn undecoded_encrypted_and_closure_gaps_keep_catalog_reasons() {
    let (mut catalog, objects) = catalog(&tables());
    let set = |catalog: &mut ResearchCatalog, table: &str, d, reason: &str| {
        let entry = catalog
            .tables
            .iter_mut()
            .find(|e| e.table == table)
            .unwrap();
        entry.disposition = d;
        entry.reason = reason.into();
    };
    set(
        &mut catalog,
        "Item",
        InventoryDisposition::UnavailableEncrypted,
        "unavailable_encrypted",
    );
    set(
        &mut catalog,
        "UiMap",
        InventoryDisposition::LayoutMismatch,
        "layout_mismatch",
    );
    set(
        &mut catalog,
        "TaxiPath",
        InventoryDisposition::RetainedUnsupported,
        "unsupported_sparse_storage",
    );
    catalog
        .tables
        .iter_mut()
        .find(|e| e.table == "TaxiNodes")
        .unwrap()
        .decoded_sha256 = None;
    let p = run(&catalog, &objects);
    for gap in [
        "equipment:Item:unavailable_encrypted:unavailable_encrypted",
        "geography:UiMap:layout_mismatch:layout_mismatch",
        "travel:TaxiPath:retained_unsupported:unsupported_sparse_storage",
        "travel:TaxiNodes:decoded_object_missing",
    ] {
        assert!(p.gaps.iter().any(|g| g == gap), "{gap}");
    }
    assert!(!p.equipment.sources_complete && !p.travel.sources_complete);
    // With Item a whole-table gap, Item-derived facts are missing, and no
    // per-row join counts are reported.
    assert!(
        p.equipment
            .items
            .iter()
            .all(|i| i.class_id == Fact::Missing)
    );
    assert!(
        !p.gaps
            .iter()
            .any(|g| g.starts_with("equipment:item_sparse_rows_without_item"))
    );
    assert!(p.travel.nodes.is_empty() && p.travel.paths.is_empty());
    assert!(
        p.gaps
            .iter()
            .any(|g| g == "travel:waypoints_for_unprojected_paths:5")
    );
}

#[test]
fn item_inventory_type_never_overrides_an_unknown_or_refused_item_sparse_value() {
    let mut decoded = tables();
    let sparse = decoded.get_mut("ItemSparse").unwrap();
    sparse.rows[0]
        .fields
        .insert("InventoryType".into(), Cell::Unknown);
    sparse.rows[1]
        .fields
        .insert("InventoryType".into(), Cell::Text("x".into()));
    sparse.rows[3].fields.remove("InventoryType");
    let (catalog, objects) = catalog(&decoded);
    let p = run(&catalog, &objects);
    let items: BTreeMap<_, _> = p.equipment.items.iter().map(|i| (i.item_id, i)).collect();
    assert_eq!(items[&10].inventory_type, Fact::Unknown);
    assert_eq!(items[&10].slots, Fact::Unknown);
    assert_eq!(items[&11].inventory_type, Fact::Unsupported);
    // Absent in ItemSparse and no Item row: still missing.
    assert_eq!(items[&13].inventory_type, Fact::Missing);
    // Item 14 exists only in Item, whose value fills the absent one.
    assert_eq!(items[&14].inventory_type, Fact::Known(0));
}

/// Every field the projector reads, with its WoWDBDefs column type and array
/// length (`None` = scalar). Kept in step with `src/projection.rs` by hand.
type FieldPin = (&'static str, char, Option<usize>);
const PROJECTED_FIELDS: &[(&str, &[FieldPin])] = &[
    (
        "ItemSparse",
        &[
            ("Display_lang", 's', None),
            ("OverallQualityID", 'i', None),
            ("ItemLevel", 'i', None),
            ("RequiredLevel", 'i', None),
            ("InventoryType", 'i', None),
            ("AllowableClass", 'i', None),
            ("AllowableRace", 'i', Some(2)),
            ("RequiredSkill", 'i', None),
            ("RequiredSkillRank", 'i', None),
            ("Bonding", 'i', None),
            ("StatModifier_bonusStat", 'i', Some(10)),
            ("StatPercentEditor", 'i', Some(10)),
        ],
    ),
    (
        "Item",
        &[
            ("ClassID", 'i', None),
            ("SubclassID", 'i', None),
            ("InventoryType", 'i', None),
        ],
    ),
    (
        "Map",
        &[
            ("MapName_lang", 's', None),
            ("Directory", 's', None),
            ("MapType", 'i', None),
            ("InstanceType", 'i', None),
            ("ExpansionID", 'i', None),
            ("ParentMapID", 'i', None),
            ("CosmeticParentMapID", 'i', None),
            ("AreaTableID", 'i', None),
            ("MaxPlayers", 'i', None),
        ],
    ),
    (
        "UiMap",
        &[
            ("Name_lang", 's', None),
            ("ParentUiMapID", 'i', None),
            ("Type", 'i', None),
            ("System", 'i', None),
            ("Flags", 'i', None),
        ],
    ),
    (
        "UiMapAssignment",
        &[
            ("UiMapID", 'i', None),
            ("MapID", 'i', None),
            ("AreaID", 'i', None),
            ("OrderIndex", 'i', None),
            ("WMODoodadPlacementID", 'i', None),
            ("WMOGroupID", 'i', None),
            ("UiMin", 'f', Some(2)),
            ("UiMax", 'f', Some(2)),
            ("Region", 'f', Some(6)),
        ],
    ),
    (
        "TaxiNodes",
        &[
            ("Name_lang", 's', None),
            ("ContinentID", 'i', None),
            ("Pos", 'f', Some(3)),
            ("MapOffset", 'f', Some(2)),
            ("FlightMapOffset", 'f', Some(2)),
            ("Flags", 'i', None),
            ("ConditionID", 'i', None),
            ("VisibilityConditionID", 'i', None),
            ("CharacterBitNumber", 'i', None),
            ("MountCreatureID", 'i', Some(2)),
        ],
    ),
    (
        "TaxiPath",
        &[
            ("FromTaxiNode", 'i', None),
            ("ToTaxiNode", 'i', None),
            ("Cost", 'i', None),
        ],
    ),
    (
        "TaxiPathNode",
        &[
            ("PathID", 'i', None),
            ("NodeIndex", 'i', None),
            ("ContinentID", 'i', None),
            ("Loc", 'f', Some(3)),
            ("Flags", 'i', None),
            ("Delay", 'i', None),
        ],
    ),
    (
        "ChrClasses",
        &[
            ("Name_lang", 's', None),
            ("Filename", 's', None),
            ("Flags", 'i', None),
            ("RolesMask", 'i', None),
            ("DisplayPower", 'i', None),
            ("PrimaryStatPriority", 'i', None),
            ("ArmorTypeMask", 'i', None),
            ("StartingLevel", 'i', None),
            ("DefaultSpec", 'i', None),
            ("SpellClassSet", 'i', None),
        ],
    ),
    (
        "ChrRaces",
        &[
            ("Name_lang", 's', None),
            ("ClientPrefix", 's', None),
            ("ClientFileString", 's', None),
            ("FactionID", 'i', None),
            ("Alliance", 'i', None),
            ("PlayableRaceBit", 'i', None),
            ("DefaultClassID", 'i', None),
            ("StartingLevel", 'i', None),
            ("Flags", 'i', None),
        ],
    ),
];

#[test]
fn pinned_layouts_hold_every_projected_field_for_every_admitted_build() {
    use azeroth_local_data::metadata::{self, ADMITTED_BUILDS};
    use wowdbdefs_rs::RawType;
    assert_eq!(ADMITTED_BUILDS, [70374, 70425, 70450]);
    assert_eq!(
        SOURCES.iter().map(|(_, t, _)| *t).collect::<Vec<_>>(),
        PROJECTED_FIELDS.iter().map(|(t, _)| *t).collect::<Vec<_>>()
    );
    for ((_, table, layouts), (_, fields)) in SOURCES.iter().zip(PROJECTED_FIELDS) {
        // One pinned layout per table: WoWDBDefs lists the same layout block for
        // 70374, 70425 and 70450, so 70450 needs no separate pin.
        assert_eq!(layouts.len(), 1, "{table}");
        let layout = u32::from_str_radix(layouts[0], 16).unwrap();
        let mut entries_by_build = Vec::new();
        for &build in ADMITTED_BUILDS {
            let definition = metadata::definition(table, build, layout)
                .unwrap_or_else(|e| panic!("{table} {build}: {e}"));
            for (field, kind, length) in *fields {
                let entry = definition
                    .layout
                    .entries
                    .iter()
                    .find(|e| e.name == *field)
                    .unwrap_or_else(|| panic!("{table}.{field} missing for {build}"));
                assert_eq!(entry.array_size, *length, "{table}.{field} {build}");
                let actual = match definition.file.columns[*field].ty {
                    RawType::Int => 'i',
                    RawType::Float => 'f',
                    RawType::String | RawType::LocString => 's',
                };
                assert_eq!(actual, *kind, "{table}.{field} {build}");
            }
            entries_by_build.push(definition.layout.entries);
        }
        // Identical entries for every admitted build: the same decoded meaning.
        assert!(entries_by_build.windows(2).all(|w| w[0] == w[1]), "{table}");
        // An unknown neighbouring build is refused, not mapped to this layout.
        assert_eq!(
            metadata::definition(table, 70451, layout).err(),
            Some("unsupported_build")
        );
    }
}
