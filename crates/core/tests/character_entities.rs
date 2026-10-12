mod common;
use azeroth_core::entities::*;
use common::*;
use serde_json::{from_value, json, to_value};
#[test]
fn profile_snapshot_and_party_preserve_unknowns_and_identity() {
    let profile = json!({"id":"char1","context":context(),"region":unknown(),"realm":unknown(),"race":unknown(),"class":unknown(),"level":unknown(),"role":unknown(),"preferences":{}});
    let v: CharacterProfile = from_value(profile.clone()).unwrap();
    assert!(from_value::<CharacterProfile>(to_value(v).unwrap()).is_ok());
    let mut bad = profile;
    bad["level"] = json!({"status":"known","value":0});
    assert!(from_value::<CharacterProfile>(bad).is_err());
    let snap: CharacterSnapshot = from_value(snapshot()).unwrap();
    assert!(from_value::<CharacterSnapshot>(to_value(&snap).unwrap()).is_ok());
    let mut bad = snapshot();
    bad["capture"] = json!({"sequence":1});
    assert!(from_value::<CharacterSnapshot>(bad).is_err());
    let party: PartySnapshot =
        from_value(json!({"id":"party1","context":context(),"participants":[snap]})).unwrap();
    assert!(from_value::<PartySnapshot>(to_value(&party).unwrap()).is_ok());
    let mut bad = to_value(party).unwrap();
    bad["participants"].as_array_mut().unwrap().push(snapshot());
    assert!(from_value::<PartySnapshot>(bad).is_err());
    let mut bad = snapshot();
    bad["equipment"] = json!({"main_hand":{"status":"known","value":{"state":"item","item":{"context":{"build":{"edition":"wow_classic","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"demo"},"item_id":"1","variant":unknown(),"instance":unknown(),"modifiers":unknown()}}}});
    assert!(from_value::<CharacterSnapshot>(bad).is_err());
}
