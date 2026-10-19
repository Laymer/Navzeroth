//! World browser and origin markers over a small synthetic world pack. The pack is
//! built here deterministically from the committed v2 contract fixture; it is not
//! game data. `UPDATE_WORLD_BROWSER_FIXTURES=1` rewrites `fixtures/desktop/world/`,
//! which Vitest and Playwright read; otherwise the files are compared.
use azeroth_app::{
    snapshots::SnapshotApplication,
    world::{self, OriginRef, WorldView},
};
use azeroth_core::types::{DatasetRevision, UtcMillis};
use azeroth_storage::content::{ActiveContentRevision, ContentRevisionV2};
use serde_json::{Value, json};
use std::sync::Arc;
const NOW: i64 = 1791791990100;
const CONTEXT: &str = r#"{"build":{"build":"UNVERIFIED_DEMO","edition":"wow_forever","interface":"UNVERIFIED_DEMO"},"namespace":"demo"}"#;
fn context() -> Value {
    serde_json::from_str(CONTEXT).unwrap()
}
fn map() -> Value {
    json!({"context":context(),"floor":"0","instance":null,"map_id":"demo-map-1"})
}
fn known(value: Value) -> Value {
    json!({"status":"known","value":value})
}
fn item() -> Value {
    json!({"context":context(),"item_id":"demo-item-1","variant":"base"})
}
fn evidence(id: &str, source: &str, confidence: &str) -> Value {
    json!({"id":id,"context":context(),"source":source,"license":{"status":"unknown"},"observed_at_utc_ms":{"status":"unknown"},"confidence":confidence,"conflicts":[],"notes":"Synthetic world-browser evidence"})
}
fn node(id: &str, x: f64, y: f64) -> Value {
    json!({"id":id,"position":{"map":map(),"x":x,"y":y},"service":{"status":"unknown"}})
}
fn entity(id: &str, kind: &str, name: &str, services: &[&str], nodes: &[&str], ev: &str) -> Value {
    json!({"id":id,"evidence":[ev],"kind":kind,"name":name,"subname":{"status":"unknown"},"levels":{"status":"unknown"},"services":services,"reaction":{"status":"unknown"},"nodes":nodes})
}
fn acquisition(id: &str, kind: &str, cost: Value, probability: Value, ev: &str) -> Value {
    json!({"id":id,"context":context(),"kind":kind,"items":[item()],"conditions":{"status":"unknown"},"cost":cost,"probability":probability,"evidence":[ev]})
}
fn facts(id: &str, origin: Value, ev: &str) -> Value {
    json!({"acquisition":id,"evidence":[ev],"origin":known(origin),"rewards":{"status":"unknown"},"costs":{"status":"unknown"},"availability":{"status":"unknown"},"prerequisites":{"status":"unknown"},"entitlement":{"status":"unknown"}})
}
/// A tiny world: Hogger from 1.12 data, a Classic Era vendor offer, a trainer, an
/// inferred walking edge and a quest with a reward, beside the Forever demo rows.
pub fn bundle() -> Value {
    let mut content: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v2/reference-content.json"
    ))
    .unwrap();
    let old: Value = serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap();
    let mut sources = old["sources"].as_array().unwrap().clone();
    sources[0]["editions"] = known(json!(["wow_forever"]));
    for (id, edition) in [
        ("synthetic-cmangos", "vanilla_1_12"),
        ("synthetic-classic", "wow_classic_era"),
    ] {
        let mut source = sources[0].clone();
        source["id"] = json!(id);
        source["editions"] = known(json!([edition]));
        sources.push(source);
    }
    let payload = &mut content["reference"]["payload"];
    for e in [
        evidence(
            "world-evidence-112",
            "synthetic-cmangos",
            "cross_edition_unverified",
        ),
        evidence(
            "world-evidence-classic",
            "synthetic-classic",
            "cross_edition_unverified",
        ),
        evidence("world-evidence-inferred", "demo-source-1", "inferred"),
    ] {
        payload["evidence"].as_array_mut().unwrap().push(e);
    }
    for n in [
        node("world-node-hogger", 0.27, 0.87),
        node("world-node-vendor", 0.42, 0.66),
    ] {
        payload["nodes"].as_array_mut().unwrap().push(n);
    }
    payload["acquisitions"].as_array_mut().unwrap().extend([
        acquisition(
            "world-acq-vendor",
            "vendor",
            known(json!("250")),
            known(json!(1)),
            "world-evidence-classic",
        ),
        acquisition(
            "world-acq-drop",
            "other",
            json!({"status":"unknown"}),
            known(json!(0.25)),
            "world-evidence-112",
        ),
        acquisition(
            "world-acq-quest",
            "quest",
            json!({"status":"not_applicable"}),
            known(json!(1)),
            "world-evidence-112",
        ),
    ]);
    payload["quests"][0]["evidence"] = json!(["world-evidence-112"]);
    let mut edge = payload["edges"][0].clone();
    edge["id"] = json!("world-edge-inferred");
    edge["to"]["id"] = json!("world-node-hogger");
    edge["evidence"] = json!(["world-evidence-inferred"]);
    payload["edges"].as_array_mut().unwrap().push(edge);
    content["acquisition_facts"].as_array_mut().unwrap().extend([
        facts(
            "world-acq-vendor",
            json!({"kind":"vendor","entity":"npc:1198","location":{"id":"world-node-vendor","map":map()}}),
            "world-evidence-classic",
        ),
        facts(
            "world-acq-drop",
            json!({"kind":"drop","entity":"npc:448","dungeon":null}),
            "world-evidence-112",
        ),
        facts(
            "world-acq-quest",
            json!({"kind":"quest","quest":"demo-quest-1"}),
            "world-evidence-112",
        ),
    ]);
    let mut hogger = entity(
        "npc:448",
        "npc",
        "Hogger",
        &[],
        &["world-node-hogger"],
        "world-evidence-112",
    );
    hogger["levels"] = known(json!([11, 11]));
    hogger["reaction"] = known(json!("hostile"));
    let mut vendor = entity(
        "npc:1198",
        "npc",
        "Synthetic Quartermaster",
        &["vendor", "repair"],
        &["world-node-vendor"],
        "demo-evidence-1",
    );
    vendor["subname"] = known(json!("Synthetic Goods"));
    vendor["reaction"] = known(json!("alliance"));
    content["entities"] = json!([
        hogger,
        vendor,
        entity(
            "npc:5000",
            "npc",
            "Synthetic Trainer",
            &["trainer"],
            &["demo-node-1"],
            "world-evidence-classic"
        ),
        entity(
            "object:1",
            "game_object",
            "Synthetic Chest",
            &[],
            &[],
            "world-evidence-inferred"
        ),
    ]);
    // Names, quest titles, givers and enders, bosses and loot templates (V2.8g).
    content["equipment"][0]["name"] = json!("Synthetic Sword");
    content["quest_facts"][0]["title"] = json!("Synthetic Quest");
    content["quest_facts"][0]["givers"] = json!(["npc:1198", "item:demo-item-1", "object:404"]);
    content["quest_facts"][0]["enders"] = json!(["npc:448"]);
    content["dungeons"][0]["bosses"] = json!(["npc:448"]);
    content["loot_tables"] = json!([
        {"id":"loot:creature:448","evidence":["world-evidence-112"],"table":"creature",
         "source":known(json!("npc:448")),"rows":[
            {"item":"demo-item-1","probability":0.4,"min_count":1,"max_count":1},
            {"item":"9001","min_count":1,"max_count":2,"group":1,"quest_only":true},
            {"item":"9002","probability":0,"min_count":1,"max_count":1,"conditional":true},
            {"table":"loot:reference:9","probability":0.05,"min_count":1,"max_count":3}]},
        {"id":"loot:reference:9","evidence":["world-evidence-112"],"table":"reference",
         "source":{"status":"unknown"},"rows":[{"item":"demo-item-1","min_count":1,"max_count":1}]}
    ]);
    json!({"content":content,"sources":sources,"conflicts":[]})
}
/// The expected pack shape: each source records only its true origin edition.
fn revision(bundle: &Value) -> ActiveContentRevision {
    ActiveContentRevision::V2(Arc::new(ContentRevisionV2 {
        content: azeroth_core::content_v2::decode(bundle["content"].to_string().as_bytes())
            .unwrap(),
        sources: serde_json::from_value(bundle["sources"].clone()).unwrap(),
        conflicts: vec![],
    }))
}
/// Staging still requires every source to list the catalog edition, so the stored
/// copy adds `wow_forever` beside the true origin; marks must not change.
fn setup(bundle: &Value) -> (SnapshotApplication, String) {
    let mut bundle = bundle.clone();
    for source in bundle["sources"].as_array_mut().unwrap() {
        let editions = source["editions"]["value"].as_array_mut().unwrap();
        if !editions.contains(&json!("wow_forever")) {
            editions.push(json!("wow_forever"));
        }
    }
    let bundle = &bundle;
    let app = SnapshotApplication::in_memory().unwrap();
    let snapshot: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    let profile = app
        .import_text(
            &format!("ANAV1:{snapshot}"),
            Some(UtcMillis::new(NOW).unwrap()),
        )
        .unwrap()
        .profile_id;
    app.stage_content(&bundle.to_string()).unwrap();
    app.activate_content(&profile, &DatasetRevision::new("demo-r1").unwrap())
        .unwrap();
    (app, profile)
}
fn page(content: &ActiveContentRevision, view: WorldView, query: &str) -> Value {
    serde_json::to_value(world::world_page(content, view, query, 0, None).unwrap()).unwrap()
}
fn app_page(app: &SnapshotApplication, profile: &str, view: &str, query: &str) -> Value {
    let request = json!({"subject":{"kind":"character","profile_id":profile},"view":view,"query":query,"offset":0});
    serde_json::to_value(app.world_page(&request.to_string()).unwrap()).unwrap()
}
fn names(page: &Value) -> Vec<String> {
    page["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            r["name"]
                .as_str()
                .or(r["entity"]["name"].as_str())
                .or(r["id"].as_str())
                .unwrap()
                .to_owned()
        })
        .collect()
}

