//! Explicit adapter admission is synthetic engineering evidence, never live verification.
mod common;

#[test]
fn only_reported_exact_tuples_admit_experimental_capture() {
    let sandbox = common::strict_core();
    sandbox
        .run::<()>(
            "target-allowlist",
            r#"
      local check=ns.Capabilities.isCaptureTarget
      for _, build in ipairs({'70338','70374','70425','70450'}) do
        assert(check({version='1.60.1',build=build,interface='16001'}))
      end
      for _, build in ipairs({'70205','70339','70375','70424','70426','70449','70451',
          'unknown','70374 ','','7042','704250','7045','704500'}) do
        assert(not check({version='1.60.1',build=build,interface='16001'}))
      end
      assert(not check({version='1.60.2',build='70374',interface='16001'}))
      assert(not check({version='1.60.1',build='70374',interface='16002'}))
      assert(not check({version='1.60.1',build='70374',interface=16001}))
      assert(not check(nil) and not check(false) and not check({}))
      assert(not check(setmetatable({}, {__index=function() error('unreadable') end})))
    "#,
        )
        .unwrap();
}

#[test]
fn new_target_preserves_combat_and_unknown_hud_gates() {
    let sandbox = common::strict_core();
    sandbox
        .run::<()>(
            "target-hud",
            r#"
      local build={version='1.60.1',build='70374',interface='16001'}
      local gate=ns.HudAppearance.gate
      local allowed,code=gate(build,function() return false end)
      assert(allowed and code=='hud_gate_allowed')
      allowed,code=gate(build,function() return true end)
      assert(not allowed and code=='hud_combat_deferred')
      for _, getter in ipairs({function() return nil end,function() return 0 end,
          function() error('restricted') end}) do
        allowed,code=gate(build,getter)
        assert(not allowed and code=='hud_combat_unknown')
      end
      allowed,code=gate(build,nil)
      assert(not allowed and code=='hud_combat_unknown')
    "#,
        )
        .unwrap();
}

#[test]
fn a_supported_target_does_not_promote_probe_capabilities() {
    let sandbox = common::stub_client("1.60.1", "70374", 16001);
    sandbox.load_toc_files().unwrap();
    sandbox
        .run::<()>(
            "target-probe",
            r#"
      local report=ns.Capabilities.probe(_G)
      assert(ns.Capabilities.isCaptureTarget(report.build))
      for _, capability in ipairs(report.capabilities) do
        assert(capability.status=='untested')
      end
    "#,
        )
        .unwrap();
}

#[test]
fn manual_70374_commands_keep_actual_context_uncertainty_and_cached_build_isolation() {
    let sandbox = common::stub_client("1.60.1", "70374", 16001);
    sandbox
        .run::<()>(
            "target-readers",
            r#"
      reads=0; mono=100; wall=1791792000
      function UnitLevel() reads=reads+1; return 12 end
      function GetTime() return mono end
      function GetServerTime() return wall end
    "#,
        )
        .unwrap();
    sandbox.load_toc_files().unwrap();
    let text: String = sandbox
        .run(
            "target-commands",
            r#"
      local state=ns.Diagnostics.state
      state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      state.onSlash('snapshot target-profile')
      local first=AzerothNavigatorDB.snapshot_export
      assert(first and AzerothNavigatorDB.snapshot_sequence==1)
      state.onPlanSlash('status'); assert(state.plan_controls)
      state.onSlash('position'); assert(state.position_diagnostics)
      state.onSlash('clock'); state.onSlash('clock trust')
      state.onSessionSlash('start target-profile')
      mono=101; state.onSessionSlash('sample'); state.onSessionSlash('stop')
      state.onSessionSlash('export'); assert(AzerothNavigatorDB.session_export)
      -- An existing capture stays its original build after a client tuple change.
      function GetBuildInfo() return '1.60.1','70338','',16001 end
      local reload=ns.Diagnostics.attach(_G)
      reload.onEvent(nil,'ADDON_LOADED','AzerothNavigator');reload.onPlanSlash('status')
      assert(reload.last_snapshot==nil and AzerothNavigatorDB.snapshot_export==first)
      -- Neither an unsupported target nor an unreadable level invents a capture value.
      function GetBuildInfo() return '1.60.1','70375','',16001 end
      local before=reads;reload.onSlash('snapshot target-profile')
      assert(reads==before and AzerothNavigatorDB.snapshot_export==first)
      return first
    "#,
        )
        .unwrap();
    let value: serde_json::Value = serde_json::from_str(&text[6..]).unwrap();
    assert_eq!(value["game"]["build"], "1.60.1.70374");
    assert_eq!(value["payload"]["level"]["value"], 12);
    assert_eq!(value["payload"]["capture"]["clock_trust"], "uncertain");
    let app = azeroth_app::snapshots::SnapshotApplication::in_memory().unwrap();
    app.import_text(&text, None).unwrap();
}

