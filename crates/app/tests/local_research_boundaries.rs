//! Synthetic empty catalogs exercise application boundaries, never client truth.
use azeroth_app::local_data::LocalDataService;
use azeroth_core::{
    local_data::{CASCLIB_REVISION, InstallationMetadata, PerformancePreset},
    local_data_catalog::{DistributionClass, ResearchCatalog},
    local_data_inventory::Mode,
    local_data_workflow::{DeliveryChoices, MaterialKind},
};
use azeroth_local_data::{guide_review::PageRequest, metadata::METADATA_SHA256};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};

struct Fixture {
    root: PathBuf,
    service: Option<LocalDataService>,
    catalog: String,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "nav-research-boundary-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        }
        #[cfg(not(unix))]
        fs::create_dir(&root).unwrap();
        for name in ["catalogs", "objects"] {
            fs::create_dir(root.join(name)).unwrap();
        }
        let catalog = ResearchCatalog {
            format_version: 1,
            classification: DistributionClass::LocalOnly,
            installation: InstallationMetadata {
                product: "wow_classic_beta".into(),
                version: "1.60.1.70425".into(),
                build: 70425,
                build_key: "a".repeat(32),
                cdn_key: "b".repeat(32),
                declared_locales: vec!["enUS".into()],
            },
            build_info_sha256: "c".repeat(64),
            extractor_version: "synthetic".into(),
            native_revision: CASCLIB_REVISION.into(),
            metadata_sha256: METADATA_SHA256.into(),
            parser_profile: "synthetic".into(),
            extractor_profile_sha256: None,
            archive_statistics: None,
            input_hashes: BTreeMap::new(),
            tables: vec![],
            gaps: vec!["synthetic_empty_catalog_not_gameplay_evidence".into()],
            gameplay_projection_sha256: None,
        };
        let bytes = serde_json::to_vec(&catalog).unwrap();
        let digest = format!("{:x}", Sha256::digest(&bytes));
        fs::write(root.join("catalogs").join(&digest), bytes).unwrap();
        Self {
            service: Some(LocalDataService::new(root.clone())),
            root,
            catalog: digest,
        }
    }
    fn app(&self) -> &LocalDataService {
        self.service.as_ref().unwrap()
    }
    fn delivery(&self) -> String {
        self.app()
            .prepare_delivery_bound(
                &self.catalog,
                "enUS",
                DeliveryChoices {
                    format_version: 1,
                    media: vec![],
                    inputs: vec![],
                    guide_facts: true,
                    preset: PerformancePreset::Small,
                },
            )
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.service.take());
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn partial_delivery_never_converts_missing_counts_into_completeness_or_advice() {
    let f = Fixture::new();
    let app = f.app();
    let delivery = f.delivery();
    let manifest = app.delivery_manifest(&delivery).unwrap();
    assert!(manifest.fixture);
    for domain in ["equipment", "geography", "travel"] {
        let page = app.delivery_page(&delivery, domain, 0, 200).unwrap();
        assert!(page.rows.is_empty());
        assert_eq!(page.total, None);
        assert!(!page.guard.actionable_advice && !page.guard.acceptance_ready);
        for limit in [0, 201] {
            assert!(app.delivery_page(&delivery, domain, 0, limit).is_err());
        }
    }
    let inventory = app
        .inventory_selected(&delivery, None, PerformancePreset::Small)
        .unwrap();
    let summary = app.inventory_summary_selected(&inventory, None).unwrap();
    assert_eq!(summary.domains.len(), 9);
    assert!(!summary.acceptance_ready);
    for domain in summary.domains {
        assert_eq!(domain.independently_required_count, None);
        assert!(!domain.independently_complete);
        let page = app
            .inventory_page_selected(&inventory, &domain.domain, 0, 200, None)
            .unwrap();
        assert!(!page.independently_complete);
        assert_eq!(page.total, domain.imported_count);
    }
    let discovery = app.media_discover(&f.catalog, "enUS", 0, 200).unwrap();
    assert!(discovery.candidates.is_empty());
    assert!(
        !app.tooltip(&manifest.projection_sha256, 1)
            .unwrap()
            .verified_for_gameplay
    );
    assert_eq!(app.artifact_page("deliveries", 0, 200).unwrap().total, 1);
    assert!(app.artifact_page("../objects", 0, 200).is_err());
}

#[test]
fn explicit_research_activation_survives_failed_replacement_and_locale_mismatch() {
    let f = Fixture::new();
    let app = f.app();
    let delivery = f.delivery();
    let initial = app.delivery_active(&delivery).unwrap();
    assert_eq!(initial.generation, 0);
    let active = app.activate_delivery(&delivery, 0, false).unwrap();
    assert_eq!(active.delivery_sha256.as_deref(), Some(delivery.as_str()));
    assert!(!active.verified_for_planning);
    assert!(app.activate_delivery("../foreign", 1, false).is_err());
    assert!(app.prepare_delivery(&f.catalog, "deDE").is_err());
    assert!(app.delivery_diff(&delivery, "0").is_err());
    assert_eq!(
        app.delivery_active(&delivery).unwrap().generation,
        active.generation
    );
    let same = app.delivery_diff(&delivery, &delivery).unwrap();
    assert!(same.changed.is_empty());
    assert!(!same.actionable_advice);
    assert!(app.media_asset("../foreign", "unselected").is_err());
    assert!(
        app.research_asset("POST", "/research/invalid", false)
            .is_none()
    );
    assert!(app.media_manifest("unselected").is_err());
}

