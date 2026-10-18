//! Research-only gameplay projections from a verified private research catalog.
//!
//! The mapping is pure and deterministic over decoded tables; the I/O wrappers
//! read hash-verified decoded objects and seal the canonical projection bytes
//! under `projections/<sha256>`. No projection is verified or activated here.
use crate::{
    Result, bounded_json, checked_path,
    extraction::MAX_DECODED_BYTES,
    hash, read_file,
    tables::{Cell, DecodedTable, ResearchRow},
    verify_catalog,
};
use azeroth_core::{
    local_data::{InventoryDisposition, PerformancePreset},
    local_data_catalog::{ResearchCatalog, TableInventory},
    local_data_projection::{
        ClassRecord, ClassRoleProjection, EquipmentItem, EquipmentProjection, Fact,
        GeographyProjection, LocalProjection, MAX_PROJECTED_RECORDS, MapRecord,
        PROJECTION_FORMAT_VERSION, PROJECTION_PROFILE, ProjectionStatus, ProjectionSummary,
        RaceRecord, SourceBinding, SourceState, StatAllocation, TaxiNodeRecord, TaxiPathRecord,
        TaxiWaypoint, TravelProjection, UiMapAssignmentRecord, UiMapRecord, inventory_type_slots,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub const MAX_PROJECTION_BYTES: usize = MAX_DECODED_BYTES;
/// Hash of the projector code only: this module and the core projection module
/// (record types, validation, slot mapping and `PROJECTION_PROFILE`). Edits to
/// unrelated local-data code don't change it. Bump `PROJECTION_PROFILE` (and the
/// slot-mapping identifier) whenever the mapping's meaning changes.
pub const PROJECTOR_PROFILE_SHA256: &str = env!("NAVZEROTH_PROJECTOR_PROFILE_SHA256");

/// Source tables and the exact decoded layouts whose field semantics this
/// projection profile was written against. Other layouts are explicit gaps.
pub const SOURCES: &[(&str, &str, &[&str])] = &[
    ("equipment", "ItemSparse", &["6fcc3191"]),
    ("equipment", "Item", &["9a2a4834"]),
    ("geography", "Map", &["d43afac3"]),
    ("geography", "UiMap", &["db51d55f"]),
    ("geography", "UiMapAssignment", &["c9cc8dfb"]),
    ("travel", "TaxiNodes", &["e7b597f0"]),
    ("travel", "TaxiPath", &["a303de51"]),
    ("travel", "TaxiPathNode", &["fe362e70"]),
    ("class_roles", "ChrClasses", &["afc9b0c2"]),
    ("class_roles", "ChrRaces", &["4f44c796"]),
];

/// Limitations that hold for every projection produced by this profile.
const STANDING_GAPS: &[&str] = &[
    "class_roles:role_models_not_projected",
    "equipment:slot_candidates_authored_not_rule_checked",
    "equipment:actual_item_stat_amounts_unverified",
    "geography:coordinate_frame_interpretation_unverified",
    "hotfix_state_unknown",
    "playable_availability_unknown",
    "research_only_not_activated",
    "target_build_unconfirmed",
    "travel:unlocks_costs_waits_unverified",
];

fn int_cell(cell: &Cell) -> Fact<i64> {
    match cell {
        Cell::Unsigned(v) => i64::try_from(*v).map_or(Fact::Unsupported, Fact::Known),
        Cell::Signed(v) => Fact::Known(*v),
        Cell::Unknown => Fact::Unknown,
        _ => Fact::Unsupported,
    }
}
fn int(row: &ResearchRow, field: &str) -> Fact<i64> {
    row.fields.get(field).map_or(Fact::Missing, int_cell)
}
fn text(row: &ResearchRow, field: &str) -> Fact<String> {
    match row.fields.get(field) {
        None => Fact::Missing,
        Some(Cell::Text(value)) => Fact::Known(value.clone()),
        Some(Cell::Unknown) => Fact::Unknown,
        Some(_) => Fact::Unsupported,
    }
}
fn array<'a>(row: &'a ResearchRow, field: &str, length: usize) -> Fact<&'a [Cell]> {
    match row.fields.get(field) {
        None => Fact::Missing,
        Some(Cell::Array(cells)) if cells.len() == length => Fact::Known(cells),
        Some(Cell::Unknown) => Fact::Unknown,
        Some(_) => Fact::Unsupported,
    }
}
fn collect<T>(cells: Fact<&[Cell]>, convert: impl Fn(&Cell) -> Fact<T>) -> Fact<Vec<T>> {
    let cells = match cells {
        Fact::Known(cells) => cells,
        Fact::Unknown => return Fact::Unknown,
        Fact::Missing => return Fact::Missing,
        Fact::Unsupported => return Fact::Unsupported,
    };
    let mut values = Vec::with_capacity(cells.len());
    for cell in cells {
        match convert(cell) {
            Fact::Known(value) => values.push(value),
            // One unknown element makes the whole vector unknown, never zero-filled.
            Fact::Unknown => return Fact::Unknown,
            _ => return Fact::Unsupported,
        }
    }
    Fact::Known(values)
}
fn ints(row: &ResearchRow, field: &str, length: usize) -> Fact<Vec<i64>> {
    collect(array(row, field, length), int_cell)
}
fn floats(row: &ResearchRow, field: &str, length: usize) -> Fact<Vec<f64>> {
    collect(array(row, field, length), |cell| match cell {
        Cell::Float(v) if v.is_finite() => Fact::Known(*v),
        Cell::Unknown => Fact::Unknown,
        _ => Fact::Unsupported,
    })
}
fn identity(fact: &Fact<i64>) -> Option<u32> {
    fact.known().and_then(|v| u32::try_from(*v).ok())
}

