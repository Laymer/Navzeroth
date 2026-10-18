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
    for (utc, trust, fixture) in [
        (
            Some(1000),
            azeroth_core::observation::ClockTrust::Trusted,
            true,
        ),
        (None, azeroth_core::observation::ClockTrust::Uncertain, true),
        (
            Some(0),
            azeroth_core::observation::ClockTrust::Uncertain,
            false,
        ),
    ] {
        let mut document = document.clone();
        document.fixture = fixture;
        if !fixture {
            document.batch.context.namespace = azeroth_core::types::DatasetNamespace::Reference;
        }
        for event in &mut document.batch.events {
            event.utc = utc.map(|start| {
                azeroth_core::types::UtcMillis::new(start + event.elapsed_ms.get() as i64).unwrap()
            });
            event.clock_trust = trust;
        }
        let text = azeroth_ingest::session::encode_text(&document).unwrap();
        let sandbox = common::strict_core();
        let returned:String=sandbox.run("rust-bytes",&format!("local text={};local d=assert(ns.Decode.decode(text:sub(14)));assert(d.version==1 and d.batch.events[1].elapsed_ms==0);return assert(ns.Session.encode(d))",serde_json::to_string(&text).unwrap())).unwrap();
        assert_eq!(
            azeroth_ingest::session::decode_text(&returned).unwrap(),
            document
        );
    }
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

#[test]
fn manually_stopping_recording_never_claims_game_logout() {
    for paused in [false, true] {
        let sandbox = common::strict_core();
        initialize(&sandbox);
        let action = if paused {
            "assert(r:pause())"
        } else {
            "assert(r:sample(t.level(12)))"
        };
        let text: String = sandbox
            .run(
                "manual-stop-evidence",
                &format!(
                    r#"
          local t=ns.recorder_test;local r=t.newRecorder();assert(r:start(t.spec))
          t.mono=10;{action};t.mono=20;assert(r:stop())
          assert(r:status().phase=='stopped' and r:status().reason=='manual_stop')
          return assert(r:export(t.db))
        "#
                ),
            )
            .unwrap();
        let document = azeroth_ingest::session::decode_text(&text).unwrap();
        assert!(
            !document.batch.events.iter().any(|event| matches!(
                event.kind,
                azeroth_core::sessions::SessionEventKind::Logout
            )),
            "manual recorder stop is not evidence of game logout"
        );
        assert!(matches!(
            document.batch.events.last().unwrap().kind,
            azeroth_core::sessions::SessionEventKind::RecordingPaused
        ));
        let build = azeroth_core::sessions::build_sessions(
            &document.batch,
            &azeroth_ingest::session::build_config(),
        )
        .unwrap();
        let summary = &build[&SessionId::new("manual-1").unwrap()];
        assert_eq!(summary.summary.elapsed_ms.get(), 20000);
        assert_eq!(summary.totals.observed_ms.get(), 0);
        assert_eq!(
            summary.totals.unclassified_ms.get(),
            if paused { 10000 } else { 20000 }
        );
        assert_eq!(summary.totals.gap_ms.get(), if paused { 10000 } else { 0 });
    }
}

#[test]
fn quest_milestones_preserve_unclassified_time_bounds_and_manual_gaps() {
    let sandbox = common::strict_core();
    initialize(&sandbox);
    let text: String = sandbox
        .run(
            "quest-milestones",
            r#"
      local t=ns.recorder_test;local r=t.newRecorder();assert(r:start(t.spec))
      t.mono=10;assert(r:questTurnIn(123))
      t.mono=15;assert(r:pause());assert(not r:questTurnIn(456))
      t.mono=20;assert(r:resume());t.mono=30;assert(r:questTurnIn(789))
      t.mono=40;assert(r:stop());assert(not r:questTurnIn(999))
      return assert(r:export(t.db))
    "#,
        )
        .unwrap();
    let document = azeroth_ingest::session::decode_text(&text).unwrap();
    let sessions = azeroth_core::sessions::build_sessions(
        &document.batch,
        &azeroth_ingest::session::build_config(),
    )
    .unwrap();
    let build = &sessions[&SessionId::new("manual-1").unwrap()];
    assert_eq!(
        build
            .summary
            .observed_quests
            .iter()
            .map(|q| q.as_str())
            .collect::<Vec<_>>(),
        ["123", "789"]
    );
    assert!(
        document
            .batch
            .events
            .iter()
            .filter(|e| matches!(
                e.kind,
                azeroth_core::sessions::SessionEventKind::QuestTurnIn { .. }
            ))
            .count()
            == 2
    );
    let app = SnapshotApplication::in_memory().unwrap();
    let snapshot = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
    let profile = app
        .import_text(&format!("ANAV1:{snapshot}"), None)
        .unwrap()
        .profile_id;
    app.import_session_text(&profile, &text).unwrap();
    let detail = app
        .get_session_detail(&profile, &SessionId::new("manual-1").unwrap())
        .unwrap();
    assert_eq!(detail.analytics.timing.totals.observed_ms.get(), 0);
    assert_eq!(detail.analytics.timing.totals.unclassified_ms.get(), 35000);
    assert_eq!(detail.analytics.timing.totals.gap_ms.get(), 5000);
}

