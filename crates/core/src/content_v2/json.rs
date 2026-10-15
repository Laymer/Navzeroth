// Strict v2 content subset of existing bounded pack JSON rules.
use serde::Deserialize;
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
/// Refuse duplicate JSON members before typed decoding (including nested maps).
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Unique;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("unique-key JSON data")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Unique, E> {
                Ok(Unique(Value::Bool(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Unique, E> {
                serde_json::Number::from_f64(v)
                    .map(|v| Unique(Value::Number(v)))
                    .ok_or_else(|| E::custom("Invalid number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Unique, E> {
                Ok(Unique(Value::String(v.to_owned())))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Unique, E> {
                Ok(Unique(Value::String(v)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut values = Vec::new();
                while let Some(v) = a.next_element::<Unique>()? {
                    values.push(v.0);
                }
                Ok(Unique(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<Unique, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = a.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom("Duplicate JSON member"));
                    }
                    values.insert(key, a.next_value::<Unique>()?.0);
                }
                Ok(Unique(Value::Object(values)))
            }
        }
        d.deserialize_any(Visitor)
    }
}
fn json(bytes: &[u8]) -> Res<Value> {
    serde_json::from_slice::<Unique>(bytes)
        .map(|v| v.0)
        .map_err(|_| "Invalid or duplicate-key pack JSON")
}
pub(super) fn typed(bytes: &[u8]) -> Res<super::ReferenceContent> {
    let value = json(bytes)?;
    let parsed: super::ReferenceContent =
        serde_json::from_value(value.clone()).map_err(|_| "Invalid typed content")?;
    let mut comparison = serde_json::to_value(&parsed).map_err(|_| "Invalid typed content")?;
    if parsed.node_facts.is_empty() && value.get("node_facts").is_some() {
        comparison["node_facts"] = Value::Array(vec![]);
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