struct Budget {
    used: usize,
    limit: usize,
}
impl Budget {
    fn take(&mut self, count: usize) -> Result<()> {
        self.used = self
            .used
            .checked_add(count)
            .filter(|n| *n <= self.limit)
            .ok_or("Projection exceeds record capacity")?;
        Ok(())
    }
}

struct Context<'a, F> {
    catalog: &'a ResearchCatalog,
    locale: &'a str,
    load: F,
    sources: Vec<SourceBinding>,
    gaps: BTreeSet<String>,
    /// Domains with a projected source that still reported decoder gaps.
    partial: BTreeSet<String>,
    budget: Budget,
}

impl<F: FnMut(&TableInventory) -> Result<DecodedTable>> Context<'_, F> {
    /// Returns the decoded rows, or records an explicit source gap.
    fn table(&mut self, domain: &str, table: &str) -> Result<Option<Vec<ResearchRow>>> {
        let layouts = SOURCES
            .iter()
            .find(|(d, t, _)| *d == domain && *t == table)
            .map(|(_, _, layouts)| *layouts)
            .or_else(|| {
                azeroth_core::local_data_gameplay::TABLES
                    .iter()
                    .find(|s| s.domain == domain && s.table == table)
                    .map(|s| s.layouts)
            })
            .ok_or("Projection source is not registered")?;
        let entry = self
            .catalog
            .tables
            .iter()
            .find(|e| e.table == table && e.locale.as_deref() == Some(self.locale));
        let mut binding = SourceBinding {
            domain: domain.into(),
            table: table.into(),
            state: SourceState::Gap,
            reason: "not_in_catalog".into(),
            layout_hash: None,
            decoded_sha256: None,
            rows: None,
        };
        let mut rows = None;
        if let Some(entry) = entry {
            binding.layout_hash = entry.layout_hash.clone();
            if entry.disposition != InventoryDisposition::Decoded {
                let disposition = serde_json::to_value(entry.disposition)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .ok_or("Catalog disposition cannot be encoded")?;
                binding.reason = format!("{disposition}:{}", entry.reason);
            } else if !entry
                .layout_hash
                .as_deref()
                .is_some_and(|l| layouts.contains(&l))
            {
                binding.reason = "projection_layout_unsupported".into();
            } else if entry.decoded_sha256.is_none() {
                binding.reason = "decoded_object_missing".into();
            } else {
                let mut decoded = (self.load)(entry)?;
                if decoded.format_version != 1 {
                    return Err("Unsupported decoded table version");
                }
                // Canonical record order never depends on stored row order.
                decoded.rows.sort_by_key(|r| r.id);
                if decoded.rows.windows(2).any(|w| w[0].id == w[1].id) {
                    return Err("Duplicate decoded record identity");
                }
                self.budget.take(decoded.rows.len())?;
                for gap in &decoded.gaps {
                    self.gaps.insert(format!("{domain}:{table}:{gap}"));
                    self.partial.insert(domain.into());
                }
                binding.state = SourceState::Projected;
                binding.reason = "projected".into();
                binding.decoded_sha256 = entry.decoded_sha256.clone();
                binding.rows =
                    Some(u32::try_from(decoded.rows.len()).map_err(|_| "Row count overflow")?);
                rows = Some(decoded.rows);
            }
        }
        if binding.state == SourceState::Gap {
            self.gaps
                .insert(format!("{domain}:{table}:{}", binding.reason));
        }
        self.sources.push(binding);
        Ok(rows)
    }
    fn complete(&self, domain: &str) -> bool {
        self.sources
            .iter()
            .filter(|s| s.domain == domain)
            .all(|s| s.state == SourceState::Projected)
            && !self.partial.contains(domain)
    }
    fn count(&mut self, gap: &str, count: usize) {
        if count > 0 {
            self.gaps.insert(format!("{gap}:{count}"));
        }
    }
}

