//! Connection-local immutable-content invalidation, independent of player writes.
use crate::error::SqliteResult;
use azeroth_core::error::Result;
use rusqlite::Connection;
use std::{
    cell::Cell,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Stamp {
    local: u64,
    external: i64,
    main_schema: i64,
    temp_schema: i64,
    temp_shadow: bool,
}

pub(crate) struct Generation {
    local: Arc<AtomicU64>,
    read: Cell<Option<Stamp>>,
}
impl Generation {
    pub(crate) fn new(conn: &Connection) -> Result<Self> {
        let local = Arc::new(AtomicU64::new(0));
        let changed = local.clone();
        // These four tables are ROWID tables. Every content insert/update/delete,
        // including a trigger's writes, invalidates. Schema changes are separate.
        // SqliteStore exclusively owns this hook; its connection is not public.
        conn.update_hook(Some(move |_, database: &str, table: &str, _| {
            if database.eq_ignore_ascii_case("main")
                && [
                    "dataset_revisions",
                    "dataset_manifests",
                    "source_records",
                    "source_conflicts",
                ]
                .iter()
                .any(|name| table.eq_ignore_ascii_case(name))
            {
                // Terminal overflow disables caching permanently; never wrap.
                let _ =
                    changed.try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1));
            }
        }))
        .db()?;
        Ok(Self {
            local,
            read: Cell::new(None),
        })
    }
    fn current(&self, conn: &Connection) -> Result<Stamp> {
        Ok(Stamp {
            // An unsupported temporary shadow may lack ROWID/update hooks.
            // Preserve full validation on every access instead of caching it.
            temp_shadow: conn.query_row("SELECT EXISTS(SELECT 1 FROM temp.sqlite_schema WHERE name COLLATE NOCASE IN ('dataset_revisions','dataset_manifests','source_records','source_conflicts'))", [], |r| r.get(0)).db()?,
            local: self.local.load(Ordering::Relaxed),
            external: conn
                .pragma_query_value(Some("main"), "data_version", |r| r.get(0))
                .db()?,
            main_schema: conn
                .pragma_query_value(Some("main"), "schema_version", |r| r.get(0))
                .db()?,
            temp_schema: conn
                .pragma_query_value(Some("temp"), "schema_version", |r| r.get(0))
                .db()?,
        })
    }
    pub(crate) fn begin_read<'a>(&'a self, conn: &Connection) -> Result<ReadScope<'a>> {
        let stamp = self.current(conn)?;
        let previous = self.read.replace(Some(stamp));
        Ok(ReadScope {
            read: &self.read,
            previous,
        })
    }
    pub(crate) fn cache_stamp(&self, conn: &Connection) -> Result<Option<Stamp>> {
        let Some(entry) = self.read.get() else {
            return Ok(None);
        };
        let now = self.current(conn)?;
        // Never publish/reuse transaction-local data after a write or DDL. In
        // arbitrary write/savepoint transactions no managed read scope exists.
        // Thus rollback, including ROLLBACK TO and reused row IDs, cannot leak a
        // transient catalog into the committed cache, even when schema rolls back.
        Ok((now.local != u64::MAX && !now.temp_shadow && entry == now).then_some(now))
    }
}
pub(crate) struct ReadScope<'a> {
    read: &'a Cell<Option<Stamp>>,
    previous: Option<Stamp>,
}
impl Drop for ReadScope<'_> {
    fn drop(&mut self) {
        self.read.set(self.previous);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ImportSource, ProfileKey, SnapshotStore, SqliteStore,
        content::{ContentRevision, ContentRevisionV2},
        reminders::StoredReminderState,
    };
    use azeroth_core::{
        catalog::{CatalogIndex, Collection},
        contract::decode_envelope,
        observation::{Observation, ObservationMetadata},
        types::PreferenceId,
    };
    use serde_json::{Value, json};
    use std::panic::{AssertUnwindSafe, catch_unwind};

    fn input(name: &str, note: &str) -> ContentRevisionV2 {
        let old: ContentRevision = serde_json::from_str(include_str!(
            "../../../fixtures/desktop/planning/synthetic-revision.json"
        ))
        .unwrap();
        let mut reference = serde_json::to_value(old.envelope).unwrap();
        reference["payload"]["revision"] = json!(name);
        reference["payload"]["evidence"][0]["notes"] = json!(note);
        let content = azeroth_core::content_v2::decode(
            &serde_json::to_vec(&json!({
            "format":"navzeroth-reference-content", "schema_version":2,
            "reference":reference, "models":[], "equipment":[], "quest_facts":[],
            "acquisition_facts":[], "dungeons":[] }))
            .unwrap(),
        )
        .unwrap();
        ContentRevisionV2 {
            content,
            sources: old.sources,
            conflicts: vec![],
        }
    }
    fn store() -> (SqliteStore, ContentRevisionV2) {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let input = input("cache-base", "committed original");
        store.stage_content_v2_revision(&input).unwrap();
        store.activate_content_revision(&input.key()).unwrap();
        (store, input)
    }
    fn pair(
        store: &SqliteStore,
        input: &ContentRevisionV2,
    ) -> (Arc<ContentRevisionV2>, Arc<CatalogIndex>) {
        let content = store
            .active_content_v2_revision(&input.key().scope)
            .unwrap()
            .unwrap();
        store
            .catalog_first_page(&input.key().scope, Collection::Items, 1)
            .unwrap();
        let index = store
            .catalog_index_cache
            .borrow()
            .as_ref()
            .unwrap()
            .2
            .clone();
        (content, index)
    }
    fn same(
        a: &(Arc<ContentRevisionV2>, Arc<CatalogIndex>),
        b: &(Arc<ContentRevisionV2>, Arc<CatalogIndex>),
    ) -> bool {
        Arc::ptr_eq(&a.0, &b.0) && Arc::ptr_eq(&a.1, &b.1)
    }
    fn canonical(name: &str, note: &str) -> ContentRevisionV2 {
        let mut temporary = SqliteStore::open_in_memory().unwrap();
        let input = input(name, note);
        temporary.stage_content_v2_revision(&input).unwrap();
        match temporary.catalog_revision(&input.key()).unwrap().unwrap() {
            crate::content::ActiveContentRevision::V2(value) => (*value).clone(),
            _ => panic!("expected canonical v2"),
        }
    }
    fn stage_in_transaction(store: &SqliteStore, input: &ContentRevisionV2) {
        crate::content_catalog::stage_canonical(&store.conn, input).unwrap();
        let id = crate::content::find(&store.conn, &input.key())
            .unwrap()
            .unwrap();
        crate::content::activate(&store.conn, id, input.key()).unwrap();
    }

    fn loads() -> usize {
        crate::content::FULL_LOADS.with(Cell::get)
    }
    #[test]
    fn duplicate_staging_retains_strict_sql_catalog_but_new_insert_stays_cold() {
        let mut store = SqliteStore::open_in_memory().unwrap();
        let input = input("retained-duplicate", "original");
        let before = loads();
        store.stage_content_v2_revision(&input).unwrap();
        assert_eq!(loads(), before);
        assert!(store.catalog_cache.borrow().is_none());
        store.stage_content_v2_revision(&input).unwrap();
        assert_eq!(loads(), before + 1);
        let cached = store.catalog_cache.borrow().as_ref().unwrap().2.clone();
        store.stage_content_v2_revision(&input).unwrap();
        store.apply_bundle_default("1.0.0", &input.key()).unwrap();
        let active = store
            .active_content_v2_revision(&input.key().scope)
            .unwrap()
            .unwrap();
        assert!(
            matches!(cached, crate::content::ActiveContentRevision::V2(value) if Arc::ptr_eq(&value, &active))
        );
        assert_eq!(
            loads(),
            before + 1,
            "duplicate/default/first-active must reuse strict SQL decode"
        );
        store
            .activate_content_with_travel(&input.key(), false)
            .unwrap();
        assert!(!store.apply_bundle_default("1.0.0", &input.key()).unwrap());
        assert!(!store.content_accepts_inferred(&input.key()).unwrap());
        assert!(store.apply_bundle_default("1.0.1", &input.key()).unwrap());
        assert!(store.content_accepts_inferred(&input.key()).unwrap());
        assert_eq!(loads(), before + 1);
        assert!(store.catalog_generation.read.get().is_none());
    }

    #[test]
    fn duplicate_cache_never_admits_conflicting_input_or_changed_provenance() {
        for sql in [
            "UPDATE source_records SET document=document||' '",
            "INSERT INTO source_conflicts VALUES(1,0,'{}')",
            "UPDATE dataset_manifests SET coverage_document='{}'",
            "UPDATE dataset_manifests SET content_sha256=printf('%064d',0)",
            "UPDATE dataset_revisions SET document='{}'",
        ] {
            let (mut store, original) = store();
            store.conn.execute_batch("DROP TRIGGER source_records_no_update; DROP TRIGGER source_conflicts_sealed; DROP TRIGGER dataset_manifests_no_update; DROP TRIGGER dataset_revisions_no_update").unwrap();
            store.stage_content_v2_revision(&original).unwrap();
            assert!(
                store
                    .stage_content_v2_revision(&input("cache-base", "different input"))
                    .is_err()
            );
            store.conn.execute_batch(sql).unwrap();
            assert!(store.stage_content_v2_revision(&original).is_err(), "{sql}");
            assert!(
                store
                    .apply_bundle_default("1.0.0", &original.key())
                    .is_err(),
                "{sql}"
            );
            assert!(store.catalog_generation.read.get().is_none());
        }
    }

    #[test]
    fn duplicate_default_trigger_rollback_and_historical_corruption_remain_checked() {
        let (mut store, first) = store();
        let second = input("retained-next", "second");
        store.stage_content_v2_revision(&second).unwrap();
        store.stage_content_v2_revision(&second).unwrap();
        let original_pointer = crate::content::pointers(&store.conn, &first.key().scope).unwrap();
        let manifest_guard: String = store
            .conn
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE name='dataset_manifests_no_update'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        store
            .conn
            .execute_batch("DROP TRIGGER dataset_manifests_no_update")
            .unwrap();
        store.conn.execute_batch("CREATE TRIGGER reject_default BEFORE INSERT ON bundled_content_defaults BEGIN UPDATE dataset_manifests SET coverage_document='{}'; SELECT RAISE(ABORT,'injected default failure'); END").unwrap();
        assert!(store.apply_bundle_default("1.0.0", &second.key()).is_err());
        assert_eq!(
            crate::content::pointers(&store.conn, &first.key().scope).unwrap(),
            original_pointer
        );
        assert!(
            store
                .bundle_default_state(&first.key().scope)
                .unwrap()
                .is_none()
        );
        assert!(!store.content_accepts_inferred(&second.key()).unwrap());
        assert!(store.catalog_generation.read.get().is_none());
        store
            .conn
            .execute_batch("DROP TRIGGER reject_default")
            .unwrap();
        let before = loads();
        store.stage_content_v2_revision(&second).unwrap();
        assert_eq!(
            loads(),
            before + 1,
            "rollback/schema mutation cannot preserve the old cache stamp"
        );
        store.conn.execute("UPDATE dataset_manifests SET coverage_document='{}' WHERE revision_id=(SELECT id FROM dataset_revisions WHERE revision=?1)", [first.key().revision.as_str()]).unwrap();
        store.conn.execute_batch(&manifest_guard).unwrap();
        store.stage_content_v2_revision(&second).unwrap();
        assert!(
            crate::recovery::validate(&store).is_err(),
            "unrelated duplicate must not exempt corrupt historical rows"
        );
    }

    #[test]
    fn duplicate_staging_rechecks_external_file_changes_before_reusing_arc() {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "navzeroth-duplicate-file-{}-{}.sqlite",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut store = SqliteStore::open(&path).unwrap();
        let input = input("external-duplicate", "original");
        store.stage_content_v2_revision(&input).unwrap();
        store.stage_content_v2_revision(&input).unwrap();
        let before = store
            .active_content_v2_revision(&input.key().scope)
            .unwrap();
        assert!(before.is_none());
        let writer = rusqlite::Connection::open(&path).unwrap();
        writer
            .execute_batch("DROP TRIGGER source_records_no_update")
            .unwrap();
        store.stage_content_v2_revision(&input).unwrap();
        writer
            .execute("UPDATE source_records SET document=document||' '", [])
            .unwrap();
        assert!(store.stage_content_v2_revision(&input).is_err());
        assert!(store.apply_bundle_default("1.0.0", &input.key()).is_err());
        drop(writer);
        drop(store);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn travel_activation_and_catalog_lookup_reuse_validated_immutable_catalog() {
        let (mut store, input) = store();
        let before = pair(&store, &input);
        let count = loads();
        for accept in [false, true, false] {
            let catalog = store.catalog_revision(&input.key()).unwrap().unwrap();
            assert!(
                matches!(catalog, crate::content::ActiveContentRevision::V2(ref value) if Arc::ptr_eq(value, &before.0))
            );
            store
                .activate_content_with_travel(&input.key(), accept)
                .unwrap();
            assert_eq!(
                store.content_accepts_inferred(&input.key()).unwrap(),
                accept
            );
            assert!(same(&before, &pair(&store, &input)));
        }
        assert_eq!(
            loads(),
            count,
            "warm toggles must not decode discarded copies"
        );
        assert!(store.catalog_generation.read.get().is_none());
    }
    #[test]
    fn catalog_lookup_inside_arbitrary_savepoint_cannot_reuse_or_publish_cache() {
        let (mut store, input) = store();
        let before = pair(&store, &input);
        let count = loads();
        store
            .conn
            .execute_batch("BEGIN IMMEDIATE; SAVEPOINT transient")
            .unwrap();
        for _ in 0..2 {
            let catalog = store.catalog_revision(&input.key()).unwrap().unwrap();
            assert!(
                matches!(catalog, crate::content::ActiveContentRevision::V2(ref value) if !Arc::ptr_eq(value, &before.0))
            );
        }
        assert_eq!(loads(), count + 2);
        assert!(
            store
                .activate_content_with_travel(&input.key(), true)
                .is_err()
        );
        store
            .conn
            .execute_batch("ROLLBACK TO transient; RELEASE transient; ROLLBACK")
            .unwrap();
        assert!(same(&before, &pair(&store, &input)));
        assert!(store.catalog_generation.read.get().is_none());
    }
    #[test]
    fn travel_activation_revalidates_tampered_content_sources_coverage_and_hash() {
        for (prepare, mutation) in [
            (
                "DROP TRIGGER dataset_revisions_no_update",
                "UPDATE dataset_revisions SET document='{}'",
            ),
            (
                "DROP TRIGGER source_records_no_update",
                "UPDATE source_records SET document='{}'",
            ),
            (
                "DROP TRIGGER dataset_manifests_no_update",
                "UPDATE dataset_manifests SET coverage_document='{}'",
            ),
            (
                "DROP TRIGGER dataset_manifests_no_update",
                "UPDATE dataset_manifests SET content_sha256=printf('%064d',0)",
            ),
        ] {
            let (mut store, input) = store();
            store.conn.execute_batch(prepare).unwrap();
            pair(&store, &input);
            let pointers = crate::content::pointers(&store.conn, &input.key().scope).unwrap();
            store.conn.execute_batch(mutation).unwrap();
            assert!(store.catalog_revision(&input.key()).is_err());
            assert!(
                store
                    .activate_content_with_travel(&input.key(), true)
                    .is_err()
            );
            assert_eq!(
                crate::content::pointers(&store.conn, &input.key().scope).unwrap(),
                pointers
            );
            assert!(!store.content_accepts_inferred(&input.key()).unwrap());
            assert!(store.catalog_generation.read.get().is_none());
        }
    }
    #[test]
    fn travel_activation_failure_rolls_back_pointer_and_choice_and_checks_disposition() {
        let (mut store, first) = store();
        let second = input("choice-next", "second");
        store.stage_content_v2_revision(&second).unwrap();
        store.catalog_revision(&second.key()).unwrap();
        store.conn.execute_batch("CREATE TRIGGER reject_choice BEFORE INSERT ON content_travel_choices BEGIN SELECT RAISE(ABORT,'injected preference failure'); END").unwrap();
        let pointers = crate::content::pointers(&store.conn, &first.key().scope).unwrap();
        assert!(
            store
                .activate_content_with_travel(&second.key(), true)
                .is_err()
        );
        assert_eq!(
            crate::content::pointers(&store.conn, &first.key().scope).unwrap(),
            pointers
        );
        assert!(!store.content_accepts_inferred(&second.key()).unwrap());
        assert!(store.catalog_generation.read.get().is_none());
        store.conn.execute_batch("DROP TRIGGER reject_choice; UPDATE content_dispositions SET state='revoked',reason_code='withdrawn_contribution'").unwrap();
        assert!(
            store
                .activate_content_with_travel(&second.key(), true)
                .is_err()
        );
        assert_eq!(
            crate::content::pointers(&store.conn, &first.key().scope).unwrap(),
            pointers
        );
        assert!(!store.content_accepts_inferred(&second.key()).unwrap());
    }

    #[test]
    fn travel_activation_schema_temp_and_rolled_back_content_changes_never_reuse_stale_catalog() {
        let (mut store, input) = store();
        let before = pair(&store, &input);
        store
            .conn
            .execute_batch("CREATE TABLE cache_ddl_probe(id INTEGER)")
            .unwrap();
        let count = loads();
        store
            .activate_content_with_travel(&input.key(), false)
            .unwrap();
        assert_eq!(loads(), count + 1);
        assert!(!Arc::ptr_eq(&before.0, &pair(&store, &input).0));
        store
            .conn
            .execute_batch("CREATE TEMP TABLE source_records AS SELECT * FROM main.source_records")
            .unwrap();
        let count = loads();
        for _ in 0..2 {
            store
                .activate_content_with_travel(&input.key(), false)
                .unwrap();
        }
        assert_eq!(loads(), count + 2, "temp shadows must bypass cache");
        store
            .conn
            .execute_batch("UPDATE temp.source_records SET document='{}'")
            .unwrap();
        assert!(
            store
                .activate_content_with_travel(&input.key(), true)
                .is_err()
        );
        store
            .conn
            .execute_batch(
                "DROP TABLE temp.source_records; DROP TRIGGER dataset_manifests_no_update",
            )
            .unwrap();
        let before = pair(&store, &input);
        store.conn.execute_batch("BEGIN IMMEDIATE; SAVEPOINT altered; UPDATE dataset_manifests SET coverage_document='{}'").unwrap();
        assert!(store.catalog_revision(&input.key()).is_err());
        store
            .conn
            .execute_batch("ROLLBACK TO altered; RELEASE altered; COMMIT")
            .unwrap();
        let count = loads();
        store
            .activate_content_with_travel(&input.key(), false)
            .unwrap();
        assert_eq!(loads(), count + 1);
        assert!(!Arc::ptr_eq(&before.0, &pair(&store, &input).0));
        assert!(!store.content_accepts_inferred(&input.key()).unwrap());
    }

    #[test]
    fn ordinary_snapshot_reminder_preferences_and_settings_writes_reuse_both_caches() {
        let (mut store, input) = store();
        let snapshot = decode_envelope(include_str!(
            "../../../fixtures/contracts/v1/character-snapshot.json"
        ))
        .unwrap();
        let profile = ProfileKey::of(&snapshot);
        store
            .import_character_snapshot(&snapshot, ImportSource::Fixture, None)
            .unwrap();
        let before = pair(&store, &input);
        assert!(same(&before, &pair(&store, &input)));
        let mut next: Value = serde_json::to_value(snapshot).unwrap();
        next["payload"]["id"] = json!("cache-next-snapshot");
        next["payload"]["capture"]["sequence"] = json!(13);
        next["producer"]["sequence"] = json!(13);
        store
            .import_character_snapshot(
                &decode_envelope(&next.to_string()).unwrap(),
                ImportSource::Fixture,
                None,
            )
            .unwrap();
        assert!(same(&before, &pair(&store, &input)));
        store
            .save_reminder_state(
                &profile,
                &StoredReminderState {
                    enabled: false,
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(same(&before, &pair(&store, &input)));
        store
            .set_preference(
                &profile,
                &PreferenceId::new("cache-false").unwrap(),
                &Observation::known(false, ObservationMetadata::default()).unwrap(),
            )
            .unwrap();
        assert!(same(&before, &pair(&store, &input)));
        let settings = store.settings().unwrap();
        store
            .replace_settings(settings.revision, &settings.settings)
            .unwrap();
        assert!(same(&before, &pair(&store, &input)));
        // Failed immutable duplicate/staging validation does not fabricate a change.
        store.stage_content_v2_revision(&input).unwrap();
        assert!(same(&before, &pair(&store, &input)));
        assert!(
            store
                .stage_content_v2_revision(&self::input("cache-base", "altered"))
                .is_err()
        );
        assert!(same(&before, &pair(&store, &input)));
    }

    #[test]
    fn staging_activation_and_rollback_keep_fresh_pointer_and_disposition_guards() {
        let (mut store, first) = store();
        let before = pair(&store, &first);
        let second = input("cache-next", "second committed");
        store.stage_content_v2_revision(&second).unwrap();
        let after_stage = pair(&store, &first);
        assert!(!same(&before, &after_stage));
        assert_eq!(after_stage.0.key(), first.key());
        store.activate_content_revision(&second.key()).unwrap();
        assert_eq!(pair(&store, &second).0.key(), second.key());
        store.rollback_content_revision(&first.key().scope).unwrap();
        assert_eq!(pair(&store, &first).0.key(), first.key());
    }

    #[test]
    fn same_connection_table_tampering_invalidates_without_a_schema_change_at_write_time() {
        for (prepare, mutation) in [
            (
                "DROP TRIGGER dataset_revisions_no_update",
                "UPDATE dataset_revisions SET document='{}'",
            ),
            (
                "DROP TRIGGER dataset_manifests_no_update",
                "UPDATE dataset_manifests SET coverage_document='{}'",
            ),
            (
                "DROP TRIGGER source_records_no_update",
                "UPDATE source_records SET document='{}'",
            ),
            (
                "DROP TRIGGER source_conflicts_sealed",
                "INSERT INTO source_conflicts VALUES(1,0,'{}')",
            ),
            (
                "DROP TRIGGER source_records_no_delete",
                "DELETE FROM source_records",
            ),
            (
                "DROP TRIGGER source_records_no_delete; DROP TRIGGER source_records_sealed",
                "INSERT OR REPLACE INTO source_records SELECT revision_id,source_id,'{}' FROM source_records",
            ),
        ] {
            let (store, input) = store();
            store.conn.execute_batch(prepare).unwrap();
            let before = pair(&store, &input);
            store.conn.execute_batch(mutation).unwrap();
            assert!(
                store.active_catalog_revision(&input.key().scope).is_err(),
                "{mutation}"
            );
            assert!(
                store
                    .catalog_first_page(&input.key().scope, Collection::Items, 1)
                    .is_err()
            );
            // No failed reload replaces the last valid immutable object.
            assert!(Arc::ptr_eq(
                &before.1,
                &store.catalog_index_cache.borrow().as_ref().unwrap().2
            ));
        }
    }

    #[test]
    fn trigger_writes_to_catalog_tables_invalidate_even_from_an_unrelated_top_level_table() {
        let (store, input) = store();
        store.conn.execute_batch("DROP TRIGGER source_records_no_update; CREATE TRIGGER cache_trigger AFTER INSERT ON app_settings BEGIN UPDATE source_records SET document='{}'; END;").unwrap();
        pair(&store, &input);
        let settings = store.settings().unwrap();
        store
            .replace_settings(settings.revision, &settings.settings)
            .unwrap();
        assert!(store.active_catalog_revision(&input.key().scope).is_err());
    }

    #[test]
    fn main_and_temp_schema_changes_invalidate_both_caches() {
        let (store, input) = store();
        let before = pair(&store, &input);
        store
            .conn
            .execute_batch("ALTER TABLE dataset_revisions ADD COLUMN harmless TEXT")
            .unwrap();
        let after_alter = pair(&store, &input);
        assert!(!same(&before, &after_alter));
        store
            .conn
            .execute_batch("CREATE TEMP TABLE cache_schema_test(id INTEGER)")
            .unwrap();
        let after_temp = pair(&store, &input);
        assert!(!same(&after_alter, &after_temp));
        store
            .conn
            .execute_batch("DROP TABLE temp.cache_schema_test")
            .unwrap();
        assert!(!same(&after_temp, &pair(&store, &input)));
    }

    #[test]
    fn temporary_catalog_shadows_bypass_cache_and_attached_names_cannot_shadow_main() {
        let (store, input) = store();
        let before = pair(&store, &input);
        store.conn.execute_batch("ATTACH DATABASE ':memory:' AS other; CREATE TABLE other.source_records(revision_id INTEGER, source_id TEXT, document TEXT); INSERT INTO other.source_records VALUES(1,'unrelated','{}');").unwrap();
        assert!(same(&before, &pair(&store, &input)));
        // Even a WITHOUT ROWID temporary shadow cannot make update_hook trust
        // its subsequent data writes. Reads use full verification every time.
        store.conn.execute_batch("CREATE TEMP TABLE source_records(revision_id INTEGER, source_id TEXT, document TEXT, PRIMARY KEY(revision_id,source_id)) WITHOUT ROWID; INSERT INTO temp.source_records SELECT * FROM main.source_records;").unwrap();
        let first = store
            .active_content_v2_revision(&input.key().scope)
            .unwrap()
            .unwrap();
        let second = store
            .active_content_v2_revision(&input.key().scope)
            .unwrap()
            .unwrap();
        assert!(!Arc::ptr_eq(&first, &second));
        store
            .conn
            .execute("UPDATE temp.source_records SET document='{}'", [])
            .unwrap();
        assert!(store.active_catalog_revision(&input.key().scope).is_err());
        store
            .conn
            .execute_batch("DROP TABLE temp.source_records; DETACH DATABASE other")
            .unwrap();
        assert_eq!(pair(&store, &input).0.key(), input.key());
    }

    #[test]
    fn case_variant_main_names_never_hide_writes() {
        let (store, input) = store();
        // Renaming through a distinct name is accepted by SQLite. Its stored
        // name becomes uppercase, while all fixed SQL still resolves to it.
        store.conn.execute_batch("DROP TRIGGER dataset_manifests_no_update; ALTER TABLE dataset_manifests RENAME TO renamed_manifest; ALTER TABLE renamed_manifest RENAME TO DATASET_MANIFESTS").unwrap();
        pair(&store, &input);
        store
            .conn
            .execute("UPDATE DATASET_MANIFESTS SET coverage_document='{}'", [])
            .unwrap();
        assert!(
            store.active_catalog_revision(&input.key().scope).is_err(),
            "uppercase main mutation must invalidate"
        );
    }

    #[test]
    fn case_variant_temporary_shadows_never_hide_writes() {
        let (store, input) = self::store();
        store.conn.execute_batch("CREATE TEMP TABLE SOURCE_RECORDS(revision_id INTEGER, source_id TEXT, document TEXT, PRIMARY KEY(revision_id,source_id)) WITHOUT ROWID; INSERT INTO temp.SOURCE_RECORDS SELECT * FROM main.source_records").unwrap();
        let first = store
            .active_content_v2_revision(&input.key().scope)
            .unwrap()
            .unwrap();
        let second = store
            .active_content_v2_revision(&input.key().scope)
            .unwrap()
            .unwrap();
        assert!(
            !Arc::ptr_eq(&first, &second),
            "uppercase temp shadow must bypass cache"
        );
        store
            .conn
            .execute("UPDATE temp.SOURCE_RECORDS SET document='{}'", [])
            .unwrap();
        assert!(store.active_catalog_revision(&input.key().scope).is_err());
    }

    #[test]
    fn arbitrary_write_and_savepoint_rollback_never_publish_transient_reused_ids() {
        let (store, input) = store();
        let original = pair(&store, &input);
        let a = canonical("reused", "transient A");
        let b = canonical("reused", "transient B");
        store
            .conn
            .execute_batch("BEGIN IMMEDIATE; SAVEPOINT branch")
            .unwrap();
        stage_in_transaction(&store, &a);
        let id_a = crate::content::find(&store.conn, &a.key()).unwrap();
        let active_a = store
            .active_content_v2_revision(&a.key().scope)
            .unwrap()
            .unwrap();
        assert_eq!(active_a.key(), a.key());
        store
            .catalog_first_page(&a.key().scope, Collection::Items, 1)
            .unwrap();
        assert!(same(
            &original,
            &(
                match store.catalog_cache.borrow().as_ref().unwrap().2.clone() {
                    crate::content::ActiveContentRevision::V2(value) => value,
                    _ => panic!("expected v2"),
                },
                store
                    .catalog_index_cache
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .2
                    .clone()
            )
        ));
        store
            .conn
            .execute_batch("ROLLBACK TO branch; RELEASE branch")
            .unwrap();
        stage_in_transaction(&store, &b);
        assert_eq!(id_a, crate::content::find(&store.conn, &b.key()).unwrap());
        let active_b = store
            .active_content_v2_revision(&b.key().scope)
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::to_value(&*active_b).unwrap(),
            serde_json::to_value(&b).unwrap()
        );
        assert!(!Arc::ptr_eq(&active_a, &active_b));
        store.conn.execute_batch("ROLLBACK").unwrap();
        assert_eq!(pair(&store, &input).0.key(), input.key());
    }

    #[test]
    fn managed_read_cannot_cache_content_writes_or_ddl_and_restores_scope_after_error_or_panic() {
        let (store, input) = store();
        let before = pair(&store, &input);
        let transient = canonical("managed-transient", "rolled back");
        let result: Result<()> = store.with_immediate_read(|store| {
            stage_in_transaction(store, &transient);
            let active = store
                .active_content_v2_revision(&input.key().scope)?
                .unwrap();
            assert_eq!(active.key(), transient.key());
            assert!(store.catalog_generation.cache_stamp(&store.conn)?.is_none());
            Err(crate::error::invalid("synthetic rollback"))
        });
        assert!(result.is_err());
        assert_eq!(pair(&store, &input).0.key(), input.key());
        let stable = pair(&store, &input);
        let result: Result<()> = store.with_immediate_read(|store| {
            store
                .conn
                .execute_batch("CREATE TEMP TABLE rolled_back_schema(id INTEGER)")
                .unwrap();
            assert!(store.catalog_generation.cache_stamp(&store.conn)?.is_none());
            let temporary = store
                .active_content_v2_revision(&input.key().scope)?
                .unwrap();
            assert!(!Arc::ptr_eq(&stable.0, &temporary));
            Err(crate::error::invalid("synthetic DDL rollback"))
        });
        assert!(result.is_err());
        assert!(same(&stable, &pair(&store, &input)));
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _: Result<()> = store.with_immediate_read(|_| panic!("synthetic unwind"));
            }))
            .is_err()
        );
        assert!(store.catalog_generation.read.get().is_none());
        assert!(
            store
                .catalog_generation
                .cache_stamp(&store.conn)
                .unwrap()
                .is_none()
        );
        assert!(same(&stable, &pair(&store, &input)));
        assert!(!Arc::ptr_eq(&before.0, &stable.0));
    }

    #[test]
    fn external_commits_reload_and_external_tampering_never_reuses_cached_bytes() {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "navzeroth-cache-generation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(directory.clone());
        let path = directory.join("store.sqlite");
        let mut store = SqliteStore::open(&path).unwrap();
        let input = input("external-base", "external fixture");
        store.stage_content_v2_revision(&input).unwrap();
        store.activate_content_revision(&input.key()).unwrap();
        let writer = SqliteStore::open(&path).unwrap();
        let before = pair(&store, &input);
        writer.settings().unwrap();
        assert!(same(&before, &pair(&store, &input)));
        let settings = writer.settings().unwrap();
        writer
            .replace_settings(settings.revision, &settings.settings)
            .unwrap();
        let after = pair(&store, &input);
        assert!(!same(&before, &after));
        writer.conn.execute_batch("DROP TRIGGER dataset_revisions_no_update; DROP TRIGGER dataset_manifests_no_update; DROP TRIGGER source_records_no_update").unwrap();
        for (table, column) in [
            ("dataset_revisions", "document"),
            ("source_records", "document"),
            ("dataset_manifests", "coverage_document"),
            ("dataset_manifests", "content_sha256"),
        ] {
            let original: String = writer
                .conn
                .query_row(&format!("SELECT {column} FROM {table} LIMIT 1"), [], |r| {
                    r.get(0)
                })
                .unwrap();
            pair(&store, &input);
            let generation = store.catalog_generation.local.load(Ordering::Relaxed);
            writer
                .conn
                .execute(
                    &format!("UPDATE {table} SET {column}=?1"),
                    [if column == "content_sha256" {
                        "0".repeat(64)
                    } else {
                        "{}".into()
                    }],
                )
                .unwrap();
            assert_eq!(
                generation,
                store.catalog_generation.local.load(Ordering::Relaxed)
            );
            assert!(store.active_catalog_revision(&input.key().scope).is_err());
            assert!(store.catalog_revision(&input.key()).is_err());
            assert!(
                store
                    .activate_content_with_travel(&input.key(), true)
                    .is_err()
            );
            assert!(
                store
                    .catalog_first_page(&input.key().scope, Collection::Items, 1)
                    .is_err()
            );
            assert!(!store.content_accepts_inferred(&input.key()).unwrap());
            writer
                .conn
                .execute(&format!("UPDATE {table} SET {column}=?1"), [&original])
                .unwrap();
            store
                .activate_content_with_travel(&input.key(), false)
                .unwrap();
        }
    }

    #[test]
    fn terminal_generation_overflow_disables_caching_without_wrapping() {
        let (mut store, input) = store();
        store
            .catalog_generation
            .local
            .store(u64::MAX - 1, Ordering::Relaxed);
        let before = pair(&store, &input);
        store
            .stage_content_v2_revision(&self::input("overflow-next", "overflow"))
            .unwrap();
        assert_eq!(
            store.catalog_generation.local.load(Ordering::Relaxed),
            u64::MAX
        );
        let first = store
            .active_content_v2_revision(&input.key().scope)
            .unwrap()
            .unwrap();
        let second = store
            .active_content_v2_revision(&input.key().scope)
            .unwrap()
            .unwrap();
        assert!(!Arc::ptr_eq(&first, &second));
        assert!(!Arc::ptr_eq(&before.0, &first));
        assert_eq!(
            store.catalog_generation.local.load(Ordering::Relaxed),
            u64::MAX
        );
    }
}
