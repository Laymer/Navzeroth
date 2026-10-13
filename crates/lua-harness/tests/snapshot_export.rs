//! Character export round trips. Injected observations and clients are synthetic;
//! no test claims a live Forever read, path or persistence check.
mod common;
use azeroth_app::snapshots::{FileInputFormat, SnapshotApplication};
use azeroth_core::contract::{AnyExchange, decode_any};
use azeroth_lua_harness::Sandbox;
use serde_json::{Value, json};

fn normalized() -> Sandbox {
    let sandbox = common::strict_core();
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/character-snapshot.json"
    ))
    .unwrap();
    value["payload"]["location"]["value"]["x"] = json!(0.4);
    value["payload"]["preferences"] = json!({});
    value["payload"]["equipment"] = json!({});
    value["payload"]["quests"] = json!({});
    value["payload"]["objectives"] = json!({});
    sandbox
        .ns()
        .set("input", value["payload"].to_string())
        .unwrap();
    sandbox
        .run::<()>(
            "setup",
            r#"
      ns.snapshot = assert(ns.Decode.decode(ns.input))
      ns.options = {fixture=true, locale='enGB'}
    "#,
        )
        .unwrap();
    sandbox
}
#[test]
fn lua_character_bytes_import_through_rust_contract_and_application() {
    let sandbox = normalized();
    let text: String = sandbox
        .run(
            "export",
            "return assert(ns.Snapshot.encode(ns.snapshot, ns.options))",
        )
        .unwrap();
    assert!(text.starts_with("ANAV1:") && text.len() <= 32768);
    let envelope = decode_any(&text[6..]).unwrap();
    assert!(matches!(envelope, AnyExchange::CharacterSnapshot(_)));
    let value: Value = serde_json::from_str(&text[6..]).unwrap();
    for field in ["preferences", "equipment", "quests", "objectives"] {
        assert!(value["payload"][field].is_object(), "{field}");
    }
    assert_eq!(value["payload"]["location"]["value"]["x"], json!(0.4));
    assert_eq!(
        value["provenance"]["created_at_utc_ms"],
        value["payload"]["capture"]["observed_at_utc_ms"]
    );
    let missing_clock: String = sandbox.run("missing_clock",
        "ns.snapshot.capture.clock_trust=nil; return assert(ns.Snapshot.encode(ns.snapshot,ns.options))").unwrap();
    decode_any(&missing_clock[6..]).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&missing_clock[6..]).unwrap()["provenance"]["clock_trust"],
        "uncertain"
    );
    let app = SnapshotApplication::in_memory().unwrap();
    let receipt = app.import_text(&text, None).unwrap();
    let duplicate = app.import_text(&text, None).unwrap();
    assert_eq!(receipt.snapshot_id, duplicate.snapshot_id);
}
#[test]
fn adapters_are_context_gated_and_keep_unknown_false_and_zero_distinct() {
    let sandbox = normalized();
    sandbox
        .run::<()>(
            "capture",
            r#"
      local s = ns.snapshot
      local spec = {id=s.id, character=s.character, context=s.context, capture=s.capture,
        source_revision=s.source_revision, fixture=true, locale='enGB',
        capabilities={level={context=s.context, status='untested'},
          preferences={context=s.context, status='synthetic_fixture'}}}
      local called = false
      local providers = {level=function() called=true; return {status='known',value=50} end,
        preferences=function() return {zero_preference={status='known', value=false}} end}
      local result = assert(ns.Snapshot.capture(spec, providers))
      assert(not called and result.level.status == 'unknown')
      assert(result.preferences.zero_preference.value == false)
      spec.capabilities.level.status = 'synthetic_fixture'
      result = assert(ns.Snapshot.capture(spec, providers))
      assert(called and result.level.value == 50)
      spec.capabilities.level.context = {namespace='demo',
        build={edition='wow_forever', build='different', interface='different'}}
      result = assert(ns.Snapshot.capture(spec, providers))
      assert(result.level.status == 'unknown')
      spec.capabilities.role = {context=s.context, status='restricted'}
      assert(ns.Snapshot.capture(spec, providers).role.status == 'restricted')
      spec.capabilities.location = {context=s.context, status='verified_unavailable'}
      assert(ns.Snapshot.capture(spec, providers).location.status == 'unsupported')
      spec.capabilities.level.context = s.context
      providers.level = function() error('sensitive client error must not be emitted') end
      local issues
      result, issues = ns.Snapshot.capture(spec, providers)
      assert(result.level.status == 'unknown' and issues.level == 'adapter_failed')
      providers.level = function() return {status='known',value=0} end
      assert(not ns.Snapshot.capture(spec, providers))
      s.location.value.x, s.location.value.y = 0, 0
      assert(ns.Snapshot.encode(s, ns.options))
    "#,
        )
        .unwrap();
}
#[test]
fn bounds_malformed_values_and_source_claims_preserve_previous_export() {
    let sandbox = normalized();
    sandbox
        .run::<()>(
            "transaction",
            r#"
      local db = {db_version=1, diagnostic_export='diagnostic'}
      local original = assert(ns.Snapshot.store(db, ns.snapshot, ns.options))
      ns.snapshot.level = {status='known', value=false}
      assert(not ns.Snapshot.store(db, ns.snapshot, ns.options))
      assert(db.snapshot_export == original and db.diagnostic_export == 'diagnostic')
      ns.snapshot = assert(ns.Decode.decode(ns.input))
      ns.snapshot.loop = ns.snapshot
      assert(not ns.Snapshot.store(db, ns.snapshot, ns.options))
      assert(db.snapshot_export == original)
      ns.snapshot = assert(ns.Decode.decode(ns.input))
      ns.snapshot.level.source = 'addon_snapshot'
      assert(not ns.Snapshot.encode(ns.snapshot, ns.options))
      ns.snapshot = assert(ns.Decode.decode(ns.input))
      ns.options.fixture = false
      assert(not ns.Snapshot.encode(ns.snapshot, ns.options))
      ns.options.fixture = true
      ns.snapshot.objectives = {}
      for i=1,1025 do ns.snapshot.objectives['objective-'..i] = {status='unknown'} end
      assert(not ns.Snapshot.store(db, ns.snapshot, ns.options))
      assert(db.snapshot_export == original)
      ns.snapshot.objectives = {}
      for i=1,1000 do
        ns.snapshot.objectives[string.rep('x', 60)..i] = {status='unknown'}
      end
      local text, code = ns.Snapshot.encode(ns.snapshot, ns.options)
      assert(not text and code == 'snapshot_bytes')
      local future = {db_version=99, snapshot_export='future'}
      assert(not ns.Snapshot.store(future, ns.snapshot, ns.options))
      assert(future.snapshot_export == 'future' and future.snapshot_export_error == nil)
    "#,
        )
        .unwrap();
}
#[test]
fn manual_client_capture_produces_importable_real_unknown_clock_state() {
    let sandbox = common::stub_client("1.60.1", "70338", 16001);
    sandbox
        .run::<()>(
            "level",
            "function UnitLevel(unit) assert(unit=='player'); return 35 end",
        )
        .unwrap();
    sandbox.load_toc_files().unwrap();
    let text: String = sandbox
        .run(
            "manual",
            r#"
      ns.Diagnostics.state.onEvent(nil, 'ADDON_LOADED', 'AzerothNavigator')
      ns.Diagnostics.state.onSlash('snapshot anonymous-one')
      assert(AzerothNavigatorDB.snapshot_sequence == 1)
      assert(AzerothNavigatorDB.snapshot_export and not AzerothNavigatorDB.diagnostic_export)
      return AzerothNavigatorDB.snapshot_export
    "#,
        )
        .unwrap();
    let value: Value = serde_json::from_str(&text[6..]).unwrap();
    assert_eq!(value["fixture"], false);
    assert_eq!(value["game"]["build"], "1.60.1.70338");
    assert_eq!(value["payload"]["level"]["value"], 35);
    assert_eq!(value["payload"]["capture"]["clock_trust"], "uncertain");
    assert_eq!(value["game"]["locale"], "unknown");
    let app = SnapshotApplication::in_memory().unwrap();
    let path = std::env::temp_dir().join(format!("anav-lua-snapshot-{}.lua", std::process::id()));
    let escaped = serde_json::to_string(&text).unwrap();
    std::fs::write(&path, format!("AzerothNavigatorDB = {{ snapshot_export = {escaped}, diagnostic_export = \"separate\" }}")).unwrap();
    let file_result = app.import_file(
        &path,
        &FileInputFormat::SavedVariables {
            global: "AzerothNavigatorDB".into(),
            key: "snapshot_export".into(),
        },
        None,
    );
    std::fs::remove_file(&path).unwrap();
    file_result.unwrap();
    app.import_text(&text, None).unwrap();
    let next: String = sandbox
        .run(
            "next",
            r#"
      ns.Diagnostics.state.onSlash('snapshot anonymous-one')
      assert(AzerothNavigatorDB.snapshot_sequence == 2)
      return AzerothNavigatorDB.snapshot_export
    "#,
        )
        .unwrap();
    let receipt = app.import_text(&next, None).unwrap();
    assert!(serde_json::to_value(receipt.disposition).unwrap()["outcome"] == "stored");
    sandbox
        .run::<()>(
            "failures",
            r#"
      local previous = AzerothNavigatorDB.snapshot_export
      AzerothNavigatorDB.snapshot_sequence = false
      ns.Diagnostics.state.onSlash('snapshot anonymous-one')
      assert(AzerothNavigatorDB.snapshot_export == previous)
      AzerothNavigatorDB.snapshot_sequence = 2
      ns.Diagnostics.state.onSlash('snapshot invalid profile with spaces')
      assert(AzerothNavigatorDB.snapshot_export == previous)
      function GetBuildInfo() return '1.60.1', 'other-build', '', 16001 end
      ns.Diagnostics.state.onSlash('snapshot anonymous-one')
      assert(AzerothNavigatorDB.snapshot_export == previous)
    "#,
        )
        .unwrap();
}

