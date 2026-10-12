//! Pure JSON Schema generation for the version 1 exchange contracts.
use super::{EnvelopeData, EvidenceBundle, GearRecommendationSet, ReferenceDataset};
use crate::entities::{CharacterSnapshot, PartySnapshot, RoutePlan};
use schemars::generate::SchemaSettings;
use serde_json::{Value, json};

pub const SCHEMA_ID: &str = "https://github.com/Laymer/Navzeroth/schemas/v1/exchange.schema.json";

/// The complete exchange schema: a root `oneOf` over the six envelope kinds
/// with shared `$defs`. Output is deterministic (sorted definitions).
/// Rust validation remains authoritative for semantic invariants.
pub fn exchange_schema() -> Value {
    let mut generator = SchemaSettings::draft2020_12()
        .for_deserialize()
        .into_generator();
    let roots = [
        generator.subschema_for::<EnvelopeData<CharacterSnapshot>>(),
        generator.subschema_for::<EnvelopeData<PartySnapshot>>(),
        generator.subschema_for::<EnvelopeData<RoutePlan>>(),
        generator.subschema_for::<EnvelopeData<EvidenceBundle>>(),
        generator.subschema_for::<EnvelopeData<ReferenceDataset>>(),
        generator.subschema_for::<EnvelopeData<GearRecommendationSet>>(),
    ];
    let defs: std::collections::BTreeMap<String, Value> =
        generator.take_definitions(true).into_iter().collect();
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": SCHEMA_ID,
        "title": "AzerothNavigatorExchangeV1",
        "description": "Azeroth Navigator exchange envelopes, schema version 1. Structural only: Rust validation enforces semantic invariants, and structural validity never establishes in-game truth.",
        "oneOf": roots.into_iter().map(|s| s.to_value()).collect::<Vec<_>>(),
        "$defs": defs,
    })
}

/// File name of the committed schema within `schemas/v1/`.
pub const SCHEMA_FILE: &str = "exchange.schema.json";

/// The exact committed text: two-space pretty JSON with a trailing newline.
pub fn exchange_schema_text() -> String {
    let mut text = serde_json::to_string_pretty(&exchange_schema()).expect("schema serializes");
    text.push('\n');
    text
}