#[test]
fn world_views_search_page_and_mark_origins() {
    let content = revision(&bundle());
    let all = page(&content, WorldView::Entities, "");
    assert_eq!(
        all["counts"],
        json!({"entities":4,"vendors":1,"trainers":1,"quests":1,"loot":2})
    );
    assert_eq!(
        names(&all),
        [
            "Hogger",
            "Synthetic Chest",
            "Synthetic Quartermaster",
            "Synthetic Trainer"
        ]
    );
    let hogger = page(&content, WorldView::Entities, "  HOG ");
    assert_eq!(hogger["total"], 1);
    let row = &hogger["rows"][0];
    assert_eq!(row["row"], "entity");
    assert_eq!(row["origin"], json!(["vanilla_1_12"]));
    assert_eq!(row["places"][0]["node"], "world-node-hogger");
    assert_eq!(row["places"][0]["x"], 0.27);
    assert_eq!(row["levels"]["value"], json!([11, 11]));
    assert_eq!(
        page(&content, WorldView::Entities, "synthetic goods")["total"],
        1
    );
    let chest = page(&content, WorldView::Entities, "object:1");
    assert_eq!(chest["rows"][0]["origin"], json!(["inferred"]));
    assert_eq!(chest["rows"][0]["row"], "entity");
    let vendors = page(&content, WorldView::Vendors, "");
    let offer = &vendors["rows"][0]["offers"][0];
    assert_eq!(vendors["rows"][0]["row"], "source");
    assert_eq!(vendors["rows"][0]["entity"]["origin"], json!([]));
    assert_eq!(offer["origin"], json!(["wow_classic_era"]));
    assert_eq!(offer["cost"]["value"], "250");
    assert_eq!(offer["location"], "world-node-vendor");
    assert_eq!(offer["items"][0]["item_id"], "demo-item-1");
    let loot = page(&content, WorldView::Loot, "hogger");
    assert_eq!(loot["rows"][0]["offers"][0]["probability"]["value"], 0.25);
    assert_eq!(loot["rows"][0]["offers"][0]["dungeon"], Value::Null);
    assert_eq!(page(&content, WorldView::Loot, "nobody")["total"], 0);
    // Loot templates: unknown probability stays absent, zero stays zero, a
    // reference table is a row of its own, and a template without a stated
    // source is its own source.
    let table = &loot["rows"][0]["tables"][0];
    assert_eq!(loot["rows"][0]["tables_total"], 1);
    assert_eq!(
        (table["id"].as_str(), table["kind"].as_str()),
        (Some("loot:creature:448"), Some("creature"))
    );
    assert_eq!(table["rows_total"], 4);
    assert_eq!(table["rows"][0]["item"]["name"], "Synthetic Sword");
    assert_eq!(table["rows"][0]["probability"], 0.4);
    assert_eq!(table["rows"][1]["probability"], Value::Null);
    assert_eq!(table["rows"][1]["quest_only"], true);
    assert_eq!(table["rows"][1]["group"], 1);
    assert_eq!(table["rows"][1]["item"].get("name"), None);
    assert_eq!(table["rows"][2]["probability"], 0.0);
    assert_eq!(table["rows"][2]["conditional"], true);
    assert_eq!(table["rows"][3]["table"], "loot:reference:9");
    assert_eq!(table["origin"], json!(["vanilla_1_12"]));
    let reference = page(&content, WorldView::Loot, "loot:reference:9");
    assert_eq!(reference["rows"][0]["id"], "loot:reference:9");
    assert_eq!(reference["rows"][0]["entity"], Value::Null);
    assert_eq!(reference["rows"][0]["tables"][0]["kind"], "reference");
    // Bosses of dungeons, and item names wherever items are listed.
    let boss = page(&content, WorldView::Entities, "hogger");
    assert_eq!(boss["rows"][0]["boss_of"][0]["id"], "dungeon-1");
    assert!(boss["rows"][0]["boss_of"][0]["title"].is_string());
    assert_eq!(
        vendors["rows"][0]["offers"][0]["items"][0]["name"],
        "Synthetic Sword"
    );
    let trainers = page(&content, WorldView::Trainers, "");
    assert_eq!(names(&trainers), ["Synthetic Trainer"]);
    assert_eq!(trainers["rows"][0]["origin"], json!(["wow_classic_era"]));
    let quests = page(&content, WorldView::Quests, "QUEST-1");
    let quest = &quests["rows"][0];
    assert_eq!(quest["row"], "quest");
    assert_eq!(quest["origin"], json!(["vanilla_1_12"]));
    assert_eq!(quest["title"], "Synthetic Quest");
    assert_eq!(quest["stages"], json!(["demo-objective-1"]));
    assert_eq!(
        quest["givers"],
        json!([{"id":"npc:1198","name":"Synthetic Quartermaster"},{"id":"item:demo-item-1","name":"Synthetic Sword"},{"id":"object:404"}])
    );
    assert_eq!(quest["givers_total"], 3);
    assert_eq!(quest["enders"], json!([{"id":"npc:448","name":"Hogger"}]));
    assert_eq!(quest["rewards"]["value"][0]["name"], "Synthetic Sword");
    assert_eq!(
        page(&content, WorldView::Quests, "synthetic quest")["total"],
        1
    );
    assert_eq!(quest["objectives"][0]["id"], "demo-objective-1");
    assert_eq!(
        quest["objectives"][0]["targets"],
        json!([{"kind":"quest_stage","stage":"demo-objective-1"},{"kind":"acquisition","source":"demo-acq-1"},{"kind":"service","node":"demo-node-1"}])
    );
    assert_eq!(quest["rewards"]["value"][0]["item_id"], "demo-item-1");
    assert_eq!(quest["offers"][0]["acquisition"], "world-acq-quest");
    assert_eq!(quest["offers"][0]["cost"]["status"], "not_applicable");
    if std::env::var_os("UPDATE_WORLD_BROWSER_FIXTURES").is_none() {
        let golden: Value =
            serde_json::from_str(include_str!("../../../fixtures/desktop/world/pages.json"))
                .unwrap();
        assert_eq!(
            golden["entities"], all,
            "regenerate the world-browser golden"
        );
        assert_eq!(golden["entities_hog"], hogger);
        assert_eq!(golden["vendors"], vendors);
        assert_eq!(golden["trainers"], trainers);
        assert_eq!(golden["quests"], quests);
        assert_eq!(golden["loot"], page(&content, WorldView::Loot, ""));
    } else {
        let records: Vec<OriginRef> = serde_json::from_value(json!([{"collection":"items","id":"demo-item-1","variant":"base"},{"collection":"edges","id":"world-edge-inferred"},{"collection":"quests","id":"demo-quest-1"}])).unwrap();
        let origins = world::record_origins(
            &content,
            &DatasetRevision::new("demo-r1").unwrap(),
            &records,
        )
        .unwrap();
        let golden = json!({
            "entities": all,
            "entities_hog": hogger,
            "vendors": vendors,
            "trainers": trainers,
            "quests": quests,
            "loot": page(&content, WorldView::Loot, ""),
            "origins": origins,
        });
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/desktop/world/pages.json"
        );
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
        std::fs::write(path, serde_json::to_string_pretty(&golden).unwrap() + "\n").unwrap();
    }
}

