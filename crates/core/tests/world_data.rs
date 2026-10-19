use azeroth_core::world_data::{
    MAX_SHARD_BYTES, Record, RecordKind, parse, world_data_schema, world_data_schema_text,
};
use serde_json::{Value, json};

const KNOWN_POS: &str = r#"{"status":"known","value":{"ui_map_id":1429,"x":0.4,"y":0.6}}"#;
fn pos() -> Value {
    serde_json::from_str(KNOWN_POS).unwrap()
}
fn world() -> Value {
    json!({"map_id":0,"x":-9449.0,"y":64.0,"z":56.0})
}
fn shard(kind: &str, record: Value) -> Value {
    let mut record = record;
    record["kind"] = json!(kind);
    json!({
        "format":"navzeroth-world-data","schema_version":1,"dataset":"world.test_set",
        "record_kind":kind,
        "origin":{"edition":"vanilla_1_12","build":{"status":"known","value":"1.12.1.5875"},"product":"cmangos","locale":"enUS"},
        "target":{"edition":"wow_forever","build":{"status":"unknown"}},
        "confidence":"cross_edition_unverified",
        "sources":[{"id":"cmangos-classic-db","url":"https://github.com/cmangos/classic-db","revision":"ClassicDB_1_12_1_z2799.sql.gz","snapshot_sha256":"a".repeat(64)}],
        "generator":{"name":"gen_world_cmangos","version":"1","credit":"Data generators based on work by NotInTheBand (TALOD), used with permission."},
        "shard":{"index":0,"count":1},
        "records":[record]
    })
}
fn check(v: &Value) -> azeroth_core::error::Result<azeroth_core::world_data::WorldData> {
    parse(&serde_json::to_vec(v).unwrap())
}
fn samples() -> Vec<(&'static str, Value)> {
    vec![
        (
            "item",
            json!({"id":2589,"name":"Linen Cloth","quality":1,"icon":"inv_fabric_linen_01","class_id":7,"subclass_id":5,
            "inventory_type":0,"item_level":5,"required_level":0,"allowable_class":-1,"allowable_race":-1,"bonding":0,
            "max_stack":20,"stats":{"stamina":0},"buy_price":0,"sell_price":13,"auction_average":40,"vendor_sold":false,
            "obtain_hint":"Cloth (drops from humanoids)","made_by":[]}),
        ),
        (
            "recipe",
            json!({"spell_id":3275,"name":"Linen Bandage","profession":"First Aid","skill_line":129,"required_skill":1,
            "colors":[0,30,45,60],"skill_up":1,"reagents":[{"item":2589,"count":1}],"tools":[],"creates":1251,"makes":1.0,
            "sources":["trainer"],"trainer_cost":0,"cooldown":false,"source_inferred":false}),
        ),
        (
            "profession_rank",
            json!({"profession":"Cooking","rank":"Expert","max_skill":225,"learn_at_skill":125,"cost":{"status":"unknown"},"note":"a book"}),
        ),
        (
            "profession_note",
            json!({"profession":"Mining","text":"mining ore veins"}),
        ),
        (
            "enhancement",
            json!({"enhancement_type":"enchant","spell_id":7418,"name":"Bracer - Minor Health","slots":[9],"effect":"+5 Health","temporary":false}),
        ),
        (
            "fishing_area",
            json!({"area_id":12,"name":"Elwynn Forest","base_skill":0}),
        ),
        (
            "fishing_goal",
            json!({"item_id":6522,"group":"rare","name":"Deviate Fish","quality":1,"places":[["Wailing Caverns","The Barrens"]],
            "quests":[{"id":1,"title":"Q","count":1,"rewards":["R"]}]}),
        ),
        (
            "timed_fish",
            json!({"item_id":13759,"hours":[18,6],"best_hours":[0,6],"label":"night only","open":"night","closed":"day","source":"wiki"}),
        ),
        (
            "fishing_event",
            json!({"name":"Extravaganza","weekday":1,"start_minute":840,"length_minutes":120,"source":"wiki"}),
        ),
        (
            "area",
            json!({"id":12,"name":"Elwynn Forest","map_id":0,"exploration_level":1,"level_range":[1,10]}),
        ),
        (
            "ui_map",
            json!({"id":1429,"name":"Elwynn Forest","parent_ui_map_id":1415,"map_type":3}),
        ),
        (
            "ui_map_assignment",
            json!({"ui_map_id":1429,"area_id":12,"map_id":0,"order_index":0,"ui_min":[0.0,0.0],"ui_max":[1.0,1.0],
            "region_min":[-10000.0,-1000.0,-500.0],"region_max":[-8000.0,1000.0,500.0]}),
        ),
        (
            "npc",
            json!({"id":448,"name":"Hogger","min_level":11,"max_level":11,"faction_template":38,"reaction":"hostile","rank":1,"roles":[]}),
        ),
        (
            "game_object",
            json!({"id":1731,"name":"Copper Vein","object_type":3,"required_skill":1}),
        ),
        (
            "spawn",
            json!({"guid":1,"entity":"npc","entity_id":448,"world":world(),"position":pos(),"area_id":12,"respawn_secs":300}),
        ),
        (
            "quest",
            json!({"id":176,"title":"Wanted: \"Hogger\"","level":11,"min_level":8,"zone_or_sort":12,"givers":[{"entity":"game_object","id":68}],
            "enders":[{"entity":"npc","id":240}],"objectives":[{"objective":"item","target_id":1931,"count":1}],
            "reward_choice_items":[{"item":2224,"count":1}],"reward_money":1500,"reward_xp":1050}),
        ),
        (
            "vendor_item",
            json!({"npc_id":1250,"item_id":159,"max_count":5,"restock_secs":3600}),
        ),
        (
            "trainer_spell",
            json!({"npc_id":198,"spell_id":1459,"cost":100,"required_level":8}),
        ),
        (
            "spell",
            json!({"id":1459,"name":"Arcane Intellect","rank":"Rank 1","class_mask":128,"base_level":8,"spell_level":8}),
        ),
        (
            "talent",
            json!({"id":1,"tab_id":41,"tab_name":"Fire","tier":0,"column":1,"ranks":[11078,11080]}),
        ),
        (
            "taxi_node",
            json!({"id":2,"name":"Stormwind","world":world(),"position":{"status":"unsupported"},"faction":"alliance","flight_master":352}),
        ),
        (
            "taxi_path",
            json!({"id":1,"from_node":2,"to_node":6,"cost":100,"points":[world()]}),
        ),
        (
            "loot",
            json!({"table":"creature","entry":448,"item":1931,"chance":-100.0,"min_count":1,"max_count":1}),
        ),
        (
            "dungeon",
            json!({"map_id":36,"name":"The Deadmines","min_level":17,"max_level":26,"entrance":{"status":"known","value":world()},"bosses":[639]}),
        ),
        (
            "graveyard",
            json!({"id":4,"world":world(),"position":pos(),"faction":"alliance","zones":[12]}),
        ),
    ]
}

