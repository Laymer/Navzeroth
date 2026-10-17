//! Research-only gameplay projections from one verified private research catalog.
//!
//! A projection is a typed, bounded, deterministic view of decoded client tables
//! for one build and locale. It binds the source catalog digest and is itself
//! content-addressed, so later tasks can pin it. Nothing here activates content,
//! establishes playable availability or makes a record usable for planning.
//! Unknown, missing, unsupported and zero values stay distinct: a zero is
//! `Fact::Known(0)`, never a default.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PROJECTION_FORMAT_VERSION: u32 = 1;
pub const PROJECTION_PROFILE: &str = "navzeroth-local-projection-v1";
/// Total projected records across every domain, including taxi waypoints.
pub const MAX_PROJECTED_RECORDS: usize = 1_000_000;
pub const MAX_PROJECTION_GAPS: usize = 4096;

/// One projected value and why it might not be known.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub enum Fact<T> {
    Known(T),
    /// The decoder reported the cell unknown (for example a missing relationship).
    Unknown,
    /// The decoded row has no such field, or the joined source row is absent.
    Missing,
    /// The decoded value has a type, range or disagreement this projection refuses.
    Unsupported,
}

impl<T> Fact<T> {
    pub fn known(&self) -> Option<&T> {
        match self {
            Fact::Known(value) => Some(value),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionStatus {
    /// Not verified, not activated, not admitted to planning.
    ResearchOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceState {
    Projected,
    Gap,
}

/// How one source table of the catalog contributed to the projection.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceBinding {
    pub domain: String,
    pub table: String,
    pub state: SourceState,
    /// `projected`, or the catalog disposition/reason or projection refusal.
    pub reason: String,
    pub layout_hash: Option<String>,
    pub decoded_sha256: Option<String>,
    pub rows: Option<u32>,
}

/// Candidate equipment slots, authored from the inventory-type enumeration.
/// Candidates only: proficiency, dual wield and class rules are not applied.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentSlot {
    Head,
    Neck,
    Shoulder,
    Shirt,
    Chest,
    Waist,
    Legs,
    Feet,
    Wrist,
    Hands,
    Finger1,
    Finger2,
    Trinket1,
    Trinket2,
    Back,
    MainHand,
    OffHand,
    Ranged,
    Tabard,
    Bag,
    Ammo,
}

/// Authored mapping `navzeroth-inventory-type-v1`. `Some(vec![])` means the
/// type is known and not equippable (0); `None` means the value is unsupported.
pub fn inventory_type_slots(inventory_type: i64) -> Option<Vec<EquipmentSlot>> {
    use EquipmentSlot::*;
    Some(match inventory_type {
        0 => vec![],
        1 => vec![Head],
        2 => vec![Neck],
        3 => vec![Shoulder],
        4 => vec![Shirt],
        5 | 20 => vec![Chest],
        6 => vec![Waist],
        7 => vec![Legs],
        8 => vec![Feet],
        9 => vec![Wrist],
        10 => vec![Hands],
        11 => vec![Finger1, Finger2],
        12 => vec![Trinket1, Trinket2],
        13 => vec![MainHand, OffHand],
        14 | 22 | 23 => vec![OffHand],
        15 | 25 | 26 | 28 => vec![Ranged],
        16 => vec![Back],
        17 | 21 => vec![MainHand],
        18 | 27 => vec![Bag],
        19 => vec![Tabard],
        24 => vec![Ammo],
        _ => return None,
    })
}

/// Raw stat-budget slot. Amounts derive from item-budget tables not projected here.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StatAllocation {
    pub slot: u8,
    pub stat_type: Fact<i64>,
    pub allocation: Fact<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EquipmentItem {
    pub item_id: u32,
    /// `ItemSparse.Display_lang` in the projection locale.
    pub name: Fact<String>,
    pub quality: Fact<i64>,
    pub item_level: Fact<i64>,
    pub required_level: Fact<i64>,
    pub inventory_type: Fact<i64>,
    pub slots: Fact<Vec<EquipmentSlot>>,
    pub class_id: Fact<i64>,
    pub subclass_id: Fact<i64>,
    pub allowable_class_mask: Fact<i64>,
    pub allowable_race_mask: Fact<Vec<i64>>,
    pub required_skill: Fact<i64>,
    pub required_skill_rank: Fact<i64>,
    pub bonding: Fact<i64>,
    pub stat_allocations: Fact<Vec<StatAllocation>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MapRecord {
    pub map_id: u32,
    pub name: Fact<String>,
    pub directory: Fact<String>,
    pub map_type: Fact<i64>,
    pub instance_type: Fact<i64>,
    pub expansion_id: Fact<i64>,
    pub parent_map_id: Fact<i64>,
    pub cosmetic_parent_map_id: Fact<i64>,
    pub area_table_id: Fact<i64>,
    pub max_players: Fact<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UiMapRecord {
    pub ui_map_id: u32,
    pub name: Fact<String>,
    pub parent_ui_map_id: Fact<i64>,
    pub ui_map_type: Fact<i64>,
    pub system: Fact<i64>,
    pub flags: Fact<i64>,
}

/// One world-to-UI coordinate frame. Arrays keep the DBD ordering: `ui_min`/
/// `ui_max` are two normalized UI components, `region` is six world components
/// (minimum then maximum). Their gameplay interpretation is unverified.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UiMapAssignmentRecord {
    pub assignment_id: u32,
    pub ui_map_id: Fact<i64>,
    pub map_id: Fact<i64>,
    pub area_id: Fact<i64>,
    pub order_index: Fact<i64>,
    pub wmo_doodad_placement_id: Fact<i64>,
    pub wmo_group_id: Fact<i64>,
    pub ui_min: Fact<Vec<f64>>,
    pub ui_max: Fact<Vec<f64>>,
    pub region: Fact<Vec<f64>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TaxiNodeRecord {
    pub node_id: u32,
    pub name: Fact<String>,
    pub continent_id: Fact<i64>,
    pub position: Fact<Vec<f64>>,
    pub map_offset: Fact<Vec<f64>>,
    pub flight_map_offset: Fact<Vec<f64>>,
    pub flags: Fact<i64>,
    pub condition_id: Fact<i64>,
    pub visibility_condition_id: Fact<i64>,
    pub character_bit_number: Fact<i64>,
    pub mount_creature_ids: Fact<Vec<i64>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TaxiWaypoint {
    pub waypoint_id: u32,
    pub node_index: Fact<i64>,
    pub continent_id: Fact<i64>,
    pub position: Fact<Vec<f64>>,
    pub flags: Fact<i64>,
    pub delay: Fact<i64>,
}

/// A directed path. Unlocks, costs paid, waits and availability are not established.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TaxiPathRecord {
    pub path_id: u32,
    pub from_node: Fact<i64>,
    pub to_node: Fact<i64>,
    pub cost: Fact<i64>,
    /// Both endpoints are known and present among the projected nodes.
    pub endpoints_resolved: bool,
    /// Every waypoint has a known, unique node index. Waypoints are sorted by it.
    pub waypoints_ordered: bool,
    pub waypoints: Vec<TaxiWaypoint>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClassRecord {
    pub class_id: u32,
    pub name: Fact<String>,
    pub filename: Fact<String>,
    pub flags: Fact<i64>,
    pub roles_mask: Fact<i64>,
    pub display_power: Fact<i64>,
    pub primary_stat_priority: Fact<i64>,
    pub armor_type_mask: Fact<i64>,
    pub starting_level: Fact<i64>,
    pub default_spec: Fact<i64>,
    pub spell_class_set: Fact<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RaceRecord {
    pub race_id: u32,
    pub name: Fact<String>,
    pub client_prefix: Fact<String>,
    pub client_file_string: Fact<String>,
    pub faction_id: Fact<i64>,
    pub alliance: Fact<i64>,
    pub playable_race_bit: Fact<i64>,
    pub default_class_id: Fact<i64>,
    pub starting_level: Fact<i64>,
    pub flags: Fact<i64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EquipmentProjection {
    pub sources_complete: bool,
    pub items: Vec<EquipmentItem>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeographyProjection {
    pub sources_complete: bool,
    pub maps: Vec<MapRecord>,
    pub ui_maps: Vec<UiMapRecord>,
    pub ui_map_assignments: Vec<UiMapAssignmentRecord>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TravelProjection {
    pub sources_complete: bool,
    pub nodes: Vec<TaxiNodeRecord>,
    pub paths: Vec<TaxiPathRecord>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClassRoleProjection {
    pub sources_complete: bool,
    pub classes: Vec<ClassRecord>,
    pub races: Vec<RaceRecord>,
}

/// The sealed artifact. Its SHA-256 over the exact stored bytes is its identity.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LocalProjection {
    pub format_version: u32,
    pub profile: String,
    pub status: ProjectionStatus,
    pub activated: bool,
    pub verified_for_planning: bool,
    pub catalog_sha256: String,
    pub product: String,
    pub version: String,
    pub build: u32,
    pub locale: String,
    pub catalog_extractor_version: String,
    pub catalog_metadata_sha256: String,
    pub catalog_extractor_profile_sha256: Option<String>,
    pub projector_version: String,
    pub projector_profile_sha256: String,
    pub sources: Vec<SourceBinding>,
    pub equipment: EquipmentProjection,
    pub geography: GeographyProjection,
    pub travel: TravelProjection,
    pub class_roles: ClassRoleProjection,
    pub gaps: Vec<String>,
}

fn strictly_increasing<T>(records: &[T], id: impl Fn(&T) -> u32) -> bool {
    records.windows(2).all(|w| id(&w[0]) < id(&w[1]))
}

impl LocalProjection {
    pub fn record_count(&self) -> usize {
        self.equipment.items.len()
            + self.geography.maps.len()
            + self.geography.ui_maps.len()
            + self.geography.ui_map_assignments.len()
            + self.travel.nodes.len()
            + self.travel.paths.len()
            + self
                .travel
                .paths
                .iter()
                .map(|p| p.waypoints.len())
                .sum::<usize>()
            + self.class_roles.classes.len()
            + self.class_roles.races.len()
    }

    /// Structural invariants every sealed projection must satisfy. This does not
    /// verify the source catalog; callers bind and verify it separately.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.format_version != PROJECTION_FORMAT_VERSION || self.profile != PROJECTION_PROFILE {
            return Err("Unsupported projection version");
        }
        if self.activated || self.verified_for_planning {
            return Err("Projection claims activation or planning verification");
        }
        if !crate::local_data::is_sha256(&self.catalog_sha256)
            || !crate::local_data::is_sha256(&self.projector_profile_sha256)
        {
            return Err("Invalid projection binding");
        }
        if self.locale.is_empty()
            || self.locale.len() > 8
            || !self.locale.bytes().all(|b| b.is_ascii_alphabetic())
        {
            return Err("Invalid projection locale");
        }
        if self.record_count() > MAX_PROJECTED_RECORDS
            || self.gaps.len() > MAX_PROJECTION_GAPS
            || self.sources.len() > 64
        {
            return Err("Projection exceeds capacity");
        }
        if !self.gaps.windows(2).all(|w| w[0] < w[1]) {
            return Err("Projection gaps are not canonical");
        }
        let mut sources = BTreeSet::new();
        for source in &self.sources {
            if !sources.insert((&source.domain, &source.table)) {
                return Err("Duplicate projection source");
            }
            if (source.state == SourceState::Projected)
                != (source.reason == "projected" && source.decoded_sha256.is_some())
            {
                return Err("Inconsistent projection source state");
            }
        }
        let ordered = strictly_increasing(&self.equipment.items, |r| r.item_id)
            && strictly_increasing(&self.geography.maps, |r| r.map_id)
            && strictly_increasing(&self.geography.ui_maps, |r| r.ui_map_id)
            && strictly_increasing(&self.geography.ui_map_assignments, |r| r.assignment_id)
            && strictly_increasing(&self.travel.nodes, |r| r.node_id)
            && strictly_increasing(&self.travel.paths, |r| r.path_id)
            && strictly_increasing(&self.class_roles.classes, |r| r.class_id)
            && strictly_increasing(&self.class_roles.races, |r| r.race_id);
        if !ordered {
            return Err("Projection records are not canonical");
        }
        let mut waypoints = BTreeSet::new();
        for path in &self.travel.paths {
            if !path
                .waypoints
                .iter()
                .all(|w| waypoints.insert(w.waypoint_id))
            {
                return Err("Duplicate taxi waypoint");
            }
        }
        Ok(())
    }

    pub fn summary(&self, projection_sha256: &str) -> ProjectionSummary {
        let counts = BTreeMap::from([
            ("equipment_items".to_owned(), self.equipment.items.len()),
            ("geography_maps".to_owned(), self.geography.maps.len()),
            ("geography_ui_maps".to_owned(), self.geography.ui_maps.len()),
            (
                "geography_ui_map_assignments".to_owned(),
                self.geography.ui_map_assignments.len(),
            ),
            ("travel_nodes".to_owned(), self.travel.nodes.len()),
            ("travel_paths".to_owned(), self.travel.paths.len()),
            (
                "travel_waypoints".to_owned(),
                self.travel.paths.iter().map(|p| p.waypoints.len()).sum(),
            ),
            (
                "class_roles_classes".to_owned(),
                self.class_roles.classes.len(),
            ),
            ("class_roles_races".to_owned(), self.class_roles.races.len()),
        ]);
        ProjectionSummary {
            format_version: PROJECTION_FORMAT_VERSION,
            projection_sha256: projection_sha256.to_owned(),
            catalog_sha256: self.catalog_sha256.clone(),
            build: self.build,
            locale: self.locale.clone(),
            status: self.status,
            activated: false,
            verified_for_planning: false,
            domains_complete: BTreeMap::from([
                ("equipment".to_owned(), self.equipment.sources_complete),
                ("geography".to_owned(), self.geography.sources_complete),
                ("travel".to_owned(), self.travel.sources_complete),
                ("class_roles".to_owned(), self.class_roles.sources_complete),
            ]),
            counts,
            gaps: self.gaps.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProjectionSummary {
    pub format_version: u32,
    pub projection_sha256: String,
    pub catalog_sha256: String,
    pub build: u32,
    pub locale: String,
    pub status: ProjectionStatus,
    pub activated: bool,
    pub verified_for_planning: bool,
    /// Every source table for the domain projected; not target completeness.
    pub domains_complete: BTreeMap<String, bool>,
    pub counts: BTreeMap<String, usize>,
    pub gaps: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_types_keep_not_equippable_distinct_from_unsupported() {
        assert_eq!(inventory_type_slots(0), Some(vec![]));
        assert_eq!(inventory_type_slots(1), Some(vec![EquipmentSlot::Head]));
        assert_eq!(inventory_type_slots(29), None);
        assert_eq!(inventory_type_slots(-1), None);
    }

    #[test]
    fn facts_serialize_states_distinctly() {
        let zero = serde_json::to_string(&Fact::Known(0i64)).unwrap();
        let unknown = serde_json::to_string(&Fact::<i64>::Unknown).unwrap();
        let missing = serde_json::to_string(&Fact::<i64>::Missing).unwrap();
        assert_eq!(zero, r#"{"state":"known","value":0}"#);
        assert_eq!(unknown, r#"{"state":"unknown"}"#);
        assert_eq!(missing, r#"{"state":"missing"}"#);
    }
}