#[test]
fn origin_lookups_follow_evidence_sources_and_confidence() {
    let (app, profile) = setup(&bundle());
    let request = |records: Value, revision: &str| {
        json!({"subject":{"kind":"character","profile_id":profile},"revision":revision,"records":records}).to_string()
    };
    let origins = app
        .record_origins(&request(
            json!([
                {"collection":"items","id":"demo-item-1","variant":"base"},
                {"collection":"items","id":"demo-item-1"},
                {"collection":"acquisitions","id":"world-acq-vendor"},
                {"collection":"acquisitions","id":"demo-acq-1"},
                {"collection":"edges","id":"world-edge-inferred"},
                {"collection":"edges","id":"demo-edge-1"},
                {"collection":"quests","id":"demo-quest-1"},
                {"collection":"objectives","id":"demo-objective-1"},
                {"collection":"dungeons","id":"dungeon-1"},
                {"collection":"entities","id":"npc:448"},
                {"collection":"nodes","id":"demo-node-1"},
                {"collection":"entities","id":"absent"}
            ]),
            "demo-r1",
        ))
        .unwrap();
    assert_eq!(
        serde_json::to_value(&origins.origins).unwrap(),
        json!([
            [],
            null,
            ["wow_classic_era"],
            [],
            ["inferred"],
            [],
            ["vanilla_1_12"],
            [],
            [],
            ["vanilla_1_12"],
            null,
            null
        ])
    );
    assert!(app.record_origins(&request(json!([]), "stale")).is_err());
    let many: Vec<Value> = (0..201)
        .map(|_| json!({"collection":"edges","id":"demo-edge-1"}))
        .collect();
    assert!(
        app.record_origins(&request(json!(many), "demo-r1"))
            .is_err()
    );
    assert!(app.record_origins("{").is_err());
    assert!(app.record_origins(&" ".repeat(128 * 1024 + 1)).is_err());
}

