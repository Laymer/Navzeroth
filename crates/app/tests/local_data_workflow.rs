use azeroth_app::local_data::LocalDataService;
use azeroth_core::{
    local_data::PerformancePreset,
    local_data_workflow::{MaterialKind, StartupMode},
};
use std::{
    fs,
    time::{Duration, Instant},
};
fn write_build(path: &std::path::Path, build: u32) {
    fs::write(path.join(".build.info"),format!("Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.{build}|{}|{}|enUS text\n","a".repeat(32),"b".repeat(32))).unwrap();
}
#[test]
fn opt_in_stages_metadata_without_activation_and_opt_out_stops_detection() {
    let root = std::env::temp_dir().join(format!("nav-workflow-service-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).unwrap();
    let source = root.join("client");
    fs::create_dir(&source).unwrap();
    write_build(&source, 70425);
    let private = root.join("private");
    let service = LocalDataService::new(private.clone());
    let selected = service.select(&source).unwrap();
    assert!(service.change_detectors().unwrap().is_empty());
    let prefs = service
        .finish_startup(0, StartupMode::LocalResearch, true)
        .unwrap();
    let prefs = service
        .save_workflow_choices(
            &selected.selection_id,
            prefs.revision,
            vec!["enUS".into()],
            PerformancePreset::Small,
            true,
        )
        .unwrap();
    let baseline = prefs.preferences.selections[&selected.selection_id]
        .baseline_sha256
        .clone();
    // Legacy preview saves its own preset independently; the detector must use
    // workflow choices after restart and remain stable during status polling.
    let _ = service.preview(&selected.selection_id, PerformancePreset::Large);
    drop(service);
    let service = LocalDataService::new(private.clone());
    write_build(&source, 70450);
    let start = Instant::now();
    let candidate = loop {
        let state = service.change_detectors().unwrap();
        if let Some(id) = state.iter().find_map(|s| s.candidate_sha256.clone()) {
            break id;
        }
        assert!(start.elapsed() < Duration::from_secs(8));
        std::thread::sleep(Duration::from_millis(50));
    };
    let staged = azeroth_local_data::changes::verify(&private, &candidate).unwrap();
    assert_eq!(staged.metadata.build, 70450);
    assert!(!staged.activated && staged.local_only);
    assert_eq!(service.artifact_page("catalogs", 0, 200).unwrap().total, 0);
    let prefs = service
        .save_workflow_choices(
            &selected.selection_id,
            prefs.revision,
            vec!["enUS".into()],
            PerformancePreset::Large,
            true,
        )
        .unwrap();
    assert_eq!(
        prefs.preferences.selections[&selected.selection_id].baseline_sha256,
        baseline
    );
    let prefs = service
        .acknowledge_build_change(&selected.selection_id, prefs.revision, &candidate)
        .unwrap();
    assert_eq!(
        prefs.preferences.selections[&selected.selection_id].baseline_sha256,
        Some(staged.current_sha256)
    );
    let prefs = service
        .save_workflow_choices(
            &selected.selection_id,
            prefs.revision,
            vec!["enUS".into()],
            PerformancePreset::Small,
            false,
        )
        .unwrap();
    assert!(service.change_detectors().unwrap().is_empty());
    let guide = root.join("private-guide.lua");
    fs::write(&guide, "error('must never execute')").unwrap();
    let material = service
        .select_material(&guide, MaterialKind::Guide)
        .unwrap();
    assert!(
        !serde_json::to_string(&service.material_selections().unwrap())
            .unwrap()
            .contains(root.to_str().unwrap())
    );
    drop(service);
    let service = LocalDataService::new(private);
    assert_eq!(
        service.workflow_preferences().unwrap().revision,
        prefs.revision
    );
    assert!(
        service
            .material_selections()
            .unwrap()
            .iter()
            .any(|m| m.id == material.id)
    );
    assert!(service.change_detectors().unwrap().is_empty());
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn research_asset_boundary_refuses_paths_queries_methods_and_unknown_revisions() {
    let service = LocalDataService::new(std::env::temp_dir().join("nav-absent-media-boundary"));
    let path = format!("/research/{}/{}", "a".repeat(64), "b".repeat(64));
    assert!(service.research_asset("GET", &path, false).is_none());
    assert!(service.research_asset("POST", &path, false).is_none());
    assert!(service.research_asset("GET", &path, true).is_none());
    assert!(
        service
            .research_asset("GET", "/research/../../private", false)
            .is_none()
    );
    assert!(
        service
            .research_asset("GET", &format!("{path}/extra"), false)
            .is_none()
    );
    assert!(
        service
            .research_asset("GET", &path.replace('a', "%61"), false)
            .is_none()
    );
}
