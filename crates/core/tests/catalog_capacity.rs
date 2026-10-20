use azeroth_core::{
    content_v2::decode,
    contract::{ReferenceDataset, decode_envelope},
};
use serde_json::{Value, json};
fn reference() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/reference-dataset.json"
    ))
    .unwrap()
}
fn content(reference: Value) -> Value {
    json!({"format":"navzeroth-reference-content","schema_version":2,"reference":reference,"models":[],"equipment":[],"quest_facts":[],"acquisition_facts":[],"dungeons":[]})
}
#[test]
fn v2_supports_large_catalog_and_v1_still_refuses_it() {
    let mut base = reference();
    let item = base["payload"]["items"][0].clone();
    for n in 0..2048 {
        let mut next = item.clone();
        next["key"]["item_id"] = format!("large-item-{n}").into();
        base["payload"]["items"].as_array_mut().unwrap().push(next);
    }
    assert!(decode_envelope::<ReferenceDataset>(&serde_json::to_string(&base).unwrap()).is_err());
    let parsed = decode(&serde_json::to_vec(&content(base)).unwrap()).unwrap();
    assert!(parsed.reference.payload.items.len() > 2048);
}
#[test]
fn catalog_keeps_cross_region_graph_references_and_refuses_missing_ones() {
    let mut base = reference();
    let map = base["payload"]["map_spaces"][0].clone();
    let mut other = map.clone();
    other["key"]["map_id"] = "region-two".into();
    base["payload"]["map_spaces"]
        .as_array_mut()
        .unwrap()
        .push(other);
    let mut cross_node = base["payload"]["nodes"][0].clone();
    cross_node["id"] = "region-two-node".into();
    cross_node["position"]["map"] = base["payload"]["map_spaces"][1]["key"].clone();
    let mut cross_edge = base["payload"]["edges"][0].clone();
    cross_edge["id"] = "cross-region-edge".into();
    cross_edge["to"] = json!({"id":"region-two-node","map":cross_node["position"]["map"]});
    base["payload"]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(cross_node);
    base["payload"]["edges"]
        .as_array_mut()
        .unwrap()
        .push(cross_edge);
    let parsed = decode(&serde_json::to_vec(&content(base.clone())).unwrap()).unwrap();
    assert_eq!(parsed.reference.payload.map_spaces.len(), 2);
    base["payload"]["map_spaces"] = json!([]);
    assert!(decode(&serde_json::to_vec(&content(base)).unwrap()).is_err());
}
#[test]
fn indexed_pages_are_revision_bound_and_at_most_200_rows() {
    use azeroth_core::{
        catalog::{CatalogIndex, Collection, PageRequest},
        types::Sha256Hex,
    };
    let mut base = reference();
    let item = base["payload"]["items"][0].clone();
    for n in 0..401 {
        let mut next = item.clone();
        next["key"]["item_id"] = format!("page-item-{n}").into();
        base["payload"]["items"].as_array_mut().unwrap().push(next);
    }
    let content = decode(&serde_json::to_vec(&content(base)).unwrap()).unwrap();
    let digest = Sha256Hex::new("a".repeat(64)).unwrap();
    let index = CatalogIndex::new(&content, digest.clone()).unwrap();
    let mut request = PageRequest {
        revision: content.reference.payload.revision.clone(),
        catalog_sha256: digest,
        collection: Collection::Items,
        offset: 0,
        limit: 200,
    };
    let page = index.page(&request).unwrap();
    assert_eq!(page.rows.len(), 200);
    assert_eq!(page.next_offset, Some(200));
    assert_eq!(
        index.find(Collection::Items, &page.rows[0].identity),
        Some(page.rows[0].record_index)
    );
    request.offset = 200;
    assert_eq!(index.page(&request).unwrap().rows.len(), 200);
    request.offset = 400;
    assert!(index.page(&request).unwrap().rows.len() < 200);
    request.limit = 201;
    assert!(index.page(&request).is_err());
    request.limit = 0;
    assert!(index.page(&request).is_err());
    request.limit = 200;
    request.catalog_sha256 = Sha256Hex::new("b".repeat(64)).unwrap();
    assert!(index.page(&request).is_err());
}
#[test]
fn catalog_identity_and_evidence_limits_are_independent_and_do_not_truncate() {
    use azeroth_core::contract::validate_catalog_capacity;
    use azeroth_core::contract::{MAX_CATALOG_EVIDENCE, MAX_CATALOG_IDENTITIES};
    assert_eq!(MAX_CATALOG_IDENTITIES, 500_000);
    assert!(validate_catalog_capacity(MAX_CATALOG_IDENTITIES, MAX_CATALOG_EVIDENCE).is_ok());
    assert!(validate_catalog_capacity(MAX_CATALOG_IDENTITIES + 1, 0).is_err());
    assert!(validate_catalog_capacity(0, MAX_CATALOG_EVIDENCE + 1).is_err());
    assert!(validate_catalog_capacity(usize::MAX, 0).is_err());
}
