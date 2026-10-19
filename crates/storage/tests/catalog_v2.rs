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
    conn.execute_batch("DROP TABLE catalog_asset_layers; DROP TABLE catalog_asset_blobs; ALTER TABLE dataset_revisions DROP COLUMN content_version; DELETE FROM schema_migrations WHERE version>=11").unwrap();
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

#[test]
fn dungeon_conflicts_validate_exact_v2_record_and_check_identities_and_refuse_v1() {
    use azeroth_core::{
        dungeon_composition::DungeonField,
        sources::{ConflictKind, RecordKey, SourceConflict},
        types::SourceId,
    };
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/v2/reference-content.json"
    ))
    .unwrap();
    let mut second = value["reference"]["payload"]["evidence"][0].clone();
    second["id"] = json!("other-proof");
    value["reference"]["payload"]["evidence"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let content = decode(&serde_json::to_vec(&value).unwrap()).unwrap();
    let old = v1();
    for (id, field, valid) in [
        ("dungeon-1", DungeonField::Map, true),
        ("dungeon-1", DungeonField::ChecklistCollection, true),
        (
            "dungeon-1",
            DungeonField::Checklist {
                check: SourceId::new("check-1").unwrap(),
            },
            true,
        ),
        (
            "dungeon-1",
            DungeonField::Checklist {
                check: SourceId::new("absent-check").unwrap(),
            },
            false,
        ),
        ("absent-dungeon", DungeonField::Map, false),
    ] {
        let conflict = SourceConflict {
            key: RecordKey::Dungeon {
                context: content.reference.data_context(),
                dungeon: SourceId::new(id).unwrap(),
                field,
            },
            kind: ConflictKind::ValueDisagreement,
            preferred: content.reference.payload.evidence[0].id.clone(),
            others: vec![content.reference.payload.evidence[1].id.clone()],
        };
        let input = ContentRevisionV2 {
            content: content.clone(),
            sources: old.sources.clone(),
            conflicts: vec![conflict.clone()],
        };
        let mut store = SqliteStore::open_in_memory().unwrap();
        assert_eq!(store.stage_content_v2_revision(&input).is_ok(), valid);
        let mut v1input = old.clone();
        let mut envelope = serde_json::to_value(&v1input.envelope).unwrap();
        envelope["payload"]["evidence"] = value["reference"]["payload"]["evidence"].clone();
        v1input.envelope = azeroth_core::contract::decode_envelope(&envelope.to_string()).unwrap();
        v1input.conflicts = vec![conflict];
        assert!(
            SqliteStore::open_in_memory()
                .unwrap()
                .stage_content_revision(&v1input)
                .is_err()
        );
    }
}

