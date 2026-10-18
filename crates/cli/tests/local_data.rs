use std::{
    fs,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn unsupported_duplicate_and_missing_options_fail_as_json_without_echoing_inputs() {
    for args in [
        vec!["local-data"],
        vec!["local-data", "extract"],
        vec!["local-data", "inspect", "--installation", "private-input"],
        vec!["local-data", "inspect", "--product", "a", "--product", "b"],
        vec!["local-data", "inspect", "--installation", "--product"],
        vec!["local-data", "plan", "--preset", "unlimited"],
        vec!["local-data", "inspect", "--destination", "private-input"],
    ] {
        let output = cli(&args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stderr.is_empty());
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["error"], "usage");
        assert!(
            !String::from_utf8(output.stdout)
                .unwrap()
                .contains("private-input")
        );
    }
}

#[test]
fn inspect_and_plan_exit_codes_preserve_unknown_and_unavailable_capabilities() {
    let root = std::env::temp_dir().join(format!(
        "navzeroth-local-cli-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    let client = root.join("client");
    let output = root.join("output");
    fs::create_dir(&client).unwrap();
    fs::create_dir(&output).unwrap();
    let info = format!(
        "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70425|{}|{}|enUS text\n",
        "a".repeat(32),
        "b".repeat(32)
    );
    fs::write(client.join(".build.info"), info).unwrap();
    let selected = client.to_str().unwrap();
    let dest = output.to_str().unwrap();
    let inspected = cli(&[
        "local-data",
        "inspect",
        "--installation",
        selected,
        "--product",
        "wow_classic_beta",
    ]);
    assert!(inspected.status.success());
    let value: serde_json::Value = serde_json::from_slice(&inspected.stdout).unwrap();
    assert_eq!(value["metadata"]["build"], 70425);
    let planned = cli(&[
        "local-data",
        "plan",
        "--product",
        "wow_classic_beta",
        "--installation",
        selected,
        "--destination",
        dest,
    ]);
    assert_eq!(planned.status.code(), Some(3));
    let value: serde_json::Value = serde_json::from_slice(&planned.stdout).unwrap();
    assert_eq!(value["preset"], "max");
    assert_eq!(value["extraction_ready"], false);
    assert!(value["required_storage_bytes"].is_null());
    assert_eq!(value["actual_workers"], 0);
    assert_eq!(fs::read_dir(output).unwrap().count(), 0);
    let bad = cli(&[
        "local-data",
        "inspect",
        "--product",
        "missing",
        "--installation",
        selected,
    ]);
    assert_eq!(bad.status.code(), Some(1));
    assert!(!String::from_utf8(bad.stdout).unwrap().contains(selected));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn oversized_internal_worker_frame_exits_before_parsing_or_touching_inputs() {
    use std::io::Write;
    use std::process::Stdio;
    let mut worker = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"))
        .arg(azeroth_local_data::worker::WORKER_ARGUMENT)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    worker
        .stdin
        .take()
        .unwrap()
        .write_all(&u32::MAX.to_le_bytes())
        .unwrap();
    let output = worker.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn isolated_worker_failure_and_cancellation_preserve_previous_active_state() {
    use azeroth_local_data::worker::{ExtractionRequest, ExtractionWorker, WorkerEvent};
    let root = std::env::temp_dir().join(format!(
        "navzeroth-worker-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    let source = root.join("source");
    let destination = root.join("private");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&destination).unwrap();
    let original = b"previous active state, never extraction output";
    fs::write(destination.join("active-marker"), original).unwrap();
    let request = ExtractionRequest {
        protocol_version: 1,
        resume: false,
        request_id: "worker-test".into(),
        installation: source.clone(),
        product: "wow_classic_beta".into(),
        destination: destination.clone(),
        preset: azeroth_core::local_data::PerformancePreset::Small,
        tables: vec![],
    };
    let mut worker = ExtractionWorker::start_executable(
        std::path::Path::new(env!("CARGO_BIN_EXE_azeroth-cli")),
        &request,
    )
    .unwrap();
    let event = worker.next_event().unwrap();
    assert!(matches!(event, WorkerEvent::Failed { .. }));
    let text = serde_json::to_string(&event).unwrap();
    assert!(!text.contains(&root.to_string_lossy().to_string()));
    let mut worker = ExtractionWorker::start_executable(
        std::path::Path::new(env!("CARGO_BIN_EXE_azeroth-cli")),
        &request,
    )
    .unwrap();
    worker.control().cancel().unwrap();
    worker.cancel().unwrap();
    assert_eq!(
        fs::read(destination.join("active-marker")).unwrap(),
        original
    );
    assert_eq!(fs::read_dir(&source).unwrap().count(), 0);
    fs::remove_dir_all(root).unwrap();
}

/// Extraction admits exactly 70374, 70425 and 70450. An admitted build passes the
/// build gate and stops at the next check (an unreviewed table, before any
/// archive is opened); an unknown build stops at the gate with no fallback.
#[test]
fn extraction_admits_70450_as_development_input_and_refuses_unknown_builds() {
    use azeroth_local_data::worker::{ExtractionRequest, ExtractionWorker, WorkerEvent};
    for (build, expected) in [
        (70374, "Requested table is absent from reviewed metadata"),
        (70425, "Requested table is absent from reviewed metadata"),
        (70450, "Requested table is absent from reviewed metadata"),
        (70449, "Build unsupported; no nearest-build fallback"),
        (70451, "Build unsupported; no nearest-build fallback"),
    ] {
        let root = std::env::temp_dir().join(format!(
            "navzeroth-admission-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let source = root.join("source");
        let destination = root.join("private");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(
            source.join(".build.info"),
            format!(
                "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.{build}|{}|{}|enUS text\n",
                "a".repeat(32),
                "b".repeat(32)
            ),
        )
        .unwrap();
        let request = ExtractionRequest {
            protocol_version: 1,
            resume: false,
            request_id: format!("admission-{build}"),
            installation: source.clone(),
            product: "wow_classic_beta".into(),
            destination: destination.clone(),
            preset: azeroth_core::local_data::PerformancePreset::Small,
            tables: vec!["NotAReviewedTable".into()],
        };
        let mut worker = ExtractionWorker::start_executable(
            std::path::Path::new(env!("CARGO_BIN_EXE_azeroth-cli")),
            &request,
        )
        .unwrap();
        match worker.next_event().unwrap() {
            WorkerEvent::Failed { code, .. } => assert_eq!(code, expected, "build {build}"),
            other => panic!("build {build}: unexpected {other:?}"),
        }
        // Nothing was written to the private destination for any build.
        assert_eq!(
            fs::read_dir(&destination).unwrap().count(),
            0,
            "build {build}"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn projection_commands_require_explicit_options_and_refuse_unknown_catalogs() {
    for args in [
        vec!["local-data", "project", "--destination", "private-input"],
        vec![
            "local-data",
            "project",
            "--destination",
            "private-input",
            "--catalog",
            "x",
        ],
        vec!["local-data", "projection-verify", "--projection", "x"],
        vec![
            "local-data",
            "project",
            "--destination",
            "d",
            "--catalog",
            "x",
            "--locale",
            "enUS",
            "--preset",
            "max",
        ],
    ] {
        let output = cli(&args);
        assert_eq!(output.status.code(), Some(2));
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["error"], "usage");
    }
    let root = std::env::temp_dir().join(format!(
        "navzeroth-local-cli-projection-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let dest = root.to_str().unwrap();
    let digest = "0".repeat(64);
    for args in [
        vec![
            "local-data",
            "project",
            "--destination",
            dest,
            "--catalog",
            &digest,
            "--locale",
            "enUS",
        ],
        vec![
            "local-data",
            "projection-verify",
            "--destination",
            dest,
            "--projection",
            &digest,
        ],
    ] {
        let output = cli(&args);
        assert_eq!(output.status.code(), Some(1));
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(!text.contains(dest));
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(value["error"].is_string());
    }
    // A refused projection never initialises a store.
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_and_projection_verify_succeed_on_a_synthetic_store() {
    use azeroth_core::{
        local_data::{CASCLIB_REVISION, InstallationMetadata, InventoryDisposition},
        local_data_catalog::{DistributionClass, ResearchCatalog, TableInventory},
    };
    use azeroth_local_data::tables::{Cell, DecodedTable, ResearchRow, TableHeader};
    use sha2::{Digest, Sha256};
    let hash = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
    let root = std::env::temp_dir().join(format!(
        "navzeroth-local-cli-projected-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    }
    fs::create_dir(root.join("objects")).unwrap();
    fs::create_dir(root.join("catalogs")).unwrap();
    let decoded = DecodedTable {
        format_version: 1,
        header: TableHeader {
            format: "WDC5".into(),
            records: 1,
            fields: 1,
            record_bytes: 4,
            table_hash: 0,
            layout_hash: 0,
            locale_mask: 2,
            sections: 1,
        },
        rows: vec![ResearchRow {
            id: 1,
            fields: [("Name_lang".to_owned(), Cell::Text("Synthetic".into()))].into(),
            copied_from: None,
        }],
        gaps: vec![],
    };
    let raw = b"synthetic raw".to_vec();
    let json = serde_json::to_vec(&decoded).unwrap();
    fs::write(root.join("objects").join(hash(&raw)), &raw).unwrap();
    fs::write(root.join("objects").join(hash(&json)), &json).unwrap();
    let catalog = ResearchCatalog {
        format_version: 1,
        classification: DistributionClass::LocalOnly,
        installation: InstallationMetadata {
            product: "wow_classic_beta".into(),
            version: "1.60.1.70425".into(),
            build: 70425,
            build_key: "a".repeat(32),
            cdn_key: "b".repeat(32),
            declared_locales: vec!["enUS".into()],
        },
        build_info_sha256: "c".repeat(64),
        extractor_version: "synthetic".into(),
        native_revision: CASCLIB_REVISION.into(),
        metadata_sha256: azeroth_local_data::metadata::METADATA_SHA256.into(),
        parser_profile: "synthetic".into(),
        extractor_profile_sha256: None,
        archive_statistics: None,
        input_hashes: Default::default(),
        tables: vec![TableInventory {
            table: "ChrClasses".into(),
            locale: Some("enUS".into()),
            file_data_id: Some(1),
            disposition: InventoryDisposition::Decoded,
            reason: "research_decoded".into(),
            retained_sha256: Some(hash(&raw)),
            decoded_sha256: Some(hash(&json)),
            encoded_bytes: Some(raw.len() as u64),
            records: Some(1),
            layout_hash: Some("afc9b0c2".into()),
        }],
        gaps: vec![],
        gameplay_projection_sha256: None,
    };
    let manifest = serde_json::to_vec(&catalog).unwrap();
    let digest = hash(&manifest);
    fs::write(root.join("catalogs").join(&digest), &manifest).unwrap();
    let dest = root.to_str().unwrap();
    let output = cli(&[
        "local-data",
        "project",
        "--destination",
        dest,
        "--catalog",
        &digest,
        "--locale",
        "enUS",
    ]);
    assert_eq!(output.status.code(), Some(3));
    let summary: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["catalog_sha256"], digest.as_str());
    assert_eq!(summary["status"], "research_only");
    assert_eq!(summary["activated"], false);
    assert_eq!(summary["counts"]["class_roles_classes"], 1);
    let projection = summary["projection_sha256"].as_str().unwrap();
    assert!(root.join("projections").join(projection).is_file());
    let output = cli(&[
        "local-data",
        "projection-verify",
        "--destination",
        dest,
        "--projection",
        projection,
    ]);
    assert_eq!(output.status.code(), Some(0));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["verified"], true);
    assert_eq!(report["reproduction"], "reproduced");
    let built = cli(&[
        "local-data",
        "guide-facts-build",
        "--destination",
        dest,
        "--projection",
        projection,
    ]);
    assert_eq!(built.status.code(), Some(0));
    let built: serde_json::Value = serde_json::from_slice(&built.stdout).unwrap();
    let facts = built["guide_facts_sha256"].as_str().unwrap();
    let verified = cli(&[
        "local-data",
        "guide-facts-verify",
        "--destination",
        dest,
        "--guide-facts",
        facts,
    ]);
    assert_eq!(verified.status.code(), Some(0));
    let delivered = cli(&[
        "local-data",
        "delivery-build",
        "--destination",
        dest,
        "--projection",
        projection,
    ]);
    assert_eq!(delivered.status.code(), Some(0));
    let delivered: serde_json::Value = serde_json::from_slice(&delivered.stdout).unwrap();
    let delivery = delivered["delivery_sha256"].as_str().unwrap();
    let lookup = cli(&[
        "local-data",
        "delivery-page",
        "--destination",
        dest,
        "--delivery",
        delivery,
        "--domain",
        "SpellName",
        "--offset",
        "0",
        "--limit",
        "200",
    ]);
    assert_eq!(lookup.status.code(), Some(0));
    let lookup: serde_json::Value = serde_json::from_slice(&lookup.stdout).unwrap();
    assert!(lookup["total"].is_null());
    assert_eq!(lookup["guard"]["actionable_advice"], false);
    let selected = cli(&[
        "local-data",
        "delivery-activate",
        "--destination",
        dest,
        "--delivery",
        delivery,
        "--expected",
        "0",
    ]);
    assert_eq!(selected.status.code(), Some(0));
    let stale = cli(&[
        "local-data",
        "delivery-activate",
        "--destination",
        dest,
        "--delivery",
        delivery,
        "--expected",
        "0",
    ]);
    assert_eq!(stale.status.code(), Some(1));
    let active = cli(&[
        "local-data",
        "delivery-active",
        "--destination",
        dest,
        "--delivery",
        delivery,
    ]);
    let active: serde_json::Value = serde_json::from_slice(&active.stdout).unwrap();
    assert_eq!(active["generation"], 1);
    assert_eq!(active["delivery_sha256"], delivery);
    let tooltip = cli(&[
        "local-data",
        "tooltip",
        "--destination",
        dest,
        "--projection",
        projection,
        "--spell",
        "1",
    ]);
    assert_eq!(tooltip.status.code(), Some(0));
    let tooltip: serde_json::Value = serde_json::from_slice(&tooltip.stdout).unwrap();
    assert_eq!(tooltip["gaps"], serde_json::json!(["missing_document"]));
    assert_eq!(tooltip["verified_for_gameplay"], false);
    let discovery = cli(&[
        "local-data",
        "media-discover",
        "--destination",
        dest,
        "--catalog",
        &digest,
        "--locale",
        "enUS",
    ]);
    assert_eq!(discovery.status.code(), Some(0));
    let discovery: serde_json::Value = serde_json::from_slice(&discovery.stdout).unwrap();
    assert_eq!(discovery["candidates"], serde_json::json!([]));
    let refused = cli(&[
        "local-data",
        "media-extract",
        "--destination",
        dest,
        "--installation",
        dest,
        "--catalog",
        &digest,
        "--locale",
        "enUS",
        "--ids",
        "1",
        "--preset",
        "small",
    ]);
    assert_eq!(refused.status.code(), Some(1));
    assert!(root.join("projections").join(projection).is_file());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn competing_worker_cannot_fail_an_existing_job_with_the_same_id() {
    use azeroth_core::{
        local_data::PerformancePreset,
        local_data_lifecycle::{Checkpoint, CheckpointBinding, JobState},
    };
    use azeroth_local_data::worker::{ExtractionRequest, ExtractionWorker, WorkerEvent};
    let root = std::env::temp_dir().join(format!(
        "nav-competing-worker-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    let source = root.join("client");
    let output = root.join("output");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&output).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&output, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let info = format!(
        "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70450|{}|{}|enUS text\n",
        "a".repeat(32),
        "b".repeat(32)
    );
    fs::write(source.join(".build.info"), info).unwrap();
    for key in ["a".repeat(32), "b".repeat(32)] {
        let dir = source.join("Data/config").join(&key[..2]).join(&key[2..4]);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(key), b"synthetic-config").unwrap();
    }
    fs::create_dir_all(source.join("Data/data")).unwrap();
    fs::write(source.join("Data/data/data.000"), b"synthetic-archive").unwrap();
    let mut db = azeroth_local_data::lifecycle::index(&output).unwrap();
    let cp = Checkpoint {
        format_version: 1,
        binding: CheckpointBinding {
            format_version: 1,
            source_sha256: "a".repeat(64),
            extractor_sha256: "b".repeat(64),
            selection_sha256: "c".repeat(64),
            preset: PerformancePreset::Small,
            total: 1,
        },
        tables: vec![],
        gaps: vec![],
    };
    db.begin("same-job", "private-request", &cp, false).unwrap();
    let lock = std::fs::File::create(output.join("writer.lock")).unwrap();
    lock.lock().unwrap();
    let request = ExtractionRequest {
        protocol_version: 2,
        resume: false,
        request_id: "same-job".into(),
        installation: source,
        product: "wow_classic_beta".into(),
        destination: output.clone(),
        preset: PerformancePreset::Small,
        tables: vec!["Map".into()],
    };
    let mut worker = ExtractionWorker::start_executable(
        std::path::Path::new(env!("CARGO_BIN_EXE_azeroth-cli")),
        &request,
    )
    .unwrap();
    assert!(matches!(
        worker.next_event().unwrap(),
        WorkerEvent::Failed { .. }
    ));
    assert_eq!(db.status("same-job").unwrap().state, JobState::Running);
    drop(worker);
    drop(lock);
    drop(db);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn durable_job_commands_recover_cancel_and_forget_without_exposing_requests() {
    use azeroth_core::{
        local_data::PerformancePreset,
        local_data_lifecycle::{Checkpoint, CheckpointBinding},
    };
    let output = std::env::temp_dir().join(format!(
        "nav-cli-lifecycle-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&output).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&output, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let mut db = azeroth_local_data::lifecycle::index(&output).unwrap();
    let cp = Checkpoint {
        format_version: 1,
        binding: CheckpointBinding {
            format_version: 1,
            source_sha256: "a".repeat(64),
            extractor_sha256: "b".repeat(64),
            selection_sha256: "c".repeat(64),
            preset: PerformancePreset::Small,
            total: 1,
        },
        tables: vec![],
        gaps: vec![],
    };
    db.begin("interrupted", "private-path-never-returned", &cp, false)
        .unwrap();
    let destination = output.to_str().unwrap();
    let result = cli(&[
        "local-data",
        "status",
        "--destination",
        destination,
        "--job",
        "interrupted",
    ]);
    assert_eq!(result.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["state"], "interrupted");
    assert!(
        !String::from_utf8(result.stdout)
            .unwrap()
            .contains("private-path")
    );
    let invalid = cli(&[
        "local-data",
        "resume",
        "--destination",
        destination,
        "--job",
        "interrupted",
    ]);
    assert_eq!(invalid.status.code(), Some(1));
    db.begin("cancelled", "private", &cp, false).unwrap();
    let cancelled = cli(&[
        "local-data",
        "cancel",
        "--destination",
        destination,
        "--job",
        "cancelled",
    ]);
    assert_eq!(cancelled.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&cancelled.stdout).unwrap();
    assert_eq!(value["state"], "cancelled");
    let forgotten = cli(&[
        "local-data",
        "forget",
        "--destination",
        destination,
        "--job",
        "cancelled",
    ]);
    assert!(forgotten.status.success());
    assert!(db.status("cancelled").is_err());
    drop(db);
    fs::remove_dir_all(output).unwrap();
}

#[test]
fn preparing_progress_is_durable_and_cancellable_before_archive_fingerprinting() {
    use azeroth_core::local_data::PerformancePreset;
    use azeroth_local_data::worker::{ExtractionRequest, ExtractionWorker, WorkerEvent};
    let root = std::env::temp_dir().join(format!(
        "nav-prepare-worker-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let source = root.join("client");
    let output = root.join("private");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir(&output).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&output, fs::Permissions::from_mode(0o700)).unwrap();
    }
    fs::write(source.join(".build.info"),format!("Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70450|{}|{}|enUS text\n","a".repeat(32),"b".repeat(32))).unwrap();
    let request = ExtractionRequest {
        protocol_version: 2,
        resume: false,
        request_id: "preparing-job".into(),
        installation: source,
        product: "wow_classic_beta".into(),
        destination: output.clone(),
        preset: PerformancePreset::Small,
        tables: vec!["Map".into()],
    };
    let mut worker = ExtractionWorker::start_executable(
        std::path::Path::new(env!("CARGO_BIN_EXE_azeroth-cli")),
        &request,
    )
    .unwrap();
    assert!(matches!(
        worker.next_event().unwrap(),
        WorkerEvent::Progress {
            completed: 0,
            total: 0,
            ..
        }
    ));
    // The ID exists before any required archive/config files are even present.
    let status = azeroth_local_data::lifecycle::index(&output)
        .unwrap()
        .status("preparing-job")
        .unwrap();
    assert_eq!(status.job_id, "preparing-job");
    worker.cancel().unwrap();
    drop(worker);
    let status = azeroth_local_data::lifecycle::status(&output, "preparing-job").unwrap();
    assert!(matches!(
        status.state,
        azeroth_core::local_data_lifecycle::JobState::Interrupted
            | azeroth_core::local_data_lifecycle::JobState::Failed
    ));
    fs::remove_dir_all(root).unwrap();
}
