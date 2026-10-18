//! Reviewed client field mappings and bounded research stat models, without I/O.
use crate::local_data_projection::{Fact, SourceState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub enum ColumnType {
    Integer,
    Number,
    Text,
    Integers(u8),
    Numbers(u8),
}
#[derive(Clone, Copy, Debug)]
pub struct Column {
    pub name: &'static str,
    pub shape: ColumnType,
}
#[derive(Clone, Copy, Debug)]
pub struct TableSpec {
    pub domain: &'static str,
    pub table: &'static str,
    pub layouts: &'static [&'static str],
    pub columns: &'static [Column],
}
macro_rules! columns { ($($name:literal => $shape:expr),* $(,)?) => { &[$(Column{name:$name,shape:$shape}),*] }; }
use ColumnType::{Integer as I, Integers as IA, Number as N, Numbers as NA, Text as T};
pub const ITEM_STAT_INPUT: TableSpec = TableSpec {
    domain: "equipment",
    table: "ItemSparse",
    layouts: &["6fcc3191"],
    columns: columns!["StatModifier_bonusStat"=>IA(10),"StatPercentEditor"=>IA(10),"StatPercentageOfSocket"=>NA(10),"SocketType"=>IA(3),"RequiredAbility"=>I,"MinFactionID"=>I,"MinReputation"=>I,"Flags"=>IA(5),"ItemDelay"=>I,"DamageType"=>I,"QualityModifier"=>N,"ItemLevelOffsetCurveID"=>I,"PlayerLevelToItemLevelCurveID"=>I],
};
macro_rules! spec { ($domain:literal,$table:literal,$layout:literal,[$($name:literal => $shape:expr),* $(,)?])=>{TableSpec{domain:$domain,table:$table,layouts:&[$layout],columns:columns![$($name=>$shape),*]}}; }
/// Exact retained 1.60.1.70450 layouts. No closest-layout or cross-edition fallback.
/// Values are client facts; these schemas do not establish server applicability.
pub const TABLES: &[TableSpec] = &[
    spec!("equipment","RandPropPoints","4fd22743",["EpicF"=>NA(5),"SuperiorF"=>NA(5),"GoodF"=>NA(5),"Epic"=>IA(5),"Superior"=>IA(5),"Good"=>IA(5)]),
    spec!("equipment","ItemClass","35680eb8",["ClassName_lang"=>T,"ClassID"=>I,"Flags"=>I]),
    spec!("equipment","ItemSubClass","1e67db87",["DisplayName_lang"=>T,"VerboseName_lang"=>T,"ClassID"=>I,"SubClassID"=>I,"PrerequisiteProficiency"=>I,"PostrequisiteProficiency"=>I,"Flags"=>I]),
    spec!("class_roles","SkillLine","6763217c",["DisplayName_lang"=>T,"Description_lang"=>T,"CategoryID"=>I,"ParentSkillLineID"=>I,"SpellBookSpellID"=>I,"Flags"=>I]),
    spec!("class_roles","SkillLineAbility","224f7ea0",["SkillLine"=>I,"Spell"=>I,"MinSkillLineRank"=>I,"ClassMask"=>I,"RaceMasks"=>IA(2),"AcquireMethod"=>I,"SupercedesSpell"=>I,"Flags"=>I]),
    spec!("class_roles","SkillRaceClassInfo","24277b48",["SkillID"=>I,"ClassMask"=>I,"RaceMasks"=>IA(2),"Availability"=>I,"MinLevel"=>I,"Flags"=>I]),
    spec!("class_roles","SpellName","782ee721",["Name_lang"=>T]),
    spec!("class_roles","Spell","e3d134fb",["NameSubtext_lang"=>T,"Description_lang"=>T,"AuraDescription_lang"=>T]),
    spec!("class_roles","SpellLevels","7eb86fdc",["SpellID"=>I,"DifficultyID"=>I,"BaseLevel"=>I,"SpellLevel"=>I,"MaxLevel"=>I]),
    spec!("class_roles","SpellMisc","434b3607",["SpellID"=>I,"DifficultyID"=>I,"Attributes"=>IA(17),"CastingTimeIndex"=>I,"DurationIndex"=>I,"RangeIndex"=>I,"SchoolMask"=>I,"SpellIconFileDataID"=>I,"ActiveIconFileDataID"=>I]),
    spec!("class_roles","SpellEffect","5362e3d4",["SpellID"=>I,"DifficultyID"=>I,"EffectIndex"=>I,"Effect"=>I,"EffectAura"=>I,"EffectBasePointsF"=>N,"EffectBonusCoefficient"=>N,"EffectRealPointsPerLevel"=>N,"EffectTriggerSpell"=>I,"EffectItemType"=>I,"EffectMiscValue"=>IA(2),"EffectRadiusIndex"=>IA(2),"ImplicitTarget"=>IA(2)]),
    spec!("class_roles","SpellEquippedItems","d9177916",["SpellID"=>I,"EquippedItemClass"=>I,"EquippedItemInvTypes"=>I,"EquippedItemSubclass"=>I]),
    spec!("class_roles","SpellLearnSpell","6b9c3ac9",["SpellID"=>I,"LearnSpellID"=>I,"OverridesSpellID"=>I]),
    spec!("class_roles","SpellDuration","a931bd2b",["Duration"=>I,"MaxDuration"=>I,"DurationPerResource"=>I]),
    spec!("class_roles","SpellRange","ada13705",["DisplayName_lang"=>T,"RangeMin"=>NA(2),"RangeMax"=>NA(2),"Flags"=>I]),
    spec!("class_roles","SpellCastTimes","75b6bd3a",["Base"=>I,"Minimum"=>I]),
    spec!("class_roles","SpellPower","61ad223f",["SpellID"=>I,"OrderIndex"=>I,"PowerType"=>I,"ManaCost"=>I,"ManaCostPerLevel"=>I,"ManaPerSecond"=>I,"PowerCostPct"=>N,"PowerCostMaxPct"=>N,"OptionalCostPct"=>N,"PowerPctPerSecond"=>N,"OptionalCost"=>I,"RequiredAuraSpellID"=>I]),
    spec!("class_roles","Talent","147b0045",["Description_lang"=>T,"TierID"=>I,"ColumnIndex"=>I,"TabID"=>I,"ClassID"=>I,"SpecID"=>I,"SpellID"=>I,"RequiredSpellID"=>I,"OverridesSpellID"=>I,"SpellRank"=>IA(9),"PrereqTalent"=>IA(3),"PrereqRank"=>IA(3),"Flags"=>I]),
    spec!("class_roles","TalentTab","b436f3c7",["Name_lang"=>T,"OrderIndex"=>I,"ClassMask"=>I,"RoleMask"=>I,"CategoryEnumID"=>I,"MasterySpellID"=>IA(2)]),
    spec!("geography","AreaTable","9995b797",["ZoneName"=>T,"AreaName_lang"=>T,"ContinentID"=>I,"ParentAreaID"=>I,"ExplorationLevel"=>I,"FactionGroupMask"=>I,"MountFlags"=>I,"Flags"=>IA(2)]),
    spec!("geography","MapDifficulty","24a16ad3",["MapID"=>I,"DifficultyID"=>I,"LockID"=>I,"ResetInterval"=>I,"MaxPlayers"=>I,"Flags"=>I]),
    spec!("geography","UiMapLink","712a46e6",["ParentUiMapID"=>I,"ChildUiMapID"=>I,"OrderIndex"=>I,"PlayerConditionID"=>I,"UiMin"=>NA(2),"UiMax"=>NA(2),"Flags"=>I]),
    spec!("geography","QuestPOIBlob","fdc814cf",["MapID"=>I,"UiMapID"=>I,"QuestID"=>I,"ObjectiveIndex"=>I,"ObjectiveID"=>I,"NumPoints"=>I,"PlayerConditionID"=>I,"NavigationPlayerConditionID"=>I,"Flags"=>I]),
    spec!("geography","QuestPOIPoint","5cbbefe7",["QuestPOIBlobID"=>I,"X"=>I,"Y"=>I,"Z"=>I]),
    spec!("quests","QuestV2","1854bdb9",["UniqueBitFlag"=>I,"UiQuestDetailsThemeID"=>I]),
    spec!("quests","QuestLineXQuest","8552f722",["QuestLineID"=>I,"QuestID"=>I,"OrderIndex"=>I,"Flags"=>I]),
];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Value {
    Integer(i64),
    Number(f64),
    Text(String),
    Array(Vec<Fact<Value>>),
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub id: u32,
    pub fields: BTreeMap<String, Fact<Value>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MappedTable {
    pub table: String,
    pub state: SourceState,
    pub records: Option<Vec<Record>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelledStat {
    pub slot: u8,
    pub stat_type: Fact<i64>,
    pub allocation: Fact<i64>,
    pub socket_fraction: Fact<f64>,
    pub budget: Fact<f64>,
    pub amount: Fact<f64>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ItemStats {
    pub item_id: u32,
    pub method: String,
    pub verified_for_gameplay: bool,
    pub stats: Vec<ModelledStat>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExpandedProjection {
    pub references: Vec<ReferenceSummary>,
    /// Cycles in known talent prerequisites, and talents blocked by those cycles.
    pub blocked_talent_ids: Vec<u32>,
    pub format_version: u32,
    pub tables: Vec<MappedTable>,
    pub item_stats: Vec<ItemStats>,
}
impl ExpandedProjection {
    pub fn record_count(&self) -> usize {
        self.tables
            .iter()
            .filter_map(|t| t.records.as_ref())
            .map(Vec::len)
            .sum::<usize>()
            + self.item_stats.len()
    }
    pub fn table(&self, name: &str) -> Option<&MappedTable> {
        self.tables.iter().find(|t| t.table == name)
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.format_version != 1 || self.tables.len() != TABLES.len() + 1 {
            return Err("Unsupported expanded projection schema");
        }
        for (table, spec) in self
            .tables
            .iter()
            .zip(std::iter::once(&ITEM_STAT_INPUT).chain(TABLES))
        {
            if table.table != spec.table
                || (table.state == SourceState::Projected) != table.records.is_some()
            {
                return Err("Inconsistent expanded source state");
            }
            if let Some(rows) = &table.records {
                if !rows.windows(2).all(|w| w[0].id < w[1].id) {
                    return Err("Expanded rows are not canonical");
                }
                for row in rows {
                    if row.fields.len() != spec.columns.len() {
                        return Err("Expanded record fields differ from profile");
                    }
                    for column in spec.columns {
                        let value = row
                            .fields
                            .get(column.name)
                            .ok_or("Missing expanded field")?;
                        validate_field(value, column.shape)?;
                    }
                }
            }
        }
        if self.references.len() != REFERENCES.len()
            || !self.blocked_talent_ids.windows(2).all(|w| w[0] < w[1])
        {
            return Err("Invalid reference diagnostics");
        }
        for (summary, reference) in self.references.iter().zip(REFERENCES) {
            if summary.table != reference.table
                || summary.field != reference.field
                || summary.target != reference.target
                || [
                    summary.matched,
                    summary.unresolved_in_projected_rows,
                    summary.target_unavailable,
                    summary.ambiguous,
                    summary.unknown,
                    summary.missing,
                    summary.unsupported,
                    summary.zero_sentinel,
                ]
                .iter()
                .map(|v| *v as u64)
                .sum::<u64>()
                    > 4_000_000
            {
                return Err("Reference diagnostics differ from profile");
            }
        }
        if !self
            .item_stats
            .windows(2)
            .all(|w| w[0].item_id < w[1].item_id)
        {
            return Err("Item stat rows are not canonical");
        }
        for item in &self.item_stats {
            if item.method != "research_budget_model_v1"
                || item.verified_for_gameplay
                || item.stats.len() != 10
            {
                return Err("Invalid item stat model");
            }
            for (i, stat) in item.stats.iter().enumerate() {
                if stat.slot != i as u8
                    || [&stat.socket_fraction, &stat.budget, &stat.amount]
                        .iter()
                        .any(|v| matches!(v,Fact::Known(n)if !n.is_finite()))
                {
                    return Err("Invalid modelled stat");
                }
            }
        }
        Ok(())
    }
}
fn validate_field(value: &Fact<Value>, shape: ColumnType) -> Result<(), &'static str> {
    let Fact::Known(value) = value else {
        return Ok(());
    };
    match (value, shape) {
        (Value::Integer(_), ColumnType::Integer) => Ok(()),
        (Value::Number(v), ColumnType::Number) if v.is_finite() => Ok(()),
        (Value::Text(v), ColumnType::Text) if v.len() <= 64 * 1024 => Ok(()),
        (Value::Array(v), ColumnType::Integers(n)) if v.len() == n as usize => {
            for cell in v {
                validate_field(cell, ColumnType::Integer)?;
            }
            Ok(())
        }
        (Value::Array(v), ColumnType::Numbers(n)) if v.len() == n as usize => {
            for cell in v {
                validate_field(cell, ColumnType::Number)?;
            }
            Ok(())
        }
        _ => Err("Expanded field type or range differs from profile"),
    }
}
/// A client mask result only. It does not establish proficiency or availability.
pub fn class_mask(mask: &Fact<i64>, class_id: u8, width: u8) -> Fact<bool> {
    if !matches!(width, 16 | 32) || class_id == 0 || class_id > width {
        return Fact::Unsupported;
    }
    match mask {
        Fact::Known(v) if *v >= -(1i64 << (width - 1)) && *v < (1i64 << width) => {
            Fact::Known((*v as u64 & (1u64 << (class_id - 1))) != 0)
        }
        Fact::Known(_) => Fact::Unsupported,
        Fact::Unknown => Fact::Unknown,
        Fact::Missing => Fact::Missing,
        Fact::Unsupported => Fact::Unsupported,
    }
}
/// Reviewed budget bucket for a research model, not confirmed Forever arithmetic.
pub fn budget_index(inventory_type: i64, subclass: Option<i64>) -> Option<usize> {
    match inventory_type {
        1 | 4 | 5 | 7 | 15 | 17 | 20 | 25 => Some(0),
        3 | 6 | 8 | 10 | 12 => Some(1),
        2 | 9 | 11 | 14 | 16 | 23 => Some(2),
        13 | 21 | 22 => Some(3),
        26 => subclass.map(|v| if v == 19 { 3 } else { 0 }),
        28 => Some(4),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Reference {
    pub table: &'static str,
    pub field: &'static str,
    pub target: &'static str,
    pub target_field: Option<&'static str>,
    pub zero_is_none: bool,
}
macro_rules! reference {
    ($t:literal,$f:literal,$target:literal,$zero:expr) => {
        Reference {
            table: $t,
            field: $f,
            target: $target,
            target_field: None,
            zero_is_none: $zero,
        }
    };
}
pub const REFERENCES: &[Reference] = &[
    Reference {
        table: "ItemSubClass",
        field: "ClassID",
        target: "ItemClass",
        target_field: Some("ClassID"),
        zero_is_none: false,
    },
    reference!("SkillLineAbility", "SkillLine", "SkillLine", false),
    reference!("SkillLineAbility", "Spell", "SpellName", true),
    reference!("SkillRaceClassInfo", "SkillID", "SkillLine", false),
    reference!("SpellLevels", "SpellID", "SpellName", true),
    reference!("SpellMisc", "SpellID", "SpellName", true),
    reference!("SpellEffect", "SpellID", "SpellName", true),
    reference!("SpellEffect", "EffectTriggerSpell", "SpellName", true),
    reference!("SpellEquippedItems", "SpellID", "SpellName", true),
    reference!("SpellLearnSpell", "SpellID", "SpellName", true),
    reference!("SpellLearnSpell", "LearnSpellID", "SpellName", true),
    reference!("SpellMisc", "CastingTimeIndex", "SpellCastTimes", true),
    reference!("SpellMisc", "DurationIndex", "SpellDuration", true),
    reference!("SpellMisc", "RangeIndex", "SpellRange", true),
    reference!("Talent", "SpellID", "SpellName", true),
    reference!("Talent", "SpellRank", "SpellName", true),
    reference!("Talent", "RequiredSpellID", "SpellName", true),
    reference!("Talent", "PrereqTalent", "Talent", true),
    reference!("Talent", "TabID", "TalentTab", true),
    reference!("TalentTab", "MasterySpellID", "SpellName", true),
    reference!("AreaTable", "ParentAreaID", "AreaTable", true),
    reference!("AreaTable", "ContinentID", "Map", false),
    reference!("MapDifficulty", "MapID", "Map", false),
    reference!("UiMapLink", "ParentUiMapID", "UiMap", true),
    reference!("UiMapLink", "ChildUiMapID", "UiMap", true),
    reference!("QuestPOIBlob", "MapID", "Map", false),
    reference!("QuestPOIBlob", "UiMapID", "UiMap", true),
    reference!("QuestPOIBlob", "QuestID", "QuestV2", true),
    reference!("QuestPOIPoint", "QuestPOIBlobID", "QuestPOIBlob", false),
    reference!("QuestLineXQuest", "QuestID", "QuestV2", true),
];
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceSummary {
    pub table: String,
    pub field: String,
    pub target: String,
    pub matched: u32,
    pub unresolved_in_projected_rows: u32,
    pub target_unavailable: u32,
    pub ambiguous: u32,
    pub unknown: u32,
    pub missing: u32,
    pub unsupported: u32,
    pub zero_sentinel: u32,
}
/// The race bit is the zero-based client PlayableRaceBit, not a race record ID.
pub fn race_mask(mask: &Fact<Vec<i64>>, race_bit: u8) -> Fact<bool> {
    if race_bit >= 64 {
        return Fact::Unsupported;
    }
    match mask {
        Fact::Known(words) if words.len() == 2 => {
            let word = words[(race_bit / 32) as usize];
            if !(-2_147_483_648..=4_294_967_295).contains(&word) {
                return Fact::Unsupported;
            }
            Fact::Known((word as u64 & (1u64 << (race_bit % 32))) != 0)
        }
        Fact::Known(_) => Fact::Unsupported,
        Fact::Unknown => Fact::Unknown,
        Fact::Missing => Fact::Missing,
        Fact::Unsupported => Fact::Unsupported,
    }
}

/// Mathematical base-budget model only, not a claim about the game's final stat.
/// A nonzero socket fraction needs a separately reviewed socket-cost table.
pub fn base_budget_amount(
    allocation: &Fact<i64>,
    budget: &Fact<f64>,
    socket: &Fact<f64>,
) -> Fact<f64> {
    match (allocation, budget, socket) {
        (_, Fact::Known(p), _) if !p.is_finite() || *p < 0.0 => Fact::Unsupported,
        (_, _, Fact::Known(f)) if *f != 0.0 => Fact::Unsupported,
        (Fact::Known(a), Fact::Known(p), Fact::Known(0.0)) if *a >= 0 => {
            let result = (*a as f64) * p / 10000.0;
            if result.is_finite() {
                Fact::Known(result)
            } else {
                Fact::Unsupported
            }
        }
        (Fact::Known(a), _, _) if *a < 0 => Fact::Unsupported,
        (Fact::Unsupported, _, _) | (_, Fact::Unsupported, _) | (_, _, Fact::Unsupported) => {
            Fact::Unsupported
        }
        (Fact::Unknown, _, _) | (_, Fact::Unknown, _) | (_, _, Fact::Unknown) => Fact::Unknown,
        _ => Fact::Missing,
    }
}
