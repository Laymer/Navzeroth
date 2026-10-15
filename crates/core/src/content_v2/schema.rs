//! Structural v2 schema; Rust graph validation remains authoritative.
use super::ReferenceContent;
use schemars::generate::SchemaSettings;
use serde_json::{Value, json};
pub const SCHEMA_FILE: &str = "reference-content.schema.json";
pub fn reference_content_schema() -> Value {
    let generator = SchemaSettings::draft2020_12()
        .for_deserialize()
        .into_generator();
    let mut value = generator
        .into_root_schema_for::<ReferenceContent>()
        .to_value();
    value["$id"] =
        json!("https://github.com/Laymer/Navzeroth/schemas/v2/reference-content.schema.json");
    value["description"] = json!(
        "Strict reference-content v2 structure. Rust validation checks bounded complete references, provenance and semantics; structural validity is not a rights grant or verified game fact."
    );
    value["properties"]["format"] = json!({"const":"navzeroth-reference-content"});
    value["properties"]["schema_version"] = json!({"const":2});
    close_records(&mut value);
    value
}
fn close_records(value: &mut Value) {
    match value {
        Value::Object(object) => {
            if object.get("type").and_then(Value::as_str) == Some("object")
                && object.contains_key("properties")
            {
                // Dynamic RecordMap types have schema-valued additionalProperties;
                // fixed records, including legacy envelope extensions, are closed.
                object.insert("additionalProperties".into(), Value::Bool(false));
            }
            if object
                .get("format")
                .and_then(Value::as_str)
                .is_some_and(|f| {
                    matches!(
                        f,
                        "int8"
                            | "int16"
                            | "int32"
                            | "int64"
                            | "uint8"
                            | "uint16"
                            | "uint32"
                            | "uint64"
                            | "float"
                            | "double"
                    )
                })
            {
                object.remove("format");
            }
            for child in object.values_mut() {
                close_records(child);
            }
        }
        Value::Array(values) => {
            for child in values {
                close_records(child);
            }
        }
        _ => {}
    }
}
pub fn reference_content_schema_text() -> String {
    let mut text =
        serde_json::to_string_pretty(&reference_content_schema()).expect("schema serializes");
    text.push('\n');
    text
}
