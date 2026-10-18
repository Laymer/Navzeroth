//! Pure reviewed mappings over decoded rows; no filesystem or gameplay trust.
use crate::{
    Result,
    tables::{Cell, ResearchRow},
};
use azeroth_core::{
    local_data_gameplay::{
        self as model, ColumnType, ExpandedProjection, ItemStats, MappedTable, ModelledStat,
        Record, TableSpec, Value,
    },
    local_data_projection::{EquipmentProjection, Fact, SourceState},
};
use std::collections::BTreeMap;
const MAX_CELLS: usize = 4_000_000;
const MAX_TEXT: usize = 32 * 1024 * 1024;
#[derive(Default)]
pub(super) struct Quota {
    cells: usize,
    text: usize,
}
impl Quota {
    fn take(&mut self, cells: usize, text: usize) -> Result<()> {
        self.cells = self
            .cells
            .checked_add(cells)
            .filter(|v| *v <= MAX_CELLS)
            .ok_or("Expanded projection cell quota exceeded")?;
        self.text = self
            .text
            .checked_add(text)
            .filter(|v| *v <= MAX_TEXT)
            .ok_or("Expanded projection text quota exceeded")?;
        Ok(())
    }
}
fn scalar(cell: &Cell, shape: ColumnType, quota: &mut Quota) -> Result<Fact<Value>> {
    quota.take(1, 0)?;
    Ok(match (cell, shape) {
        (Cell::Unknown, _) => Fact::Unknown,
        (Cell::Signed(v), ColumnType::Integer) => Fact::Known(Value::Integer(*v)),
        (Cell::Unsigned(v), ColumnType::Integer) => {
            i64::try_from(*v).map_or(Fact::Unsupported, |n| Fact::Known(Value::Integer(n)))
        }
        (Cell::Float(v), ColumnType::Number) if v.is_finite() => Fact::Known(Value::Number(*v)),
        (Cell::Text(v), ColumnType::Text) if v.len() <= 64 * 1024 => {
            quota.take(0, v.len())?;
            Fact::Known(Value::Text(v.clone()))
        }
        (Cell::Array(v), ColumnType::Integers(n)) if v.len() == n as usize => {
            Fact::Known(Value::Array(
                v.iter()
                    .map(|cell| scalar(cell, ColumnType::Integer, quota))
                    .collect::<Result<_>>()?,
            ))
        }
        (Cell::Array(v), ColumnType::Numbers(n)) if v.len() == n as usize => {
            Fact::Known(Value::Array(
                v.iter()
                    .map(|cell| scalar(cell, ColumnType::Number, quota))
                    .collect::<Result<_>>()?,
            ))
        }
        _ => Fact::Unsupported,
    })
}
pub(super) fn map_table(
    spec: &TableSpec,
    rows: Option<&[ResearchRow]>,
    quota: &mut Quota,
) -> Result<MappedTable> {
    let records = rows
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let mut fields = BTreeMap::new();
                    for column in spec.columns {
                        let value = match row.fields.get(column.name) {
                            Some(cell) => scalar(cell, column.shape, quota)?,
                            None => {
                                quota.take(1, 0)?;
                                Fact::Missing
                            }
                        };
                        fields.insert(column.name.into(), value);
                    }
                    Ok(Record { id: row.id, fields })
                })
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?;
    Ok(MappedTable {
        table: spec.table.into(),
        state: if records.is_some() {
            SourceState::Projected
        } else {
            SourceState::Gap
        },
        records,
    })
}
fn propagate<T, U>(value: &Fact<T>) -> Fact<U> {
    match value {
        Fact::Known(_) => Fact::Unsupported,
        Fact::Unknown => Fact::Unknown,
        Fact::Missing => Fact::Missing,
        Fact::Unsupported => Fact::Unsupported,
    }
}
fn number(record: &Record, column: &str, index: usize) -> Fact<f64> {
    match record.fields.get(column) {
        Some(Fact::Known(Value::Array(values))) => match values.get(index) {
            Some(Fact::Known(Value::Number(v))) => Fact::Known(*v),
            Some(v) => propagate(v),
            None => Fact::Unsupported,
        },
        Some(v) => propagate(v),
        None => Fact::Missing,
    }
}
fn budget(
    item: &azeroth_core::local_data_projection::EquipmentItem,
    budgets: Option<&[Record]>,
) -> Fact<f64> {
    let Some(level) = item.item_level.known() else {
        return propagate(&item.item_level);
    };
    let Some(quality) = item.quality.known() else {
        return propagate(&item.quality);
    };
    let Some(inv) = item.inventory_type.known() else {
        return propagate(&item.inventory_type);
    };
    let Some(index) = model::budget_index(*inv, item.subclass_id.known().copied()) else {
        return Fact::Unsupported;
    };
    let column = match quality {
        2 => "GoodF",
        3 | 7 => "SuperiorF",
        4..=6 => "EpicF",
        _ => return Fact::Unsupported,
    };
    let Some(rows) = budgets else {
        return Fact::Missing;
    };
    let Ok(id) = u32::try_from(*level) else {
        return Fact::Unsupported;
    };
    let Ok(index_row) = rows.binary_search_by_key(&id, |r| r.id) else {
        return Fact::Missing;
    };
    match number(&rows[index_row], column, index) {
        Fact::Known(v) if v < 0.0 => Fact::Unsupported,
        v => v,
    }
}
/// The output is an unverified base-budget model. It cannot be treated as an
/// observed item amount; socket costs/bonuses/scaling need additional inputs.
pub(super) fn item_stats(
    equipment: &EquipmentProjection,
    expanded: &ExpandedProjection,
) -> Vec<ItemStats> {
    let sparse = expanded
        .table("ItemSparse")
        .and_then(|t| t.records.as_deref());
    let budgets = expanded
        .table("RandPropPoints")
        .and_then(|t| t.records.as_deref());
    equipment
        .items
        .iter()
        .map(|item| {
            let raw = sparse.and_then(|rows| {
                rows.binary_search_by_key(&item.item_id, |r| r.id)
                    .ok()
                    .map(|i| &rows[i])
            });
            let points = budget(item, budgets);
            let stats = (0..10)
                .map(|slot| {
                    let (stat_type, allocation) = match &item.stat_allocations {
                        Fact::Known(values) => values
                            .get(slot)
                            .map(|v| (v.stat_type.clone(), v.allocation.clone()))
                            .unwrap_or((Fact::Unsupported, Fact::Unsupported)),
                        v => (propagate(v), propagate(v)),
                    };
                    let fraction =
                        raw.map_or(Fact::Missing, |r| number(r, "StatPercentageOfSocket", slot));
                    let amount = model::base_budget_amount(&allocation, &points, &fraction);
                    ModelledStat {
                        slot: slot as u8,
                        stat_type,
                        allocation,
                        socket_fraction: fraction,
                        budget: points.clone(),
                        amount,
                    }
                })
                .collect();
            ItemStats {
                item_id: item.item_id,
                method: "research_budget_model_v1".into(),
                verified_for_gameplay: false,
                stats,
            }
        })
        .collect()
}

