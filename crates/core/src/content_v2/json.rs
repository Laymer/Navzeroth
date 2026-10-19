// Strict v2 content subset of existing bounded pack JSON rules.
use serde_json::Value;
type Res<T> = std::result::Result<T, &'static str>;
fn known_fields(input: &Value, typed: &Value) -> Res<()> {
    match (input, typed) {
        (Value::Object(a), Value::Object(b)) => {
            for (key, value) in a {
                let expected = b
                    .get(key)
                    .ok_or("Untyped or noncanonical optional fields are unsupported in packs")?;
                known_fields(value, expected)?;
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (a, b) in a.iter().zip(b) {
                known_fields(a, b)?;
            }
        }
        _ => {}
    }
    Ok(())
}
pub(super) fn typed(bytes: &[u8]) -> Res<super::ReferenceContent> {
    let value = crate::strict_json::value(bytes, super::MAX_CONTENT_BYTES)?;
    let parsed: super::ReferenceContent =
        serde_json::from_value(value.clone()).map_err(|_| "Invalid typed content")?;
    let mut comparison = serde_json::to_value(&parsed).map_err(|_| "Invalid typed content")?;
    for key in ["node_facts", "entities"] {
        if value
            .get(key)
            .is_some_and(|v| v.as_array().is_some_and(Vec::is_empty))
        {
            comparison[key] = Value::Array(vec![]);
        }
    }
    for key in ["map_images", "map_embeddings"] {
        if value
            .pointer(&format!("/reference/payload/{key}"))
            .is_some_and(|v| v.as_array().is_some_and(Vec::is_empty))
        {
            comparison["reference"]["payload"]
                .as_object_mut()
                .ok_or("Invalid comparison")?
                .insert(key.into(), Value::Array(vec![]));
        }
    }
    if parsed.reference.dependencies.is_empty() && value["reference"].get("dependencies").is_some()
    {
        comparison["reference"]["dependencies"] =
            serde_json::to_value(&parsed.reference.dependencies)
                .map_err(|_| "Invalid dependencies")?;
    }
    known_fields(&value, &comparison)?;
    Ok(parsed)
}
