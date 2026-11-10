use super::dto::*;
use azeroth_core::{
    error::Result,
    observation::{Observation, ObservationMetadata},
    progression::*,
    types::*,
};
use serde::Serialize;

fn observed<T: Serialize>(
    value: Option<T>,
    capture: &ObservationMetadata,
) -> Result<Observation<T>> {
    match value {
        Some(value) => Observation::known(value, capture.clone()),
        None => Observation::unavailable(
            azeroth_core::observation::ObservationStatus::Unknown,
            capture.clone(),
        ),
    }
}
fn value<T, U: Serialize>(
    input: &ManualValue<T>,
    capture: &ObservationMetadata,
    map: impl FnOnce(&T) -> Result<U>,
) -> Result<Observation<U>> {
    observed(
        match input {
            ManualValue::Unknown => None,
            ManualValue::Known { value } => Some(map(value)?),
        },
        capture,
    )
}
fn enumeration<T, U: Serialize>(
    input: &ManualValue<Enumeration<T>>,
    capture: &ObservationMetadata,
    map: impl Fn(&T) -> Result<U>,
) -> Result<Observation<Enumeration<U>>> {
    value(input, capture, |e| {
        Ok(Enumeration {
            completeness: e.completeness,
            entries: RecordMap::new(
                e.entries
                    .iter()
                    .map(|(k, v)| Ok((k.clone(), map(v)?)))
                    .collect::<Result<Vec<_>>>()?,
            )?,
        })
    })
}
pub(super) fn payload(
    input: &ManualProgressionRequest,
    capture: ObservationMetadata,
) -> Result<ProgressionSnapshot> {
    let c = &capture;
    let inventory = |input: &ManualValue<Enumeration<ManualInventory>>| {
        enumeration(input, c, |v| {
            Ok(InventoryQuantity {
                item: v.item.clone(),
                quantity: observed(Some(v.quantity), c)?,
            })
        })
    };
    ProgressionSnapshot::try_new(ProgressionSnapshotData {
        payload_version: SchemaVersion,
        id: input.id.clone(),
        binding: input.expected.clone(),
        capture: capture.clone(),
        spells: enumeration(&input.spells, c, |v| observed(Some(*v), c))?,
        money: value(&input.money, c, |v| Ok(*v))?,
        talents: enumeration(&input.talents, c, |v| observed(Some(*v), c))?,
        available_talent_points: value(&input.available_talent_points, c, |v| Ok(*v))?,
        professions: enumeration(&input.professions, c, |v| {
            Ok(ProfessionState {
                skill: observed(v.skill, c)?,
                maximum_skill: observed(v.maximum_skill, c)?,
                rank: observed(v.rank.clone(), c)?,
            })
        })?,
        recipes: enumeration(&input.recipes, c, |v| observed(Some(*v), c))?,
        bags: inventory(&input.bags)?,
        bank: inventory(&input.bank)?,
        equipment_enhancements: RecordMap::new(
            input
                .equipment_enhancements
                .iter()
                .map(|(slot, v)| {
                    Ok((
                        slot.clone(),
                        value(v, c, |v| {
                            Ok(EquipmentEnhancements {
                                item: v.item.clone(),
                                enhancements: enumeration(&v.enhancements, c, |e| {
                                    Ok(ObservedEnhancement {
                                        duration: observed(e.duration, c)?,
                                        expires_at_utc_ms: observed(e.expires_at_utc_ms, c)?,
                                    })
                                })?,
                            })
                        })?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?,
        )?,
    })
}