#[test]
fn selected_guide_is_inert_bounded_and_cannot_establish_duo_eligibility() {
    let f = Fixture::new();
    let app = f.app();
    let delivery = f.delivery();
    let guide = f.root.join("synthetic-guide.lua");
    fs::write(&guide,"RXPGuides.RegisterGuide([[\n#name Synthetic\n#forever\n#defaultfor Human Warrior\nstep\n.accept 1\n.turnin 1\n]])").unwrap();
    let selected = app.select_material(&guide, MaterialKind::Guide).unwrap();
    assert!(
        !serde_json::to_string(&app.material_selections().unwrap())
            .unwrap()
            .contains(&f.root.to_string_lossy().to_string())
    );
    let review = app
        .review_selected_human_guide_page(
            &delivery,
            &selected.id,
            "Warrior",
            Mode::Duo,
            PageRequest {
                expected: None,
                offset: 0,
                limit: 1,
            },
        )
        .unwrap();
    assert!(!review.acceptance_ready && !review.duo_eligibility_verified);
    assert!(
        app.review_selected_human_guide_page(
            &delivery,
            &selected.id,
            "FutureClass",
            Mode::Duo,
            PageRequest {
                expected: None,
                offset: 0,
                limit: 1
            }
        )
        .is_err()
    );
    assert!(
        app.review_selected_human_guide_page(
            &delivery,
            &selected.id,
            "Warrior",
            Mode::Duo,
            PageRequest {
                expected: None,
                offset: 1,
                limit: 200
            }
        )
        .is_err()
    );
    assert!(
        app.stage_selected_input(&f.catalog, &selected.id, "enUS", PerformancePreset::Small)
            .is_err()
    );
    assert!(
        app.inventory_selected(&delivery, Some(&selected.id), PerformancePreset::Small)
            .is_err()
    );
    app.forget_material(&selected.id).unwrap();
    assert!(app.material_selections().unwrap().is_empty());
    assert!(guide.exists());
    assert!(
        app.review_selected_human_guide_page(
            &delivery,
            &selected.id,
            "Warrior",
            Mode::Solo,
            PageRequest {
                expected: None,
                offset: 0,
                limit: 200
            }
        )
        .is_err()
    );
}

#[test]
fn direct_research_services_keep_empty_catalog_and_bad_identity_boundaries() {
    use azeroth_core::local_data_inputs::InputKind;
    let f = Fixture::new();
    let app = f.app();
    let delivery = app.prepare_delivery(&f.catalog, "enUS").unwrap();
    let manifest = app.delivery_manifest(&delivery).unwrap();
    let facts = app
        .build_guide_facts(&manifest.projection_sha256, &[], PerformancePreset::Small)
        .unwrap();
    let verified = app.verify_guide_facts(&facts.guide_facts_sha256).unwrap();
    assert!(!verified.verified_for_planning);
    let page = app.inventory(&f.catalog, 0, 200).unwrap();
    assert_eq!(page.total, 0);
    assert!(page.rows.is_empty());
    assert!(app.inventory(&f.catalog, 1, 200).is_err());
    assert!(app.inventory(&f.catalog, 0, 0).is_err());
    assert!(app.media_page("invalid", 0, 200).is_err());
    assert!(app.media_display_page("invalid", 0, 200).is_err());
    assert!(app.verify_input("invalid").is_err());
    assert!(app.input_page("invalid", 0, 200).is_err());
    assert!(
        app.stage_input(
            &f.catalog,
            &f.root.join("absent"),
            InputKind::CharacterSnapshot,
            "enUS",
            PerformancePreset::Small
        )
        .is_err()
    );
    assert!(
        app.stage_input(
            &f.catalog,
            &f.root.join("absent"),
            InputKind::HotfixCache,
            "enUS",
            PerformancePreset::Small
        )
        .is_err()
    );
    assert!(app.jobs().unwrap().is_empty());
    assert!(app.status("missing").is_err());
    assert!(app.cancel("missing").is_err());
    assert!(app.resume("missing").is_err());
    assert!(app.forget("missing").is_err());
    let initial = app.active(&f.catalog).unwrap();
    assert_eq!(initial.generation, 0);
    assert!(app.activate(&f.catalog, 1, false).is_err());
    assert_eq!(app.active(&f.catalog).unwrap().generation, 0);
    let path = format!("/research/{}/{}", "a".repeat(64), "b".repeat(64));
    assert!(app.research_asset("GET", &path, false).is_none());
    assert!(app.research_asset("GET", &path, true).is_none());
    assert!(
        app.research_asset("GET", &path.replace("/research/", "/foreignx/"), false)
            .is_none()
    );
}
