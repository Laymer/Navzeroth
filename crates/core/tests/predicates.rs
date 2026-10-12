use azeroth_core::{predicate::*, types::*};
use serde_json::{from_value, json};

#[test]
fn predicate_json_is_closed_typed_and_bounded() {
    assert!(from_value::<Predicate>(json!({"op":"eval","code":"arbitrary"})).is_err());
    assert!(from_value::<Predicate>(json!({"op":"greater_than_or_equal","field":{"field":"role"},"value":{"type":"text","value":"tank"}})).is_err());
    let leaf = PredicateNode::LevelAtLeast {
        level: Level::new(22).unwrap(),
    };
    let mut tree = leaf.clone();
    for _ in 1..32 {
        tree = PredicateNode::Not {
            predicate: Box::new(tree),
        };
    }
    assert!(Predicate::try_new(tree.clone()).is_ok());
    assert!(
        Predicate::try_new(PredicateNode::Not {
            predicate: Box::new(tree)
        })
        .is_err()
    );
    assert!(
        Predicate::try_new(PredicateNode::All {
            predicates: vec![leaf.clone(); 1023]
        })
        .is_ok()
    );
    assert!(
        Predicate::try_new(PredicateNode::All {
            predicates: vec![leaf; 1024]
        })
        .is_err()
    );
    assert!(RuleLimits::new(33, 1024, 1024).is_err());
}
#[test]
fn positions_are_map_scoped_and_rectangles_include_boundaries() {
    let area:SupportedArea=from_value(json!({"map":{"context":{"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"demo"},"map_id":"1","floor":"ground","instance":null},"bounds":{"min_x":0,"min_y":0,"max_x":1,"max_y":1}})).unwrap();
    let mut point = Position {
        map: area.map.clone(),
        x: Normalized::new(1.).unwrap(),
        y: Normalized::new(0.).unwrap(),
    };
    assert!(area.contains(&point).unwrap());
    point.map.floor = FloorId::new("upstairs").unwrap();
    assert!(!area.contains(&point).unwrap());
    point.map.context.namespace = DatasetNamespace::Reference;
    assert!(area.contains(&point).is_err());
    assert!(from_value::<Bounds>(json!({"min_x":1,"max_x":0,"min_y":0,"max_y":1})).is_err());
}