fn missing_item(item_id: u32) -> EquipmentItem {
    EquipmentItem {
        item_id,
        name: Fact::Missing,
        quality: Fact::Missing,
        item_level: Fact::Missing,
        required_level: Fact::Missing,
        inventory_type: Fact::Missing,
        slots: Fact::Missing,
        class_id: Fact::Missing,
        subclass_id: Fact::Missing,
        allowable_class_mask: Fact::Missing,
        allowable_race_mask: Fact::Missing,
        required_skill: Fact::Missing,
        required_skill_rank: Fact::Missing,
        bonding: Fact::Missing,
        stat_allocations: Fact::Missing,
    }
}

fn stat_allocations(row: &ResearchRow) -> Fact<Vec<StatAllocation>> {
    let types = array(row, "StatModifier_bonusStat", 10);
    let amounts = array(row, "StatPercentEditor", 10);
    match (types, amounts) {
        (Fact::Known(types), Fact::Known(amounts)) => Fact::Known(
            types
                .iter()
                .zip(amounts)
                .enumerate()
                .map(|(slot, (t, a))| StatAllocation {
                    slot: slot as u8,
                    stat_type: int_cell(t),
                    allocation: int_cell(a),
                })
                .collect(),
        ),
        (Fact::Missing, _) | (_, Fact::Missing) => Fact::Missing,
        (Fact::Unknown, _) | (_, Fact::Unknown) => Fact::Unknown,
        _ => Fact::Unsupported,
    }
}

