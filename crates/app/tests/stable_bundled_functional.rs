//! Explicit actual-pack rehearsal with disclosed manual engineering character inputs.
//! This proves app behavior over shipped sources, not live-game correctness.
use azeroth_app::{authored_scores::GearComparison, snapshots::SnapshotApplication};
use azeroth_core::types::UtcMillis;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
const NOW: i64 = 1100;
fn known(value: Value) -> Value {
    json!({"status":"known","value":value,"clock_trust":"trusted","source":"manual_entry","observed_at_utc_ms":NOW-100})
}
fn snapshot() -> Value {
    let mut value: Value =
        serde_json::from_str(include_str!("support/stable_engineering_snapshot.json")).unwrap();
    fn contexts(v: &mut Value) {
        match v {
            Value::Object(map) => {
                if map.contains_key("namespace") {
                    map.insert("namespace".into(), json!("reference"));
                }
                if map.contains_key("edition") && map.contains_key("build") {
                    map.insert("build".into(), json!("1.60.1.70450"));
                    map.insert("interface".into(), json!("16001"));
                }
                for v in map.values_mut() {
                    contexts(v);
                }
            }
            Value::Array(a) => {
                for v in a {
                    contexts(v);
                }
            }
            _ => (),
        }
    }
    contexts(&mut value);
    value["fixture"] = json!(false);
    value["payload"]["level"] = known(json!(60));
    value["payload"]["role"] = json!({"status":"unknown"});
    value["payload"]["quests"] = json!({});
    value["payload"]["objectives"] = json!({});
    value
}
fn import(app: &SnapshotApplication, value: &mut Value, sequence: u64) -> String {
    value["producer"]["sequence"] = json!(sequence);
    value["payload"]["capture"]["sequence"] = json!(sequence);
    value["payload"]["id"] = json!(format!("actual-pack-synthetic-{sequence}"));
    app.import_text(
        &format!("ANAV1:{value}"),
        Some(UtcMillis::new(NOW).unwrap()),
    )
    .unwrap()
    .profile_id
}
fn engineering_profile(
    app: &SnapshotApplication,
    label: &str,
    level: u64,
    role: Value,
    sequence: u64,
) -> (String, Value) {
    let mut snap = snapshot();
    snap["payload"]["character"] = json!(label);
    snap["producer"]["session_id"] = json!(format!("session-{label}"));
    snap["payload"]["capture"]["session_id"] = json!(format!("session-{label}"));
    snap["payload"]["level"] = known(json!(level));
    snap["payload"]["role"] = role;
    let profile = import(app, &mut snap, sequence);
    (profile, snap["payload"]["id"].clone())
}