#[test]
fn level_reads_never_coerce_unavailable_or_invalid_values() {
    let sandbox = normalized();
    sandbox
        .run::<()>(
            "level_reads",
            r#"
      local missing = ns.Snapshot.readLevel(nil, {})
      assert(missing.status == 'unsupported')
      for _, value in ipairs({false, 0, -1, 1.5, 65536, '35'}) do
        local observed, code = ns.Snapshot.readLevel(function() return value end, {})
        assert(observed.status == 'unknown' and code == 'level_read_failed')
      end
      local observed = ns.Snapshot.readLevel(function()
        error('sensitive client details are not exported')
      end, {})
      assert(observed.status == 'unknown')
      observed = ns.Snapshot.readLevel(function() return 35 end,
        {source='addon_snapshot', clock_trust='uncertain'})
      assert(observed.value == 35 and observed.clock_trust == 'uncertain')
      ns.snapshot.extra = {}
      local current = ns.snapshot.extra
      for _=1,33 do current.nested = {}; current = current.nested end
      local text, code = ns.Snapshot.encode(ns.snapshot, ns.options)
      assert(not text and code == 'snapshot_depth')
    "#,
        )
        .unwrap();
}

#[test]
fn explicit_capture_retains_diagnostics_and_never_runs_from_login() {
    let sandbox = common::stub_client("1.60.1", "70338", 16001);
    sandbox
        .run::<()>(
            "level",
            "level_calls=0; function UnitLevel() level_calls=level_calls+1; return 35 end",
        )
        .unwrap();
    sandbox.load_toc_files().unwrap();
    sandbox
        .run::<()>(
            "explicit_only",
            r#"
      local state = ns.Diagnostics.state
      state.onEvent(nil, 'ADDON_LOADED', 'AzerothNavigator')
      state.onEvent(nil, 'PLAYER_LOGIN')
      assert(level_calls == 0 and AzerothNavigatorDB.snapshot_export == nil)
      local diagnostic = AzerothNavigatorDB.diagnostic_export
      state.onSlash('snapshot anonymous-one')
      assert(level_calls == 1 and AzerothNavigatorDB.diagnostic_export == diagnostic)
      assert(ns.Decode.decode(AzerothNavigatorDB.snapshot_export:sub(7)).payload.level.value == 35)
      function UnitLevel() error('private error not echoed') end
      state.onSlash('snapshot anonymous-one')
      local envelope = assert(ns.Decode.decode(AzerothNavigatorDB.snapshot_export:sub(7)))
      assert(envelope.payload.level.status == 'unknown')
      assert(AzerothNavigatorDB.snapshot_sequence == 2)
      for _, line in ipairs(ns.Diagnostics.state.env.calls or {}) do
        assert(not line:find('private error', 1, true))
      end
    "#,
        )
        .unwrap();
    assert!(
        sandbox
            .printed()
            .iter()
            .all(|line| !line.contains("private error"))
    );
}
