//! Manual recorder checks use injected clocks and synthetic reads only.
mod common;
use azeroth_app::snapshots::SnapshotApplication;
use azeroth_core::{sessions::SessionDocument, types::SessionId};
fn initialize(sandbox: &azeroth_lua_harness::Sandbox) {
    sandbox.run::<()>("init",r#"local t={};ns.recorder_test=t;

    t.mono=0;t.utc=100000
    function t.newRecorder() return ns.Session.new(function() return t.mono end,function() return {clock_trust='trusted',observed_at_utc_ms=t.utc+t.mono*1000} end) end
    t.spec={session='manual-1',character='demo-character-paladin',fixture=true,context={namespace='demo',build={edition='wow_forever',build='UNVERIFIED_DEMO',interface='UNVERIFIED_DEMO'}}}
    function t.level(n) return function(metadata) local x={status='known',value=n};for k,v in pairs(metadata) do x[k]=v end;return x end end
    t.db={db_version=ns.Version.db_version}
"#).unwrap();
}
#[test]
fn manual_unclassified_progression_pause_export_desktop_pipeline() {
    let sandbox = common::strict_core();
    initialize(&sandbox);
    let text: String = sandbox
        .run(
            "record",
            r#"local t=ns.recorder_test;

      local r=t.newRecorder();assert(r:start(t.spec));t.mono=10;assert(r:sample(t.level(12)))
      t.mono=20;assert(r:pause());t.mono=30;assert(not r:sample(t.level(13)))
      t.mono=40;assert(r:resume());t.mono=50;assert(r:sample(t.level(13)));t.mono=60;assert(r:stop())
      local text=assert(r:export(t.db));assert(r:status().reason=='manual_stop');return text
    "#,
        )
        .unwrap();
    let document = azeroth_ingest::session::decode_text(&text).unwrap();
    assert_eq!(document.batch.events[0].elapsed_ms.get(), 0);
    let app = SnapshotApplication::in_memory().unwrap();
    let snapshot = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
    let id = app
        .import_text(&format!("ANAV1:{snapshot}"), None)
        .unwrap()
        .profile_id;
    app.import_session_text(&id, &text).unwrap();
    let detail = app
        .get_session_detail(&id, &SessionId::new("manual-1").unwrap())
        .unwrap();
    assert_eq!(detail.analytics.timing.covered_ms.get(), 60000);
    assert_eq!(detail.analytics.timing.totals.unclassified_ms.get(), 40000);
    assert_eq!(detail.analytics.timing.totals.gap_ms.get(), 20000);
    assert_eq!(detail.analytics.timing.totals.observed_ms.get(), 0);
    assert_eq!(detail.analytics.progression.observed_level_change, Some(1));
}
#[test]
fn stopped_exports_are_immutable_and_reload_reset_never_resume() {
    let sandbox = common::strict_core();
    initialize(&sandbox);
    sandbox.run::<()>("reload",r#"local t=ns.recorder_test;

    local r=t.newRecorder();assert(r:start(t.spec));assert(not r:export(t.db));t.mono=5;assert(r:stop());local prior=assert(r:export(t.db));assert(not r:start(t.spec))
    t.spec.session='manual-2';assert(r:start(t.spec));assert(not r:export(t.db));assert(t.db.session_export==prior)
    t.mono=1;local ok,code=r:sample(t.level(12));assert(not ok and code=='session_clock_reset');assert(r:status().phase=='stopped')
    local fresh=t.newRecorder();assert(fresh:status().phase=='idle');assert(not fresh:resume());assert(not fresh:export(t.db));assert(t.db.session_export==prior)
    t.mono=2;t.spec.session='manual-3';assert(fresh:start(t.spec));assert(fresh:status().session=='manual-3')
  "#).unwrap();
}
#[test]
fn bounds_unknown_reads_clock_drift_and_failed_export_keep_previous() {
    let sandbox = common::strict_core();
    initialize(&sandbox);
    let text:String=sandbox.run("bounds",r#"local t=ns.recorder_test;

    local r=t.newRecorder();assert(r:start(t.spec));t.mono=1;assert(r:sample(function() error('restricted') end))
    t.utc=t.utc+6000;t.mono=2;assert(r:sample(t.level(12)));assert(r:status().clock_warning=='session_utc_changed')
    for i=3,200 do t.mono=i;if not r:sample(t.level(12)) then break end end
    assert(r:status().phase=='stopped' and r:status().reason)
    local prior=assert(r:export(t.db));t.db.db_version=-1;assert(not r:export(t.db));assert(t.db.session_export==prior)
    t.db.db_version=ns.Version.db_version
    local encode=ns.Json.encode;ns.Json.encode=function() return nil,'forced_bound' end
    assert(not r:export(t.db));assert(t.db.session_export==prior);ns.Json.encode=encode
    return prior
  "#).unwrap();
    let document = azeroth_ingest::session::decode_text(&text).unwrap();
    assert!(document.batch.events.len() <= 128);
    assert!(
        matches!(document.batch.events[2].kind,azeroth_core::sessions::SessionEventKind::Level{ref level} if level.value().is_none())
    );
}
#[test]
fn actual_rust_session_bytes_pass_lua_and_desktop_loaders() {
    let document:SessionDocument=serde_json::from_value(serde_json::json!({"format":"anav-session-events","version":1,"fixture":true,"batch":{"character":"demo-character-paladin","context":{"namespace":"demo","build":{"edition":"wow_forever","build":"UNVERIFIED_DEMO","interface":"UNVERIFIED_DEMO"}},"events":[{"session":"rust-1","sequence":1,"elapsed_ms":0,"utc":1000,"clock_trust":"trusted","kind":{"type":"session_start"}},{"session":"rust-1","sequence":2,"elapsed_ms":5000,"utc":6000,"clock_trust":"trusted","kind":{"type":"activity","quality":"unclassified","label":"sample"}}]}})).unwrap();
    let text = azeroth_ingest::session::encode_text(&document).unwrap();
    let sandbox = common::strict_core();
    let returned:String=sandbox.run("rust-bytes",&format!("local text={};local d=assert(ns.Decode.decode(text:sub(14)));assert(d.version==1 and d.batch.events[1].elapsed_ms==0);return assert(ns.Session.encode(d))",serde_json::to_string(&text).unwrap())).unwrap();
    assert_eq!(
        azeroth_ingest::session::decode_text(&returned).unwrap(),
        document
    );
}
#[test]
fn explicit_commands_new_identity_and_no_background_sampling() {
    let sandbox = common::stub_client("1.60.1", "70338", 16001);
    sandbox.run::<()>("clocks",r#"mono=0;function GetTime() return mono end;function GetServerTime() return 100000+math.floor(mono) end;function UnitLevel() return 12 end"#).unwrap();
    sandbox.load_toc_files().unwrap();
    let text:String=sandbox.run("commands",r#"
      local state=ns.Diagnostics.state
      state.onEvent(nil,'ADDON_LOADED','AzerothNavigator');assert(SLASH_ANAVSESSION1=='/anavsession')
      state.onSessionSlash('start profile');assert(AzerothNavigatorDB.session_recording_sequence==1)
      mono=10;assert(state.session_recorder:status().events==1)
      state.onSessionSlash('sample');assert(state.session_recorder:status().events==3)
      state.onSessionSlash('export');assert(AzerothNavigatorDB.session_export==nil)
      mono=20;state.onSessionSlash('pause');mono=40;state.onSessionSlash('resume');mono=50;state.onSessionSlash('stop')
      state.onSessionSlash('export');local old=AzerothNavigatorDB.session_export;assert(old)
      state.onSessionSlash('start profile');assert(AzerothNavigatorDB.session_recording_sequence==2)
      state.onSessionSlash('export');assert(AzerothNavigatorDB.session_export==old)
      function GetBuildInfo() return '1.60.1','70205','',16001 end
      local reads=0;function GetTime() reads=reads+1;return mono end
      state.onSessionSlash('sample');assert(reads==0)
      local reloaded=ns.Diagnostics.attach(_G);reloaded.onEvent(nil,'ADDON_LOADED','AzerothNavigator');reloaded.onSessionSlash('resume')
      assert(reloaded.session_recorder:status().phase=='idle' and AzerothNavigatorDB.session_export==old)
      return old
    "#).unwrap();
    let document = azeroth_ingest::session::decode_text(&text).unwrap();
    assert_eq!(
        document.batch.events[0].session.as_str(),
        "recording-profile-1"
    );
    assert!(!document.fixture);
}
