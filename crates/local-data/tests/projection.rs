//! Synthetic catalogs only: no client tables, private paths or real records.
use azeroth_core::{
    local_data::{CASCLIB_REVISION, InstallationMetadata, InventoryDisposition},
    local_data_catalog::{DistributionClass, ResearchCatalog, TableInventory},
    local_data_projection::{EquipmentSlot, Fact, LocalProjection, SourceState},
};
use azeroth_local_data::{
    metadata::METADATA_SHA256,
    projection::{self, SOURCES},
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

#[test]
fn sealed_projection_verifies_reproduces_and_detects_tampering() {
    let root = private_store("seal");
    let (catalog, objects) = catalog(&tables());
    fs::create_dir(root.join("objects")).unwrap();
    fs::create_dir(root.join("catalogs")).unwrap();
    for (digest, bytes) in &objects {
        fs::write(root.join("objects").join(digest), bytes).unwrap();
    }
    let manifest = serde_json::to_vec(&catalog).unwrap();
    let catalog_sha256 = hash(&manifest);
    fs::write(root.join("catalogs").join(&catalog_sha256), &manifest).unwrap();

    let summary = projection::project_catalog(&root, &catalog_sha256, "enUS").unwrap();
    assert_eq!(summary.catalog_sha256, catalog_sha256);
    assert!(!summary.activated && !summary.verified_for_planning);
    assert_eq!(summary.counts["equipment_items"], 5);
    assert_eq!(summary.counts["travel_waypoints"], 4);
    assert!(!summary.domains_complete["class_roles"]);
    let again = projection::project_catalog(&root, &catalog_sha256, "enUS").unwrap();
    assert_eq!(again.projection_sha256, summary.projection_sha256);

    let verified = projection::verify_projection(&root, &summary.projection_sha256).unwrap();
    assert!(verified.verified && verified.reproduced);
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
