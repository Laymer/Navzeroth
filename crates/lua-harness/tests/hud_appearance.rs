//! Synthetic permitted frame bindings only; no live-client taint/focus acceptance.
mod common;

const UI: &str = r#"
  combat=false; mutations=0; frames={}; UIParent={}
  function InCombatLockdown() return combat end
  function CreateFrame(kind)
    mutations=mutations+1
    local f={kind=kind,scripts={},shown=false,point={'TOP',UIParent,'TOP',0,-120}}
    frames[#frames+1]=f
    local function mutate() mutations=mutations+1 end
    for _,name in ipairs({'SetMovable','EnableMouse','RegisterForDrag','SetAllPoints',
      'SetJustifyH','StartMoving','StopMovingOrSizing','ClearAllPoints'}) do f[name]=mutate end
    function f:RegisterEvent(event) if reject_resume and event=='PLAYER_REGEN_ENABLED' then error('unsupported') end end
    function f:SetScript(name,handler) mutate();self.scripts[name]=handler end
    function f:SetSize(w,h) mutate();self.width=w;self.height=h end
    function f:SetPoint(...) mutate();self.point={...} end
    function f:GetNumPoints() return 1 end
    function f:GetPoint() return unpack(self.point) end
    function f:GetFont() return 'synthetic-font',12,'OUTLINE' end
    function f:SetFont(path,size,flags) mutate();self.font={path,size,flags};return true end
    function f:GetTextColor() return 0.8,0.7,0.6,1 end
    function f:SetTextColor(...) mutate();self.color={...} end
    function f:SetColorTexture(...) mutate();self.color={...} end
    function f:SetText(text) mutate();self.text=text end
    function f:Show() mutate();self.shown=true end
    function f:Hide() mutate();self.shown=false end
    function f:CreateFontString() return CreateFrame('FontString') end
    function f:CreateTexture() return CreateFrame('Texture') end
    return f
  end
"#;

fn client() -> azeroth_lua_harness::Sandbox {
    let sandbox = common::stub_client("1.60.1", "70338", 16001);
    sandbox.run::<()>("frame-stubs", UI).unwrap();
    sandbox.load_toc_files().unwrap();
    sandbox
        .run::<()>(
            "loaded",
            "ns.Diagnostics.state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')",
        )
        .unwrap();
    sandbox
}

#[test]
fn preferences_are_bounded_and_additive_without_sharing_or_export_changes() {
    common::strict_core()
        .run::<()>(
            "bounds",
            r#"
      local a=ns.HudAppearance
      for _,v in ipairs({0,279,961,0/0,math.huge,-math.huge,280.5,'420',false}) do
        assert(not a.validate('hud_width',v))
      end
      assert(a.validate('hud_width',280) and a.validate('hud_width',960))
      assert(a.validate('hud_height',160) and a.validate('hud_height',720))
      assert(not a.validate('hud_height',721))
      for _,v in ipairs({100,125,150,175,200}) do assert(a.validate('hud_font_percent',v)) end
      assert(not a.validate('hud_font_percent',101))
      assert(not a.position('UNKNOWN','TOP',0,0) and not a.position('TOP','TOP',4097,0))
      assert(not a.position('TOP','TOP',0/0,0) and not a.position('TOP','TOP',0,0.1))
      assert(not a.validate('hud_position',{anchor='TOP',relative='TOP',x=0,y=0,secret='bad'}))
      assert(not a.command('size',string.rep('1',81),{}))
      assert(not a.command('size','420 220 extra',{}))
      assert(not a.command('font','150 extra',{}))
      assert(not a.command('contrast','HIGH',{}))
      assert(not a.command('reset','extra',{}))
      local db={db_version=1,diagnostic_export='kept',snapshot_export='kept-too',
        settings={auto_probe=false,hud_visible=false,chat_summary=false,hud_width=960,
          hud_height=0,hud_font_percent=200,hud_contrast='high'}}
      local merged=assert(ns.Settings.apply(db))
      assert(not merged.auto_probe and not merged.hud_visible and not merged.chat_summary)
      assert(merged.hud_width==960 and merged.hud_height==220 and merged.hud_font_percent==200)
      assert(db.diagnostic_export=='kept' and db.snapshot_export=='kept-too' and db.db_version==1)
      merged.hud_position.x=44
      assert(ns.Settings.defaults().hud_position.x==0)
      local future={db_version=2,settings={hud_width=9999,hud_visible=false},other='untouched'}
      local old=future.settings
      assert(not ns.Settings.apply(future) and future.settings==old and future.other=='untouched')
    "#,
        )
        .unwrap();
}

#[test]
fn resize_font_contrast_reset_reload_and_actual_movement_are_observable() {
    client().run::<()>("appearance",r#"
      local state=ns.Diagnostics.state
      state.onPlanSlash('size 960 720')
      assert(state.hud_frame.width==960 and state.hud_frame.height==720)
      state.onPlanSlash('font 200')
      assert(state.hud_text.font[2]==24)
      state.onPlanSlash('font 200')
      assert(state.hud_text.font[2]==24) -- baseline, never compounded
      state.onPlanSlash('contrast high')
      assert(state.hud_background.shown and state.hud_background.color[4]==1)
      assert(state.hud_text.color[1]==1)
      state.onPlanSlash('contrast default')
      assert(not state.hud_background.shown and state.hud_text.color[1]==0.8)
      state.hud_frame.point={'BOTTOM',UIParent,'BOTTOM',12.4,-14.6}
      state.hud_frame.scripts.OnDragStop(state.hud_frame)
      assert(AzerothNavigatorDB.settings.hud_position.x==12 and
        AzerothNavigatorDB.settings.hud_position.y==-15)
      assert(state.hud_movement_code=='hud_movement_persisted')
      local again=ns.Diagnostics.attach(_G)
      again.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      again.onPlanSlash('show')
      assert(again.hud_frame.point[1]=='BOTTOM' and again.hud_frame.point[4]==12)
      assert(again.hud_frame.width==960 and again.hud_text.font[2]==24)
      again.hud_frame.point={'TOP',{},'TOP',0,0}
      again.hud_frame.scripts.OnDragStop(again.hud_frame)
      assert(again.hud_movement_code=='hud_movement_session_only')
      assert(AzerothNavigatorDB.settings.hud_position.x==12)
      again.onPlanSlash('reset')
      assert(again.hud_frame.width==420 and again.hud_frame.height==220 and again.hud_text.font[2]==12)
      assert(again.hud_frame.point[1]=='TOP' and again.hud_frame.point[5]==-120)
      local width=again.hud_frame.width
      again.onPlanSlash('size 961 200')
      assert(again.hud_frame.width==width and AzerothNavigatorDB.settings.hud_width==420)
    "#).unwrap();
}

#[test]
fn unknown_combat_defers_every_hud_mutation_and_resume_uses_latest_request_and_model() {
    client().run::<()>("defer",r#"
      local state=ns.Diagnostics.state
      InCombatLockdown=nil
      local before=mutations
      state.onPlanSlash('size 300 200')
      state.onPlanSlash('size 500 300')
      state.onPlanSlash('font 175')
      assert(mutations==before and state.hud_frame==nil and state.hud_pending)
      assert(state.plan_ui_code=='hud_combat_unknown')
      InCombatLockdown=function() return 'restricted' end
      state.onPlanSlash('show'); assert(mutations==before)
      InCombatLockdown=function() error('restricted secret') end
      state.onPlanSlash('status');assert(mutations==before)
      InCombatLockdown=function() return true end
      state.onPlanSlash('show');assert(mutations==before and state.plan_ui_code=='hud_combat_deferred')
      local refresh=state.plan_controls.refresh
      local refreshed=0
      state.plan_controls.refresh=function(self) refreshed=refreshed+1;return refresh(self) end
      -- Bogus previously queued advice must not be replayed on resume.
      state.hud_model={status='ready',primary={instruction='OBSOLETE'}}
      InCombatLockdown=function() return false end
      state.frame.scripts.OnEvent(state.frame,'PLAYER_REGEN_ENABLED')
      assert(refreshed==1 and state.hud_model.status=='no_plan')
      assert(not state.hud_pending and state.plan_ui_code=='hud_applied')
      assert(state.hud_frame.width==500 and state.hud_text.font[2]==21)
      assert(not state.hud_text.text:find('OBSOLETE',1,true))
      assert(state.hud_text.text:find('Last evaluated addon observation',1,true))
      combat=true;InCombatLockdown=function() return combat end
      before=mutations
      state.onPlanSlash('hide')
      state.hud_frame.scripts.OnDragStart(state.hud_frame)
      state.hud_frame.scripts.OnDragStop(state.hud_frame)
      assert(mutations==before and state.hud_stop_pending)
      combat=false;state.resumeHud()
      assert(not state.hud_frame.shown and not state.hud_stop_pending)
      state.onPlanSlash('show')
      assert(not state.hud_stop_pending and state.hud_frame.shown)
    "#).unwrap();
}

#[test]
fn unsupported_or_throwing_methods_and_event_registration_remain_truthful() {
    client()
        .run::<()>(
            "unsupported",
            r#"
      local state=ns.Diagnostics.state
      state.onPlanSlash('show')
      local text=state.hud_text
      text.GetFont=function() error('unavailable') end
      state.hud_font=nil
      state.onPlanSlash('font 150')
      assert(state.hud_pending and state.plan_ui_code=='hud_ui_unavailable')
      assert(state.hud_applied.hud_font_percent==100)
      text.GetFont=function() return nil end
      state.onPlanSlash('status')
      assert(state.plan_ui_code=='hud_font_unavailable')
      text.GetFont=function() return 'synthetic-font',12,'' end
      text.SetFont=function() return false end
      state.onPlanSlash('status')
      assert(state.plan_ui_code=='hud_font_unavailable' and state.hud_pending)
      text.SetFont=function(_,path,size) text.font={path,size};return true end
      state.hud_frame.CreateTexture=nil
      state.onPlanSlash('contrast high')
      assert(state.plan_ui_code=='hud_contrast_unavailable')
      reject_resume=true
      local other=ns.Diagnostics.attach(_G)
      assert(other.frame and not other.hud_resume_event_available)
      other.frame.scripts.OnEvent(other.frame,'ADDON_LOADED','AzerothNavigator')
      other.onPlanSlash('show') -- explicit refresh remains usable
      assert(other.settings.hud_width==420)
    "#,
        )
        .unwrap();
}

#[test]
fn build_and_combat_gate_never_interpret_truthy_unknown_values() {
    common::strict_core()
        .run::<()>(
            "gate",
            r#"
      local a=ns.HudAppearance
      local build={version='1.60.1',build='70338',interface='16001'}
      assert(not a.gate(build,nil))
      for _,value in ipairs({0,1,'false','restricted',{}}) do
        assert(not a.gate(build,function() return value end))
      end
      assert(not a.gate(build,function() error('blocked') end))
      assert(not a.gate(build,function() return true end))
      assert(a.gate(build,function() return false end))
      build.build='70205';assert(not a.gate(build,function() return false end))
      local unreadable=setmetatable({}, {__index=function() error('restricted build') end})
      assert(not a.gate(unreadable,function() return false end))
    "#,
        )
        .unwrap();
}

#[test]
fn unsafe_observed_positions_and_build_changes_preserve_saved_geometry() {
    client()
        .run::<()>(
            "geometry",
            r#"
      local state=ns.Diagnostics.state
      state.onPlanSlash('show')
      local frame=state.hud_frame
      for _,point in ipairs({{'TOP',UIParent,'TOP',math.huge,0},
        {'TOP',UIParent,'TOP',4097,0},{'WRONG',UIParent,'TOP',0,0}}) do
        frame.point=point;frame.scripts.OnDragStop(frame)
        assert(state.hud_movement_code=='hud_movement_session_only')
        assert(AzerothNavigatorDB.settings.hud_position.y==-120)
      end
      frame.GetPoint=function() error('restricted') end
      frame.scripts.OnDragStop(frame)
      assert(state.hud_movement_code=='hud_movement_session_only')
      frame.GetNumPoints=function() error('restricted point count') end
      frame.scripts.OnDragStop(frame)
      assert(state.hud_movement_code=='hud_movement_session_only')
      frame.GetPoint=nil;frame.scripts.OnDragStop(frame)
      assert(state.hud_movement_code=='hud_movement_session_only')
      function GetBuildInfo() return '1.60.1','70205','',16001 end
      local before=mutations
      state.onPlanSlash('font 200')
      assert(mutations==before and state.hud_pending and state.plan_ui_code=='hud_build_unrecorded')
      assert(state.hud_text.font[2]==12)
    "#,
        )
        .unwrap();
}

#[test]
fn resume_rechecks_actual_plan_expiry_with_new_clock_before_rendering() {
    let sandbox = client();
    let captured: String = sandbox
        .run(
            "clock-capture",
            r#"
      wall=1791792000;mono=10
      function GetServerTime() return wall end
      function GetTime() return mono end
      function UnitLevel() return 12 end
      local state=ns.Diagnostics.state
      state.onSlash('clock');state.onSlash('clock trust')
      state.onSlash('snapshot ui-profile')
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
    plan["producer"] = serde_json::json!({"component":"desktop","version":"synthetic-expiry-test"});
    plan["provenance"] = serde_json::json!({"source":"manual_entry","clock_trust":"trusted"});
    plan["payload"]["context"] = snapshot["payload"]["context"].clone();
    plan["payload"]["inputs"] = serde_json::json!([reference]);
    plan["dependencies"]["snapshots"] = plan["payload"]["inputs"].clone();
    plan["payload"]["steps"] = serde_json::json!([]);
    plan["payload"]["travel"] = serde_json::json!([]);
    plan["payload"]["capability_assumptions"] = serde_json::json!({});
    plan["payload"]["generated_at_utc_ms"] = 1791792000000_i64.into();
    plan["payload"]["expires_at_utc_ms"] = 1791792002000_i64.into();
    plan["payload"]["goal"] = "Synthetic expiry check".into();
    sandbox
        .ns()
        .set("transport", format!("ANAV1:{plan}"))
        .unwrap();
    sandbox
        .run::<()>(
            "expire-deferred",
            r#"
      local state=ns.Diagnostics.state
      state.onPlanSlash('import '..ns.transport)
      assert(state.hud_model.status=='no_steps',state.hud_model.status)
      combat=true;state.onPlanSlash('font 200')
      local prior=state.hud_text.text
      wall=wall+3;mono=mono+3
      combat=false;state.frame.scripts.OnEvent(state.frame,'PLAYER_REGEN_ENABLED')
      assert(state.hud_model.status=='expired',state.hud_model.status)
      assert(state.hud_text.text~=prior and state.hud_text.text:find('expired',1,true))
    "#,
        )
        .unwrap();
}

#[test]
fn session_only_drag_survives_refresh_and_appearance_changes_until_explicit_reset() {
    client()
        .run::<()>(
            "session-position",
            r#"
      for _,getter in ipairs({false,function() error('restricted') end}) do
        local state=ns.Diagnostics.attach(_G)
        state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
        state.onPlanSlash('show')
        local frame=state.hud_frame
        frame.point={'BOTTOM',UIParent,'BOTTOM',120,140}
        frame.GetPoint=getter
        frame.scripts.OnDragStop(frame)
        assert(state.hud_movement_code=='hud_movement_session_only')
        assert(AzerothNavigatorDB.settings.hud_position.y==-120)
        local function unchanged()
          assert(frame.point[1]=='BOTTOM' and frame.point[4]==120 and frame.point[5]==140,
            'session-only drag was reset by model or appearance refresh')
          assert(AzerothNavigatorDB.settings.hud_position.y==-120)
        end
        state.plan_controls:refresh();unchanged()
        state.onPlanSlash('font 175');unchanged()
        assert(state.hud_text.font[2]==21)
        state.onPlanSlash('size 500 300');unchanged()
        assert(frame.width==500 and frame.height==300)
        state.onPlanSlash('contrast high');unchanged()
        state.onPlanSlash('hide');state.onPlanSlash('show');unchanged()
        -- Saved/requested position is still the default, yet reset must re-anchor.
        combat=true;state.onPlanSlash('reset');unchanged()
        assert(state.hud_pending)
        combat=false;state.frame.scripts.OnEvent(state.frame,'PLAYER_REGEN_ENABLED')
        assert(frame.point[1]=='TOP' and frame.point[4]==0 and frame.point[5]==-120)
        assert(frame.width==420 and frame.height==220 and state.hud_text.font[2]==12)
        assert(not state.hud_pending)
        -- Another explicit reset with all equal defaults is still an operation.
        frame.point={'BOTTOM',UIParent,'BOTTOM',120,140}
        frame.scripts.OnDragStop(frame)
        state.onPlanSlash('reset')
        assert(frame.point[1]=='TOP' and frame.point[5]==-120)
      end
    "#,
        )
        .unwrap();
}

#[test]
fn deferred_reset_supersedes_pending_drag_and_retries_a_failed_anchor_operation() {
    client()
        .run::<()>(
            "reset-position",
            r#"
      local state=ns.Diagnostics.state
      state.onPlanSlash('show')
      local frame=state.hud_frame
      frame.point={'BOTTOM',UIParent,'BOTTOM',120,140}
      combat=true;frame.scripts.OnDragStop(frame)
      assert(state.hud_stop_pending)
      state.onPlanSlash('reset');state.onPlanSlash('font 150')
      assert(frame.point[1]=='BOTTOM' and AzerothNavigatorDB.settings.hud_position.y==-120)
      local setter=frame.SetPoint
      frame.SetPoint=function() error('unavailable') end
      combat=false;state.frame.scripts.OnEvent(state.frame,'PLAYER_REGEN_ENABLED')
      assert(not state.hud_stop_pending and state.hud_pending)
      assert(AzerothNavigatorDB.settings.hud_position.y==-120)
      assert(state.hud_reset_position) -- request is not consumed by a failed setter
      frame.SetPoint=setter;state.plan_controls:refresh()
      assert(frame.point[1]=='TOP' and frame.point[4]==0 and frame.point[5]==-120)
      assert(state.hud_text.font[2]==18 and not state.hud_pending)
      assert(not state.hud_reset_position)
    "#,
        )
        .unwrap();
}