fn reference_cells(value: &Fact<Value>) -> Vec<&Fact<Value>> {
    match value {
        Fact::Known(Value::Array(values)) => values.iter().collect(),
        _ => vec![value],
    }
}
pub(super) fn references(
    expanded: &ExpandedProjection,
    geography: &azeroth_core::local_data_projection::GeographyProjection,
    sources: &[azeroth_core::local_data_projection::SourceBinding],
) -> Vec<model::ReferenceSummary> {
    model::REFERENCES
        .iter()
        .map(|reference| {
            let mut summary = model::ReferenceSummary {
                table: reference.table.into(),
                field: reference.field.into(),
                target: reference.target.into(),
                matched: 0,
                unresolved_in_projected_rows: 0,
                target_unavailable: 0,
                ambiguous: 0,
                unknown: 0,
                missing: 0,
                unsupported: 0,
                zero_sentinel: 0,
            };
            let mut targets = BTreeMap::<i64, u32>::new();
            let available = match reference.target {
                "Map" => {
                    for map in &geography.maps {
                        targets.insert(map.map_id as i64, 1);
                    }
                    sources
                        .iter()
                        .any(|s| s.table == "Map" && s.state == SourceState::Projected)
                }
                "UiMap" => {
                    for map in &geography.ui_maps {
                        targets.insert(map.ui_map_id as i64, 1);
                    }
                    sources
                        .iter()
                        .any(|s| s.table == "UiMap" && s.state == SourceState::Projected)
                }
                name => {
                    let table = expanded.table(name);
                    if let Some(records) = table.and_then(|t| t.records.as_ref()) {
                        for row in records {
                            let value = match reference.target_field {
                                Some(column) => match row.fields.get(column) {
                                    Some(Fact::Known(Value::Integer(v))) => Some(*v),
                                    _ => None,
                                },
                                None => Some(row.id as i64),
                            };
                            if let Some(value) = value {
                                *targets.entry(value).or_default() += 1;
                            }
                        }
                    }
                    table.is_some_and(|t| t.state == SourceState::Projected)
                }
            };
            for row in expanded
                .table(reference.table)
                .and_then(|t| t.records.as_ref())
                .into_iter()
                .flatten()
            {
                let Some(value) = row.fields.get(reference.field) else {
                    summary.missing += 1;
                    continue;
                };
                for value in reference_cells(value) {
                    match value {
                        Fact::Known(Value::Integer(0)) if reference.zero_is_none => {
                            summary.zero_sentinel += 1
                        }
                        Fact::Known(Value::Integer(id)) if (0..=u32::MAX as i64).contains(id) => {
                            if !available {
                                summary.target_unavailable += 1;
                            } else {
                                match targets.get(id) {
                                    Some(1) => summary.matched += 1,
                                    Some(_) => summary.ambiguous += 1,
                                    None => summary.unresolved_in_projected_rows += 1,
                                }
                            }
                        }
                        Fact::Known(_) | Fact::Unsupported => summary.unsupported += 1,
                        Fact::Unknown => summary.unknown += 1,
                        Fact::Missing => summary.missing += 1,
                    }
                }
            }
            summary
        })
        .collect()
}
pub(super) fn blocked_talents(expanded: &ExpandedProjection) -> Vec<u32> {
    use std::collections::{BTreeSet, VecDeque};
    let Some(rows) = expanded.table("Talent").and_then(|t| t.records.as_ref()) else {
        return vec![];
    };
    let mut incoming: BTreeMap<u32, usize> = rows.iter().map(|r| (r.id, 0)).collect();
    let mut outgoing = BTreeMap::<u32, BTreeSet<u32>>::new();
    for row in rows {
        if let Some(value) = row.fields.get("PrereqTalent") {
            for cell in reference_cells(value) {
                if let Fact::Known(Value::Integer(id)) = cell
                    && let Ok(id) = u32::try_from(*id)
                    && id != 0
                    && incoming.contains_key(&id)
                    && outgoing.entry(id).or_default().insert(row.id)
                {
                    *incoming.get_mut(&row.id).unwrap() += 1;
                }
            }
        }
    }
    let mut queue: VecDeque<u32> = incoming
        .iter()
        .filter_map(|(id, n)| (*n == 0).then_some(*id))
        .collect();
    while let Some(id) = queue.pop_front() {
        if let Some(children) = outgoing.get(&id) {
            for child in children {
                let n = incoming.get_mut(child).unwrap();
                *n -= 1;
                if *n == 0 {
                    queue.push_back(*child);
                }
            }
        }
    }
    incoming
        .into_iter()
        .filter_map(|(id, n)| (n > 0).then_some(id))
        .collect()
}
