use azeroth_core::source_admission::*;

fn registry() -> Registry {
    let proof = Evidence {
        url: "https://example.invalid/grant".into(),
        revision: "a".repeat(40),
        sha256: "b".repeat(64),
        reviewed_scope: "Synthetic fixture only; no actual rights grant".into(),
    };
    Registry {
        schema_version: 1,
        sources: vec![Source {
            id: "synthetic".into(),
            publisher: "test".into(),
            revision: "c".repeat(40),
            origin: Origin {
                edition: None,
                build: None,
            },
            rights: Rights {
                status: Status::Admitted,
                license: License::ShareAlike,
                license_identifier: Some("synthetic-license".into()),
                reviewer: Some("test".into()),
                attribution: Some("fixture".into()),
                obligations: Some("fixture retention and distribution".into()),
                evidence: vec![proof.clone()],
            },
            retrieval: Retrieval {
                method: Method::ManualLocal,
                max_total_bytes: 1024,
                minimum_interval_ms: 1000,
                max_requests: 1,
                access_evidence: Some(proof),
            },
            files: vec![PermittedFile {
                path: "data/quests.lua".into(),
                url: "https://example.invalid/pinned/data".into(),
                sha256: "d".repeat(64),
                bytes: 32,
                fields: vec!["id".into(), "level".into()],
                transformation: "inert-literal-v1".into(),
            }],
        }],
    }
}
#[test]
fn exact_admission_keeps_unknown_origin() {
    let r = registry();
    let a = r
        .authorize("synthetic", "data/quests.lua", &["id"], Method::ManualLocal)
        .unwrap();
    assert_eq!(a.source.origin.build, None);
    assert_eq!(a.file.bytes, 32);
    assert!(matches!(
        r.authorize(
            "synthetic",
            "data/quests.lua",
            &["rewards"],
            Method::ManualLocal
        ),
        Err(Rejection::FieldNotPermitted)
    ));
    assert!(matches!(
        r.authorize("synthetic", "other.lua", &["id"], Method::ManualLocal),
        Err(Rejection::FileNotPermitted)
    ));
    assert!(matches!(
        r.authorize("synthetic", "data/quests.lua", &["id"], Method::HttpsPinned),
        Err(Rejection::MethodNotPermitted)
    ));
}
#[test]
fn all_unreviewed_and_excluded_rights_fail_closed() {
    for status in [Status::Unreviewed, Status::Rejected] {
        let mut r = registry();
        r.sources[0].rights.status = status;
        assert!(matches!(
            r.authorize("synthetic", "data/quests.lua", &["id"], Method::ManualLocal),
            Err(Rejection::RightsNotAdmitted)
        ));
    }
    for license in [License::Unknown, License::Noncommercial] {
        let mut r = registry();
        r.sources[0].rights.license = license;
        assert_eq!(r.validate(), Err(Rejection::RightsNotAdmitted));
    }
    let mut r = registry();
    r.sources[0].rights.obligations = None;
    assert_eq!(r.validate(), Err(Rejection::RightsNotAdmitted));
    let mut r = registry();
    r.sources[0].retrieval.access_evidence = None;
    assert_eq!(r.validate(), Err(Rejection::RightsNotAdmitted));
}
#[test]
fn strict_version_size_unknown_fields_and_duplicate_fields() {
    let mut r = registry();
    r.schema_version = 2;
    assert_eq!(r.validate(), Err(Rejection::UnsupportedVersion));
    assert!(matches!(
        Registry::parse(&vec![b' '; MAX_REGISTRY_BYTES + 1]),
        Err(Rejection::Capacity)
    ));
    assert!(Registry::parse(br#"{"schema_version":1,"sources":[],"approved":true}"#).is_err());
    assert!(Registry::parse(br#"{"schema_version":1,"schema_version":1,"sources":[]}"#).is_err());
    let mut r = registry();
    r.sources.push(r.sources[0].clone());
    assert_eq!(r.validate(), Err(Rejection::InvalidRegistry));
}
#[test]
fn rejects_mutable_revision_traversal_wildcards_and_overflow() {
    for value in ["main", "v1", "", "A"] {
        let mut r = registry();
        r.sources[0].revision = value.into();
        assert!(r.validate().is_err());
    }
    for value in [
        "../data", "/data", "a//b", "a/./b", "a\\b", "a%2fb", "*.lua",
    ] {
        let mut r = registry();
        r.sources[0].files[0].path = value.into();
        assert!(r.validate().is_err());
    }
    let mut r = registry();
    r.sources[0].files[0].fields = vec!["*".into()];
    assert!(r.validate().is_err());
    let mut r = registry();
    r.sources[0].files[0].bytes = MAX_FILE_BYTES + 1;
    assert!(r.validate().is_err());
    let mut r = registry();
    r.sources[0].retrieval.max_total_bytes = 31;
    assert_eq!(r.validate(), Err(Rejection::Capacity));
}
#[test]
fn round_trip_and_committed_registry_admit_no_source() {
    Registry::parse(&serde_json::to_vec(&registry()).unwrap()).unwrap();
    let r = Registry::parse(include_bytes!(
        "../../../docs/sources/source-admission-v1.json"
    ))
    .unwrap();
    assert!(r.sources.is_empty());
    assert!(matches!(
        r.authorize("questiedb", "data", &["id"], Method::HttpsPinned),
        Err(Rejection::UnknownSource)
    ));
}
