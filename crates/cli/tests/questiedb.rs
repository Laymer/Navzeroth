//! Synthetic inputs only; upstream gameplay rows are never fixtures.
use azeroth_ingest::questiedb::{AUDITED_REVISION, Entity, MAX_FILE_BYTES};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

fn cli() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_azeroth-cli"));
    command.arg("stage-questiedb");
    command
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "anav-staging-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, bytes: impl AsRef<[u8]>) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn source(entity: Entity, rows: &str) -> String {
    let prefix = match entity {
        Entity::Quest => "quest",
        Entity::Npc => "npc",
        Entity::Item => "item",
        Entity::Object => "object",
    };
    let fields = entity
        .fields()
        .iter()
        .enumerate()
        .map(|(index, name)| format!("['{name}']={},", index + 1))
        .collect::<String>();
    format!(
        "-- synthetic only\nlocal QuestieDB = QuestieLoader:ImportModule(\"QuestieDB\");\n\
         QuestieDB.{prefix}Keys = {{{fields}}}\n\
         QuestieDB.{prefix}Data = [[return {{{rows}}}]]\n"
    )
}
fn raw(entity: Entity) -> String {
    // Staging preserves these values, but reports must never serialize them.
    source(
        entity,
        "[42]={'private-row-sentinel',nil,false,0,{},8589934770},[99]={}",
    )
}
fn success(output: Output) -> Value {
    assert!(output.status.success(), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let text = std::str::from_utf8(&output.stdout).unwrap();
    assert!(!text.contains("sentinel"));
    assert!(!text.contains("8589934770"));
    assert!(!text.contains("\"records\":"));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    for flag in [
        "correction_coverage_complete",
        "derived_passes_applied",
        "source_build_verified",
        "target_inventory_complete",
        "distribution_eligible",
        "playable_dataset",
    ] {
        assert_eq!(report[flag], false, "{flag}");
    }
    assert_eq!(report["corrections_applied"], 0);
    assert_eq!(report["missing_corrections"].as_array().unwrap().len(), 18);
    assert_eq!(report["unsupported_corrections"], json!([]));
    report
}
fn failure(output: Output, code: i32) -> String {
    assert_eq!(output.status.code(), Some(code));
    assert!(output.stdout.is_empty());
    let text = String::from_utf8(output.stderr).unwrap();
    assert!(!text.contains("sentinel"));
    assert!(text.len() < 4096);
    text
}
fn selected(path: &Path) -> Output {
    cli().arg("--quest").arg(path).output().unwrap()
}

#[test]
fn all_four_files_produce_deterministic_metadata_without_source_rows_or_paths() {
    let temp = Temp::new();
    let entries = [
        ("--quest", Entity::Quest),
        ("--npc", Entity::Npc),
        ("--item", Entity::Item),
        ("--object", Entity::Object),
    ]
    .map(|(flag, entity)| {
        let bytes = raw(entity);
        let path = temp.write(&format!("{flag}-path-sentinel.lua"), &bytes);
        (flag, entity, path, bytes)
    });
    let mut command = cli();
    for (flag, _, path, _) in entries.iter().rev() {
        command.arg(flag).arg(path);
    }
    let first = command.output().unwrap();
    let mut command = cli();
    for (flag, _, path, _) in &entries {
        command.arg(flag).arg(path);
    }
    let second = command.output().unwrap();
    assert_eq!(first.stdout, second.stdout);
    let report = success(first);
    success(second);
    assert_eq!(report["missing_raw_entities"], json!([]));
    assert_eq!(report["raw_files"].as_array().unwrap().len(), 4);
    assert_eq!(
        report["caller_declared_origin"],
        json!({"revision":null,"edition":null,"build":null})
    );
    assert_eq!(report["declared_revision_matches_audit"], false);
    for ((_, entity, path, bytes), actual) in
        entries.iter().zip(report["raw_files"].as_array().unwrap())
    {
        assert_eq!(actual["entity"], serde_json::to_value(entity).unwrap());
        assert_eq!(actual["parsed_records"], 2);
        assert_eq!(actual["fingerprint"]["bytes"], bytes.len());
        assert_eq!(
            actual["fingerprint"]["sha256"],
            format!("{:x}", Sha256::digest(bytes.as_bytes()))
        );
        assert_eq!(fs::read(path).unwrap(), bytes.as_bytes());
    }
    // The command creates no sidecars or automatic content outputs.
    assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 4);
}

