use azeroth_core::local_data_media::{Asset, Disposition, MediaManifest, ReadRefusal};
#[test]
fn read_refusals_need_distinct_provenance_and_cannot_be_fixture_decodes() {
    for reason in [
        ReadRefusal::Missing,
        ReadRefusal::Encrypted,
        ReadRefusal::Capacity,
        ReadRefusal::Unavailable,
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

fn decoded_manifest() -> MediaManifest {
    MediaManifest {
        format_version: 1,
        catalog_sha256: "a".repeat(64),
        locale: "enUS".into(),
        discovery_sha256: "b".repeat(64),
        profile_sha256: "c".repeat(64),
        fixture: true,
        local_only: true,
        calibrated: false,
        verified_for_gameplay: false,
        assets: vec![Asset {
            file_data_id: 1,
            disposition: Disposition::Decoded,
            source_sha256: Some("d".repeat(64)),
            read_refusal: None,
            png_sha256: Some("e".repeat(64)),
            width: Some(2048),
            height: Some(2048),
        }],
    }
}
#[test]
fn decoded_dimensions_and_lineage_are_bounded_without_promoting_calibration() {
    let good = decoded_manifest();
    assert!(good.validate().is_ok());
    for (width, height) in [
        (None, Some(1)),
        (Some(1), None),
        (Some(0), Some(1)),
        (Some(1), Some(0)),
        (Some(2049), Some(2048)),
        (Some(u32::MAX), Some(u32::MAX)),
    ] {
        let mut changed = good.clone();
        changed.assets[0].width = width;
        changed.assets[0].height = height;
        assert!(changed.validate().is_err());
    }
    for changed in [
        {
            let mut m = good.clone();
            m.assets[0].source_sha256 = None;
            m
        },
        {
            let mut m = good.clone();
            m.assets[0].png_sha256 = None;
            m
        },
        {
            let mut m = good.clone();
            m.assets[0].png_sha256 = Some("invalid".into());
            m
        },
        {
            let mut m = good.clone();
            m.assets[0].read_refusal = Some(ReadRefusal::Missing);
            m
        },
        {
            let mut m = good.clone();
            m.assets[0].disposition = Disposition::Unsupported;
            m
        },
        {
            let mut m = good.clone();
            m.calibrated = true;
            m
        },
        {
            let mut m = good.clone();
            m.verified_for_gameplay = true;
            m
        },
        {
            let mut m = good.clone();
            m.local_only = false;
            m
        },
        {
            let mut m = good.clone();
            m.format_version = 2;
            m
        },
    ] {
        assert!(changed.validate().is_err());
    }
    assert!(!good.calibrated);
    assert!(!good.verified_for_gameplay);
}
#[test]
fn manifest_identity_order_and_asset_count_refuse_ambiguous_or_excessive_sets() {
    let mut manifest = decoded_manifest();
    manifest.fixture = false;
    manifest.assets = (1..=128)
        .map(|id| Asset {
            file_data_id: id,
            disposition: Disposition::Missing,
            source_sha256: None,
            read_refusal: Some(ReadRefusal::Missing),
            png_sha256: None,
            width: None,
            height: None,
        })
        .collect();
    assert!(manifest.validate().is_ok());
    let original = manifest.clone();
    manifest.assets.push(Asset {
        file_data_id: 129,
        ..manifest.assets[0].clone()
    });
    assert!(manifest.validate().is_err());
    manifest = original.clone();
    manifest.assets[1].file_data_id = 1;
    assert!(manifest.validate().is_err());
    manifest = original.clone();
    manifest.assets.swap(0, 1);
    assert!(manifest.validate().is_err());
    manifest = original.clone();
    manifest.assets[0].file_data_id = 0;
    assert!(manifest.validate().is_err());
    manifest = original.clone();
    manifest.assets.clear();
    assert!(manifest.validate().is_err());
    manifest = original;
    manifest.locale = "enUS-extra".into();
    assert!(manifest.validate().is_err());
}
