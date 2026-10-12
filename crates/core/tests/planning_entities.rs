use azeroth_core::entities::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
fn check<T: DeserializeOwned + Serialize>(good: Value, bad: Value) {
    let v: T = serde_json::from_value(good).unwrap();
    assert!(serde_json::from_value::<T>(serde_json::to_value(v).unwrap()).is_ok());
    assert!(serde_json::from_value::<T>(bad).is_err());
}
#[test]
fn planning_records_validate_inputs_exclusions_and_missing_evidence() {
    let c = json!({"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"demo"});
    let u = json!({"status":"unknown"});
    let snap = json!({"id":"snap1","character":"c1","context":c});
    let plan = json!({"schema_version":1,"id":"plan1","context":c,"generated_at_utc_ms":100,"inputs":[snap],"dataset_revision":"demo-r1","ruleset_version":"rules1","score_model_version":"scores1","goal":"synthetic","time_budget_ms":1000,"exclusions":[],"manual_assumptions":[],"steps":[{"id":"step1","objective":"o1","participants":["c1"],"alternatives":[],"prerequisite":u,"required_fields":[],"invalidation":u,"explanation":"synthetic"}],"travel":[],"evidence":[],"expires_at_utc_ms":200});
    let mut bad = plan.clone();
    bad["exclusions"] = json!(["o1"]);
    check::<RoutePlan>(plan, bad);
    let gear = json!({"context":c,"snapshot":snap,"dataset_revision":"demo-r1","score_model_version":"scores1","scenario":"test","ranking":"theoretical","baseline":{},"proposed":{},"eligibility":u,"baseline_score":u,"proposed_score":u,"acquisitions":[],"assumptions":[],"unmodeled_effects":["unknown"],"explanation":"synthetic","evidence":[]});
    let mut bad = gear.clone();
    bad["ranking"] = json!("obtainable");
    check::<GearRecommendation>(gear.clone(), bad);
    let item = json!({"item":{"context":c,"item_id":"1","variant":"base"},"occupied_slots":["main_hand","off_hand"],"unique_group":u});
    let mut bad = gear;
    bad["proposed"] = json!({"main_hand":item,"off_hand":item});
    assert!(serde_json::from_value::<GearRecommendation>(bad).is_err());
    let rule = json!({"id":"r1","context":c,"trigger":"manual","predicate":{"op":"level_at_least","level":22},"result_template":"synthetic","priority":0,"deduplication_key":"r1","cooldown_ms":10,"dismissal":"session","expires_at_utc_ms":null,"evidence":[]});
    let mut bad = rule.clone();
    bad["result_template"] = json!("");
    check::<ContextRule>(rule, bad);
    let summary = json!({"session":"s1","character":"c1","context":c,"elapsed_ms":100,"starting_level":u,"ending_level":u,"observed_quests":[],"changed_equipment":[],"intervals":[{"start_ms":0,"end_ms":50,"quality":"estimated","label":"travel"}],"gaps":[{"start_ms":50,"end_ms":100,"reason":"reload"}],"methodology":"synthetic intervals"});
    let mut bad = summary.clone();
    bad["gaps"][0]["start_ms"] = json!(49);
    check::<SessionSummary>(summary, bad);
}