fn equipment<F: FnMut(&TableInventory) -> Result<DecodedTable>>(
    context: &mut Context<'_, F>,
    quota: &mut crate::gameplay::Quota,
) -> Result<(
    EquipmentProjection,
    azeroth_core::local_data_gameplay::MappedTable,
)> {
    let mut items = BTreeMap::new();
    let sparse = context.table("equipment", "ItemSparse")?;
    let sparse_projected = sparse.is_some();
    let stat_inputs = crate::gameplay::map_table(
        &azeroth_core::local_data_gameplay::ITEM_STAT_INPUT,
        sparse.as_deref(),
        quota,
    )?;
    for row in sparse.into_iter().flatten() {
        let mut item = missing_item(row.id);
        item.name = text(&row, "Display_lang");
        item.quality = int(&row, "OverallQualityID");
        item.item_level = int(&row, "ItemLevel");
        item.required_level = int(&row, "RequiredLevel");
        item.inventory_type = int(&row, "InventoryType");
        item.allowable_class_mask = int(&row, "AllowableClass");
        item.allowable_race_mask = ints(&row, "AllowableRace", 2);
        item.required_skill = int(&row, "RequiredSkill");
        item.required_skill_rank = int(&row, "RequiredSkillRank");
        item.bonding = int(&row, "Bonding");
        item.stat_allocations = stat_allocations(&row);
        items.insert(row.id, item);
    }
    let base = context.table("equipment", "Item")?;
    let base_projected = base.is_some();
    let (mut without_sparse, mut disagreements) = (0, 0);
    let mut seen = BTreeSet::new();
    for row in base.into_iter().flatten() {
        seen.insert(row.id);
        let item = items.entry(row.id).or_insert_with(|| {
            without_sparse += 1;
            missing_item(row.id)
        });
        item.class_id = int(&row, "ClassID");
        item.subclass_id = int(&row, "SubclassID");
        let other = int(&row, "InventoryType");
        item.inventory_type = match (&item.inventory_type, other) {
            (Fact::Known(a), Fact::Known(b)) if *a != b => {
                disagreements += 1;
                Fact::Unsupported
            }
            // ItemSparse is authoritative when present; Item only fills an
            // absent value and never overrides an unknown or refused one.
            (Fact::Missing, b) => b,
            (a, _) => a.clone(),
        };
    }
    let without_base = if base_projected && sparse_projected {
        items.keys().filter(|id| !seen.contains(id)).count()
    } else {
        0
    };
    if sparse_projected && base_projected {
        context.count("equipment:item_rows_without_item_sparse", without_sparse);
        context.count("equipment:item_sparse_rows_without_item", without_base);
    }
    context.count("equipment:inventory_type_disagreements", disagreements);
    let mut unsupported_slots = 0;
    for item in items.values_mut() {
        item.slots = match &item.inventory_type {
            Fact::Known(t) => inventory_type_slots(*t).map_or_else(
                || {
                    unsupported_slots += 1;
                    Fact::Unsupported
                },
                Fact::Known,
            ),
            Fact::Unknown => Fact::Unknown,
            Fact::Missing => Fact::Missing,
            Fact::Unsupported => Fact::Unsupported,
        };
    }
    context.count(
        "equipment:inventory_types_without_slot_mapping",
        unsupported_slots,
    );
    Ok((
        EquipmentProjection {
            sources_complete: context.complete("equipment"),
            items: items.into_values().collect(),
        },
        stat_inputs,
    ))
}

fn geography<F: FnMut(&TableInventory) -> Result<DecodedTable>>(
    context: &mut Context<'_, F>,
) -> Result<GeographyProjection> {
    let maps = context
        .table("geography", "Map")?
        .unwrap_or_default()
        .iter()
        .map(|row| MapRecord {
            map_id: row.id,
            name: text(row, "MapName_lang"),
            directory: text(row, "Directory"),
            map_type: int(row, "MapType"),
            instance_type: int(row, "InstanceType"),
            expansion_id: int(row, "ExpansionID"),
            parent_map_id: int(row, "ParentMapID"),
            cosmetic_parent_map_id: int(row, "CosmeticParentMapID"),
            area_table_id: int(row, "AreaTableID"),
            max_players: int(row, "MaxPlayers"),
        })
        .collect();
    let ui_maps = context
        .table("geography", "UiMap")?
        .unwrap_or_default()
        .iter()
        .map(|row| UiMapRecord {
            ui_map_id: row.id,
            name: text(row, "Name_lang"),
            parent_ui_map_id: int(row, "ParentUiMapID"),
            ui_map_type: int(row, "Type"),
            system: int(row, "System"),
            flags: int(row, "Flags"),
        })
        .collect();
    let ui_map_assignments = context
        .table("geography", "UiMapAssignment")?
        .unwrap_or_default()
        .iter()
        .map(|row| UiMapAssignmentRecord {
            assignment_id: row.id,
            ui_map_id: int(row, "UiMapID"),
            map_id: int(row, "MapID"),
            area_id: int(row, "AreaID"),
            order_index: int(row, "OrderIndex"),
            wmo_doodad_placement_id: int(row, "WMODoodadPlacementID"),
            wmo_group_id: int(row, "WMOGroupID"),
            ui_min: floats(row, "UiMin", 2),
            ui_max: floats(row, "UiMax", 2),
            region: floats(row, "Region", 6),
        })
        .collect();
    Ok(GeographyProjection {
        sources_complete: context.complete("geography"),
        maps,
        ui_maps,
        ui_map_assignments,
    })
}

