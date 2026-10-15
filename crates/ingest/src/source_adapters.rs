//! Narrow inert transformations for reviewed open-source inputs. These describe
//! source declarations, not server behaviour or complete target inventories.
use crate::savedvars::{LuaKey, LuaValue, source_literal_prefix};
use azeroth_core::{
    entities::Confidence,
    source_pipeline::{Datum, Fact},
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
type Result<T> = std::result::Result<T, &'static str>;
fn fact(domain: &str, record: &str, field: &str, datum: Datum) -> Fact {
    Fact {
        domain: domain.into(),
        record: record.into(),
        field: field.into(),
        datum,
        confidence: Confidence::CrossEditionUnverified,
        observed_at_utc_ms: None,
        clock_trust: azeroth_core::observation::ClockTrust::Uncertain,
    }
}
fn known(value: serde_json::Value) -> Datum {
    Datum::Known { value }
}
/// This initial transformation is deliberately scoped to the reviewed file.
/// It extracts declared legacy IDs only; conditional loot/runtime expressions
/// are not interpreted. New revisions require a fresh adapter/schema review.
pub fn atlasloot_areas(bytes: &[u8]) -> Result<Vec<Fact>> {
    if format!("{:x}", Sha256::digest(bytes))
        != "ab12de3f642620ff52e5bb0c47139e4146e3c7021be99d55af76e5eae7bda557"
    {
        return Err("AtlasLoot adapter does not support this source revision");
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "AtlasLoot source must be UTF-8")?;
    let mut facts = extract_areas(text)?;
    facts.extend(extract_normal_loot(text)?);
    Ok(facts)
}
fn extract_areas(text: &str) -> Result<Vec<Fact>> {
    let mut result = Vec::new();
    let mut ids = BTreeSet::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let Some(rest) = line.strip_prefix("data[\"") else {
            continue;
        };
        let (name, suffix) = rest
            .split_once("\"]")
            .ok_or("Malformed AtlasLoot record declaration")?;
        if name.is_empty()
            || name.len() > 128
            || !suffix.trim_start().starts_with("= {")
            || !ids.insert(name)
        {
            return Err("Unsupported AtlasLoot record declaration");
        }
        let mut header = Vec::new();
        for line in lines.by_ref().take(64) {
            let line = line.split("--").next().unwrap_or("").trim();
            if line.starts_with("items = {") {
                break;
            }
            header.push(line);
        }
        for (key, field) in [
            ("MapID", "declared_legacy_area_id"),
            ("InstanceID", "declared_legacy_instance_id"),
        ] {
            let prefix = format!("{key} = ");
            let datum = match header.iter().find_map(|line| line.strip_prefix(&prefix)) {
                None => Datum::Unknown,
                Some(value) => match value
                    .strip_suffix(',')
                    .and_then(|v| v.trim().parse::<u32>().ok())
                    .filter(|v| *v > 0)
                {
                    Some(id) => known(json!(id)),
                    None => Datum::Unsupported,
                },
            };
            result.push(fact("dungeons", name, field, datum));
        }
    }
    if result.is_empty() {
        return Err("AtlasLoot source contains no supported declarations");
    }
    Ok(result)
}
// The hash gate above binds the exact reviewed layout. These fields record
// declarations, not runtime loot entitlement, availability or drop probability.
fn extract_normal_loot(text: &str) -> Result<Vec<Fact>> {
    let mut result = Vec::new();
    let mut group: Option<&str> = None;
    let mut npc = Datum::Unknown;
    let mut ordinal = 0;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if let Some(rest) = line.strip_prefix("data[\"") {
            group = rest.split_once("\"]").map(|(name, _)| name);
            ordinal = 0;
            npc = Datum::Unknown;
        }
        if line.starts_with("\t\t{") {
            npc = Datum::Unknown;
        }
        if let Some(raw) = line.strip_prefix("\t\t\tnpcID = ") {
            let raw = raw.split("--").next().unwrap_or("").trim();
            npc = match raw
                .strip_suffix(',')
                .and_then(|v| v.trim().parse::<u32>().ok())
                .filter(|v| *v > 0)
            {
                Some(id) => known(json!(id)),
                None => Datum::Unsupported,
            };
        }
        if let Some(name) = group
            && let Some(start) = line
                .find("[NORMAL_DIFF] = {")
                .filter(|start| line[..*start].trim().is_empty())
        {
            ordinal += 1;
            let record = format!("{name}/normal/{ordinal}");
            let start = offset + start + "[NORMAL_DIFF] = ".len();
            let items = match source_literal_prefix(&text[start..]) {
                Ok((LuaValue::Table(rows), _)) => {
                    let mut items = BTreeSet::new();
                    for row in rows.values() {
                        if let LuaValue::Table(row) = row
                            && let Some(LuaValue::Int(id)) = row.get(&LuaKey::Int(2))
                            && *id > 0
                        {
                            items.insert(*id);
                        }
                    }
                    known(json!(items))
                }
                _ => Datum::Unsupported,
            };
            result.push(fact("acquisition", &record, "declared_npc_id", npc.clone()));
            result.push(fact(
                "acquisition",
                &record,
                "declared_normal_item_ids",
                items,
            ));
        }
        offset += line.len();
    }
    Ok(result)
}
fn skip(mut rest: &str) -> Result<&str> {
    loop {
        rest = rest.trim_start();
        if let Some(comment) = rest.strip_prefix("--") {
            if comment.starts_with('[') {
                return Err("Long Lua comments are unsupported");
            }
            rest = comment.split_once('\n').map_or("", |(_, s)| s);
        } else {
            return Ok(rest);
        }
    }
}
fn token(rest: &mut &str, expected: &str) -> Result<()> {
    *rest = skip(rest)?
        .strip_prefix(expected)
        .ok_or("Unsupported source wrapper")?;
    Ok(())
}
fn count(v: &LuaValue) -> usize {
    1 + match v {
        LuaValue::Table(t) => t.values().map(count).sum(),
        _ => 0,
    }
}
pub fn mapzeroth_nodes(bytes: &[u8]) -> Result<Vec<Fact>> {
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Mapzeroth source exceeds byte bound");
    }
    let mut rest = std::str::from_utf8(bytes).map_err(|_| "Mapzeroth source must be UTF-8")?;
    for t in [
        "local", "_", ",", "ns", "=", "...", "ns.Nodes", "=", "ns.Nodes", "or", "{}",
    ] {
        token(&mut rest, t)?;
    }
    let mut groups = BTreeSet::new();
    let mut result = Vec::new();
    let mut total = 0;
    loop {
        rest = skip(rest)?;
        if rest.is_empty() {
            break;
        }
        token(&mut rest, "ns.Nodes.")?;
        let len = rest
            .bytes()
            .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == b'_')
            .count();
        if len == 0 || len > 64 {
            return Err("Invalid Mapzeroth source group");
        }
        let group = rest[..len].to_owned();
        rest = &rest[len..];
        if !groups.insert(group.clone()) || groups.len() > 64 {
            return Err("Duplicate or excessive Mapzeroth groups");
        }
        token(&mut rest, "=")?;
        let (value, used) =
            source_literal_prefix(rest).map_err(|_| "Unsupported Mapzeroth literal data")?;
        rest = &rest[used..];
        total += count(&value);
        if total > 1_000_000 {
            return Err("Mapzeroth source exceeds aggregate value bound");
        }
        let LuaValue::Table(nodes) = value else {
            return Err("Mapzeroth group must be a literal table");
        };
        for (name, node) in nodes {
            let (LuaKey::Str(name), LuaValue::Table(node)) = (name, node) else {
                return Err("Invalid Mapzeroth node identity");
            };
            if node.keys().any(|key|!matches!(key,LuaKey::Str(key) if ["name","mapID","mapArtID","x","y","interior","category"].contains(&key.as_str()))){return Err("Unknown Mapzeroth node field");}
            let record = format!("{group}/{name}");
            for (field, output) in [
                ("mapID", "map_ui_id"),
                ("x", "normalized_x"),
                ("y", "normalized_y"),
                ("interior", "interior"),
            ] {
                let value = match node.get(&LuaKey::Str(field.into())) {
                    None => Datum::Unknown,
                    Some(LuaValue::Int(v)) if field == "mapID" && *v > 0 => known(json!(v)),
                    Some(LuaValue::Int(v)) if ["x", "y"].contains(&field) && [0, 1].contains(v) => {
                        known(json!(v))
                    }
                    Some(LuaValue::Float(v))
                        if ["x", "y"].contains(&field) && (0.0..=1.0).contains(v) =>
                    {
                        known(json!(v))
                    }
                    Some(LuaValue::Bool(v)) if field == "interior" => known(json!(v)),
                    _ => return Err("Mapzeroth selected field has unsupported type or range"),
                };
                result.push(fact("geography", &record, output, value));
                if result.len() > 250_000 {
                    return Err("Mapzeroth source exceeds fact bound");
                }
            }
        }
    }
    if groups.is_empty() {
        return Err("Mapzeroth source is empty");
    }
    Ok(result)
}
/// Retain the openly licensed implementation as source evidence, without
/// pretending that it has already produced validated Forever model coefficients.
pub fn model_reference(path: &str, sha256: &str) -> Fact {
    let mut row = fact(
        "class_roles",
        path,
        "model_source",
        known(
            json!({"path":path,"sha256":sha256,"model_coefficients":"not_derived","target_validation":"unverified"}),
        ),
    );
    row.confidence = Confidence::Inferred;
    row
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reviewed_declarations_never_evaluate_instance_expressions() {
        let rows =
            extract_areas("data[\"sample\"] = {\nMapID = 1,\nInstanceID = dangerous(),\n").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].datum, Datum::Unsupported);
        assert!(atlasloot_areas(b"arbitrary source").is_err());
        assert_eq!(
            extract_areas("data[\"sample\"] = {\nMapID = f(),\n").unwrap()[0].datum,
            Datum::Unsupported
        );
    }
    #[test]
    fn literal_nodes_preserve_zero_false_and_unknown() {
        let rows=mapzeroth_nodes(b"local _, ns = ...\nns.Nodes = ns.Nodes or {}\nns.Nodes.GROUP = { A = {name='x',mapID=84,x=0,y=0.5,interior=false}, B={mapID=1} }").unwrap();
        assert_eq!(rows[1].datum, known(json!(0)));
        assert_eq!(rows[3].datum, known(json!(false)));
        assert_eq!(rows[5].datum, Datum::Unknown);
        assert!(
            mapzeroth_nodes(b"local _, ns = ... ns.Nodes = ns.Nodes or {} ns.Nodes.G = {A=f()}")
                .is_err()
        );
        assert!(
            mapzeroth_nodes(b"local _, ns = ... ns.Nodes = ns.Nodes or {} ns.Nodes.G = {} evil()")
                .is_err()
        );
    }
}

#[cfg(test)]
mod loot_tests {
    use super::*;
    #[test]
    fn literal_loot_declarations_keep_ids_and_do_not_evaluate_calls_or_comments() {
        let text = "data[\"Synthetic\"] = {\n\t\t{ -- boss\n\t\t\tnpcID = 42,\n\t\t\t[NORMAL_DIFF] = {{1, 10}, {2, 11}, {3, 10}},\n\t\t},\n\t\t{ -- unknown\n\t\t\t[NORMAL_DIFF] = {{1, danger()}},\n-- [NORMAL_DIFF] = {{1, 99}},\n";
        let rows = extract_normal_loot(text).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].datum, known(json!(42)));
        assert_eq!(rows[1].datum, known(json!([10, 11])));
        assert_eq!(rows[2].datum, Datum::Unknown);
        assert_eq!(rows[3].datum, Datum::Unsupported);
    }
}
