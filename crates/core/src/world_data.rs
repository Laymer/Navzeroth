//! Generated world reference data (`navzeroth-world-data` v1): professions, fishing,
//! NPCs, spawns, quests, spells, travel, loot and dungeons produced by the Python
//! generators under `data-generators/`. Every shard names one origin edition and
//! build; records never claim another edition. An absent optional field means the
//! source did not provide it (unknown), never zero; fields where the distinction
//! between unknown and unsupported matters use [`Observation`].
use crate::{
    entities::Confidence,
    error::{Result, invalid},
    observation::{Observation, ObservationStatus},
    types::{BuildId, EditionId, Locale, SourceId},
};
use schemars::{JsonSchema, generate::SchemaSettings};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const FORMAT: &str = "navzeroth-world-data";
pub const SCHEMA_VERSION: u32 = 1;
/// Bytes per shard file; the CLI enforces it before parsing.
pub const MAX_SHARD_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_RECORDS: usize = 250_000;
pub const MAX_SHARDS: u32 = 4096;
pub const MAX_TEXT: usize = 8192;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorldData {
    pub format: String,
    pub schema_version: u32,
    /// Dotted dataset name, for example `professions.recipes` or `world.spawns`.
    pub dataset: String,
    pub record_kind: RecordKind,
    pub origin: Origin,
    pub target: Target,
    pub confidence: Confidence,
    pub sources: Vec<SourceRef>,
    pub generator: Generator,
    pub shard: Shard,
    pub records: Vec<Record>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    /// `wow_forever`, `wow_classic_era` or `vanilla_1_12`; never relabelled.
    pub edition: EditionId,
    pub build: Observation<BuildId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    pub locale: Locale,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub edition: EditionId,
    /// Unknown until the 1.0.0 target build is frozen (ADR 0006 §1).
    pub build: Observation<BuildId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceRef {
    pub id: SourceId,
    pub url: String,
    /// Upstream revision (commit, dump name or build); absent for live pages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieved_at_utc_ms: Option<u64>,
    /// SHA-256 of the retained snapshot or of the cache manifest that lists them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_sha256: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Generator {
    pub name: String,
    pub version: String,
    /// Required attribution, e.g. the TALOD credit.
    pub credit: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Shard {
    pub index: u32,
    pub count: u32,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RecordKind {
    Item,
    Recipe,
    ProfessionRank,
    ProfessionNote,
    Enhancement,
    FishingArea,
    FishingGoal,
    TimedFish,
    FishingEvent,
    Area,
    UiMap,
    UiMapAssignment,
    Npc,
    GameObject,
    Spawn,
    Quest,
    VendorItem,
    TrainerSpell,
    Spell,
    Talent,
    TaxiNode,
    TaxiPath,
    Loot,
    Dungeon,
    Graveyard,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Record {
    Item(Item),
    Recipe(Recipe),
    ProfessionRank(ProfessionRank),
    ProfessionNote(ProfessionNote),
    Enhancement(Enhancement),
    FishingArea(FishingArea),
    FishingGoal(FishingGoal),
    TimedFish(TimedFish),
    FishingEvent(FishingEvent),
    Area(Area),
    UiMap(UiMap),
    UiMapAssignment(UiMapAssignment),
    Npc(Npc),
    GameObject(GameObject),
    Spawn(Spawn),
    Quest(Quest),
    VendorItem(VendorItem),
    TrainerSpell(TrainerSpell),
    Spell(Spell),
    Talent(Talent),
    TaxiNode(TaxiNode),
    TaxiPath(TaxiPath),
    Loot(Loot),
    Dungeon(Dungeon),
    Graveyard(Graveyard),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ItemQty {
    pub item: u32,
    pub count: u32,
}
/// A point in a map's world coordinates (yards), as server data stores it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorldPoint {
    pub map_id: u32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
/// A uiMap position, normalized 0..1 from the top-left like `types::Position`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MapPosition {
    pub ui_map_id: u32,
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Npc,
    GameObject,
    Item,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EntityRef {
    pub entity: EntityKind,
    pub id: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subclass_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_type: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_level: Option<u32>,
    /// Class and race bit masks; -1 means every class or race.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowable_class: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowable_race: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bonding: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_stack: Option<u32>,
    /// Stat name (source vocabulary, e.g. `stamina`, `armor`) to amount.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub stats: BTreeMap<String, i64>,
    /// Copper. `vendor_sold` says whether a vendor sells it; a price alone does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buy_price: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sell_price: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auction_average: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor_sold: Option<bool>,
    /// How a raw material is obtained, in words (e.g. `Mining`, `Herbalism`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obtain_hint: Option<String>,
    /// Recipe spells that create this item.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub made_by: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub spell_id: u32,
    pub name: String,
    pub profession: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_skill: Option<u32>,
    /// Orange, yellow, green and grey thresholds; 0 means the colour is absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colors: Option<[u32; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_up: Option<u32>,
    #[serde(default)]
    pub reagents: Vec<ItemQty>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creates: Option<u32>,
    /// Average items made per craft (at least 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub makes: Option<f64>,
    /// `trainer`, `pattern`, `drop`, `quest`, `vendor`, `starter`, ...
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trainer_cost: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern_item: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern_price: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specialization: Option<String>,
    #[serde(default)]
    pub cooldown: bool,
    /// The learning source was inferred by a rule, not listed by the source.
    #[serde(default)]
    pub source_inferred: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfessionRank {
    pub profession: String,
    pub rank: String,
    pub max_skill: u32,
    pub learn_at_skill: u32,
    pub cost: Observation<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfessionNote {
    pub profession: String,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnhancementType {
    Enchant,
    Item,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Enhancement {
    pub enhancement_type: EnhancementType,
    pub spell_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<u32>,
    pub name: String,
    /// Inventory slot IDs (1 head .. 19 tabard).
    pub slots: Vec<u8>,
    /// Narrows weapon slots: `shield`, `twohand`, `weapon`, `blunt`, `bladed`, `bowgun`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weapon_kind: Option<String>,
    pub effect: String,
    #[serde(default)]
    pub temporary: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_item_level: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FishingArea {
    pub area_id: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_name: Option<String>,
    pub base_skill: i32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FishingQuest {
    pub id: u32,
    pub title: String,
    pub count: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rewards: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FishingGoal {
    pub item_id: u32,
    /// `pagle`, `stv` or `rare`.
    pub group: String,
    pub name: String,
    pub quality: u8,
    /// Each place is `[area]` or `[area, zone]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub places: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quests: Vec<FishingQuest>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TimedFish {
    pub item_id: u32,
    /// Server hours `[from, to)`, wrapping past midnight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hours: Option<[u32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_hours: Option<[u32; 2]>,
    /// `[from, to]` as month * 100 + day, wrapping past New Year.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season: Option<[u32; 2]>,
    pub label: String,
    pub open: String,
    pub closed: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub source: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FishingEvent {
    pub name: String,
    /// 1 = Sunday.
    pub weekday: u32,
    pub start_minute: u32,
    pub length_minutes: u32,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Area {
    pub id: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exploration_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction_group_mask: Option<u32>,
    /// Lowest and highest creature level found in the area by the generator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level_range: Option<[u32; 2]>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UiMap {
    pub id: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_ui_map_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_type: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UiMapAssignment {
    pub ui_map_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_id: Option<u32>,
    pub map_id: u32,
    pub order_index: u32,
    pub ui_min: [f64; 2],
    pub ui_max: [f64; 2],
    pub region_min: [f64; 3],
    pub region_max: [f64; 3],
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum NpcRole {
    QuestGiver,
    Vendor,
    Repair,
    Trainer,
    ClassTrainer,
    ProfessionTrainer,
    Innkeeper,
    FlightMaster,
    Banker,
    Auctioneer,
    SpiritHealer,
    StableMaster,
    Battlemaster,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Npc {
    pub id: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction_template: Option<u32>,
    /// `alliance`, `horde`, `both` or `hostile`, from the faction template.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reaction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npc_flags: Option<u64>,
    /// 0 normal, 1 elite, 2 rare elite, 3 boss, 4 rare.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creature_type: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<NpcRole>,
    /// Class trained, as a class ID, for class trainers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trainer_class: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GameObject {
    pub id: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_type: Option<u32>,
    /// Lock or skill requirement for gathering nodes and chests, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_skill: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Spawn {
    pub guid: u32,
    pub entity: EntityKind,
    pub entity_id: u32,
    pub world: WorldPoint,
    /// Unsupported when the map has no uiMap (instances without a region).
    pub position: Observation<MapPosition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub respawn_secs: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveKind {
    Kill,
    Item,
    Object,
    Spell,
    Explore,
    Event,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QuestObjective {
    pub objective: ObjectiveKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_id: Option<u32>,
    pub count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Quest {
    pub id: u32,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_level: Option<u32>,
    /// Positive: area ID; negative: quest sort (class, profession, holiday).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone_or_sort: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_races: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_classes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_skill: Option<[u32; 2]>,
    /// Positive: must be completed; negative: must be active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_quest: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_quest: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_in_chain: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclusive_group: Option<i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub givers: Vec<EntityRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enders: Vec<EntityRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objectives_text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objectives: Vec<QuestObjective>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reward_choice_items: Vec<ItemQty>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reward_items: Vec<ItemQty>,
    /// Copper; negative means the quest costs money.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reward_money: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reward_xp: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reward_spell: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_item: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VendorItem {
    pub npc_id: u32,
    pub item_id: u32,
    /// Limited stock; absent means unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restock_secs: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrainerSpell {
    pub npc_id: u32,
    pub spell_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_skill_line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_skill: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_level: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Spell {
    pub id: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_mask: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub race_mask: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spell_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_level: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Talent {
    pub id: u32,
    pub tab_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_mask: Option<i64>,
    pub tier: u32,
    pub column: u32,
    /// Spell ID per rank, in rank order.
    pub ranks: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prerequisite: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaxiNode {
    pub id: u32,
    pub name: String,
    pub world: WorldPoint,
    pub position: Observation<MapPosition>,
    /// Faction usable by: `alliance`, `horde`, `both` or `unknown`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flight_master: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaxiPath {
    pub id: u32,
    pub from_node: u32,
    pub to_node: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub points: Vec<WorldPoint>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LootTable {
    Creature,
    GameObject,
    Item,
    Reference,
    Fishing,
    Pickpocketing,
    Skinning,
    Disenchant,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Loot {
    pub table: LootTable,
    pub entry: u32,
    /// Exactly one of `item` or `reference` is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<u32>,
    /// Percent; negative marks a quest-only drop (CMaNGOS); 0 inside a group means
    /// an equal share of the group's remainder.
    pub chance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<u32>,
    pub min_count: u32,
    pub max_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<u32>,
}
/// A planar entrance estimate; elevation is intentionally absent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlanarEntranceEstimate {
    pub map_id: u32,
    pub x: f64,
    pub y: f64,
    pub derivation: String,
    pub uncertainty: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Dungeon {
    pub map_id: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_level: Option<u32>,
    pub entrance: Observation<WorldPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entrance_estimate: Option<PlanarEntranceEstimate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excluded_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bosses: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_id: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Graveyard {
    pub id: u32,
    pub world: WorldPoint,
    pub position: Observation<MapPosition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<u32>,
}

impl Record {
    pub fn kind(&self) -> RecordKind {
        match self {
            Self::Item(_) => RecordKind::Item,
            Self::Recipe(_) => RecordKind::Recipe,
            Self::ProfessionRank(_) => RecordKind::ProfessionRank,
            Self::ProfessionNote(_) => RecordKind::ProfessionNote,
            Self::Enhancement(_) => RecordKind::Enhancement,
            Self::FishingArea(_) => RecordKind::FishingArea,
            Self::FishingGoal(_) => RecordKind::FishingGoal,
            Self::TimedFish(_) => RecordKind::TimedFish,
            Self::FishingEvent(_) => RecordKind::FishingEvent,
            Self::Area(_) => RecordKind::Area,
            Self::UiMap(_) => RecordKind::UiMap,
            Self::UiMapAssignment(_) => RecordKind::UiMapAssignment,
            Self::Npc(_) => RecordKind::Npc,
            Self::GameObject(_) => RecordKind::GameObject,
            Self::Spawn(_) => RecordKind::Spawn,
            Self::Quest(_) => RecordKind::Quest,
            Self::VendorItem(_) => RecordKind::VendorItem,
            Self::TrainerSpell(_) => RecordKind::TrainerSpell,
            Self::Spell(_) => RecordKind::Spell,
            Self::Talent(_) => RecordKind::Talent,
            Self::TaxiNode(_) => RecordKind::TaxiNode,
            Self::TaxiPath(_) => RecordKind::TaxiPath,
            Self::Loot(_) => RecordKind::Loot,
            Self::Dungeon(_) => RecordKind::Dungeon,
            Self::Graveyard(_) => RecordKind::Graveyard,
        }
    }
    fn validate(&self) -> Result<()> {
        match self {
            Self::Item(r) => name(&r.name),
            Self::Recipe(r) => {
                name(&r.name)?;
                name(&r.profession)?;
                all(r.sources.iter().map(String::as_str))?;
                if r.reagents.iter().any(|q| q.count == 0) {
                    return Err(invalid("Recipe reagent count must be positive"));
                }
                match r.makes {
                    Some(m) if !(m.is_finite() && m >= 1.0) => {
                        Err(invalid("Recipe yield must be at least one"))
                    }
                    _ => Ok(()),
                }
            }
            Self::ProfessionRank(r) => {
                name(&r.profession)?;
                name(&r.rank)
            }
            Self::ProfessionNote(r) => {
                name(&r.profession)?;
                name(&r.text)
            }
            Self::Enhancement(r) => {
                name(&r.name)?;
                if r.slots.is_empty() || r.slots.iter().any(|s| !(1..=19).contains(s)) {
                    return Err(invalid("Enhancement slots must be inventory slots 1..19"));
                }
                Ok(())
            }
            Self::FishingArea(r) => name(&r.name),
            Self::FishingGoal(r) => {
                name(&r.name)?;
                all(r.places.iter().flatten().map(String::as_str))
            }
            Self::TimedFish(r) => {
                if r.hours.is_none() && r.season.is_none() {
                    return Err(invalid("Timed fish needs hours or a season"));
                }
                name(&r.label)?;
                name(&r.source)
            }
            Self::FishingEvent(r) => {
                if !(1..=7).contains(&r.weekday) || r.start_minute >= 1440 {
                    return Err(invalid("Fishing event weekday or start is out of range"));
                }
                name(&r.name)
            }
            Self::Area(r) => {
                if let Some([lo, hi]) = r.level_range
                    && lo > hi
                {
                    return Err(invalid("Area level range is reversed"));
                }
                name(&r.name)
            }
            Self::UiMap(r) => name(&r.name),
            Self::UiMapAssignment(r) => {
                if !r.ui_min.iter().chain(&r.ui_max).all(|v| unit(*v))
                    || !r
                        .region_min
                        .iter()
                        .chain(&r.region_max)
                        .all(|v| v.is_finite())
                {
                    return Err(invalid("UiMap assignment bounds are out of range"));
                }
                Ok(())
            }
            Self::Npc(r) => {
                if let (Some(lo), Some(hi)) = (r.min_level, r.max_level)
                    && lo > hi
                {
                    return Err(invalid("NPC level range is reversed"));
                }
                name(&r.name)
            }
            Self::GameObject(r) => name(&r.name),
            Self::Spawn(r) => located(&r.world, &r.position),
            Self::Quest(r) => {
                name(&r.title)?;
                if r.objectives.iter().any(|o| o.count == 0) {
                    return Err(invalid("Quest objective count must be positive"));
                }
                Ok(())
            }
            Self::VendorItem(_) | Self::TrainerSpell(_) => Ok(()),
            Self::Spell(r) => name(&r.name),
            Self::Talent(r) => {
                if r.ranks.is_empty() {
                    return Err(invalid("Talent needs at least one rank"));
                }
                Ok(())
            }
            Self::TaxiNode(r) => {
                name(&r.name)?;
                located(&r.world, &r.position)
            }
            Self::TaxiPath(r) => r.points.iter().try_for_each(point),
            Self::Loot(r) => {
                if r.item.is_some() == r.reference.is_some() {
                    return Err(invalid("Loot row needs exactly one of item or reference"));
                }
                if !(r.chance.is_finite() && (0.0..=100.0).contains(&r.chance.abs()))
                    || r.min_count > r.max_count
                {
                    return Err(invalid("Loot chance or counts are out of range"));
                }
                Ok(())
            }
            Self::Dungeon(r) => {
                name(&r.name)?;
                r.entrance.value().map_or(Ok(()), point)?;
                if let Some(estimate) = &r.entrance_estimate {
                    if ![estimate.x, estimate.y].iter().all(|v| v.is_finite()) {
                        return Err(invalid("Planar entrance coordinates must be finite"));
                    }
                    name(&estimate.derivation)?;
                    name(&estimate.uncertainty)?;
                }
                r.excluded_reason.as_deref().map_or(Ok(()), name)
            }
            Self::Graveyard(r) => located(&r.world, &r.position),
        }
    }
}

fn name(s: &str) -> Result<()> {
    if s.trim().is_empty() || s.len() > MAX_TEXT || s.chars().any(|c| c.is_control() && c != '\n') {
        return Err(invalid(
            "Text must be 1..8192 bytes without control characters",
        ));
    }
    Ok(())
}
fn all<'a>(mut values: impl Iterator<Item = &'a str>) -> Result<()> {
    values.try_for_each(name)
}
fn unit(v: f64) -> bool {
    v.is_finite() && (0.0..=1.0).contains(&v)
}
fn point(p: &WorldPoint) -> Result<()> {
    if ![p.x, p.y, p.z].iter().all(|v| v.is_finite()) {
        return Err(invalid("World coordinates must be finite"));
    }
    Ok(())
}
fn located(world: &WorldPoint, position: &Observation<MapPosition>) -> Result<()> {
    point(world)?;
    match position.value() {
        Some(p) if !(unit(p.x) && unit(p.y)) => {
            Err(invalid("Map position must be normalized 0..1"))
        }
        _ => Ok(()),
    }
}

impl WorldData {
    /// Structural and semantic checks; byte limits belong to the reader.
    pub fn validate(&self) -> Result<()> {
        if self.format != FORMAT || self.schema_version != SCHEMA_VERSION {
            return Err(invalid("Not a navzeroth-world-data v1 document"));
        }
        let dotted = self.dataset.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
        });
        if !dotted || self.dataset.len() > 128 {
            return Err(invalid("Dataset must be dotted lowercase words"));
        }
        if self.shard.count == 0
            || self.shard.count > MAX_SHARDS
            || self.shard.index >= self.shard.count
        {
            return Err(invalid("Shard index must be below a bounded shard count"));
        }
        if self.sources.is_empty() || self.sources.len() > 64 {
            return Err(invalid("A shard names 1..64 sources"));
        }
        for source in &self.sources {
            if !source.url.starts_with("https://") || name(&source.url).is_err() {
                return Err(invalid("Source URLs must be https"));
            }
            if let Some(hash) = &source.snapshot_sha256
                && !(hash.len() == 64
                    && hash.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')))
            {
                return Err(invalid("Snapshot hash must be lowercase SHA-256"));
            }
        }
        name(&self.generator.name)?;
        name(&self.generator.version)?;
        name(&self.generator.credit)?;
        if self.confidence == Confidence::VerifiedOnBuild {
            let same_build = self.origin.build.value().is_some()
                && self.origin.build.value() == self.target.build.value();
            if self.origin.edition != self.target.edition || !same_build {
                return Err(invalid(
                    "Only data from the target edition and build can be verified on build",
                ));
            }
        }
        if self.origin.edition != self.target.edition
            && self.confidence != Confidence::CrossEditionUnverified
            && self.confidence != Confidence::DisputedOrStale
        {
            return Err(invalid(
                "Data from another edition must stay cross_edition_unverified",
            ));
        }
        if self.origin.build.status() == ObservationStatus::Known
            && self.origin.build.value().is_none()
        {
            return Err(invalid("Known origin build requires a value"));
        }
        if self.records.is_empty() || self.records.len() > MAX_RECORDS {
            return Err(invalid("A shard holds 1..250000 records"));
        }
        for record in &self.records {
            if record.kind() != self.record_kind {
                return Err(invalid("Every record must match the shard's record kind"));
            }
            record.validate()?;
        }
        Ok(())
    }
}

/// Parse and validate one shard from bytes, enforcing [`MAX_SHARD_BYTES`].
pub fn parse(bytes: &[u8]) -> Result<WorldData> {
    if bytes.len() > MAX_SHARD_BYTES {
        return Err(invalid("World-data shard exceeds 16 MiB"));
    }
    let data: WorldData =
        serde_json::from_slice(bytes).map_err(|_| invalid("World-data shard is not valid JSON"))?;
    data.validate()?;
    Ok(data)
}

/// The draft 2020-12 schema; Rust validation stays authoritative.
pub fn world_data_schema() -> Value {
    let generator = SchemaSettings::draft2020_12()
        .for_deserialize()
        .into_generator();
    let mut value = generator.into_root_schema_for::<WorldData>().to_value();
    value["$id"] =
        json!("https://github.com/Laymer/Navzeroth/schemas/world-data/v1/world-data.schema.json");
    value["description"] = json!(
        "navzeroth-world-data v1 shard. Absent optional fields mean the source did not provide them (unknown), never zero. Structural validity is not a rights grant or a verified Forever fact."
    );
    value["properties"]["format"] = json!({"const": FORMAT});
    value["properties"]["schema_version"] = json!({"const": SCHEMA_VERSION});
    value["properties"]["records"]["maxItems"] = json!(MAX_RECORDS);
    value
}
/// Pretty schema text with a trailing newline, as committed under `schemas/world-data/v1/`.
pub fn world_data_schema_text() -> String {
    let mut text = serde_json::to_string_pretty(&world_data_schema()).expect("schema serializes");
    text.push('\n');
    text
}