fn travel<F: FnMut(&TableInventory) -> Result<DecodedTable>>(
    context: &mut Context<'_, F>,
) -> Result<TravelProjection> {
    let nodes: Vec<_> = context
        .table("travel", "TaxiNodes")?
        .unwrap_or_default()
        .iter()
        .map(|row| TaxiNodeRecord {
            node_id: row.id,
            name: text(row, "Name_lang"),
            continent_id: int(row, "ContinentID"),
            position: floats(row, "Pos", 3),
            map_offset: floats(row, "MapOffset", 2),
            flight_map_offset: floats(row, "FlightMapOffset", 2),
            flags: int(row, "Flags"),
            condition_id: int(row, "ConditionID"),
            visibility_condition_id: int(row, "VisibilityConditionID"),
            character_bit_number: int(row, "CharacterBitNumber"),
            mount_creature_ids: ints(row, "MountCreatureID", 2),
        })
        .collect();
    let node_ids: BTreeSet<u32> = nodes.iter().map(|n| n.node_id).collect();
    let mut paths: BTreeMap<u32, TaxiPathRecord> = context
        .table("travel", "TaxiPath")?
        .unwrap_or_default()
        .iter()
        .map(|row| {
            let from_node = int(row, "FromTaxiNode");
            let to_node = int(row, "ToTaxiNode");
            let endpoints_resolved = [&from_node, &to_node]
                .iter()
                .all(|f| identity(f).is_some_and(|id| node_ids.contains(&id)));
            (
                row.id,
                TaxiPathRecord {
                    path_id: row.id,
                    from_node,
                    to_node,
                    cost: int(row, "Cost"),
                    endpoints_resolved,
                    waypoints_ordered: true,
                    waypoints: Vec::new(),
                },
            )
        })
        .collect();
    let (mut unknown_path, mut unprojected_path) = (0, 0);
    for row in context.table("travel", "TaxiPathNode")?.unwrap_or_default() {
        let path = int(&row, "PathID");
        let Some(path) = identity(&path) else {
            unknown_path += 1;
            continue;
        };
        let Some(path) = paths.get_mut(&path) else {
            unprojected_path += 1;
            continue;
        };
        path.waypoints.push(TaxiWaypoint {
            waypoint_id: row.id,
            node_index: int(&row, "NodeIndex"),
            continent_id: int(&row, "ContinentID"),
            position: floats(&row, "Loc", 3),
            flags: int(&row, "Flags"),
            delay: int(&row, "Delay"),
        });
    }
    let (mut unresolved, mut unordered) = (0, 0);
    for path in paths.values_mut() {
        path.waypoints.sort_by_key(|w| match w.node_index {
            Fact::Known(index) => (0, index, w.waypoint_id),
            _ => (1, 0, w.waypoint_id),
        });
        let mut indices = BTreeSet::new();
        path.waypoints_ordered = path
            .waypoints
            .iter()
            .all(|w| w.node_index.known().is_some_and(|i| indices.insert(*i)));
        unordered += usize::from(!path.waypoints_ordered);
        unresolved += usize::from(!path.endpoints_resolved);
    }
    context.count("travel:waypoints_without_known_path", unknown_path);
    context.count("travel:waypoints_for_unprojected_paths", unprojected_path);
    context.count("travel:paths_with_unresolved_endpoints", unresolved);
    context.count("travel:paths_with_unordered_waypoints", unordered);
    Ok(TravelProjection {
        sources_complete: context.complete("travel"),
        nodes,
        paths: paths.into_values().collect(),
    })
}