#[test]
fn typed_canonicalization_preserves_unknown_states_and_exact_stored_bytes() {
    let db = TempDb::new("typed-catalog-states");
    let mut revision = v2("typed-catalog-states", 1);
    // Exercise different unavailable declarations through actual v2 fields.
    let mut value = serde_json::to_value(&revision).unwrap();
    value["content"]["dungeons"] = serde_json::from_str::<Value>(include_str!(
        "../../../fixtures/contracts/v2/reference-content.json"
    ))
    .unwrap()["dungeons"]
        .clone();
    value["content"]["reference"]["payload"]["items"][0]["two_handed"]["value"] = json!(false);
    value["content"]["reference"]["payload"]["items"][0]["attributes"] =
        json!({"status":"known","value":{"stamina":0},"source":"synthetic_fixture"});
    value["content"]["dungeons"][0]["entrance"] = json!({"status":"restricted"});
    value["content"]["dungeons"][0]["access"] = json!({"status":"unsupported"});
    value["content"]["dungeons"][0]["preparation_quests"] = json!({"status":"known","value":[]});
    value["content"]["dungeons"][0]["travel_edges"] = json!({"status":"unknown"});
    value["content"]["dungeons"][0]["rewards"] = json!({"status":"not_applicable"});
    revision = serde_json::from_value(value).unwrap();
    // The former roundtrip must be a typed fixed point; exact serializer output
    // remains the stored contract, including false/zero and empty observations.
    let old_content = decode(&serde_json::to_vec(&revision.content).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(&old_content).unwrap(),
        serde_json::to_value(&revision.content).unwrap()
    );
    let mut store = SqliteStore::open(&db.path).unwrap();
    store.stage_content_v2_revision(&revision).unwrap();
    let expected = serde_json::to_string(&revision).unwrap();
    let before: (String,String) = raw(&db.path).query_row("SELECT document,content_sha256 FROM dataset_revisions JOIN dataset_manifests ON revision_id=id", [], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(before.0, expected);
    store.activate_content_revision(&revision.key()).unwrap();
    drop(store);
    let reopened = SqliteStore::open(&db.path).unwrap();
    let active = reopened
        .active_catalog_revision(&revision.key().scope)
        .unwrap()
        .unwrap();
    assert_eq!(serde_json::to_string(&active).unwrap(), expected);
    let after: (String,String) = raw(&db.path).query_row("SELECT document,content_sha256 FROM dataset_revisions JOIN dataset_manifests ON revision_id=id", [], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(before, after);
}

#[test]
fn malformed_typed_catalogs_still_fail_before_changing_stored_content() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let good = v2("typed-good", 0);
    store.stage_content_v2_revision(&good).unwrap();
    store.activate_content_revision(&good.key()).unwrap();
    for case in ["format", "schema", "graph", "capacity"] {
        let mut bad = good.clone();
        match case {
            "format" => bad.content.format = "wrong-format".into(),
            "schema" => bad.content.schema_version = 99,
            "graph" => {
                bad.content.node_facts.push(
                    serde_json::from_value(json!({
                        "node":"absent-node","evidence":[]
                    }))
                    .unwrap(),
                );
            }
            "capacity" => bad.sources = vec![good.sources[0].clone(); 1025],
            _ => unreachable!(),
        }
        assert!(store.stage_content_v2_revision(&bad).is_err(), "{case}");
        assert_eq!(
            store
                .active_catalog_revision(&good.key().scope)
                .unwrap()
                .unwrap()
                .key(),
            good.key()
        );
    }
}

#[test]
fn stored_original_json_rejects_nested_unknown_and_duplicate_fields_before_hash_check() {
    for case in ["unknown", "duplicate", "outer"] {
        let db = TempDb::new("strict-stored-wire");
        let revision = v2("wire-original", 1);
        let mut store = SqliteStore::open(&db.path).unwrap();
        store.stage_content_v2_revision(&revision).unwrap();
        store.activate_content_revision(&revision.key()).unwrap();
        drop(store);
        let conn = raw(&db.path);
        let document: String = conn
            .query_row("SELECT document FROM dataset_revisions", [], |r| r.get(0))
            .unwrap();
        let changed = match case {
            "unknown" => {
                let mut value: Value = serde_json::from_str(&document).unwrap();
                value["content"]["reference"]["payload"]["items"][0]["untyped_field"] = json!(true);
                serde_json::to_string(&value).unwrap()
            }
            "duplicate" => {
                document.replacen("\"content\":{", "\"content\":{\"schema_version\":2,", 1)
            }
            "outer" => document.replacen("{", "{\"unexpected_wrapper\":true,", 1),
            _ => unreachable!(),
        };
        conn.execute_batch("DROP TRIGGER dataset_revisions_no_update")
            .unwrap();
        conn.execute("UPDATE dataset_revisions SET document=?1", [changed])
            .unwrap();
        drop(conn);
        let store = SqliteStore::open(&db.path).unwrap();
        assert!(
            store
                .active_catalog_revision(&revision.key().scope)
                .is_err(),
            "{case}"
        );
    }
}