#[test]
fn partial_selection_preserves_unknown_origin_and_does_not_discover_neighbors() {
    let temp = Temp::new();
    let quest = temp.write("quest-path-sentinel.lua", raw(Entity::Quest));
    temp.write("foreverNpcDB.lua", raw(Entity::Npc));
    let report = success(selected(&quest));
    assert_eq!(report["raw_files"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["missing_raw_entities"],
        json!(["npc", "item", "object"])
    );
    assert_eq!(
        report["caller_declared_origin"],
        json!({"revision":null,"edition":null,"build":null})
    );
    // Relative selections work without changing the parent test process's cwd.
    let output = cli()
        .current_dir(&temp.0)
        .args(["--quest", "./quest-path-sentinel.lua"])
        .output()
        .unwrap();
    assert_eq!(success(output), report);
}

#[test]
fn an_empty_raw_table_is_zero_rows_and_still_incomplete() {
    let temp = Temp::new();
    let path = temp.write("empty-quest-sentinel.lua", source(Entity::Quest, ""));
    let report = success(selected(&path));
    assert_eq!(report["raw_files"][0]["parsed_records"], 0);
    assert_eq!(
        report["missing_raw_entities"],
        json!(["npc", "item", "object"])
    );
}

#[test]
fn declarations_are_explicit_and_never_authenticate_synthetic_input() {
    let temp = Temp::new();
    let quest = temp.write("quest-path-sentinel.lua", raw(Entity::Quest));
    for revision in [AUDITED_REVISION, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"] {
        let output = cli()
            .arg("--quest")
            .arg(&quest)
            .args([
                "--revision",
                revision,
                "--edition",
                "wow_forever",
                "--build",
                "1.60.1.70374",
            ])
            .output()
            .unwrap();
        let report = success(output);
        assert_eq!(
            report["caller_declared_origin"],
            json!({"revision":revision,"edition":"wow_forever","build":"1.60.1.70374"})
        );
        assert_eq!(
            report["declared_revision_matches_audit"],
            revision == AUDITED_REVISION
        );
    }
    // Build can be declared without inventing edition or revision evidence.
    let output = cli()
        .arg("--quest")
        .arg(&quest)
        .args(["--build", "0"])
        .output()
        .unwrap();
    assert_eq!(
        success(output)["caller_declared_origin"],
        json!({"revision":null,"edition":null,"build":"0"})
    );
    for (flag, value) in [
        ("--edition", "wow_classic"),
        ("--revision", "not-a-revision"),
        ("--build", "invalid/build"),
    ] {
        failure(
            cli()
                .arg("--quest")
                .arg(&quest)
                .args([flag, value])
                .output()
                .unwrap(),
            1,
        );
    }
}

#[test]
fn invalid_arguments_are_usage_errors_without_echoing_them() {
    for args in [
        &[][..],
        &["--quest"],
        &["--quest", ""],
        &["--quest", "--item"],
        &["--quest", "private-path-sentinel", "--json"],
        &["--private-sentinel", "file-sentinel"],
        &["private-positional-sentinel"],
        &["--revision", AUDITED_REVISION],
        &["--quest", "first-sentinel", "--quest", "second-sentinel"],
        &[
            "--quest",
            "private-path-sentinel",
            "--build",
            "first-sentinel",
            "--build",
            "second-sentinel",
        ],
        &[
            "--quest",
            "private-path-sentinel",
            "--revision",
            AUDITED_REVISION,
            "--revision",
            AUDITED_REVISION,
        ],
        &[
            "--quest",
            "private-path-sentinel",
            "--edition",
            "wow_forever",
            "--edition",
            "wow_forever",
        ],
        &["--quest=private-path-sentinel"],
        &["--input", "private-directory-sentinel"],
        &[
            "--quest",
            "private-path-sentinel",
            "--output",
            "private-output-sentinel",
        ],
    ] {
        failure(cli().args(args).output().unwrap(), 2);
    }
    failure(
        cli()
            .args(["--quest", "private-path-sentinel", "--build"])
            .arg("x".repeat(65))
            .output()
            .unwrap(),
        2,
    );
}

#[test]
fn duplicate_selection_aliases_fail_before_parsing() {
    let temp = Temp::new();
    let quest = temp.write("quest-path-sentinel.lua", raw(Entity::Quest));
    let output = cli()
        .arg("--quest")
        .arg(&quest)
        .arg("--npc")
        .arg(temp.0.join(".").join("quest-path-sentinel.lua"))
        .output()
        .unwrap();
    assert!(failure(output, 1).contains("more than once"));
}

#[test]
fn missing_nonregular_parent_traversal_and_oversized_inputs_fail() {
    let temp = Temp::new();
    let quest = temp.write("quest-path-sentinel.lua", raw(Entity::Quest));
    fs::create_dir(temp.0.join("child")).unwrap();
    for path in [
        temp.0.join("missing-sentinel.lua"),
        temp.0.clone(),
        temp.0.join("child/../quest-path-sentinel.lua"),
        quest.join("not-a-directory-sentinel"),
    ] {
        failure(selected(&path), 1);
    }
    // A valid wrapper preceded by an inert comment reaches the exact byte bound.
    let raw = raw(Entity::Quest);
    let mut bytes = b"--".to_vec();
    bytes.resize(MAX_FILE_BYTES - raw.len() - 1, b'x');
    bytes.push(b'\n');
    bytes.extend_from_slice(raw.as_bytes());
    assert_eq!(bytes.len(), MAX_FILE_BYTES);
    fs::write(&quest, &bytes).unwrap();
    assert_eq!(
        success(selected(&quest))["raw_files"][0]["fingerprint"]["bytes"],
        MAX_FILE_BYTES
    );
    bytes.push(b'\n');
    fs::write(&quest, &bytes).unwrap();
    assert!(failure(selected(&quest), 1).contains("8 MiB"));
}

#[test]
fn malformed_unsafe_and_wrong_entity_data_fail_without_partial_stdout() {
    let temp = Temp::new();
    let quest = temp.write("quest-path-sentinel.lua", raw(Entity::Quest));
    let npc = temp.write("npc-path-sentinel.lua", []);
    for bytes in [
        b"private-malformed-sentinel".to_vec(),
        vec![0xff],
        Vec::new(),
        raw(Entity::Quest).into_bytes(),
        source(
            Entity::Npc,
            "[1]={os.execute('private-execution-sentinel')}",
        )
        .into_bytes(),
        format!("{}\nerror('private-execution-sentinel')", raw(Entity::Npc)).into_bytes(),
    ] {
        fs::write(&npc, &bytes).unwrap();
        failure(
            cli()
                .arg("--quest")
                .arg(&quest)
                .arg("--npc")
                .arg(&npc)
                .output()
                .unwrap(),
            1,
        );
        assert_eq!(fs::read(&npc).unwrap(), bytes);
    }
    assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 2);
}

