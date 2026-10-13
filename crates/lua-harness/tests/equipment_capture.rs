//! Numeric slots and position API shapes are synthetic; no live verification.
mod common;
use azeroth_app::snapshots::SnapshotApplication;
use serde_json::Value;

#[test]
fn equipment_identity_preserves_unknown_variants_and_missing_links_do_not_mean_empty() {
    let sandbox = common::strict_core();
    sandbox.run::<()>("equipment",r#"
      local context={namespace='reference',build={edition='wow_forever',build='1.60.1.70338',interface='16001'}}
      local metadata={source='addon_snapshot',observed_at_utc_ms=0,clock_trust='uncertain'}
      local links={'|cffffffff|Hitem:00123:4:5:6|h[Synthetic item]|h|r',
        false,'|Hitem:0:0|h[zero]|h','|Hitem:12:0|h[Test]|h'}
      local equipment,reasons=ns.Snapshot.readEquipment(function(slot)
        if slot==5 then error('private restriction error') end
        return links[slot]
      end,context,metadata)
      assert(equipment['inventory-slot-1'].value.item.item_id=='123')
      assert(equipment['inventory-slot-1'].observed_at_utc_ms==0)
      assert(equipment['inventory-slot-1'].value.item.variant.status=='unknown')
      assert(equipment['inventory-slot-1'].value.item.instance.status=='unknown')
      assert(equipment['inventory-slot-1'].value.item.modifiers.status=='unknown')
      assert(equipment['inventory-slot-2'].status=='unknown' and equipment['inventory-slot-3'].status=='unknown')
      assert(equipment['inventory-slot-4'].value.item.item_id=='12')
      assert(equipment['inventory-slot-5'].status=='unknown' and equipment['inventory-slot-6'].status=='unknown')
      assert(reasons.equipment_link_unreadable==3 and reasons.equipment_link_absent_unknown==14)
      equipment,reasons=ns.Snapshot.readEquipment(nil,context,metadata)
      assert(equipment['inventory-slot-1'].status=='unsupported' and reasons.equipment_api_missing==19)
      local called=0
      equipment=ns.Snapshot.readEquipment(function() called=called+1; return string.rep('x',513) end,context,metadata)
      assert(called==19 and equipment['inventory-slot-19'].status=='unknown')
    "#).unwrap();
}

#[test]
fn position_readability_keeps_zero_without_creating_map_floor_or_instance_identity() {
    let sandbox = common::strict_core();
    sandbox.run::<()>("position",r#"
      local readable=ns.Snapshot.positionDiagnostics(function() return 123 end,function(map)
        assert(map==123); return {GetXY=function() return 0,0 end}
      end)
      assert(readable.map_readable and readable.coordinates_readable)
      assert(readable.code=='position_readable_mapping_unknown' and readable.location_status=='unknown')
      assert(readable.x==nil and readable.map==nil)
      assert(ns.Snapshot.positionDiagnostics(nil,nil).code=='position_api_missing')
      for _,value in ipairs({false,0,-1,'123',1.5}) do
        local result=ns.Snapshot.positionDiagnostics(function() return value end,function() error('unused') end)
        assert(result.code=='position_read_failed' and not result.map_readable)
      end
      for _,value in ipairs({false,-1,2,math.huge}) do
        local result=ns.Snapshot.positionDiagnostics(function() return 123 end,function()
          return {GetXY=function() return value,0 end}
        end)
        assert(result.map_readable and not result.coordinates_readable)
      end
      assert(ns.Snapshot.positionDiagnostics(function() error('private read') end,function() end).code=='position_read_failed')
    "#).unwrap();
}

#[test]
fn explicit_capture_imports_item_identity_and_position_command_never_changes_location() {
    let sandbox = common::stub_client("1.60.1", "70338", 16001);
    sandbox
        .run::<()>(
            "client",
            r#"
      function UnitLevel() return 12 end
      function GetInventoryItemLink(unit,slot)
        assert(unit=='player')
        if slot==1 then return '|Hitem:123:0|h[Synthetic item]|h' end
      end
      function C_Map.GetBestMapForUnit(unit) assert(unit=='player'); return 123 end
      function C_Map.GetPlayerMapPosition(map,unit)
        assert(map==123 and unit=='player'); return {GetXY=function() return 0,0 end}
      end
    "#,
        )
        .unwrap();
    sandbox.load_toc_files().unwrap();
    let captured:String=sandbox.run("capture",r#"
      local state=ns.Diagnostics.state
      state.onEvent(nil,'ADDON_LOADED','AzerothNavigator')
      state.onSlash('snapshot equipment-profile')
      assert(AzerothNavigatorDB.snapshot_equipment_diagnostics.equipment_link_absent_unknown==18)
      local old=AzerothNavigatorDB.snapshot_export
      state.onSlash('position')
      assert(AzerothNavigatorDB.position_diagnostics.coordinates_readable)
      assert(AzerothNavigatorDB.snapshot_export==old and state.last_snapshot.location.status=='unknown')
      return old
    "#).unwrap();
    let value: Value = serde_json::from_str(&captured[6..]).unwrap();
    assert_eq!(
        value["payload"]["equipment"]["inventory-slot-1"]["value"]["item"]["item_id"],
        "123"
    );
    assert_eq!(
        value["payload"]["equipment"]["inventory-slot-2"]["status"],
        "unknown"
    );
    assert_eq!(value["payload"]["location"]["status"], "unknown");
    SnapshotApplication::in_memory()
        .unwrap()
        .import_text(&captured, None)
        .unwrap();
    sandbox
        .run::<()>(
            "wrong_build",
            r#"
      function GetBuildInfo() return '1.60.1','70205','',16001 end
      local called=false
      function C_Map.GetBestMapForUnit() called=true end
      ns.Diagnostics.state.onSlash('position')
      assert(not called)
    "#,
        )
        .unwrap();
}