#[test]
fn every_record_kind_round_trips() {
    let all = samples();
    assert_eq!(all.len(), 25);
    for (kind, record) in all {
        let parsed = check(&shard(kind, record.clone()))
            .unwrap_or_else(|e| panic!("{kind} rejected: {e:?}"));
        let kind_value: RecordKind = serde_json::from_value(json!(kind)).unwrap();
        assert_eq!(parsed.record_kind, kind_value);
        assert_eq!(parsed.records[0].kind(), kind_value);
        let again: Record =
            serde_json::from_value(serde_json::to_value(&parsed.records[0]).unwrap()).unwrap();
        assert_eq!(again, parsed.records[0]);
    }
}

fn rejected(kind: &str, record: Value) {
    assert!(
        check(&shard(kind, record.clone())).is_err(),
        "{kind} accepted {record}"
    );
}

#[test]
fn record_rules_refuse_invalid_values() {
    rejected("item", json!({"id":1,"name":" "}));
    rejected("item", json!({"id":1,"name":"bad\u{0007}"}));
    rejected("item", json!({"id":1,"name":"x".repeat(9000)}));
    rejected("item", json!({"id":1,"name":"ok","surprise":1}));
    rejected(
        "recipe",
        json!({"spell_id":1,"name":"R","profession":"P","reagents":[{"item":1,"count":0}],"sources":[]}),
    );
    rejected(
        "recipe",
        json!({"spell_id":1,"name":"R","profession":"P","reagents":[],"makes":0.5,"sources":[]}),
    );
    rejected(
        "recipe",
        json!({"spell_id":1,"name":"R","profession":"P","reagents":[],"sources":[""]}),
    );
    rejected(
        "profession_rank",
        json!({"profession":"P","rank":"","max_skill":1,"learn_at_skill":1,"cost":{"status":"unknown"}}),
    );
    rejected("profession_note", json!({"profession":"P","text":""}));
    rejected(
        "enhancement",
        json!({"enhancement_type":"item","spell_id":1,"name":"E","slots":[],"effect":""}),
    );
    rejected(
        "enhancement",
        json!({"enhancement_type":"item","spell_id":1,"name":"E","slots":[20],"effect":""}),
    );
    rejected(
        "fishing_area",
        json!({"area_id":1,"name":"","base_skill":1}),
    );
    rejected(
        "fishing_goal",
        json!({"item_id":1,"group":"rare","name":"F","quality":1,"places":[[""]]}),
    );
    rejected(
        "timed_fish",
        json!({"item_id":1,"label":"l","open":"o","closed":"c","source":"s"}),
    );
    rejected(
        "fishing_event",
        json!({"name":"E","weekday":8,"start_minute":0,"length_minutes":1,"source":"s"}),
    );
    rejected(
        "fishing_event",
        json!({"name":"E","weekday":1,"start_minute":1440,"length_minutes":1,"source":"s"}),
    );
    rejected("area", json!({"id":1,"name":"A","level_range":[10,1]}));
    rejected("ui_map", json!({"id":1,"name":""}));
    rejected(
        "ui_map_assignment",
        json!({"ui_map_id":1,"map_id":0,"order_index":0,"ui_min":[0.0,0.0],"ui_max":[1.5,1.0],"region_min":[0.0,0.0,0.0],"region_max":[1.0,1.0,1.0]}),
    );
    rejected(
        "npc",
        json!({"id":1,"name":"N","min_level":10,"max_level":5}),
    );
    rejected("game_object", json!({"id":1,"name":""}));
    rejected(
        "spawn",
        json!({"guid":1,"entity":"npc","entity_id":1,"world":world(),"position":{"status":"known","value":{"ui_map_id":1,"x":1.2,"y":0.5}}}),
    );
    rejected(
        "spawn",
        json!({"guid":1,"entity":"npc","entity_id":1,"world":world(),"position":{"status":"known"}}),
    );
    rejected(
        "quest",
        json!({"id":1,"title":"Q","objectives":[{"objective":"kill","target_id":1,"count":0}]}),
    );
    rejected("quest", json!({"id":1,"title":""}));
    rejected("spell", json!({"id":1,"name":""}));
    rejected(
        "talent",
        json!({"id":1,"tab_id":1,"tier":0,"column":0,"ranks":[]}),
    );
    rejected(
        "taxi_node",
        json!({"id":1,"name":"","world":world(),"position":{"status":"unknown"}}),
    );
    rejected(
        "loot",
        json!({"table":"creature","entry":1,"chance":5.0,"min_count":1,"max_count":1}),
    );
    rejected(
        "loot",
        json!({"table":"creature","entry":1,"item":1,"reference":2,"chance":5.0,"min_count":1,"max_count":1}),
    );
    rejected(
        "loot",
        json!({"table":"creature","entry":1,"item":1,"chance":150.0,"min_count":1,"max_count":1}),
    );
    rejected(
        "loot",
        json!({"table":"creature","entry":1,"item":1,"chance":5.0,"min_count":3,"max_count":1}),
    );
    rejected(
        "dungeon",
        json!({"map_id":36,"name":"","entrance":{"status":"unknown"}}),
    );
    rejected(
        "graveyard",
        json!({"id":1,"world":world(),"position":{"status":"known","value":{"ui_map_id":1,"x":0.5,"y":-0.1}}}),
    );
}

