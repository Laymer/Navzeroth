use azeroth_core::{entities::*, types::*};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

fn check<T: DeserializeOwned + Serialize>(good: Value, bad: Value) {
    let v: T = serde_json::from_value(good).unwrap();
    assert!(serde_json::from_value::<T>(serde_json::to_value(v).unwrap()).is_ok());
    assert!(serde_json::from_value::<T>(bad).is_err());
}
fn context() -> Value {
    json!({"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"demo"})
}
fn map(id: &str) -> Value {
    json!({"context":context(),"map_id":id,"floor":"0","instance":null})
}
fn layer(id: &str, w: u32, h: u32, divisor: u32) -> Value {
    json!({"id":id,"kind":"roads_traced","file":format!("atlas/{id}.png"),"sha256":"a".repeat(64),
        "width_px":w,"height_px":h,"frame_divisor":divisor,"registration_offset_px":{"x":0,"y":0}})
}
fn image() -> Value {
    json!({"id":"img1","map":map("m1"),"source":"src1","evidence":[],
        "applicable_builds":[{"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"evidence":[]}],
        "width_px":100,"height_px":51,
        "layers":[layer("full",100,51,1),layer("half",50,26,2)],
        "pins":[{"id":"p1","label":"Pin","kind":{"kind":"settlement"},"pixel":{"x":100,"y":51},"measured_on":"full","evidence":[]}],
        "calibration":{"status":"unknown"},"calibration_evidence":[],"control_points":[],"gaps":"none"})
}

#[test]
fn primitives_validate_digests_paths_and_extents() {
    assert!(Sha256Hex::new("0".repeat(64)).is_ok());
    assert!(Sha256Hex::new("0".repeat(63)).is_err());
    assert!(Sha256Hex::new("A".repeat(64)).is_err());
    for good in [
        "a.png",
        "atlas/eastern-kingdoms/layers/roads.png",
        "x_1-2.tar.gz",
    ] {
        assert!(AssetPath::new(good).is_ok(), "{good}");
    }
    for bad in [
        "",
        "/a.png",
        "a/../b.png",
        "./a.png",
        "a//b",
        "A.png",
        "a\\b.png",
        "c:a",
        "con.png",
        "layers/nul",
        "a.",
        ".hidden",
        "a b",
    ] {
        assert!(AssetPath::new(bad).is_err(), "{bad}");
    }
    assert!(AssetPath::new("a".repeat(257)).is_err());
    assert!(PixelExtent::new(0).is_err());
    assert!(PixelExtent::new(16_384).is_ok());
    assert!(PixelExtent::new(16_385).is_err());
}

#[test]
fn map_images_validate_layers_pins_builds_and_calibration() {
    check::<MapImage>(image(), json!({"id": "img1"}));
    let mut bad = image();
    bad["layers"][1]["width_px"] = json!(49);
    check::<MapImage>(image(), bad);
    let mut bad = image();
    bad["layers"] = json!([]);
    check::<MapImage>(image(), bad);
    let mut bad = image();
    bad["pins"][0]["pixel"]["y"] = json!(51.1);
    check::<MapImage>(image(), bad);
    let mut bad = image();
    bad["pins"][0]["measured_on"] = json!("missing");
    check::<MapImage>(image(), bad);
    let mut bad = image();
    bad["applicable_builds"] = json!([]);
    check::<MapImage>(image(), bad);
    let mut bad = image();
    bad["applicable_builds"].as_array_mut().unwrap().push(
        json!({"build":{"edition":"wow_forever","build":"LATER","interface":"X"},"evidence":[]}),
    );
    check::<MapImage>(image(), bad);
    let mut bad = image();
    bad["applicable_builds"]
        .as_array_mut()
        .unwrap()
        .push(json!({"build":{"edition":"other","build":"SYNTHETIC","interface":"SYNTHETIC"},"evidence":["e1"]}));
    check::<MapImage>(image(), bad);
    // Known calibration needs pixel units and evidence; a residual needs a calibration.
    let mut good = image();
    good["calibration"] = json!({"status":"known","value":{"units":"pixel","scale_x":100,"scale_y":51,"offset_x":0,"offset_y":0}});
    good["calibration_evidence"] = json!(["e1"]);
    let mut bad = good.clone();
    bad["calibration"]["value"]["units"] = json!("yards");
    check::<MapImage>(good.clone(), bad);
    let mut bad = good.clone();
    bad["calibration_evidence"] = json!([]);
    check::<MapImage>(good.clone(), bad);
    let mut bad = image();
    bad["control_points"] =
        json!([{"pin":"p1","node":"n1","residual_px":{"status":"known","value":0}}]);
    check::<MapImage>(good, bad);
    let mut bad = image();
    bad["control_points"] = json!([{"pin":"p9","node":"n1","residual_px":{"status":"unknown"}}]);
    check::<MapImage>(image(), bad);
}

#[test]
fn map_embeddings_require_distinct_maps_one_context_and_positive_area() {
    let good = json!({"child":map("zone"),"parent":map("continent"),
        "bounds":{"min_x":0.2,"min_y":0.3,"max_x":0.4,"max_y":0.6},"evidence":[]});
    let mut bad = good.clone();
    bad["parent"] = map("zone");
    check::<MapEmbedding>(good.clone(), bad);
    let mut bad = good.clone();
    bad["bounds"]["max_x"] = json!(0.2);
    check::<MapEmbedding>(good.clone(), bad);
    let mut bad = good.clone();
    bad["parent"]["context"]["namespace"] = json!("reference");
    check::<MapEmbedding>(good.clone(), bad);
    let embedding: MapEmbedding = serde_json::from_value(good).unwrap();
    let (x, y) = embedding
        .to_parent(Normalized::new(0.5).unwrap(), Normalized::new(1.0).unwrap())
        .unwrap();
    assert!((x.get() - 0.3).abs() < 1e-12 && (y.get() - 0.6).abs() < 1e-12);
}

#[test]
fn control_point_residuals_cannot_be_negative() {
    let mut good = image();
    good["calibration"] = json!({"status":"known","value":{"units":"pixel","scale_x":100,"scale_y":51,"offset_x":0,"offset_y":0}});
    good["calibration_evidence"] = json!(["e1"]);
    good["control_points"] =
        json!([{"pin":"p1","node":"n1","residual_px":{"status":"known","value":0.25}}]);
    let mut bad = good.clone();
    bad["control_points"][0]["residual_px"]["value"] = json!(-0.4);
    check::<MapImage>(good, bad);
}