#[test]
fn admission_labels_each_build_and_keeps_70425_unverified_and_distinct() {
    let sandbox = common::strict_core();
    sandbox
        .run::<()>(
            "admission",
            r#"
      local admission=ns.Capabilities.admission
      local function tuple(build) return {version='1.60.1',build=build,interface='16001'} end
      assert(admission(tuple('70374'))=='release_target')
      assert(admission(tuple('70425'))=='development_unverified')
      assert(admission(tuple('70450'))=='development_unverified')
      assert(admission(tuple('70338'))=='previous_recorded')
      for _, build in ipairs({'70424','70426','70449','70451','70205','unknown',''}) do
        assert(admission(tuple(build))==nil, build)
      end
      assert(admission({version='1.60.2',build='70425',interface='16001'})==nil)
      assert(admission({version='1.60.1',build='70425',interface='16002'})==nil)
      assert(admission({version='1.60.1',build=70425,interface='16001'})==nil)
      assert(admission({version='1.60.2',build='70450',interface='16001'})==nil)
      assert(admission({version='1.60.1',build='70450',interface='16002'})==nil)
      assert(admission({version='1.60.1',build='70450',interface=16001})==nil)
      assert(admission({version='1.60.1',build=70450,interface='16001'})==nil)
      assert(admission(nil)==nil and admission(false)==nil and admission({})==nil)
      assert(admission(setmetatable({}, {__index=function() error('unreadable') end}))==nil)
      -- isCaptureTarget is exactly "has an admission label"
      for _, build in ipairs({'70338','70374','70425','70426','70450','70451'}) do
        assert(ns.Capabilities.isCaptureTarget(tuple(build))==(admission(tuple(build))~=nil))
      end
    "#,
        )
        .unwrap();
}

#[test]
fn build_70425_runs_the_guarded_commands_and_stays_labelled_as_its_own_build() {
    let sandbox = common::stub_client("1.60.1", "70425", 16001);
    sandbox
        .run::<()>(
            "readers",
            r#"
      mono=100; wall=1791792000
      function UnitLevel() return 12 end
      function GetTime() return mono end
      function GetServerTime() return wall end
    "#,
        )
        .unwrap();
    sandbox.load_toc_files().unwrap();
    let text: String = sandbox
        .run(
            "dev-capture",
            r#"
      local state=ns.Diagnostics.state
      state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      state.onSlash('snapshot dev-profile')
      assert(AzerothNavigatorDB.snapshot_sequence==1)
      state.onSlash('clock'); state.onSlash('clock trust')
      state.onSessionSlash('start dev-profile')
      assert(state.session_recorder:status().phase=='recording')
      return AzerothNavigatorDB.snapshot_export
    "#,
        )
        .unwrap();
    // The export records the exact build, so 70425 data is never mistaken for 70374.
    let value: serde_json::Value = serde_json::from_str(&text[6..]).unwrap();
    assert_eq!(value["game"]["build"], "1.60.1.70425");
    let app = azeroth_app::snapshots::SnapshotApplication::in_memory().unwrap();
    app.import_text(&text, None).unwrap();
}

#[test]
fn build_70450_runs_the_guarded_commands_and_stays_labelled_as_its_own_build() {
    let sandbox = common::stub_client("1.60.1", "70450", 16001);
    sandbox
        .run::<()>(
            "readers",
            r#"
      mono=100; wall=1791792000
      function UnitLevel() return 12 end
      function GetTime() return mono end
      function GetServerTime() return wall end
    "#,
        )
        .unwrap();
    sandbox.load_toc_files().unwrap();
    let text: String = sandbox
        .run(
            "dev-capture-70450",
            r#"
      local state=ns.Diagnostics.state
      state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      assert(ns.Capabilities.admission({version='1.60.1',build='70450',interface='16001'})
        =='development_unverified')
      state.onSlash('snapshot dev-profile')
      assert(AzerothNavigatorDB.snapshot_sequence==1)
      state.onSessionSlash('start dev-profile')
      assert(state.session_recorder:status().phase=='recording')
      return AzerothNavigatorDB.snapshot_export
    "#,
        )
        .unwrap();
    // The export records the exact build, so 70450 data is never mistaken for 70425 or 70374.
    let value: serde_json::Value = serde_json::from_str(&text[6..]).unwrap();
    assert_eq!(value["game"]["build"], "1.60.1.70450");
    let app = azeroth_app::snapshots::SnapshotApplication::in_memory().unwrap();
    app.import_text(&text, None).unwrap();
}

#[test]
fn unknown_build_70451_stays_unadmitted_and_refuses_capture() {
    let sandbox = common::stub_client("1.60.1", "70451", 16001);
    sandbox
        .run::<()>(
            "readers",
            "reads=0; function UnitLevel() reads=reads+1; return 12 end",
        )
        .unwrap();
    sandbox.load_toc_files().unwrap();
    sandbox
        .run::<()>(
            "unknown-70451",
            r#"
      local state=ns.Diagnostics.state
      state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      local build={version='1.60.1',build='70451',interface='16001'}
      assert(ns.Capabilities.admission(build)==nil)
      assert(not ns.Capabilities.isCaptureTarget(build))
      state.onSlash('snapshot dev-profile')
      assert(reads==0 and AzerothNavigatorDB.snapshot_export==nil)
      state.onSessionSlash('start dev-profile')
      assert(state.session_recorder==nil or state.session_recorder:status().phase=='idle')
    "#,
        )
        .unwrap();
}