fn class_roles<F: FnMut(&TableInventory) -> Result<DecodedTable>>(
    context: &mut Context<'_, F>,
) -> Result<ClassRoleProjection> {
    let classes = context
        .table("class_roles", "ChrClasses")?
        .unwrap_or_default()
        .iter()
        .map(|row| ClassRecord {
            class_id: row.id,
            name: text(row, "Name_lang"),
            filename: text(row, "Filename"),
            flags: int(row, "Flags"),
            roles_mask: int(row, "RolesMask"),
            display_power: int(row, "DisplayPower"),
            primary_stat_priority: int(row, "PrimaryStatPriority"),
            armor_type_mask: int(row, "ArmorTypeMask"),
            starting_level: int(row, "StartingLevel"),
            default_spec: int(row, "DefaultSpec"),
            spell_class_set: int(row, "SpellClassSet"),
        })
        .collect();
    let races = context
        .table("class_roles", "ChrRaces")?
        .unwrap_or_default()
        .iter()
        .map(|row| RaceRecord {
            race_id: row.id,
            name: text(row, "Name_lang"),
            client_prefix: text(row, "ClientPrefix"),
            client_file_string: text(row, "ClientFileString"),
            faction_id: int(row, "FactionID"),
            alliance: int(row, "Alliance"),
            playable_race_bit: int(row, "PlayableRaceBit"),
            default_class_id: int(row, "DefaultClassID"),
            starting_level: int(row, "StartingLevel"),
            flags: int(row, "Flags"),
        })
        .collect();
    Ok(ClassRoleProjection {
        sources_complete: context.complete("class_roles"),
        classes,
        races,
    })
}

/// Pure projection of one verified catalog for one declared locale. `load` must
/// return the hash-verified decoded object named by the inventory entry.
pub fn project(
    catalog: &ResearchCatalog,
    catalog_sha256: &str,
    locale: &str,
    load: impl FnMut(&TableInventory) -> Result<DecodedTable>,
) -> Result<LocalProjection> {
    project_within(catalog, catalog_sha256, locale, MAX_PROJECTED_RECORDS, load)
}