#[test]
fn unreadable_quest_payload_and_event_overflow_stop_before_partial_append() {
    let sandbox = common::strict_core();
    initialize(&sandbox);
    sandbox
        .run::<()>(
            "quest-refusal",
            r#"
      local t=ns.recorder_test
      for index,value in ipairs({0,-1,1.5,4294967296,'123',false,math.huge}) do
        local r=t.newRecorder();t.spec.session='bad-quest-'..index;assert(r:start(t.spec))
        local ok,code=r:questTurnIn(value);assert(not ok and code=='session_quest_unreadable')
        assert(r:status().events==1 and r:status().phase=='stopped')
      end
      local r=t.newRecorder();t.spec.session='quest-cap';assert(r:start(t.spec))
      local limit=ns.Session.MAX_EVENTS;ns.Session.MAX_EVENTS=5
      for i=1,3 do t.mono=i;local ok,code=r:questTurnIn(i)
        if i<3 then assert(ok) else assert(not ok and code=='session_event_limit') end
      end
      ns.Session.MAX_EVENTS=limit
      assert(r:status().events==5 and r:status().phase=='stopped')
      assert(r:export(t.db));local prior=t.db.session_export
      t.spec.session='quest-context';assert(r:start(t.spec));r:unavailable()
      assert(r:status().reason=='session_observation_unavailable' and t.db.session_export==prior)
    "#,
        )
        .unwrap();
}