fn level_only(v: &Value) -> bool {
    match v["op"].as_str() {
        Some("level_at_least") => v["level"].as_u64().is_some_and(|n| n <= 60),
        Some("all") => v["predicates"]
            .as_array()
            .is_some_and(|a| a.iter().all(level_only)),
        _ => false,
    }
}
#[test]
#[ignore = "Explicit full shipped packs required; synthetic character facts are disclosed"]
fn actual_packs_support_source_backed_plans_roles_and_dungeon_guidance() {
    let root = PathBuf::from(std::env::var("NAVZEROTH_STABLE_BUNDLE_ROOT").unwrap());
    let app = SnapshotApplication::in_memory().unwrap();
    let mut snap = snapshot();
    let profile = import(&app, &mut snap, 100);
    // Pre-import boundary profiles: request-only model changes should not repeatedly
    // invalidate the immutable catalog through unrelated snapshot writes.
    let mut level_profiles = BTreeMap::new();
    let mut sequence = 200;
    for level in [1, 20, 21, 40, 41, 60, 61] {
        sequence += 1;
        level_profiles.insert(
            level,
            engineering_profile(
                &app,
                &format!("engineering-level-{level}"),
                level,
                json!({"status":"unknown"}),
                sequence,
            ),
        );
    }
    let mut reminder_profiles = BTreeMap::new();
    for role in ["tank", "healer", "melee_damage", "ranged_damage"] {
        for level in [1, 20, 21, 40, 41, 60] {
            sequence += 1;
            reminder_profiles.insert(
                (role, level),
                engineering_profile(
                    &app,
                    &format!("engineering-{role}-{level}"),
                    level,
                    known(json!(role)),
                    sequence,
                ),
            );
        }
    }
    let mut revisions = Vec::new();
    for name in ["world-v2", "authored-v2"] {
        let inspection = app
            .stage_verified_content_bundle(&root.join("content/release-packs").join(name))
            .unwrap();
        revisions.push(inspection.report.dataset.unwrap().revision);
        eprintln!("verified and staged {name}");
    }
    let active = app
        .activate_content_set(&profile, &revisions, true)
        .unwrap();
    let revision = active.key.revision;
    eprintln!("activated actual content set {:?}", revision);
    let catalog = app.planning_catalog(&profile).unwrap();
    eprintln!(
        "loaded planner page with {} objectives",
        catalog.objectives.len()
    );
    assert!(!catalog.fixture);
    let positive = catalog
        .objectives
        .iter()
        .find(|o| {
            o.requirements
                .value()
                .is_some_and(|p| level_only(&serde_json::to_value(p).unwrap()))
        })
        .expect("actual first planner page contains an unrestricted source-backed quest stage")
        .clone();
    let unresolved = catalog
        .objectives
        .iter()
        .find(|o| o.requirements.value().is_none())
        .expect("actual unresolved eligibility remains explicit")
        .clone();
    assert!(positive.expected_ms.value().is_none());
    assert!(!positive.evidence.is_empty());
    for objective in [&positive, &unresolved] {
        let value = serde_json::to_value(objective).unwrap();
        let target = &value["targets"][0];
        let quest = target["quest"].as_str().unwrap();
        let stage = target["stage"].as_str().unwrap();
        snap["payload"]["quests"][quest] = json!({"quest":quest,"context":snap["payload"]["context"],"accepted":known(json!(true)),"completed":known(json!(false)),"turned_in":known(json!(false)),"objectives":{stage:known(json!(false))}});
        snap["payload"]["objectives"][stage] = known(json!(false));
    }
    assert_eq!(import(&app, &mut snap, 101), profile);
    let catalog = app.planning_catalog(&profile).unwrap();
    let node = &catalog.nodes[0].id;
    let candidates:Vec<_>=[&positive,&unresolved].iter().map(|o|json!({"objective":o.id,"node":node,"utility":10,"completion_ms":1000,"service_ms":0})).collect();
    let request = json!({"profile_id":profile,"snapshot_id":catalog.snapshot_id,"revision":revision,"goal_profile":"manual-v1","origin":node,"stopping_node":node,"budget_ms":100000,"stopping_service_ms":0,"candidates":candidates,"excluded":[]});
    let plan = app
        .plan_session(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap();
    assert_eq!(plan.result.selected.len(), 1);
    assert_eq!(plan.filtered.len(), 1);
    assert_eq!(plan.filtered[0].objective, unresolved.id);
    let export = app
        .export_plan(&request.to_string(), Some(UtcMillis::new(NOW).unwrap()))
        .unwrap();
    assert!(export.text.contains("planner-quest-stage-projection-v2"));
    let authored: Value = serde_json::from_slice(
        &std::fs::read(root.join("content/authored/slice-v1/catalog.json")).unwrap(),
    )
    .unwrap();
    let models = authored["models"].as_array().unwrap();
    assert_eq!(models.len(), 54);
    let classes: BTreeSet<_> = models
        .iter()
        .map(|m| m["class"].as_str().unwrap())
        .collect();
    assert_eq!(classes.len(), 9);
    let item = app
        .gear_catalog(&profile)
        .unwrap()
        .items
        .iter()
        .find(|i| {
            serde_json::to_value(&i.attributes).unwrap()["value"]["stat:stamina"]
                .as_f64()
                .is_some_and(|n| n > 0.0)
        })
        .expect("actual first gear page includes a source-backed positive stamina contribution")
        .key
        .clone();
    let mut score_boundary_cases = 0;
    for model in models {
        for boundary in ["minimum_level", "maximum_level"] {
            let (score_profile, score_snapshot) =
                &level_profiles[&model[boundary].as_u64().unwrap()];
            let mut score = json!({"profile_id":score_profile,"snapshot_id":score_snapshot,"revision":revision,"authored_model":model["id"],"model":model["profile"],"layout":{"slots":[],"hands":null},"class":known(model["class"].clone()),"proficiency":{"classes":{}},"candidates":[{"item":item,"category":{"status":"unknown"},"acquisition":null}]});
            assert!(
                app.compare_gear_or_score(&score.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                    .is_err()
            );
            score["score_role_assumption"] = model["profile"]["role"].clone();
            let GearComparison::ScoreOnly(result) = app
                .compare_gear_or_score(&score.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                .unwrap()
            else {
                panic!("only explicit heuristic arithmetic allowed")
            };
            assert!(result.imported_role.value().is_none());
            assert!(result.role_assumption.is_some());
            assert!(result.equipment_eligibility.value().is_none());
            assert!(result.acquisition_availability.value().is_none());
            assert_eq!(result.candidates.len(), 1);
            assert!(result.candidates[0].score.total > 0);
            score_boundary_cases += 1;
            assert!(
                app.compare_gear(&score.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                    .is_err()
            );
            for outside in [
                model["minimum_level"].as_u64().unwrap().saturating_sub(1),
                model["maximum_level"].as_u64().unwrap() + 1,
            ] {
                if outside == 0 {
                    continue;
                }
                let (outside_profile, outside_snapshot) = &level_profiles[&outside];
                let mut wrong_band = score.clone();
                wrong_band["profile_id"] = json!(outside_profile);
                wrong_band["snapshot_id"] = outside_snapshot.clone();
                assert!(
                    app.compare_gear_or_score(
                        &wrong_band.to_string(),
                        Some(UtcMillis::new(NOW).unwrap())
                    )
                    .is_err()
                );
            }
            score["class"] = known(json!("class:other"));
            assert!(
                app.compare_gear_or_score(&score.to_string(), Some(UtcMillis::new(NOW).unwrap()))
                    .is_err()
            );
        }
    }
    // Known roles here are explicit manual engineering inputs, never captured facts.
    let mut reminder_scenarios = 0;
    for role in ["tank", "healer", "melee_damage", "ranged_damage"] {
        for (lo, hi) in [(1, 20), (21, 40), (41, 60)] {
            for level in [lo, hi] {
                let (reminder_profile, _) = &reminder_profiles[&(role, level)];
                let view = app
                    .reminder_view(reminder_profile, Some(UtcMillis::new(NOW).unwrap()))
                    .unwrap();
                let v = serde_json::to_value(&view).unwrap();
                assert_eq!(v["rules"].as_array().unwrap().len(), 18);
                assert!(
                    v["rules"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|r| r["delivery"] == "authored_heuristic")
                );
                let actual: Vec<_> = v["plan"]["queue"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|i| {
                        i["rule"].as_str().unwrap().starts_with("reminder:role-")
                            && i["kind"]["kind"] == "advice"
                    })
                    .collect();
                assert_eq!(actual.len(), 1);
                assert_eq!(actual[0]["rule"], format!("reminder:role-{role}-{lo}-{hi}"));
                assert!(
                    actual[0]["kind"]["text"]
                        .as_str()
                        .unwrap()
                        .starts_with("Authored heuristic: ")
                );
                reminder_scenarios += 1;
            }
        }
    }
    let view = serde_json::to_value(
        app.reminder_view(&level_profiles[&61].0, Some(UtcMillis::new(NOW).unwrap()))
            .unwrap(),
    )
    .unwrap();
    assert!(
        view["plan"]["queue"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["rule"] == "reminder:heuristic-level-boundary"
                && i["kind"]["kind"] == "advice")
    );
    assert!(!view["plan"]["queue"].as_array().unwrap().iter().any(|i| {
        i["rule"].as_str().unwrap().starts_with("reminder:role-") && i["kind"]["kind"] == "advice"
    }));
    let choices=app.authored_catalog(&json!({"subject":{"kind":"character","profile_id":profile},"kind":"dungeons","revision":null,"catalog_sha256":null,"offset":0}).to_string()).unwrap();
    assert_eq!(choices.total, 26);
    assert_eq!(choices.records.len(), 26);
    for dungeon in &choices.records {
        let report=app.dungeon_preparation(&json!({"schema_version":1,"subject":{"kind":"character","profile_id":profile},"package":null,"origin":null,"authored":{"revision":revision,"id":dungeon.id}}).to_string(),Some(UtcMillis::new(NOW).unwrap())).unwrap();
        let package = report.package.unwrap();
        assert!(package.entrance.observation.value().is_some());
        assert!(package.access.observation.value().is_none());
        let checks = package.checks.observation.value().unwrap();
        assert!(!checks.is_empty());
        assert!(
            checks
                .iter()
                .any(|c| c.label.as_str().contains("unverified"))
        );
        assert!(
            checks
                .iter()
                .any(|c| c.requirement.observation.value().is_none())
        );
    }
    let raw: Value =
        serde_json::from_slice(
            &std::fs::read(root.join(
                "data-generators/output/dungeons/dungeons/vanilla_1_12/1.12.1/part-0000.json",
            ))
            .unwrap(),
        )
        .unwrap();
    assert_eq!(raw["records"].as_array().unwrap().len(), 27);
    let excluded = raw["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["map_id"] == 169)
        .unwrap();
    assert!(
        excluded["excluded_reason"]
            .as_str()
            .unwrap()
            .contains("Unused/test map 169")
    );
    assert!(
        !choices
            .records
            .iter()
            .any(|r| r.label.contains("Emerald Dream"))
    );
    if let Ok(output) = std::env::var("NAVZEROTH_FUNCTIONAL_PROOF_OUTPUT") {
        let source_path = "crates/app/tests/stable_bundled_functional.rs";
        let compiled = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/stable_bundled_functional.rs"
        ));
        assert_eq!(std::fs::read(root.join(source_path)).unwrap(), compiled);
        assert!(
            std::process::Command::new("git")
                .args(["cat-file", "-e", &format!("HEAD:{source_path}")])
                .current_dir(&root)
                .status()
                .unwrap()
                .success(),
            "functional source must be tracked at HEAD"
        );
        assert!(
            std::process::Command::new("git")
                .args([
                    "diff",
                    "--quiet",
                    "HEAD",
                    "--",
                    source_path,
                    "crates/app/src",
                    "crates/core/src",
                    "crates/storage/src",
                    "crates/ingest/src"
                ])
                .current_dir(&root)
                .status()
                .unwrap()
                .success(),
            "functional source/code must be committed before proof"
        );
        let head = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(head.status.success());
        let head = String::from_utf8(head.stdout).unwrap().trim().to_owned();
        let file_ref = |path: &str| {
            let bytes = std::fs::read(root.join(path)).unwrap();
            assert!(bytes.len() < 8 * 1024 * 1024);
            json!({"path":path,"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(&bytes))})
        };
        let world_revision = revisions[0].as_str();
        let authored_revision = authored["reference"]["payload"]["revision"]
            .as_str()
            .unwrap();
        let mut quest_ids = Vec::new();
        for o in [&positive, &unresolved] {
            let v = serde_json::to_value(o).unwrap();
            let t = &v["targets"][0];
            quest_ids.push(json!([world_revision, "quests", t["quest"]]).to_string());
            quest_ids
                .push(json!([world_revision, "quest_stages", t["quest"], t["stage"]]).to_string());
        }
        quest_ids.sort();
        quest_ids.dedup();
        let mut dungeon_ids: Vec<_> = choices
            .records
            .iter()
            .map(|d| json!([world_revision, "dungeons", d.id]).to_string())
            .collect();
        dungeon_ids
            .push(json!([authored_revision, "dungeons", "dungeon:the-deadmines"]).to_string());
        dungeon_ids.sort();
        let units = json!([
            {"domain":"quests","scenario":"source_quest_projection_v1","scope":"Actual source-backed stages produce a partial plan using explicit manual time/node estimates; unknown eligibility/details remain unavailable, original quest/stage identities retained.","source_record_ids":quest_ids},
            {"domain":"class_roles","scenario":"authored_score_model_boundaries_v1","scope":"All54 original models/nine historical-input classes: both band boundaries, wrong class/outside-band refusal, explicit role assumption, positive real-item stamina arithmetic; equipment eligibility and acquisition remain unknown; strict practical paths refuse.","source_record_ids":models.iter().map(|m|json!([authored_revision,"models",m["id"]]).to_string()).collect::<Vec<_>>()},
            {"domain":"reminders","scenario":"authored_heuristic_reminder_boundaries_v1","scope":"All18 source rules retain explicit heuristic classification; twelve role prompts evaluated at both band boundaries, general level boundary and unknown-role withholding tested; unavailable predicates are not promoted.","source_record_ids":authored["reference"]["payload"]["rules"].as_array().unwrap().iter().map(|r|json!([authored_revision,"rules",r["id"]]).to_string()).collect::<Vec<_>>()},
            {"domain":"dungeons","scenario":"supported_dungeon_guidance_v1","scope":"26 composed supported entrances and nonempty heuristic preparation records, preserving unknown access/conditions and unverified entrance labels; original raw map169 explicit test exclusion retained. Source-linked quests are not claimed complete or target-verified.","source_record_ids":dungeon_ids}
        ]);
        let artifact = json!({"schema_version":1,"kind":"manual_engineering_full_pack_functional_scenarios","status":"pass","game_observation":false,"platform":std::env::consts::OS,"test_source":{"path":source_path,"commit":head,"sha256":format!("{:x}",Sha256::digest(compiled))},"packs":[file_ref("content/release-packs/world-v2/manifest.json"),file_ref("content/release-packs/authored-v2/manifest.json")],"source_inputs":[file_ref("data-generators/output/dungeons/dungeons/vanilla_1_12/1.12.1/part-0000.json"),file_ref("docs/data/role-coverage-v1.json"),file_ref("data-generators/output/quests/quests/vanilla_1_12/1.12.1/part-0000.json")],"additional_source_records":[{"domain":"equipment","id":json!([world_revision,"items",item.item_id,item.variant]).to_string()}],"units":units,"counts":{"models":models.len(),"classes":classes.len(),"score_boundary_cases":score_boundary_cases,"reminder_boundary_cases":reminder_scenarios,"dungeon_packages":choices.total,"raw_dungeons":27,"excluded_test_maps":[169]},"disclosure":"Permitted manual_entry engineering snapshot and explicit class/role/time/node assumptions. No addon capture, live-client correctness, Windows/manual acceptance or whole-game completeness is established."});
        std::fs::write(output, serde_json::to_vec_pretty(&artifact).unwrap()).unwrap();
    }
    println!(
        "ACTUAL-BUNDLE-PROOF revision={revision:?} projected_source_quest={:?} unresolved={:?} models=54 classes=9 dungeon_packages=26; character facts are disclosed manual engineering inputs; time/node estimates manual; target-game correctness unverified",
        positive.id, unresolved.id
    );
}

#[test]
fn permitted_engineering_profiles_and_quest_updates_keep_exchange_guards() {
    let app = SnapshotApplication::in_memory().unwrap();
    let (unknown, _) = engineering_profile(
        &app,
        "engineering-boundary-unknown",
        20,
        json!({"status":"unknown"}),
        500,
    );
    let (declared, _) = engineering_profile(
        &app,
        "engineering-boundary-declared",
        21,
        known(json!("tank")),
        501,
    );
    assert_ne!(unknown, declared);
    let mut value = snapshot();
    let first = import(&app, &mut value, 502);
    value["payload"]["quests"]["quest:1"] = json!({"quest":"quest:1","context":value["payload"]["context"],"accepted":known(json!(true)),"completed":known(json!(false)),"turned_in":known(json!(false)),"objectives":{"quest:1:complete":known(json!(false))}});
    value["payload"]["objectives"]["quest:1:complete"] = known(json!(false));
    assert_eq!(import(&app, &mut value, 503), first);
    value["fixture"] = json!(true);
    assert!(
        app.import_text(
            &format!("ANAV1:{value}"),
            Some(UtcMillis::new(NOW).unwrap())
        )
        .is_err()
    );
}