/// `project` with a smaller decoded-row budget, for exercising the capacity
/// refusal. The budget can only be lowered below `MAX_PROJECTED_RECORDS`.
#[doc(hidden)]
pub fn project_within(
    catalog: &ResearchCatalog,
    catalog_sha256: &str,
    locale: &str,
    max_records: usize,
    load: impl FnMut(&TableInventory) -> Result<DecodedTable>,
) -> Result<LocalProjection> {
    if catalog.installation.product != "wow_classic_beta"
        || !crate::metadata::ADMITTED_BUILDS.contains(&catalog.installation.build)
        || catalog.installation.version != format!("1.60.1.{}", catalog.installation.build)
    {
        return Err("Unsupported projection development tuple");
    }
    if !azeroth_core::local_data::is_sha256(catalog_sha256) {
        return Err("Invalid catalog identity");
    }
    if !catalog
        .installation
        .declared_locales
        .iter()
        .any(|l| l == locale)
    {
        return Err("Locale is not declared by the catalog");
    }
    let mut context = Context {
        catalog,
        locale,
        load,
        sources: Vec::new(),
        gaps: STANDING_GAPS.iter().map(|g| (*g).to_owned()).collect(),
        partial: BTreeSet::new(),
        budget: Budget {
            used: 0,
            limit: max_records.min(MAX_PROJECTED_RECORDS),
        },
    };
    if catalog.gaps.iter().any(|g| g == "explicit_table_subset") {
        context.gaps.insert("explicit_table_subset".into());
    }
    let mut quota = crate::gameplay::Quota::default();
    let (mut equipment, stat_inputs) = equipment(&mut context, &mut quota)?;
    let mut geography = geography(&mut context)?;
    let travel = travel(&mut context)?;
    let mut class_roles = class_roles(&mut context)?;
    let mut expanded = azeroth_core::local_data_gameplay::ExpandedProjection {
        format_version: 1,
        references: vec![],
        blocked_talent_ids: vec![],
        tables: vec![stat_inputs],
        item_stats: vec![],
    };
    for spec in azeroth_core::local_data_gameplay::TABLES {
        let rows = context.table(spec.domain, spec.table)?;
        expanded.tables.push(crate::gameplay::map_table(
            spec,
            rows.as_deref(),
            &mut quota,
        )?);
    }
    // Expanded sources participate in the same domain completeness contract.
    equipment.sources_complete = context.complete("equipment");
    geography.sources_complete = context.complete("geography");
    class_roles.sources_complete = context.complete("class_roles");
    expanded.item_stats = crate::gameplay::item_stats(&equipment, &expanded);
    expanded.references = crate::gameplay::references(&expanded, &geography, &context.sources);
    expanded.blocked_talent_ids = crate::gameplay::blocked_talents(&expanded);
    if !expanded.blocked_talent_ids.is_empty() {
        context
            .gaps
            .insert("class_roles:known_talent_cycle_or_dependency".into());
    }
    for gap in [
        "equipment:budget_model_applicability_unverified",
        "equipment:socket_cost_bonus_scaling_unverified",
        "class_roles:learned_spells_and_talent_choices_unknown",
        "geography:poi_coordinates_uncalibrated",
        "quests:client_id_does_not_establish_server_availability",
        "quests:line_order_is_not_a_prerequisite",
    ] {
        context.gaps.insert(gap.into());
    }
    let projection = LocalProjection {
        format_version: PROJECTION_FORMAT_VERSION,
        profile: PROJECTION_PROFILE.into(),
        status: ProjectionStatus::ResearchOnly,
        activated: false,
        verified_for_planning: false,
        catalog_sha256: catalog_sha256.into(),
        product: catalog.installation.product.clone(),
        version: catalog.installation.version.clone(),
        build: catalog.installation.build,
        locale: locale.into(),
        catalog_extractor_version: catalog.extractor_version.clone(),
        catalog_metadata_sha256: catalog.metadata_sha256.clone(),
        catalog_extractor_profile_sha256: catalog.extractor_profile_sha256.clone(),
        projector_version: env!("CARGO_PKG_VERSION").into(),
        projector_profile_sha256: PROJECTOR_PROFILE_SHA256.into(),
        sources: context.sources,
        equipment,
        geography,
        travel,
        class_roles,
        gaps: context.gaps.into_iter().collect(),
        expanded: Some(expanded),
    };
    projection.validate()?;
    Ok(projection)
}

fn load_decoded(root: &Path, entry: &TableInventory) -> Result<DecodedTable> {
    let digest = entry
        .decoded_sha256
        .as_deref()
        .filter(|d| azeroth_core::local_data::is_sha256(d))
        .ok_or("Invalid object identity")?;
    let (bytes, _) = read_file(&root.join("objects").join(digest), MAX_DECODED_BYTES)?;
    if hash(&bytes) != digest {
        return Err("Decoded object hash mismatch");
    }
    // Content-addressed and verified above; typed decoding refuses unknown fields.
    serde_json::from_slice(&bytes).map_err(|_| "Invalid decoded table object")
}

fn encode(projection: &LocalProjection) -> Result<Vec<u8>> {
    let bytes = bounded_json(projection, MAX_PROJECTION_BYTES)?;
    if bytes.len() > MAX_PROJECTION_BYTES {
        return Err("Projection encoding exceeds capacity");
    }
    Ok(bytes)
}

