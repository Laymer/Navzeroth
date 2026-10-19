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
fn committed_registry_admits_only_reviewed_scopes_and_excludes_unknown_candidates() {
    Registry::parse(&serde_json::to_vec(&registry()).unwrap()).unwrap();
    let r = Registry::parse(include_bytes!(
        "../../../docs/sources/source-admission-v1.json"
    ))
    .unwrap();
    let accepted: Vec<_> = r
        .sources
        .iter()
        .filter(|s| s.rights.status == Status::Admitted)
        .map(|s| s.id.as_str())
        .collect();
    assert_eq!(
        accepted,
        [
            "atlasloot-classic-areas",
            "mapzeroth-retail-ek",
            "wowsims-classic-paladin-model",
            "allthethings-forever-riverglades",
            "cmangos-classic-db",
            "talod-generators",
            "wago-tools-classic-era",
            "wago-tools-wowf",
            "wowhead-classic",
            "wowhead-forever"
        ]
    );
    assert!(r.sources.iter().all(|s| s.origin.build.is_none()));
    // ADR 0006 §14 sources: explicit owner acceptance (or GPL share-alike for
    // CMaNGOS), generator-only retrieval, generated files registered later.
    for s in r.sources.iter().filter(|s| s.files.is_empty()) {
        if s.rights.status != Status::Admitted {
            continue;
        }
        assert!(
            s.rights
                .evidence
                .iter()
                .any(|e| e.reviewed_scope.contains(OWNER_ACCEPTANCE_CITATION)),
            "{}",
            s.id
        );
        let expected = if s.id == "talod-generators" {
            Method::ManualLocal
        } else {
            Method::HttpsRateLimited
        };
        assert_eq!(s.retrieval.method, expected, "{}", s.id);
        let license = if s.id == "cmangos-classic-db" {
            License::ShareAlike
        } else {
            License::OwnerAccepted
        };
        assert_eq!(s.rights.license, license, "{}", s.id);
    }
    let talod = r
        .sources
        .iter()
        .find(|s| s.id == "talod-generators")
        .unwrap();
    assert_eq!(
        talod.rights.evidence[0].sha256,
        "08abb624e55bd7383eefca9d14a02f5309c44c0012d865a78d01fc45736a25f3"
    );
    assert!(matches!(
        r.authorize(
            "questiedb-forever-candidate",
            "data",
            &["id"],
            Method::ManualLocal
        ),
        Err(Rejection::RightsNotAdmitted)
    ));
    assert!(matches!(
        r.authorize("questiedb", "data", &["id"], Method::HttpsPinned),
        Err(Rejection::UnknownSource)
    ));
}

#[test]
fn rejects_ambiguous_urls_and_windows_path_aliases() {
    for value in [
        "https:///file",
        "https://127.0.0.1/file",
        "https://127.1/file",
        "https://0x7f.0x1/file",
        "https://:443/file",
        "https://host:443/file",
        "https://host./file",
        "https://a..invalid/file",
        "https://-a.invalid/file",
        "https://a.invalid%2fother/file",
    ] {
        let mut r = registry();
        r.sources[0].files[0].url = value.into();
        assert!(r.validate().is_err(), "{value}");
    }
    for value in [
        "data/con.lua",
        "data/CON.lua",
        "data/nul",
        "data/file.",
        "data/file ",
        "data /file",
        "data/com1.txt",
    ] {
        let mut r = registry();
        r.sources[0].files[0].path = value.into();
        assert!(r.validate().is_err(), "{value}");
    }
    let mut r = registry();
    let mut duplicate = r.sources[0].files[0].clone();
    duplicate.path = "DATA/Quests.lua".into();
    r.sources[0].files.push(duplicate);
    r.sources[0].retrieval.max_requests = 2;
    assert!(r.validate().is_err());
}

#[test]
fn canonical_encoded_spaces_only_in_https_path_preserve_authority_guards() {
    let mut r = registry();
    r.sources[0].files[0].url =
        "https://example.invalid/pinned/eastern%20kingdoms/source.lua".into();
    r.validate().unwrap();
    for value in [
        "https://example.invalid%20.evil/file",
        "https://example.invalid/a%2fb",
        "https://example.invalid/%2e%2e/file",
        "https://example.invalid/a%5cb",
        "https://example.invalid/a%2520b",
        "https://example.invalid/a%3fb",
        "https://example.invalid/a b",
    ] {
        r.sources[0].files[0].url = value.into();
        assert!(r.validate().is_err(), "{value}");
    }
}