#[test]
fn explicit_quest_mode_binds_only_active_turn_ins_in_matching_context() {
    let sandbox = common::stub_client("1.60.1", "70374", 16001);
    sandbox
        .run::<()>(
            "quest-event-support",
            r#"
      mono=0;function GetTime() return mono end
      function GetServerTime() return 100000+math.floor(mono) end
      C_EventUtils={IsEventValid=function(event) return event=='QUEST_TURNED_IN' end}
    "#,
        )
        .unwrap();
    sandbox.load_toc_files().unwrap();
    let text: String = sandbox.run("quest-event-binding", r#"
      local state=ns.Diagnostics.state
      state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      assert(state.quest_turn_in_event_available)
      local function event(id) state.frame.scripts.OnEvent(state.frame,'QUEST_TURNED_IN',id,'ignored-reward') end
      state.onSessionSlash('start profile');mono=1;event(111)
      assert(state.session_recorder:status().events==1)
      state.onSessionSlash('stop');state.onSessionSlash('export')
      local prior=AzerothNavigatorDB.session_export
      state.onSessionSlash('start-quests profile');mono=5;event(222)
      assert(state.session_recorder:status().events==3)
      mono=6;state.onSessionSlash('pause');event(333)
      assert(state.session_recorder:status().events==5)
      mono=7;state.onSessionSlash('resume');mono=8;event(444)
      assert(state.session_recorder:status().events==8)
      install_build_info('1.60.1','70338',16001);event(555)
      assert(state.session_recorder:status().phase=='stopped')
      assert(state.session_recorder:status().reason=='session_observation_unavailable')
      assert(state.session_recorder:status().events==8 and AzerothNavigatorDB.session_export==prior)
      state.onSessionSlash('export');event(666)
      return AzerothNavigatorDB.session_export
    "#).unwrap();
    let document = azeroth_ingest::session::decode_text(&text).unwrap();
    assert!(!document.fixture);
    assert_eq!(document.batch.context.build.build.as_str(), "1.60.1.70374");
    let quests: Vec<_> = document
        .batch
        .events
        .iter()
        .filter_map(|event| match &event.kind {
            azeroth_core::sessions::SessionEventKind::QuestTurnIn { quest } => Some(quest.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(quests, ["222", "444"]);
}

#[test]
fn unavailable_or_failed_event_support_refuses_opt_in_without_mutation() {
    for setup in [
        "C_EventUtils=nil",
        "C_EventUtils={IsEventValid=function() return false end}",
        "C_EventUtils={IsEventValid=function() return 'true' end}",
        "C_EventUtils={IsEventValid=function() error('Private-Error') end}",
        r#"C_EventUtils={IsEventValid=function() return true end};local create=CreateFrame
        function CreateFrame(...) local f=create(...);local register=f.RegisterEvent
          function f:RegisterEvent(event) if event=='QUEST_TURNED_IN' then return false end;return register(self,event) end
          return f end"#,
        r#"C_EventUtils={IsEventValid=function() return true end};local create=CreateFrame
        function CreateFrame(...) local f=create(...);local register=f.RegisterEvent
          function f:RegisterEvent(event) if event=='QUEST_TURNED_IN' then error('Private-Error') end;return register(self,event) end
          return f end"#,
        r#"C_EventUtils={IsEventValid=function() return true end};local create=CreateFrame
        function CreateFrame(...) local f=create(...);local register=f.RegisterEvent
          function f:RegisterEvent(event) if event=='QUEST_TURNED_IN' then return 'unexpected' end;return register(self,event) end
          return f end"#,
    ] {
        let sandbox = common::stub_client("1.60.1", "70374", 16001);
        sandbox
            .run::<()>(
                "unavailable-support",
                &format!(
                    r#"
          mono=0;function GetTime() return mono end;function GetServerTime() return 100000 end
          {setup}
        "#
                ),
            )
            .unwrap();
        sandbox.load_toc_files().unwrap();
        sandbox.run::<()>("refuse-quest-mode", r#"
          local state=ns.Diagnostics.state;state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
          state.onSessionSlash('start profile');state.onSessionSlash('stop');state.onSessionSlash('export')
          local prior=AzerothNavigatorDB.session_export;local count=AzerothNavigatorDB.session_recording_sequence
          state.onSessionSlash('start-quests profile')
          assert(not state.quest_turn_in_event_available)
          assert(state.session_recorder:status().phase=='stopped')
          assert(AzerothNavigatorDB.session_export==prior and AzerothNavigatorDB.session_recording_sequence==count)
        "#).unwrap();
    }
}

#[test]
fn malformed_callback_stops_without_reading_rewards_or_appending_partial_event() {
    let sandbox = common::stub_client("1.60.1", "70374", 16001);
    sandbox
        .run::<()>(
            "event-support",
            r#"
      mono=0;function GetTime() return mono end;function GetServerTime() return 100000 end
      C_EventUtils={IsEventValid=function() return true end}
    "#,
        )
        .unwrap();
    sandbox.load_toc_files().unwrap();
    let text: String = sandbox.run("malformed-event", r#"
      local state=ns.Diagnostics.state;state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      state.onSessionSlash('start-quests profile')
      local unreadable=setmetatable({},{__tostring=function() error('Private-Reward') end})
      state.frame.scripts.OnEvent(state.frame,'QUEST_TURNED_IN','Private-Quest',unreadable,unreadable)
      assert(state.session_recorder:status().phase=='stopped')
      assert(state.session_recorder:status().reason=='session_quest_unreadable')
      assert(state.session_recorder:status().events==1)
      state.onSessionSlash('export');return AzerothNavigatorDB.session_export
    "#).unwrap();
    assert!(!text.contains("Private-"));
    assert_eq!(
        azeroth_ingest::session::decode_text(&text)
            .unwrap()
            .batch
            .events
            .len(),
        1
    );
}
#[test]
fn explicit_locale_extension_roundtrips_without_changing_legacy_recorder() {
    let sandbox = common::strict_core();
    initialize(&sandbox);
    let text:String=sandbox.run("locale",r#"local t=ns.recorder_test;local r=t.newRecorder();t.spec.locale='enUS';assert(r:start(t.spec));t.mono=1;assert(r:stop());return assert(r:export(t.db))"#).unwrap();
    let d = azeroth_ingest::session::decode_text(&text).unwrap();
    assert_eq!(d.version, 2);
    assert_eq!(d.locale.as_ref().unwrap().as_str(), "enUS");
    sandbox.run::<()>("bad-locale",r#"local t=ns.recorder_test;local r=t.newRecorder();t.spec.session='bad';t.spec.locale='unknown';assert(not r:start(t.spec));assert(r:status().phase=='idle')"#).unwrap();
}
#[test]
fn manual_session_binding_records_observable_locale_and_falls_back_on_unavailable_api() {
    for (body, expected) in [
        ("return 'enUS'", Some("enUS")),
        ("return 'unknown'", None),
        ("error('restricted')", None),
        ("return nil", None),
    ] {
        let sandbox = common::stub_client("1.60.1", "70450", 16001);
        sandbox.run::<()>("locale-api",&format!("mono=0;localeReads=0;function GetTime() return mono end;function GetLocale() localeReads=localeReads+1;{body} end")).unwrap();
        sandbox.load_toc_files().unwrap();
        let text:String=sandbox.run("manual-locale",r#"local state=ns.Diagnostics.state;state.onEvent(nil,'ADDON_LOADED','AzerothNavigator');assert(localeReads==0);state.onSessionSlash('start profile');assert(localeReads==1);mono=1;state.onSessionSlash('stop');state.onSessionSlash('export');assert(localeReads==1);return assert(AzerothNavigatorDB.session_export)"#).unwrap();
        let d = azeroth_ingest::session::decode_text(&text).unwrap();
        assert_eq!(d.locale.as_ref().map(|v| v.as_str()), expected);
        assert_eq!(d.version, if expected.is_some() { 2 } else { 1 });
    }
}