#[cfg(unix)]
#[test]
fn symlinks_in_any_component_and_special_files_are_rejected() {
    use std::os::unix::{fs::symlink, net::UnixListener};
    let temp = Temp::new();
    let quest = temp.write("quest-path-sentinel.lua", raw(Entity::Quest));
    let file_link = temp.0.join("file-link-sentinel");
    symlink(&quest, &file_link).unwrap();
    let dir_link = temp.0.join("dir-link-sentinel");
    symlink(&temp.0, &dir_link).unwrap();
    let socket_path = temp.0.join("socket-sentinel");
    let _listener = UnixListener::bind(&socket_path).unwrap();
    for path in [
        file_link,
        dir_link.join("quest-path-sentinel.lua"),
        socket_path,
    ] {
        failure(selected(&path), 1);
    }
}

#[cfg(unix)]
#[test]
fn non_utf8_local_paths_work_but_non_utf8_origin_declarations_do_not() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let temp = Temp::new();
    let path = temp.0.join(OsString::from_vec(b"quest-\xff.lua".to_vec()));
    fs::write(&path, raw(Entity::Quest)).unwrap();
    success(selected(&path));
    failure(
        cli()
            .arg("--quest")
            .arg(&path)
            .arg("--build")
            .arg(OsString::from_vec(vec![0xff]))
            .output()
            .unwrap(),
        2,
    );
    failure(
        cli()
            .arg("--quest")
            .arg(OsString::from_vec(b"--private-sentinel-\xff".to_vec()))
            .output()
            .unwrap(),
        2,
    );
}