#[test]
fn non_finite_world_points_are_refused() {
    // JSON cannot carry NaN; build the records in Rust instead.
    use azeroth_core::world_data::*;
    let mut data = check(&shard(
        "taxi_path",
        json!({"id":1,"from_node":1,"to_node":2,"points":[world()]}),
    ))
    .unwrap();
    if let Record::TaxiPath(path) = &mut data.records[0] {
        path.points[0].x = f64::NAN;
    }
    assert!(data.validate().is_err());
    let mut data = check(&shard("dungeon", samples()[23].1.clone())).unwrap();
    if let Record::Dungeon(d) = &mut data.records[0] {
        d.entrance = azeroth_core::observation::Observation::known(
            WorldPoint {
                map_id: 0,
                x: f64::INFINITY,
                y: 0.0,
                z: 0.0,
            },
            Default::default(),
        )
        .unwrap();
    }
    assert!(data.validate().is_err());
    let mut data = check(&shard("ui_map_assignment", samples()[11].1.clone())).unwrap();
    if let Record::UiMapAssignment(a) = &mut data.records[0] {
        a.region_max[2] = f64::NAN;
    }
    assert!(data.validate().is_err());
}

fn envelope_rejected(edit: impl Fn(&mut Value)) {
    let mut v = shard("npc", samples()[12].1.clone());
    edit(&mut v);
    assert!(check(&v).is_err(), "accepted {v}");
}

