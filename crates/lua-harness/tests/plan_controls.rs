//! Experimental frame behavior is mocked; this is not live UI acceptance.
mod common;

#[test]
fn slash_paste_is_explicit_runtime_only_and_visibility_survives_reload() {
    let sandbox = common::stub_client("1.60.1", "70338", 16001);
    sandbox
        .run::<()>(
            "ui",
            r#"
      frames={}
      function CreateFrame(kind)
        local f={kind=kind,scripts={},shown=false,text=''}
        frames[#frames+1]=f
        local noop=function() end
        for _,name in ipairs({'SetSize','ClearAllPoints','SetPoint','SetMovable','EnableMouse','RegisterForDrag',
          'SetMultiLine','SetAutoFocus','SetFontObject','SetAllPoints','SetJustifyH',
          'StartMoving','StopMovingOrSizing','RegisterEvent'}) do f[name]=noop end
        function f:SetScript(name,callback) self.scripts[name]=callback end
        function f:SetText(value) self.text=value end
        function f:GetText() return self.text end
        function f:SetMaxLetters(value) self.max=value end
        function f:Show() self.shown=true end
        function f:Hide() self.shown=false end
        function f:GetFont() return 'synthetic-font',12,'' end
        function f:SetFont() return true end
        function f:GetTextColor() return 1,1,1,1 end
        function f:SetTextColor() end
        function f:CreateFontString() return CreateFrame('FontString') end
        return f
      end
      function InCombatLockdown() return false end
      function UnitLevel() return 12 end
      messages={}
      function print(message) messages[#messages+1]=message end
    "#,
        )
        .unwrap();
    sandbox.load_toc_files().unwrap();
    sandbox
        .run::<()>(
            "controls",
            r#"
      local state=ns.Diagnostics.state
      state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      assert(SLASH_ANAVPLAN1=='/anavplan' and SlashCmdList.ANAVPLAN)
      assert(state.plan_controls==nil and state.paste_frame==nil)
      state.onPlanSlash('paste')
      assert(state.paste_frame.shown and state.paste_input.max==32769)
      assert(state.plan_controls==nil) -- opening a paste UI does not import
      state.paste_input:SetText('ANAV1:{')
      local importButton
      for _,f in ipairs(frames) do if f.text=='Import' then importButton=f end end
      assert(importButton)
      importButton.scripts.OnClick()
      assert(state.hud_model==nil and state.plan_controls:status().status=='no_plan')
      state.onPlanSlash('hide')
      assert(not AzerothNavigatorDB.settings.hud_visible)
      state.onPlanSlash('show')
      assert(AzerothNavigatorDB.settings.hud_visible and state.hud_frame.shown)
      state.onPlanSlash('hide')
      assert(not state.hud_frame.shown)
      state.onSlash('snapshot ui-profile')
      assert(state.last_snapshot and state.hud_model.status=='no_plan')
      local reloaded=ns.Diagnostics.attach(_G)
      reloaded.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      assert(reloaded.plan_controls==nil and reloaded.last_snapshot==nil)
      assert(not reloaded.settings.hud_visible)
      assert(AzerothNavigatorDB.loaded_plan==nil)
      reloaded.onPlanSlash('status')
      assert(reloaded.hud_model.status=='no_plan')
    "#,
        )
        .unwrap();
}

#[test]
fn missing_frame_api_preserves_text_commands_and_wrong_build_refuses_import() {
    let sandbox = common::stub_client("1.60.1", "70338", 16001);
    sandbox.run::<()>("ui_missing", "CreateFrame=nil").unwrap();
    sandbox.load_toc_files().unwrap();
    sandbox
        .run::<()>(
            "fallback",
            r#"
      local state=ns.Diagnostics.state
      state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      state.onPlanSlash('paste')
      state.onPlanSlash('status')
      assert(state.hud_model.status=='no_plan' and state.paste_frame==nil)
      state.onPlanSlash('import '..string.rep('x',32769))
      assert(state.plan_controls:status().status=='no_plan')
      function GetBuildInfo() return '1.60.1','70205','',16001 end
      local other=ns.Diagnostics.attach(_G)
      other.onPlanSlash('import ANAV1:{}')
      assert(other.plan_controls==nil)
    "#,
        )
        .unwrap();
}

#[test]
fn bound_import_replaces_transactionally_and_new_capture_requests_revalidation() {
    // Empty, explicitly synthetic protocol test; supplies no authored game content.
    let sandbox = common::stub_client("1.60.1", "70338", 16001);
    sandbox
        .run::<()>("level", "function UnitLevel() return 12 end")
        .unwrap();
    sandbox.load_toc_files().unwrap();
    let captured: String = sandbox
        .run(
            "snapshot",
            r#"
      ns.Diagnostics.state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      ns.Diagnostics.state.onSlash('snapshot ui-profile')
      return AzerothNavigatorDB.snapshot_export
    "#,
        )
        .unwrap();
    let snapshot: serde_json::Value = serde_json::from_str(&captured[6..]).unwrap();
    let mut plan: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/route-plan.json"
    ))
    .unwrap();
    let reference = serde_json::json!({"id":snapshot["payload"]["id"],"character":snapshot["payload"]["character"],"context":snapshot["payload"]["context"]});
    plan["fixture"] = false.into();
    plan["namespace"] = "reference".into();
    plan["game"] = snapshot["game"].clone();
    plan["producer"] =
        serde_json::json!({"component":"desktop","version":"synthetic-protocol-test"});
    plan["provenance"] = serde_json::json!({"source":"manual_entry","clock_trust":"uncertain"});
    plan["payload"]["context"] = snapshot["payload"]["context"].clone();
    plan["payload"]["inputs"] = serde_json::json!([reference]);
    plan["dependencies"]["snapshots"] = plan["payload"]["inputs"].clone();
    plan["payload"]["steps"] = serde_json::json!([]);
    plan["payload"]["travel"] = serde_json::json!([]);
    plan["payload"]["capability_assumptions"] = serde_json::json!({});
    plan["payload"]["expires_at_utc_ms"] = serde_json::Value::Null;
    plan["payload"]["goal"] = "Synthetic empty plan protocol test".into();
    sandbox
        .ns()
        .set("transport", format!("ANAV1:{plan}"))
        .unwrap();
    sandbox.run::<()>("bound_import",r#"
      local state=ns.Diagnostics.state
      state.onPlanSlash('import '..ns.transport)
      assert(state.hud_model.plan_id=='demo-plan-001' and state.hud_model.status=='no_steps')
      assert(state.hud_model.requires_revalidation and not state.hud_model.dependency_check_available)
      state.onPlanSlash('import ANAV1:{')
      assert(state.plan_controls:status().plan_id=='demo-plan-001')
      local reloaded=ns.Diagnostics.attach(_G)
      reloaded.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      assert(reloaded.last_snapshot==nil)
      reloaded.onPlanSlash('import '..ns.transport)
      assert(reloaded.hud_model.status=='no_steps' and reloaded.hud_model.cached_snapshot,
        reloaded.hud_model.status..':'..tostring(reloaded.cached_snapshot_code)..':'..tostring(reloaded.hud_model.cached_snapshot))
      assert(reloaded.hud_model.observation_clock_uncertain)
      assert(reloaded.plan_controls:describe():find('Cached prior addon capture',1,true))
      reloaded.onSlash('snapshot ui-profile')
      assert(reloaded.hud_model.status=='snapshot_superseded')
      assert(not reloaded.hud_model.cached_snapshot)
      state.onSlash('snapshot ui-profile')
      assert(state.hud_model.status=='snapshot_superseded')
      assert(AzerothNavigatorDB.loaded_plan==nil)
      state.onPlanSlash('clear')
      assert(state.hud_model.status=='no_plan')
    "#).unwrap();
}
