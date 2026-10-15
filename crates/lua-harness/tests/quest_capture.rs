//! Experimental quest bindings exercised with synthetic client APIs only.
mod common;
use azeroth_app::snapshots::SnapshotApplication;
use serde_json::Value;

#[test]
fn bounded_quest_reads_preserve_false_and_unknown_without_history_or_stage_mapping() {
    let sandbox = common::strict_core();
    sandbox.run::<()>("quests", r#"
      local context={namespace='reference',build={edition='wow_forever',build='1.60.1.70338',interface='16001'}}
      local metadata={source='addon_snapshot',clock_trust='uncertain',observed_at_utc_ms=0}
      local entries={
        {isHeader=true,questID=123},
        {isHeader=false,questID=123,isComplete=false},
        {isHeader=false,questID=456,isComplete=true},
        {isHeader=false,questID=789,isComplete=0},
        {isHeader=false,questID=0},
      }
      local function info(i) return entries[i] end
      local quests, reasons=ns.Snapshot.readQuests(function() return #entries end,info,context,metadata)
      assert(quests['123'].accepted.value and quests['123'].completed.value == false)
      assert(quests['123'].completed.observed_at_utc_ms == 0)
      assert(quests['456'].completed.value == true)
      assert(quests['789'].completed.status == 'unknown')
      assert(quests['123'].turned_in.status == 'unknown' and next(quests['123'].objectives)==nil)
      assert(quests['0']==nil and quests['999']==nil)
      assert(reasons.quest_header_excluded==1 and reasons.quest_id_invalid==1)
      assert(reasons.quest_completion_unreadable==1 and reasons.objective_mapping_unavailable==1)
      assert(reasons.history_unobserved==1)
      local missing, why=ns.Snapshot.readQuests(nil,info,context,metadata)
      assert(next(missing)==nil and why.quest_api_missing==1)
      for _, count in ipairs({-1,1.5,false,'1',1025,math.huge}) do
        assert(not ns.Snapshot.readQuests(function() return count end,info,context,metadata))
      end
      assert(not ns.Snapshot.readQuests(function() return 2 end,function(i)
        if i==1 then return entries[2] end
        error('private error')
      end,context,metadata))
      assert(not ns.Snapshot.readQuests(function() return 2 end,function() return nil end,context,metadata))
      local value, code=ns.Snapshot.readQuests(function() return 2 end,function() return entries[2] end,context,metadata)
      assert(value==nil and code=='quest_duplicate')
      local calls=0
      value,code=ns.Snapshot.readQuests(function() calls=calls+1; return calls==1 and 1 or 0 end,
        info,context,metadata)
      assert(value==nil and code=='quest_log_changed')
      local reads=0
      value=assert(ns.Snapshot.readQuests(function() return 1024 end,function()
        reads=reads+1; return {isHeader=true}
      end,context,metadata))
      assert(reads==1024 and next(value)==nil)
      reads=0
      assert(not ns.Snapshot.readQuests(function() return 1025 end,function() reads=reads+1 end,context,metadata))
      assert(reads==0)
    "#).unwrap();
}

#[test]
fn explicit_client_capture_imports_and_failed_partial_replacement_is_atomic() {
    let sandbox = common::stub_client("1.60.1", "70338", 16001);
    sandbox
        .run::<()>(
            "setup",
            r#"
      function UnitLevel() return 12 end
      function C_QuestLog.GetNumQuestLogEntries() return 2 end
      function C_QuestLog.GetInfo(i)
        return {isHeader=false,questID=i,isComplete=i==2}
      end
    "#,
        )
        .unwrap();
    sandbox.load_toc_files().unwrap();
    let text: String = sandbox
        .run(
            "capture",
            r#"
      local state=ns.Diagnostics.state
      state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      state.onSlash('snapshot quests')
      assert(AzerothNavigatorDB.snapshot_quest_diagnostics.history_unobserved==1)
      assert(next(state.last_snapshot.objectives)==nil)
      return AzerothNavigatorDB.snapshot_export
    "#,
        )
        .unwrap();
    let value: Value = serde_json::from_str(&text[6..]).unwrap();
    assert_eq!(value["payload"]["quests"]["1"]["completed"]["value"], false);
    assert_eq!(value["payload"]["quests"]["2"]["completed"]["value"], true);
    assert_eq!(
        value["payload"]["quests"]["2"]["turned_in"]["status"],
        "unknown"
    );
    SnapshotApplication::in_memory()
        .unwrap()
        .import_text(&text, None)
        .unwrap();
    sandbox
        .run::<()>(
            "atomic",
            r#"
      local old=AzerothNavigatorDB.snapshot_export
      local oldDiagnostics=AzerothNavigatorDB.snapshot_quest_diagnostics
      function C_QuestLog.GetInfo(i)
        if i==1 then return {isHeader=false,questID=1,isComplete=true} end
        error('restricted read')
      end
      ns.Diagnostics.state.onSlash('snapshot quests')
      assert(AzerothNavigatorDB.snapshot_export==old and AzerothNavigatorDB.snapshot_sequence==1)
      assert(AzerothNavigatorDB.snapshot_quest_diagnostics==oldDiagnostics)
      function C_QuestLog.GetNumQuestLogEntries() return 1025 end
      ns.Diagnostics.state.onSlash('snapshot quests')
      assert(AzerothNavigatorDB.snapshot_export==old and AzerothNavigatorDB.snapshot_sequence==1)
      function GetBuildInfo() return '1.60.1','70205','',16001 end
      local read=false
      function C_QuestLog.GetNumQuestLogEntries() read=true; return 0 end
      ns.Diagnostics.state.onSlash('snapshot quests')
      assert(not read and AzerothNavigatorDB.snapshot_export==old)
    "#,
        )
        .unwrap();
}

#[test]
fn separate_completion_getter_is_bounded_strict_and_never_reads_history() {
    let sandbox = common::strict_core();
    sandbox.run::<()>("completion-getter", r#"
      local context={namespace='reference',build={edition='wow_forever',build='1.60.1.70374',interface='16001'}}
      local metadata={source='addon_snapshot',clock_trust='uncertain',observed_at_utc_ms=0}
      local entries={
        {isHeader=true,questID=99},
        {isHeader=false,questID=1,isComplete=true},
        {isHeader=false,questID=2,isComplete=false},
        {isHeader=false,questID=3,isComplete=true},
        {isHeader=false,questID=4,isComplete=true},
        {isHeader=false,questID=0},
      }
      local calls={}
      local function getter(id)
        calls[#calls+1]=id
        if id==1 then return false end
        if id==2 then return true end
        if id==3 then return 0 end
        error('private restricted details')
      end
      local quests,reasons=ns.Snapshot.readQuests(function() return #entries end,
        function(i) return entries[i] end,context,metadata,getter)
      assert(#calls==4 and calls[1]==1 and calls[4]==4)
      assert(quests['1'].completed.value==false and quests['2'].completed.value==true)
      assert(quests['1'].completed.observed_at_utc_ms==0)
      assert(quests['3'].completed.status=='unknown' and quests['4'].completed.status=='unknown')
      assert(reasons.quest_completion_unreadable==1 and reasons.quest_completion_failed==1)
      assert(reasons.history_unobserved==1 and reasons.objective_mapping_unavailable==1)
      assert(quests['2'].turned_in.status=='unknown' and next(quests['2'].objectives)==nil)
      local missing=assert(ns.Snapshot.readQuests(function() return 1 end,
        function() return {isHeader=false,questID=1} end,context,metadata,nil))
      assert(missing['1'].completed.status=='unknown')
      local count=0
      assert(not ns.Snapshot.readQuests(function() return 1025 end,
        function() error('must not read') end,context,metadata,function() count=count+1 end))
      assert(count==0)
      local large=assert(ns.Snapshot.readQuests(function() return 1024 end,
        function(i) return {isHeader=false,questID=i} end,context,metadata,
        function() count=count+1;return false end))
      assert(count==1024 and large['1024'].completed.value==false)
    "#).unwrap();
}

#[test]
fn separate_live_getter_reaches_actual_capture_and_rust_import() {
    let sandbox = common::stub_client("1.60.1", "70374", 16001);
    sandbox
        .run::<()>(
            "setup",
            r#"
      function UnitLevel() return 30 end
      function C_QuestLog.GetNumQuestLogEntries() return 2 end
      function C_QuestLog.GetInfo(i) return {isHeader=false,questID=i} end
      function C_QuestLog.IsComplete(id) return id==2 end
      function C_QuestLog.IsQuestFlaggedCompleted() error('history must not be read') end
    "#,
        )
        .unwrap();
    sandbox.load_toc_files().unwrap();
    let text: String = sandbox
        .run(
            "capture",
            r#"
      local state=ns.Diagnostics.state
      state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      state.onSlash('snapshot completion')
      assert(AzerothNavigatorDB.snapshot_quest_diagnostics.quest_completion_unreadable==nil)
      return AzerothNavigatorDB.snapshot_export
    "#,
        )
        .unwrap();
    let value: Value = serde_json::from_str(&text[6..]).unwrap();
    assert_eq!(value["payload"]["quests"]["1"]["completed"]["value"], false);
    assert_eq!(value["payload"]["quests"]["2"]["completed"]["value"], true);
    assert_eq!(
        value["payload"]["quests"]["2"]["turned_in"]["status"],
        "unknown"
    );
    SnapshotApplication::in_memory()
        .unwrap()
        .import_text(&text, None)
        .unwrap();
}
