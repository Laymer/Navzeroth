use azeroth_core::{
    content_v2::records::reference_records,
    contract::{Envelope, ReferenceDataset},
};
use serde_json::json;
use std::collections::BTreeSet;
#[test]
fn imagery_inventory_binds_embeddings_pins_and_sources_without_inventing_node_provenance() {
    let envelope: Envelope<ReferenceDataset> = serde_json::from_slice(include_bytes!(
        "../../../fixtures/contracts/v1/reference-dataset-imagery.json"
    ))
    .unwrap();
    let catalog = reference_records(&envelope.payload).unwrap();
    assert_eq!(catalog["geography"].len(), 5);
    let key = serde_json::to_string(&json!([
        "demo-imagery-r1",
        "map_embeddings",
        "demo-zone-1",
        "0",
        null,
        "demo-map-1",
        "0",
        null
    ]))
    .unwrap();
    assert_eq!(
        catalog["geography"][&key],
        BTreeSet::from(["demo-source-1".into()])
    );
    assert_eq!(catalog["travel"].len(), 1);
    assert!(catalog["travel"].values().all(BTreeSet::is_empty));
    let mut duplicated = envelope.payload.clone();
    duplicated
        .map_embeddings
        .push(duplicated.map_embeddings[0].clone());
    assert_eq!(
        reference_records(&duplicated).unwrap_err(),
        "Duplicate record identity across release packs"
    );
    let mut missing = envelope.payload.clone();
    missing.evidence.clear();
    assert_eq!(
        reference_records(&missing).unwrap_err(),
        "Unresolved record provenance"
    );
}
