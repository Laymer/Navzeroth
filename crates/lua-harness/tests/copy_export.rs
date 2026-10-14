//! Synthetic UI checks. Actual selection/Ctrl+C on the recorded client is separate.
mod common;

fn client(build: &str) -> azeroth_lua_harness::Sandbox {
    let sandbox = common::stub_client("1.60.1", build, 16001);
    sandbox
        .run::<()>(
            "copy-ui",
            r#"
      frames={}; messages={}; combat=false; levelReads=0
      function InCombatLockdown() return combat end
      function UnitLevel() levelReads=levelReads+1; return 12 end
      function print(value) messages[#messages+1]=value end
      function CreateFrame(kind)
        local f={kind=kind,scripts={},shown=false,text='',focus=false}
        frames[#frames+1]=f
        local noop=function() end
        for _,name in ipairs({'SetSize','SetPoint','SetMultiLine','SetFontObject',
          'SetFrameStrata','EnableMouse','SetAllPoints','SetColorTexture',
          'RegisterEvent'}) do f[name]=noop end
        function f:SetScript(name,callback) self.scripts[name]=callback end
        function f:SetText(value)
          self.text=value
          if self.scripts.OnTextChanged then self.scripts.OnTextChanged(self,false) end
        end
        function f:GetText() return self.text end
        function f:SetAutoFocus(value) self.autoFocus=value end
        function f:SetMaxLetters(value)
          self.max=value
          if #self.text>value then self:SetText(self.text:sub(1,value)) end
        end
        function f:SetFocus() self.focus=true end
        function f:ClearFocus() self.focus=false end
        function f:HighlightText() self.highlight=self.text end
        function f:Show() self.shown=true end
        function f:Hide() self.shown=false end
        function f:SetScrollChild(child) self.child=child end
        function f:CreateFontString() return CreateFrame('FontString') end
        function f:CreateTexture() return CreateFrame('Texture') end
        return f
      end
    "#,
        )
        .unwrap();
    sandbox.load_toc_files().unwrap();
    sandbox
        .run::<()>(
            "bind",
            "ns.Diagnostics.state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')",
        )
        .unwrap();
    sandbox
}

#[test]
fn copies_existing_diagnostic_on_unrecorded_build_without_capturing_or_auto_focus() {
    let sandbox = client("70374");
    sandbox
        .run::<()>(
            "diagnostic-copy",
            r#"
      local s=ns.Diagnostics.state
      s.onSlash('copy')
      assert(s.copy_view==nil and messages[#messages]:find('copy_export_missing',1,true))
      s.onSlash('probe')
      local original=AzerothNavigatorDB.diagnostic_export
      local callCount=#calls
      s.onSlash('copy')
      local v=s.copy_view
      assert(s.copy_ui_code=='copy_shown' and v.frame.shown)
      assert(v.input:GetText()==original and v.input.max==12288)
      assert(v.input.autoFocus==false and not v.input.focus)
      assert(v.input.highlight==nil and #calls==callCount and levelReads==0)
      assert(v.title.text:find('Not a new capture.',1,true))
      v.select.scripts.OnClick()
      assert(v.input.focus and v.input.highlight==original)
      v.input:SetText('attempted replacement')
      assert(v.input:GetText()==original and AzerothNavigatorDB.diagnostic_export==original)
      v.input.scripts.OnEscapePressed()
      assert(not v.frame.shown and not v.input.focus)
      local frameCount=#frames
      s.onSlash('copy diagnostic')
      assert(#frames==frameCount and v.frame.shown and not v.input.focus)
      s.onSlash('snapshot blocked-profile')
      assert(AzerothNavigatorDB.snapshot_export==nil and levelReads==0)
      s.onSlash('copy snapshot')
      assert(v.input:GetText()==original and messages[#messages]:find('copy_export_missing',1,true))
      s.onSlash('copy snapshot extra')
      assert(messages[#messages]:find('usage:',1,true))
      assert(v.input:GetText()==original and s.plan_controls==nil)
    "#,
        )
        .unwrap();
}

#[test]
fn snapshot_copy_preserves_original_transport_age_sequence_and_reuses_one_view() {
    let sandbox = client("70338");
    sandbox
        .run::<()>(
            "snapshot-copy",
            r#"
      local s=ns.Diagnostics.state
      s.onSlash('snapshot copy-profile')
      local original=AzerothNavigatorDB.snapshot_export
      local sequence=AzerothNavigatorDB.snapshot_sequence
      local reads=levelReads
      s.onSlash('copy snapshot')
      local v=s.copy_view
      assert(v.input:GetText()==original and v.input.max==32768)
      assert(AzerothNavigatorDB.snapshot_sequence==sequence and levelReads==reads)
      local frameCount=#frames
      -- A full-size stored transport switched to a smaller diagnostic limit must
      -- not restore the old 32 KiB text recursively during SetMaxLetters.
      AzerothNavigatorDB.snapshot_export=original..string.rep(' ',32768-#original)
      s.onSlash('copy snapshot')
      assert(#v.input:GetText()==32768)
      s.onSlash('probe'); s.onSlash('copy')
      assert(#frames==frameCount and v.input:GetText()==AzerothNavigatorDB.diagnostic_export)
      assert(s.copy_ui_code=='copy_shown')
      AzerothNavigatorDB.snapshot_export=original
      s.onSlash('copy snapshot')
      assert(#frames==frameCount and v.input:GetText()==original)
      assert(AzerothNavigatorDB.snapshot_export==original)
      v.input:SetText('') -- simulated delete/paste restores exact bytes, not re-encoded JSON
      assert(v.input:GetText()==original)
      v.close.scripts.OnClick()
      assert(not v.frame.shown)
      local again=ns.Diagnostics.attach(_G)
      again.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      assert(again.copy_view==nil)
      again.onSlash('copy snapshot')
      assert(again.copy_view.input:GetText()==original)
      assert(AzerothNavigatorDB.snapshot_sequence==sequence and levelReads==reads)
    "#,
        )
        .unwrap();
}

#[test]
fn bounded_selection_rejects_malformed_executable_wrong_kind_and_preserves_unicode_bytes() {
    let sandbox = common::strict_core();
    sandbox
        .run::<()>(
            "copy-boundaries",
            r#"
      local c=ns.CopyExport
      local function refused(db,kind,expected)
        local value,code=c.select(db,kind)
        assert(value==nil and code==expected, tostring(code))
      end
      refused(nil,'diagnostic','copy_db_unavailable')
      refused({db_version=2},'diagnostic','copy_db_unavailable')
      refused({db_version=1},'diagnostic','copy_export_missing')
      refused({db_version=1,diagnostic_export=false},'diagnostic','copy_export_invalid')
      refused({db_version=1,diagnostic_export=string.rep('x',12289)},
        'diagnostic','copy_export_too_large')
      refused({db_version=1,snapshot_export=string.rep('x',32769)},
        'snapshot','copy_export_too_large')
      for _,bad in ipairs({'return {}','{}','[]','{"format":"anav-diagnostic","version":2}',
        '{"format":"anav-diagnostic","format":"anav-diagnostic","version":1}',
        '{"format":"anav-diagnostic","version":1,"text":"'..string.char(255)..'"}'}) do
        refused({db_version=1,diagnostic_export=bad},'diagnostic','copy_export_invalid')
      end
      for _,bad in ipairs({'ANAV1:{','ANAV1:{"kind":"route_plan","schema_version":1}',
        'ANAV1:{"kind":"character_snapshot","schema_version":1,"payload":[]}'}) do
        refused({db_version=1,snapshot_export=bad},'snapshot','copy_export_invalid')
      end
      local text=' {"format":"anav-diagnostic","version":1,"text":"'..
        string.char(195,169)..' | literal \\n"}\n'
      local db={db_version=1,diagnostic_export=text,snapshot_sequence=42}
      local result=assert(c.select(db,'diagnostic'))
      assert(result.text==text and result.bytes==#text and db.diagnostic_export==text)
      assert(db.snapshot_sequence==42)
      local base='{"format":"anav-diagnostic","version":1}'
      db.diagnostic_export=base..string.rep(' ',12288-#base)
      assert(c.select(db,'diagnostic').bytes==12288)
      refused(db,'session','copy_kind_invalid')
    "#,
        )
        .unwrap();
}

#[test]
fn combat_and_unknown_queries_refuse_open_without_mutation_or_automatic_retry() {
    let sandbox = client("70374");
    sandbox
        .run::<()>(
            "copy-gates",
            r#"
      local s=ns.Diagnostics.state
      s.onSlash('probe')
      local original=AzerothNavigatorDB.diagnostic_export
      local frameCount=#frames
      for _,value in ipairs({true,'unknown',1}) do
        combat=value; s.onSlash('copy')
        assert(s.copy_view==nil and #frames==frameCount)
      end
      InCombatLockdown=function() error('private error') end
      s.onSlash('copy')
      assert(s.copy_view==nil and messages[#messages]:find('copy_combat_unknown',1,true))
      InCombatLockdown=nil; s.onSlash('copy')
      assert(s.copy_view==nil)
      InCombatLockdown=function() return false end
      s.onEvent(nil,'PLAYER_REGEN_ENABLED')
      assert(s.copy_view==nil) -- no queued opening/focus change
      s.onSlash('copy')
      local v=s.copy_view
      local text=v.input:GetText()
      InCombatLockdown=function() return true end
      s.onSlash('copy')
      assert(v.input:GetText()==text and v.frame.shown and not v.input.focus)
      assert(AzerothNavigatorDB.diagnostic_export==original)
      for _,message in ipairs(messages) do assert(not message:find('private error',1,true)) end
    "#,
        )
        .unwrap();
}

#[test]
fn missing_throwing_or_truncating_methods_refuse_without_touching_saved_exports() {
    let sandbox = client("70374");
    sandbox
        .run::<()>(
            "copy-ui-failures",
            r#"
      local s=ns.Diagnostics.state
      s.onSlash('probe')
      local original=AzerothNavigatorDB.diagnostic_export
      local create=CreateFrame
      CreateFrame=nil; s.onSlash('copy'); assert(s.copy_view==nil)
      CreateFrame=function() error('private implementation detail') end
      s.onSlash('copy'); assert(s.copy_ui_code=='copy_ui_unavailable')
      CreateFrame=create; s.onSlash('copy')
      local v=s.copy_view
      local setter=v.input.SetText
      v.input.SetText=function(self,text) setter(self,text:sub(1,20)) end
      s.onSlash('copy')
      assert(s.copy_ui_code=='copy_ui_unavailable' and not v.frame.shown)
      local frameCount=#frames
      for i=1,3 do s.onSlash('copy') end
      assert(#frames==frameCount and AzerothNavigatorDB.diagnostic_export==original)
      v.input.SetText=setter; v.input.HighlightText=nil
      s.onSlash('copy')
      assert(s.copy_ui_code=='copy_ui_unavailable' and not v.frame.shown)
      for _,message in ipairs(messages) do
        assert(not message:find('private implementation detail',1,true))
      end
    "#,
        )
        .unwrap();
}