#[test]
fn envelope_rules_keep_editions_and_bounds() {
    envelope_rejected(|v| v["format"] = json!("other"));
    envelope_rejected(|v| v["schema_version"] = json!(2));
    envelope_rejected(|v| v["dataset"] = json!("World.Npcs"));
    envelope_rejected(|v| v["dataset"] = json!("world..npcs"));
    envelope_rejected(|v| v["dataset"] = json!("a".repeat(129)));
    envelope_rejected(|v| v["shard"] = json!({"index":1,"count":1}));
    envelope_rejected(|v| v["shard"] = json!({"index":0,"count":0}));
    envelope_rejected(|v| v["shard"] = json!({"index":0,"count":5000}));
    envelope_rejected(|v| v["sources"] = json!([]));
    envelope_rejected(|v| v["sources"][0]["url"] = json!("http://insecure.example"));
    envelope_rejected(|v| v["sources"][0]["snapshot_sha256"] = json!("ABC"));
    envelope_rejected(|v| v["generator"]["credit"] = json!(""));
    envelope_rejected(|v| v["generator"]["name"] = json!(""));
    envelope_rejected(|v| v["generator"]["version"] = json!(""));
    envelope_rejected(|v| v["records"] = json!([]));
    envelope_rejected(|v| v["record_kind"] = json!("item"));
    // Classic data never claims Forever verification, nor plain publisher confidence.
    envelope_rejected(|v| v["confidence"] = json!("verified_on_build"));
    envelope_rejected(|v| v["confidence"] = json!("publisher_documented"));
    envelope_rejected(|v| v["origin"]["build"] = json!({"status":"known","value":null}));
    envelope_rejected(|v| v["extra"] = json!(true));
    let too_many = vec![json!({"kind":"profession_note","profession":"P","text":"t"}); 250_001];
    let mut big = shard("profession_note", json!({"profession":"P","text":"t"}));
    big["records"] = json!(too_many);
    assert!(check(&big).is_err());
}

#[test]
fn same_edition_rules() {
    let forever = |v: &mut Value| {
        v["origin"]["edition"] = json!("wow_forever");
        v["origin"]["build"] = json!({"status":"known","value":"1.60.1.70245"});
    };
    let mut v = shard("npc", samples()[12].1.clone());
    forever(&mut v);
    v["confidence"] = json!("publisher_documented");
    assert!(check(&v).is_ok());
    // Verified needs the frozen target build to equal the origin build.
    v["confidence"] = json!("verified_on_build");
    assert!(check(&v).is_err());
    v["target"]["build"] = json!({"status":"known","value":"1.60.1.70245"});
    assert!(check(&v).is_ok());
    v["origin"]["build"] = json!({"status":"unknown"});
    assert!(check(&v).is_err());
    let mut stale = shard("npc", samples()[12].1.clone());
    stale["confidence"] = json!("disputed_or_stale");
    assert!(check(&stale).is_ok());
}

#[test]
fn reader_limits_and_schema_artifact() {
    assert!(parse(&vec![b' '; MAX_SHARD_BYTES + 1]).is_err());
    assert!(parse(b"{not json").is_err());
    assert_eq!(
        world_data_schema_text(),
        include_str!("../../../schemas/world-data/v1/world-data.schema.json")
    );
    let schema = world_data_schema();
    assert_eq!(
        schema["properties"]["format"]["const"],
        "navzeroth-world-data"
    );
    assert_eq!(schema["properties"]["records"]["maxItems"], 250_000);
}