#[test]
fn world_pages_refuse_stale_unbounded_or_invalid_requests() {
    let (app, profile) = setup(&bundle());
    let subject = json!({"kind":"character","profile_id":profile});
    let call = |request: Value| app.world_page(&request.to_string());
    assert!(call(json!({"subject":subject,"view":"entities","revision":"other"})).is_err());
    assert!(call(json!({"subject":subject,"view":"entities","offset":5})).is_err());
    assert!(call(json!({"subject":subject,"view":"entities","query":"x".repeat(65)})).is_err());
    assert!(call(json!({"subject":subject,"view":"planets"})).is_err());
    assert!(app.world_page(&" ".repeat(4097)).is_err());
    let ok =
        call(json!({"subject":subject,"view":"entities","revision":"demo-r1","offset":4})).unwrap();
    assert_eq!(ok.total, 4);
    assert!(ok.rows.is_empty());
    assert_eq!(ok.next_offset, None);
}

#[test]
fn stored_packs_serve_the_same_pages_through_the_application() {
    let (app, profile) = setup(&bundle());
    let content = revision(&bundle());
    for (view, name) in [
        (WorldView::Entities, "entities"),
        (WorldView::Vendors, "vendors"),
        (WorldView::Trainers, "trainers"),
        (WorldView::Quests, "quests"),
        (WorldView::Loot, "loot"),
    ] {
        assert_eq!(app_page(&app, &profile, name, ""), page(&content, view, ""));
    }
}