/// Project a verified catalog and seal it as `projections/<sha256>`. The store's
/// writer lock is held throughout; nothing is activated or replaced.
pub fn project_catalog(
    destination: &Path,
    catalog_sha256: &str,
    locale: &str,
) -> Result<ProjectionSummary> {
    if !azeroth_core::local_data::is_sha256(catalog_sha256) {
        return Err("Invalid catalog identity");
    }
    // Never initialise a store as a side effect of projecting.
    let existing = checked_path(destination)?;
    if !existing.join("catalogs").join(catalog_sha256).is_file() {
        return Err("Catalog is not present in the private store");
    }
    let mut store = crate::extraction::Store::open(
        destination,
        PerformancePreset::Max.limits().total_storage_bytes,
    )?;
    let catalog = verify_catalog(destination, catalog_sha256)?;
    let root = checked_path(destination)?;
    let projection = project(&catalog, catalog_sha256, locale, |entry| {
        load_decoded(&root, entry)
    })?;
    let digest = store.put("projections", &encode(&projection)?)?;
    Ok(projection.summary(&digest))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reproduction {
    /// The current projector re-derived byte-identical output from the catalog.
    Reproduced,
    /// A different projector profile sealed it; integrity was checked but the
    /// records were not re-derived, so `verified` is false.
    SkippedProfileMismatch,
}

#[derive(Debug, serde::Serialize)]
pub struct ProjectionVerification {
    pub format_version: u32,
    /// True only when the projection was re-derived (`reproduced`).
    pub verified: bool,
    /// Hash, canonical encoding, invariants, catalog closure and source bindings.
    pub integrity_verified: bool,
    pub reproduction: Reproduction,
    pub projector_profile_sha256: &'static str,
    pub projection: ProjectionSummary,
}

/// Verify a sealed projection, its source catalog and every bound decoded
/// object. When the projector profile matches, re-project and compare bytes.
pub fn verify_projection(destination: &Path, digest: &str) -> Result<ProjectionVerification> {
    if !azeroth_core::local_data::is_sha256(digest) {
        return Err("Invalid projection identity");
    }
    let root = checked_path(destination)?;
    let (bytes, _) = read_file(&root.join("projections").join(digest), MAX_PROJECTION_BYTES)?;
    if hash(&bytes) != digest {
        return Err("Projection hash mismatch");
    }
    let projection: LocalProjection =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid projection")?;
    projection.validate()?;
    if encode(&projection)? != bytes {
        return Err("Projection encoding is not canonical");
    }
    let catalog = verify_catalog(&root, &projection.catalog_sha256)?;
    if catalog.installation.build != projection.build
        || catalog.installation.product != projection.product
        || catalog.installation.version != projection.version
        || catalog.metadata_sha256 != projection.catalog_metadata_sha256
    {
        return Err("Projection is not bound to its catalog");
    }
    for source in &projection.sources {
        if source.state == SourceState::Projected {
            let bound = catalog.tables.iter().any(|e| {
                e.table == source.table
                    && e.locale.as_deref() == Some(projection.locale.as_str())
                    && e.decoded_sha256 == source.decoded_sha256
            });
            if !bound {
                return Err("Projection source is not bound to its catalog");
            }
        }
    }
    let reproduction = if projection.projector_profile_sha256 == PROJECTOR_PROFILE_SHA256 {
        let again = project(
            &catalog,
            &projection.catalog_sha256,
            &projection.locale,
            |entry| load_decoded(&root, entry),
        )?;
        if encode(&again)? != bytes {
            return Err("Projection is not reproducible from its catalog");
        }
        Reproduction::Reproduced
    } else {
        Reproduction::SkippedProfileMismatch
    };
    Ok(ProjectionVerification {
        format_version: 1,
        verified: reproduction == Reproduction::Reproduced,
        integrity_verified: true,
        reproduction,
        projector_profile_sha256: PROJECTOR_PROFILE_SHA256,
        projection: projection.summary(digest),
    })
}