fn owner_accepted() -> Registry {
    let mut r = registry();
    let s = &mut r.sources[0];
    s.rights.license = License::OwnerAccepted;
    s.rights.evidence[0].reviewed_scope =
        "Owner accepts the publisher's terms for generator retrieval; ADR 0006 §14".into();
    s.retrieval.method = Method::HttpsRateLimited;
    r
}
#[test]
fn owner_accepted_rights_require_every_field_and_the_adr_citation() {
    let r = owner_accepted();
    r.validate().unwrap();
    r.authorize(
        "synthetic",
        "data/quests.lua",
        &["id"],
        Method::HttpsRateLimited,
    )
    .unwrap();
    assert!(matches!(
        r.authorize("synthetic", "data/quests.lua", &["id"], Method::HttpsPinned),
        Err(Rejection::MethodNotPermitted)
    ));
    // Generated material is registered later: no files is admissible, yet nothing
    // can be authorized from it.
    let mut empty = owner_accepted();
    empty.sources[0].files.clear();
    empty.validate().unwrap();
    assert!(matches!(
        empty.authorize(
            "synthetic",
            "data/quests.lua",
            &["id"],
            Method::HttpsRateLimited
        ),
        Err(Rejection::FileNotPermitted)
    ));
    let missing: [fn(&mut Rights); 6] = [
        |r| r.license_identifier = None,
        |r| r.reviewer = None,
        |r| r.attribution = None,
        |r| r.obligations = None,
        |r| r.evidence.clear(),
        |r| r.evidence[0].reviewed_scope = "Owner accepts the publisher's terms".into(),
    ];
    for strip in missing {
        let mut r = owner_accepted();
        strip(&mut r.sources[0].rights);
        assert_eq!(r.validate(), Err(Rejection::RightsNotAdmitted));
    }
    let mut r = owner_accepted();
    r.sources[0].rights.license_identifier = Some("  ".into());
    assert_eq!(r.validate(), Err(Rejection::InvalidRegistry));
    let mut r = owner_accepted();
    r.sources[0].retrieval.access_evidence = None;
    assert_eq!(r.validate(), Err(Rejection::RightsNotAdmitted));
    // The citation in a second evidence entry suffices.
    let mut r = owner_accepted();
    let mut extra = r.sources[0].rights.evidence[0].clone();
    r.sources[0].rights.evidence[0].reviewed_scope = "Publisher terms page".into();
    extra.reviewed_scope = "Recorded by ADR 0006 §14".into();
    r.sources[0].rights.evidence.push(extra);
    r.validate().unwrap();
    // Unreviewed owner-accepted entries are inert, not errors.
    let mut r = owner_accepted();
    r.sources[0].rights.status = Status::Unreviewed;
    r.sources[0].rights.evidence.clear();
    r.validate().unwrap();
    assert!(matches!(
        r.authorize(
            "synthetic",
            "data/quests.lua",
            &["id"],
            Method::HttpsRateLimited
        ),
        Err(Rejection::RightsNotAdmitted)
    ));
}
#[test]
fn permissive_and_unknown_rights_keep_their_rules_beside_owner_acceptance() {
    let mut r = registry();
    r.sources[0].files.clear();
    assert_eq!(r.validate(), Err(Rejection::RightsNotAdmitted));
    for license in [License::Unknown, License::Noncommercial] {
        let mut r = owner_accepted();
        r.sources[0].rights.license = license;
        assert_eq!(r.validate(), Err(Rejection::RightsNotAdmitted));
    }
    // A share-alike source retrieved by the generators registers files later.
    let mut r = registry();
    r.sources[0].retrieval.method = Method::HttpsRateLimited;
    r.validate().unwrap();
    r.sources[0].files.clear();
    r.validate().unwrap();
    r.sources[0].rights.reviewer = None;
    assert_eq!(r.validate(), Err(Rejection::RightsNotAdmitted));
    let bounds: [fn(&mut Retrieval); 4] = [
        |t| t.minimum_interval_ms = 999,
        |t| t.max_requests = 0,
        |t| t.max_requests = 1025,
        |t| t.max_total_bytes = MAX_RETRIEVAL_BYTES + 1,
    ];
    for bad in bounds {
        let mut r = owner_accepted();
        bad(&mut r.sources[0].retrieval);
        assert_eq!(r.validate(), Err(Rejection::InvalidRegistry));
    }
    assert_eq!(
        serde_json::to_value(License::OwnerAccepted).unwrap(),
        "owner_accepted"
    );
    assert_eq!(
        serde_json::to_value(Method::HttpsRateLimited).unwrap(),
        "https_rate_limited"
    );
}
