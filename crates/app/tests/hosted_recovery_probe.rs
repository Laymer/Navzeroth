//! Explicit local diagnostic only: manual engineering input, no acceptance/proof output.
use azeroth_app::{bundled_content, recovery::DatabaseHost};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
fn timed<T>(name: &str, run: impl FnOnce() -> T) -> T {
    eprintln!("{}", json!({"phase":name,"event":"start"}));
    let start = Instant::now();
    let result = run();
    eprintln!(
        "{}",
        json!({"phase":name,"event":"complete","elapsed_ms":start.elapsed().as_millis()})
    );
    result
}
fn ready(host: &DatabaseHost) {
    let start = Instant::now();
    loop {
        let status = host.status().unwrap();
        if status.state == "ready" {
            assert!(status.error.is_none());
            eprintln!(
                "{}",
                json!({"startup_measurements":status.startup_measurements})
            );
            return;
        }
        assert_eq!(status.state, "checking");
        assert!(start.elapsed() < Duration::from_secs(1200));
        std::thread::sleep(Duration::from_millis(50));
    }
}
fn state(host: &DatabaseHost, profile: &str) -> Value {
    host.with_application(|app| {
        let active=app.active_content(profile)?.expect("actual content required");
        Ok(json!({"profiles":app.profiles()?,"settings":app.app_settings()?.settings,"key":active.key,"accepts_inferred_travel":active.accepts_inferred_travel,"bundle_defaults":active.bundle_defaults}))
    }).unwrap()
}
#[test]
#[ignore = "Engineering only: explicit verified historical full bundle and disposable directory"]
fn hosted_historical_fullpack_recovery() {
    let source = std::env::var("NAVZEROTH_RECOVERY_SOURCE_ROOT").unwrap();
    let bundle = std::env::var("NAVZEROTH_RECOVERY_BUNDLE").unwrap();
    let output = std::env::var("NAVZEROTH_RECOVERY_OUTPUT").unwrap();
    let output = Path::new(&output);
    std::fs::create_dir(output).unwrap();
    let identity =
        std::fs::read(Path::new(&source).join("release/v1/bundled-content-build-identity.json"))
            .unwrap();
    let policy = std::fs::read(Path::new(&source).join("content/bundled/signers-v1.json")).unwrap();
    let database = output.join("snapshots.sqlite3");
    let backup = output.join("backup.sqlite3");
    let start_host = || {
        bundled_content::start(
            &database,
            Path::new(&bundle),
            "1.0.0",
            Some(&identity),
            &policy,
        )
        .unwrap()
    };
    eprintln!(
        "{}",
        json!({"kind":"diagnostic_not_acceptance","diagnostic_runtime_commit":option_env!("NAVZEROTH_DIAGNOSTIC_RUNTIME_COMMIT").expect("compile with reviewed runtime commit"),"diagnostic_harness_commit":option_env!("NAVZEROTH_DIAGNOSTIC_HARNESS_COMMIT").expect("compile with harness commit"),"signed_bundle_identity":serde_json::from_slice::<Value>(&identity).unwrap(),"signed_bundle_receipt":serde_json::from_slice::<Value>(&std::fs::read(Path::new(&bundle).join("receipt.json")).unwrap()).unwrap(),"platform":std::env::consts::OS,"bundle":bundle})
    );
    let host = timed("initial_signed_bundle_start", || {
        let host = start_host();
        ready(&host);
        host
    });
    let profile = timed("manual_profile_and_settings", || {
        host.with_application(|app|{
        let snapshot=std::fs::read_to_string(Path::new(&source).join("crates/app/tests/support/stable_engineering_snapshot.json")).unwrap();
        let profile=app.import_text(&format!("ANAV1:{snapshot}"),None)?.profile_id;app.select_profile(&profile)?;
        let record=app.app_settings()?;let mut settings=serde_json::to_value(record.settings).unwrap();settings["accessibility"]["text_scale_percent"]=json!(125);
        app.update_settings(&json!({"schema_version":1,"expected_revision":record.revision,"settings":settings}).to_string())?;
        Ok(profile)
    }).unwrap()
    });
    timed("select_false_inferred_travel", || {
        host.with_application(|app| {
            let active = app.active_content(&profile)?.unwrap();
            app.activate_content_set(&profile, &[active.key.revision], false)
                .map(|_| ())
        })
        .unwrap()
    });
    let expected = timed("initial_state", || state(&host, &profile));
    assert_eq!(expected["accepts_inferred_travel"], false);
    timed("backup", || host.backup(&backup).unwrap());
    eprintln!(
        "{}",
        json!({"backup_bytes":std::fs::metadata(&backup).unwrap().len()})
    );
    let preview = timed("inspect_backup", || host.inspect(&backup).unwrap());
    let receipt = timed("restore_and_initializer_reopen", || {
        host.restore(&preview.candidate_id).unwrap()
    });
    assert!(!receipt.pending_marker_cleanup_required);
    assert!(receipt.original_files_retained);
    assert!(!receipt.recovery_directory.is_empty());
    timed("restored_ready", || ready(&host));
    assert_eq!(
        timed("restored_profiles_settings_content_choice", || state(
            &host, &profile
        )),
        expected
    );
    timed("restored_check_database", || host.check().unwrap());
    assert_eq!(Arc::strong_count(&host), 1);
    timed("drop_restored_host", || drop(host));
    let reopened = timed("explicit_close_reopen_signed_initializer", || {
        let host = start_host();
        ready(&host);
        host
    });
    assert_eq!(
        timed("reopened_profiles_settings_content_choice", || state(
            &reopened, &profile
        )),
        expected
    );
    timed("reopened_check_database", || reopened.check().unwrap());
    drop(reopened);
    eprintln!(
        "{}",
        json!({"diagnostic_passed":true,"acceptance_observations_promoted":false})
    );
}
