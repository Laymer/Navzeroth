use azeroth_app::local_data::LocalDataService;
use azeroth_core::local_data::PerformancePreset;
use std::fs;
#[test]
fn selected_installation_and_preset_survive_service_recreation() {
    let root = std::env::temp_dir().join(format!("nav-local-service-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let source = root.join("client");
    let output = root.join("private");
    fs::create_dir(&source).unwrap();
    let info = format!(
        "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70450|{}|{}|enUS text\n",
        "a".repeat(32),
        "b".repeat(32)
    );
    fs::write(source.join(".build.info"), info).unwrap();
    let service = LocalDataService::new(output.clone());
    let view = service.select(&source).unwrap();
    service
        .preview(&view.selection_id, PerformancePreset::Small)
        .unwrap();
    drop(service);
    let service = LocalDataService::new(output);
    let saved = service.selections().unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].preset, PerformancePreset::Small);
    let restored = service.restore_selection(&view.selection_id).unwrap();
    assert_eq!(restored.preset, PerformancePreset::Small);
    assert_eq!(restored.inspection.metadata.build, 70450);
    assert!(
        !serde_json::to_string(&saved)
            .unwrap()
            .contains(source.to_str().unwrap())
    );
    assert!(service.restore_selection("missing").is_err());
    drop(service);
    fs::remove_dir_all(root).unwrap();
}
