use azeroth_core::entities::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
fn check<T: DeserializeOwned + Serialize>(good: Value, bad: Value) {
    let v: T = serde_json::from_value(good).unwrap();
    let encoded = serde_json::to_value(v).unwrap();
    assert!(serde_json::from_value::<T>(encoded).is_ok());
    assert!(serde_json::from_value::<T>(bad).is_err());
}
#[test]
fn reference_records_round_trip_and_reject_semantic_errors() {
    let context = json!({"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"demo"});
    let u = json!({"status":"unknown"});
    let key = json!({"context":context,"item_id":"9007199254740993","variant":"base"});
    let game = json!({"key":context["build"],"locale":"enGB","capabilities":u});
    let mut bad = game.clone();
    bad["capabilities"] =
        json!({"status":"known","value":{"level":{"status":"verified_available","evidence":[]}}});
    check::<GameBuild>(game, bad);
    let evidence = json!({"id":"e1","context":context,"source":"fixture","license":u,"observed_at_utc_ms":u,"confidence":"inferred","conflicts":[],"notes":"synthetic"});
    let mut bad = evidence.clone();
    bad["confidence"] = json!("verified_on_build");
    check::<EvidenceRecord>(evidence, bad);
    let provider = json!({"provider":"p1","context":context,"status":"unverified","datasets":[],"endpoints":[],"freshness":{},"evidence":[]});
    let mut bad = provider.clone();
    bad["status"] = json!("available");
    check::<ProviderCapability>(provider, bad);
    let item = json!({"key":key,"slots":["main_hand"],"two_handed":u,"unique_group":u,"attributes":u,"requirements":u,"effects":u,"evidence":[]});
    let mut bad = item.clone();
    bad["slots"] = json!([]);
    check::<ItemDefinition>(item, bad);
    let equipped = json!({"context":context,"item_id":"1","variant":u,"instance":u,"modifiers":u});
    let mut bad = equipped.clone();
    bad["variant"] = json!({"status":"unknown","value":"base"});
    check::<EquippedItem>(equipped, bad);
    let source = json!({"id":"a1","context":context,"kind":"vendor","items":[key],"conditions":u,"cost":u,"probability":u,"evidence":[]});
    let mut bad = source.clone();
    bad["items"][0]["context"]["namespace"] = json!("reference");
    check::<AcquisitionSource>(source, bad);
    let quest = json!({"id":"q1","context":context,"eligibility":u,"prerequisites":u,"stages":["o1"],"locations":u,"rewards":u,"completion":u,"shared_progress":u,"evidence":[]});
    let mut bad = quest.clone();
    bad["stages"] = json!(["o1", "o1"]);
    check::<QuestDefinition>(quest, bad);
    let progress = json!({"quest":"q1","context":context,"accepted":u,"completed":u,"turned_in":u,"objectives":{}});
    let mut bad = progress.clone();
    bad["completed"] = json!({"status":"known"});
    check::<QuestProgress>(progress, bad);
}
