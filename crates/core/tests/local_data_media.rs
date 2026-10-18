use azeroth_core::local_data_media::{Asset, MediaManifest, ReadRefusal};
#[test]
fn read_refusals_need_distinct_provenance_and_cannot_be_fixture_decodes() {
    for reason in [
        ReadRefusal::Unsupported,
        ReadRefusal::Invalid,
        ReadRefusal::Truncated,
        ReadRefusal::EncryptedFrames,
        ReadRefusal::ShortRead,
    ] {
        let mut manifest = MediaManifest {
            format_version: 1,
            catalog_sha256: "a".repeat(64),
            locale: "enUS".into(),
            discovery_sha256: "b".repeat(64),
            profile_sha256: "c".repeat(64),
            fixture: false,
            local_only: true,
            calibrated: false,
            verified_for_gameplay: false,
            assets: vec![Asset {
                file_data_id: 1,
                disposition: reason.disposition(),
                source_sha256: None,
                read_refusal: Some(reason),
                png_sha256: None,
                width: None,
                height: None,
            }],
        };
        assert!(manifest.validate().is_ok());
        manifest.fixture = true;
        assert!(manifest.validate().is_err());
        manifest.fixture = false;
        manifest.assets[0].read_refusal = None;
        assert!(manifest.validate().is_err());
    }
}