#[test]
fn large_packs_page_on_the_server() {
    let mut bundle = bundle();
    let entities = bundle["content"]["entities"].as_array_mut().unwrap();
    for i in 0..120 {
        entities.push(entity(
            &format!("npc:{}", 10000 + i),
            "npc",
            &format!("Gnoll {i:03}"),
            &[],
            &[],
            "world-evidence-112",
        ));
    }
    let (app, profile) = setup(&bundle);
    let subject = json!({"kind":"character","profile_id":profile});
    let first: Value = serde_json::to_value(
        app.world_page(&json!({"subject":subject,"view":"entities","query":"gnoll"}).to_string())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(first["total"], 120);
    assert_eq!(first["rows"].as_array().unwrap().len(), 50);
    assert_eq!(first["next_offset"], 50);
    assert_eq!(first["rows"][0]["name"], "Gnoll 000");
    let last = app
        .world_page(
            &json!({"subject":subject,"view":"entities","query":"gnoll","offset":100,"revision":"demo-r1"})
                .to_string(),
        )
        .unwrap();
    assert_eq!(last.rows.len(), 20);
    assert_eq!(last.next_offset, None);
}

#[test]
fn without_active_content_the_world_is_empty_not_an_error() {
    let app = SnapshotApplication::in_memory().unwrap();
    let snapshot: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    let profile = app
        .import_text(
            &format!("ANAV1:{snapshot}"),
            Some(UtcMillis::new(NOW).unwrap()),
        )
        .unwrap()
        .profile_id;
    let subject = json!({"kind":"character","profile_id":profile});
    let empty = app
        .world_page(&json!({"subject":subject,"view":"vendors"}).to_string())
        .unwrap();
    assert!(empty.revision.is_none());
    assert_eq!(empty.total, 0);
    assert!(
        app.world_page(&json!({"subject":subject,"view":"vendors","offset":1}).to_string())
            .is_err()
    );
    assert!(
        app.world_page(
            &json!({"subject":subject,"view":"vendors","query":"y".repeat(65)}).to_string()
        )
        .is_err()
    );
    assert!(
        app.record_origins(
            &json!({"subject":subject,"revision":"demo-r1","records":[]}).to_string()
        )
        .is_err()
    );
}

/// Measurement over the real whole-world pack; needs the Git LFS catalog.
/// `NAVZEROTH_WORLD_CATALOG=content/world/1.60.1.70450/catalog.json cargo test --release
/// -p azeroth-app --test world_browser -- --ignored --nocapture`
#[test]
#[ignore = "needs the Git LFS whole-world catalog"]
fn whole_world_pages_are_fast() {
    let Ok(path) = std::env::var("NAVZEROTH_WORLD_CATALOG") else {
        return;
    };
    let started = std::time::Instant::now();
    let content = azeroth_core::content_v2::decode(&std::fs::read(path).unwrap()).unwrap();
    eprintln!("decode {:?}", started.elapsed());
    let content = ActiveContentRevision::V2(Arc::new(ContentRevisionV2 {
        content,
        sources: vec![],
        conflicts: vec![],
    }));
    for (view, query) in [
        (WorldView::Loot, ""),
        (WorldView::Loot, "hogger"),
        (WorldView::Quests, "wolf"),
        (WorldView::Entities, "deadmines"),
        (WorldView::Vendors, ""),
    ] {
        let started = std::time::Instant::now();
        let page = world::world_page(&content, view, query, 0, None).unwrap();
        let bytes = serde_json::to_vec(&page).unwrap().len();
        eprintln!(
            "{view:?} {query:?}: total {} rows {} json {} KiB in {:?}",
            page.total,
            page.rows.len(),
            bytes / 1024,
            started.elapsed()
        );
    }
}
