mod common;
use azeroth_core::{
    catalog::{Collection, PageRequest},
    content_v2::decode,
};
use azeroth_storage::{
    SqliteStore,
    content::{ActiveContentRevision, ContentRevision, ContentRevisionV2, catalog_content_sha256},
    content_dispositions::{DispositionMutation, DispositionReason, DispositionState},
};
use common::{TempDb, raw};
use serde_json::{Value, json};
fn v1() -> ContentRevision {
    serde_json::from_str(include_str!(
        "../../../fixtures/desktop/planning/synthetic-revision.json"
    ))
    .unwrap()
}
fn v2(name: &str, extra: usize) -> ContentRevisionV2 {
    let old = v1();
    let mut reference = serde_json::to_value(&old.envelope).unwrap();
    reference["payload"]["revision"] = name.into();
    let sample: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v1/reference-dataset.json"
    ))
    .unwrap();
    let item = sample["payload"]["items"][0].clone();
    for n in 0..extra {
        let mut next = item.clone();
        next["key"]["item_id"] = format!("catalog-item-{n}").into();
        reference["payload"]["items"]
            .as_array_mut()
            .unwrap()
            .push(next);
    }
    let value = json!({"format":"navzeroth-reference-content","schema_version":2,"reference":reference,"models":[],"equipment":[],"quest_facts":[],"acquisition_facts":[],"dungeons":[]});
    ContentRevisionV2 {
        content: decode(&serde_json::to_vec(&value).unwrap()).unwrap(),
        sources: old.sources,
        conflicts: vec![],
    }
}
#[test]
fn migration_preserves_v1_bytes_hashes_and_pointer_then_rolls_between_versions() {
    let db = TempDb::new("catalog-migration");
    let old = v1();
    let mut store = SqliteStore::open(&db.path).unwrap();
    store.stage_content_revision(&old).unwrap();
    store.activate_content_revision(&old.key()).unwrap();
    drop(store);
    let conn = raw(&db.path);
    let before:(String,String)=conn.query_row("SELECT document,content_sha256 FROM dataset_revisions JOIN dataset_manifests ON revision_id=id",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    conn.execute_batch("DROP TABLE bundled_content_defaults; DROP TABLE content_travel_choices; DROP TABLE catalog_asset_layers; DROP TABLE catalog_asset_blobs; ALTER TABLE dataset_revisions DROP COLUMN content_version; DELETE FROM schema_migrations WHERE version>=11").unwrap();
    drop(conn);
    let mut store = SqliteStore::open(&db.path).unwrap();
    let after:(String,String)=raw(&db.path).query_row("SELECT document,content_sha256 FROM dataset_revisions JOIN dataset_manifests ON revision_id=id",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(before, after);
    let next = v2("catalog-v2", 2048);
    store.stage_content_v2_revision(&next).unwrap();
    store.activate_content_revision(&next.key()).unwrap();
    assert!(store.active_content_revision(&next.key().scope).is_err());
    let active = store
        .active_catalog_revision(&next.key().scope)
        .unwrap()
        .unwrap();
    assert!(active.reference().items.len() >= 2048);
    let again = store
        .active_catalog_revision(&next.key().scope)
        .unwrap()
        .unwrap();
    match (&active, &again) {
        (ActiveContentRevision::V2(a), ActiveContentRevision::V2(b)) => {
            assert!(std::sync::Arc::ptr_eq(a, b))
        }
        _ => panic!("expected v2 cache"),
    };
    store
        .rollback_content_revision(&next.key().scope)
        .unwrap()
        .unwrap();
    assert_eq!(
        store
            .active_content_revision(&old.key().scope)
            .unwrap()
            .unwrap()
            .key(),
        old.key()
    );
    store
        .rollback_content_revision(&next.key().scope)
        .unwrap()
        .unwrap();
    assert_eq!(
        store
            .active_catalog_revision(&old.key().scope)
            .unwrap()
            .unwrap()
            .key(),
        next.key()
    );
    azeroth_storage::recovery::validate(&store).unwrap();
}
#[test]
fn v2_pages_reject_stale_tokens_and_external_tampering_invalidates_cache() {
    let db = TempDb::new("catalog-page-cache");
    let mut store = SqliteStore::open(&db.path).unwrap();
    let next = v2("paged-v2", 401);
    store.stage_content_v2_revision(&next).unwrap();
    store.activate_content_revision(&next.key()).unwrap();
    let mut req = PageRequest {
        revision: next.key().revision,
        catalog_sha256: catalog_content_sha256(&next.content).unwrap(),
        collection: Collection::Items,
        offset: 0,
        limit: 200,
    };
    let initial = store
        .catalog_first_page(&next.key().scope, Collection::Items, 200)
        .unwrap();
    assert_eq!(initial.catalog_sha256, req.catalog_sha256);
    assert_eq!(initial.revision, req.revision);
    let first = store.catalog_page(&next.key().scope, &req).unwrap();
    assert_eq!(first.rows.len(), 200);
    req.offset = 200;
    assert_eq!(
        store
            .catalog_page(&next.key().scope, &req)
            .unwrap()
            .rows
            .len(),
        200
    );
    req.catalog_sha256 = serde_json::from_value(Value::String("f".repeat(64))).unwrap();
    assert!(store.catalog_page(&next.key().scope, &req).is_err());
    raw(&db.path)
        .execute_batch(
            "DROP TRIGGER dataset_revisions_no_update; UPDATE dataset_revisions SET document='{}'",
        )
        .unwrap();
    assert!(store.active_catalog_revision(&next.key().scope).is_err());
    assert!(store.catalog_page(&next.key().scope, &req).is_err());
}
#[test]
fn v2_restriction_falls_back_to_eligible_v1_and_invalid_stage_preserves_active() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let old = v1();
    store.stage_content_revision(&old).unwrap();
    store.activate_content_revision(&old.key()).unwrap();
    let next = v2("restricted-v2", 0);
    store.stage_content_v2_revision(&next).unwrap();
    store.activate_content_revision(&next.key()).unwrap();
    let d = store.content_disposition(&next.key()).unwrap().unwrap();
    store
        .change_content_disposition(
            &DispositionMutation {
                guard: d.guard(),
                state: DispositionState::Quarantined,
                reason_code: DispositionReason::OperatorReview,
            },
            None,
        )
        .unwrap();
    assert_eq!(
        store
            .active_catalog_revision(&old.key().scope)
            .unwrap()
            .unwrap()
            .key(),
        old.key()
    );
    assert!(store.validate_usable_catalog_guard(&d.guard()).is_err());
    let mut invalid = v2("invalid-v2", 0);
    invalid.sources.clear();
    assert!(store.stage_content_v2_revision(&invalid).is_err());
    assert_eq!(
        store
            .active_content_revision(&old.key().scope)
            .unwrap()
            .unwrap()
            .key(),
        old.key()
    );
}

#[test]
fn v2_conflict_closure_is_preserved_and_invalid_replacements_are_atomic() {
    use azeroth_core::sources::{ConflictKind, RecordKey, SourceConflict};
    let mut store = SqliteStore::open_in_memory().unwrap();
    let mut revision = v2("conflict-closure", 1);
    let data = &revision.content.reference.payload;
    let conflict = SourceConflict {
        key: RecordKey::Item(data.items[0].key.clone()),
        kind: ConflictKind::CrossEditionRelabel,
        preferred: data.evidence[0].id.clone(),
        others: vec![],
    };
    revision.conflicts.push(conflict.clone());
    store.stage_content_v2_revision(&revision).unwrap();
    store.activate_content_revision(&revision.key()).unwrap();
    assert_eq!(
        store
            .active_content_v2_revision(&revision.key().scope)
            .unwrap()
            .unwrap()
            .conflicts
            .as_slice(),
        std::slice::from_ref(&conflict)
    );
    let mut variants = Vec::new();
    let mut empty = conflict.clone();
    empty.kind = ConflictKind::ValueDisagreement;
    variants.push(vec![empty]);
    let mut absent = conflict.clone();
    absent.preferred = azeroth_core::types::EvidenceId::new("missing-evidence").unwrap();
    variants.push(vec![absent]);
    let mut self_reference = conflict.clone();
    self_reference.others = vec![self_reference.preferred.clone()];
    variants.push(vec![self_reference]);
    let mut absent_item = conflict.clone();
    let RecordKey::Item(ref mut item) = absent_item.key else {
        unreachable!()
    };
    item.item_id = azeroth_core::types::ItemId::new("absent-item").unwrap();
    variants.push(vec![absent_item]);
    let mut absent_quest = conflict.clone();
    absent_quest.key = RecordKey::Quest {
        context: revision.content.reference.data_context(),
        quest: azeroth_core::types::QuestId::new("absent-quest").unwrap(),
    };
    variants.push(vec![absent_quest]);
    variants.push(vec![conflict.clone(), conflict]);
    for conflicts in variants {
        let mut invalid = revision.clone();
        invalid.conflicts = conflicts;
        assert!(store.stage_content_v2_revision(&invalid).is_err());
        assert_eq!(
            store
                .active_content_v2_revision(&revision.key().scope)
                .unwrap()
                .unwrap()
                .conflicts,
            revision.conflicts
        );
    }
}
