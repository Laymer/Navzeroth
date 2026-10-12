use azeroth_core::entities::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
fn check<T: DeserializeOwned + Serialize>(good: Value, bad: Value) {
    let v: T = serde_json::from_value(good).unwrap();
    assert!(serde_json::from_value::<T>(serde_json::to_value(v).unwrap()).is_ok());
    assert!(serde_json::from_value::<T>(bad).is_err());
}
#[test]
fn spatial_records_validate_ranges_context_and_relationships() {
    let context = json!({"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"demo"});
    let map = json!({"context":context,"map_id":"map1","floor":"ground","instance":null});
    let u = json!({"status":"unknown"});
    let space = json!({"key":map,"transform":u,"evidence":[]});
    let mut bad = space.clone();
    bad["transform"] = json!({"status":"known","value":{"units":"yards","scale_x":0,"scale_y":1,"offset_x":0,"offset_y":0}});
    check::<MapSpace>(space, bad);
    let node = json!({"id":"n1","position":{"map":map,"x":0.1,"y":0.2},"service":u});
    let mut bad = node.clone();
    bad["position"]["x"] = json!(-1);
    check::<TravelNode>(node, bad);
    let edge = json!({"id":"edge1","context":context,"from":{"id":"n1","map":map},"to":{"id":"n2","map":map},"mode":"walking","requirements":u,"expected_ms":{"status":"known","value":100},"wait_ms":u,"uncertainty":{"status":"known","value":{"min_ms":90,"max_ms":110}},"status":"estimated","evidence":[],"consumes_resource":u,"cooldown_ms":u});
    let mut bad = edge.clone();
    bad["uncertainty"]["value"]["min_ms"] = json!(120);
    check::<TravelEdge>(edge, bad);
    let objective = json!({"id":"o1","context":context,"participants":["c1"],"targets":[{"kind":"quest_stage","quest":"q1","stage":"stage1"}],"requirements":u,"expected_ms":u,"evidence":[]});
    let mut bad = objective.clone();
    bad["participants"] = json!([]);
    check::<Objective>(objective, bad);
}
