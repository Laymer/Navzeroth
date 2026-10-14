//! Bounded synthetic application-service replay. No imported Lua is executed.
use azeroth_app::{
    joint_planning::{JointEstimate, JointParticipantInput, JointPlanningRequest},
    planning::{CandidateEstimate, PlanningRequest},
    snapshots::SnapshotApplication,
};
use azeroth_core::{
    contract::{Envelope, MAX_DATASET_BYTES, MAX_ENVELOPE_BYTES},
    entities::CharacterSnapshot,
    observation::Freshness,
    observation::ObservationStatus,
    sources::Redistribution,
    types::*,
};
use azeroth_storage::content::{ContentRevision, MAX_CONTENT_REVISION_BYTES};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Component, Path},
};
pub const MAX_FIXTURE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
type Res<T> = Result<T, &'static str>;
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    format: String,
    schema_version: u32,
    now_utc_ms: UtcMillis,
    snapshots: Vec<Envelope<CharacterSnapshot>>,
    content: ContentRevision,
    request: Request,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Request {
    mode: Mode,
    participants: Vec<usize>,
    goal_profile: SourceId,
    origin: NodeId,
    stopping_node: NodeId,
    budget_ms: Millis,
    stopping_service_ms: Millis,
    candidates: Vec<Estimate>,
    excluded: Vec<ObjectiveId>,
}
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Solo,
    Joint,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Estimate {
    objective: ObjectiveId,
    node: NodeId,
    utilities: Vec<u32>,
    completion_ms: Millis,
    service_ms: Millis,
}
#[derive(Serialize)]
struct PlanOutput<P, F> {
    plan: P,
    filtered: F,
    assumptions: Vec<&'static str>,
}
#[derive(Serialize)]
struct Policy {
    source_index: usize,
    redistribution_status: ObservationStatus,
    redistribution: Option<Redistribution>,
    offline_status: ObservationStatus,
    offline_usable: Option<bool>,
}
#[derive(Serialize)]
struct Output<T> {
    format: &'static str,
    schema_version: u32,
    fixture: bool,
    declared_now_utc_ms: UtcMillis,
    participants: Vec<usize>,
    mode: Mode,
    context: DataContext,
    revision: DatasetRevision,
    source_policies: Vec<Policy>,
    result: T,
}
/// Duplicate detection preserves serde_json's normal bounded recursion.
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Unique;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("unique-key JSON")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Unique, E> {
                Ok(Unique(v.into()))
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
                    .ok_or_else(|| E::custom("Invalid JSON number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Unique, E> {
                Ok(Unique(v.into()))
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
fn known(input: &Value, typed: &Value) -> Res<()> {
    match (input, typed) {
        (Value::Object(a), Value::Object(b)) => {
            for (key, value) in a {
                known(
                    value,
                    b.get(key)
                        .ok_or("Unknown or noncanonical optional replay field")?,
                )?
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (a, b) in a.iter().zip(b) {
                known(a, b)?
            }
        }
        _ => {}
    }
    Ok(())
}
struct Bounded(Vec<u8>);
impl Write for Bounded {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        if b.len() > MAX_OUTPUT_BYTES - self.0.len() {
            return Err(std::io::Error::other("Replay output exceeds limit"));
        }
        self.0.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn output<T: Serialize>(fixture: &Fixture, result: T) -> Res<Vec<u8>> {
    let mut writer = Bounded(Vec::new());
    serde_json::to_writer_pretty(
        &mut writer,
        &Output {
            format: "azeroth-synthetic-planner-report",
            schema_version: 1,
            fixture: true,
            declared_now_utc_ms: fixture.now_utc_ms,
            participants: fixture.request.participants.clone(),
            mode: fixture.request.mode,
            context: fixture.content.envelope.data_context(),
            revision: fixture.content.envelope.payload.revision.clone(),
            source_policies: fixture
                .content
                .sources
                .iter()
                .enumerate()
                .map(|(source_index, source)| Policy {
                    source_index,
                    redistribution_status: source.redistribution.status(),
                    redistribution: source.redistribution.value().copied(),
                    offline_status: source.offline_usable.status(),
                    offline_usable: source.offline_usable.value().copied(),
                })
                .collect(),
            result,
        },
    )
    .map_err(|_| "Replay output exceeds limit or cannot be serialized")?;
    writer
        .write_all(b"\n")
        .map_err(|_| "Replay output exceeds limit")?;
    Ok(writer.0)
}
fn encoded<T: Serialize>(value: &T) -> Res<String> {
    serde_json::to_string(value).map_err(|_| "Replay data cannot be serialized")
}
fn validate(f: &Fixture) -> Res<()> {
    if f.format != "azeroth-synthetic-planner-replay" || f.schema_version != 1 {
        return Err("Unsupported replay format or schema");
    }
    let count = f.snapshots.len();
    if !match f.request.mode {
        Mode::Solo => count == 1,
        Mode::Joint => (2..=40).contains(&count),
    } || f.request.participants.len() != count
    {
        return Err("Invalid replay participant count");
    }
    let indexes = f
        .request
        .participants
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if indexes != (0..count).collect() {
        return Err("Replay indexes must select the entire distinct cohort");
    }
    if f.request.candidates.len() > 128
        || f.request.excluded.len() > 128
        || f.request.excluded.iter().collect::<BTreeSet<_>>().len() != f.request.excluded.len()
    {
        return Err("Replay candidate or exclusion limit/duplicate");
    }
    if encoded(&f.content)?.len() > MAX_CONTENT_REVISION_BYTES
        || encoded(&f.content.envelope)?.len() > MAX_DATASET_BYTES
    {
        return Err("Replay content exceeds contract bounds");
    }
    if !f.content.envelope.fixture || f.content.envelope.namespace != DatasetNamespace::Demo {
        return Err("Only explicitly synthetic demo replay content is supported");
    }
    let mut characters = BTreeSet::new();
    for s in &f.snapshots {
        if !s.fixture
            || s.namespace != DatasetNamespace::Demo
            || s.data_context() != f.content.envelope.data_context()
            || !characters.insert(s.payload.character.clone())
        {
            return Err("Replay snapshot/context/character mismatch");
        }
        if encoded(s)?.len() > MAX_ENVELOPE_BYTES {
            return Err("Replay snapshot exceeds contract bounds");
        }
    }
    if f.request
        .candidates
        .iter()
        .any(|c| c.utilities.len() != count)
    {
        return Err("Replay utility vector differs from participant cohort");
    }
    Ok(())
}
/// Returns a completely serialized bounded result before the caller prints anything.
pub fn replay(bytes: &[u8]) -> Res<Vec<u8>> {
    if bytes.len() > MAX_FIXTURE_BYTES {
        return Err("Replay fixture exceeds size limit");
    }
    let raw = serde_json::from_slice::<Unique>(bytes)
        .map_err(|_| "Invalid or duplicate-key replay JSON")?
        .0;
    let fixture: Fixture =
        serde_json::from_value(raw.clone()).map_err(|_| "Invalid typed replay fixture")?;
    known(
        &raw,
        &serde_json::to_value(&fixture).map_err(|_| "Invalid replay fixture")?,
    )?;
    validate(&fixture)?;
    let app = SnapshotApplication::in_memory().map_err(|_| "Replay store cannot be initialized")?;
    let mut profiles = Vec::new();
    for s in &fixture.snapshots {
        let profile = app
            .import_text(&format!("ANAV1:{}", encoded(s)?), Some(fixture.now_utc_ms))
            .map_err(|_| "Replay snapshot import failed")?
            .profile_id;
        let current = app
            .current(&profile, Some(fixture.now_utc_ms))
            .map_err(|_| "Replay capture inspection failed")?
            .ok_or("Replay has no current capture")?;
        match current.freshness {
            Freshness::Fresh { age_ms } if age_ms.get() <= 10_000 => {}
            _ => {
                return Err(
                    "Replay capture is stale, uncertain or outside the ten-second synthetic window",
                );
            }
        }
        profiles.push(profile);
    }
    app.stage_content(&encoded(&fixture.content)?)
        .map_err(|_| "Replay content staging failed")?;
    for profile in &profiles {
        app.activate_content(profile, &fixture.content.envelope.payload.revision)
            .map_err(|_| "Replay content activation failed")?;
    }
    let r = &fixture.request;
    let revision = fixture.content.envelope.payload.revision.clone();
    match r.mode {
        Mode::Solo => {
            let actor = r.participants[0];
            let request = PlanningRequest {
                profile_id: profiles[actor].clone(),
                snapshot_id: fixture.snapshots[actor].payload.id.clone(),
                revision,
                goal_profile: r.goal_profile.clone(),
                origin: r.origin.clone(),
                stopping_node: r.stopping_node.clone(),
                budget_ms: r.budget_ms,
                stopping_service_ms: r.stopping_service_ms,
                candidates: r
                    .candidates
                    .iter()
                    .map(|c| CandidateEstimate {
                        objective: c.objective.clone(),
                        node: c.node.clone(),
                        utility: c.utilities[0],
                        completion_ms: c.completion_ms,
                        service_ms: c.service_ms,
                    })
                    .collect(),
                excluded: r.excluded.iter().cloned().collect(),
            };
            let report = app
                .plan_session(&encoded(&request)?, Some(fixture.now_utc_ms))
                .map_err(|_| "Replay solo planning failed")?;
            output(
                &fixture,
                PlanOutput {
                    plan: report.result,
                    filtered: report.filtered,
                    assumptions: report.assumptions,
                },
            )
        }
        Mode::Joint => {
            let party = PartyId::new("synthetic-replay-party").expect("constant");
            let participants = r
                .participants
                .iter()
                .map(|&i| profiles[i].clone())
                .collect::<Vec<_>>();
            app.save_party(&party, &participants)
                .map_err(|_| "Replay party creation failed")?;
            let request = JointPlanningRequest {
                party_id: party,
                participants: r
                    .participants
                    .iter()
                    .map(|&i| JointParticipantInput {
                        profile_id: profiles[i].clone(),
                        snapshot_id: fixture.snapshots[i].payload.id.clone(),
                    })
                    .collect(),
                revision,
                goal_profile: r.goal_profile.clone(),
                origin: r.origin.clone(),
                stopping_node: r.stopping_node.clone(),
                budget_ms: r.budget_ms,
                stopping_service_ms: r.stopping_service_ms,
                candidates: r
                    .candidates
                    .iter()
                    .map(|c| JointEstimate {
                        objective: c.objective.clone(),
                        node: c.node.clone(),
                        utilities: r
                            .participants
                            .iter()
                            .enumerate()
                            .map(|(utility, &actor)| {
                                (profiles[actor].clone(), c.utilities[utility])
                            })
                            .collect::<BTreeMap<_, _>>(),
                        completion_ms: c.completion_ms,
                        service_ms: c.service_ms,
                    })
                    .collect(),
                excluded: r.excluded.iter().cloned().collect(),
            };
            let report = app
                .plan_joint_session(&encoded(&request)?, Some(fixture.now_utc_ms))
                .map_err(|_| "Replay joint planning failed")?;
            output(
                &fixture,
                PlanOutput {
                    plan: report.result,
                    filtered: report.filtered,
                    assumptions: report.assumptions,
                },
            )
        }
    }
}
/// One explicit regular file; refuse symlink/reparse/parent-traversal components.
pub fn file(path: &Path) -> Res<Vec<u8>> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|_| "Replay current directory unavailable")?
            .join(path)
    };
    let mut prefix = std::path::PathBuf::new();
    for part in absolute.components() {
        if matches!(part, Component::ParentDir) {
            return Err("Replay parent traversal rejected");
        }
        prefix.push(part.as_os_str());
        let meta = fs::symlink_metadata(&prefix).map_err(|_| "Replay file cannot be inspected")?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err("Replay reparse path rejected");
            }
        }
        if meta.file_type().is_symlink() {
            return Err("Replay symlink path rejected");
        }
        if prefix != absolute && !meta.is_dir() {
            return Err("Replay ancestor is not a directory");
        }
    }
    let meta = fs::symlink_metadata(path).map_err(|_| "Replay file cannot be inspected")?;
    if !meta.is_file() || meta.len() > MAX_FIXTURE_BYTES as u64 {
        return Err("Replay input must be a bounded regular file");
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| "Replay input cannot be read")?
        .take(MAX_FIXTURE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Replay input cannot be read")?;
    replay(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_writer_never_extends_beyond_cap() {
        let mut writer = Bounded(Vec::new());
        writer.write_all(&vec![b'x'; MAX_OUTPUT_BYTES]).unwrap();
        assert!(writer.write_all(b"x").is_err());
        assert_eq!(writer.0.len(), MAX_OUTPUT_BYTES);
        let mut writer = Bounded(Vec::new());
        assert!(serde_json::to_writer(&mut writer, &"x".repeat(MAX_OUTPUT_BYTES)).is_err());
        assert!(writer.0.len() <= MAX_OUTPUT_BYTES);
    }
}
