use azeroth_core::local_data_inputs::*;
fn cache(version: u32, build: u32) -> Vec<u8> {
    let mut v = b"XFTH".to_vec();
    v.extend(version.to_le_bytes());
    v.extend(build.to_le_bytes());
    if version >= 5 {
        v.extend([0; 32]);
    }
    v
}
fn record(version: u32, push: i32, id: u32, op: u8, payload: &[u8]) -> Vec<u8> {
    let mut v = b"XFTH".to_vec();
    let mut fields = match version {
        1 => vec![push as u32, payload.len() as u32, 123, id],
        2..=6 => vec![version, push as u32, payload.len() as u32, 123, id],
        7 => vec![push as u32, 123, id, payload.len() as u32],
        9 => vec![2, push as u32, 7, 123, id, payload.len() as u32],
        _ => panic!(),
    };
    for f in fields.drain(..) {
        v.extend(f.to_le_bytes());
    }
    v.extend([op, 0, 0, 0]);
    v.extend(payload);
    v
}
#[test]
fn reviewed_envelopes_keep_order_flags_zero_and_payload_ranges() {
    for version in [1, 2, 3, 4, 5, 6, 7, 9] {
        let mut bytes = cache(version, 70450);
        bytes.extend(record(version, -1, 7, 1, &[0, 255]));
        bytes.extend(record(version, 0, 8, 0, &[]));
        let parsed = parse_cache(&bytes, InputKind::HotfixCache, 70450).unwrap();
        assert!(parsed.unsupported.is_none());
        assert_eq!(parsed.records.len(), 2);
        assert_eq!(parsed.records[0].push_id, -1);
        assert_eq!(parsed.records[1].push_id, 0);
        assert_eq!(parsed.records[0].operation, CacheOperation::UpsertCandidate);
        assert_eq!(
            parsed.records[1].operation,
            CacheOperation::InvalidationUnknown
        );
        assert_eq!(parsed.records[1].payload_size, 0);
        let r = &parsed.records[0];
        assert_eq!(
            &bytes[r.payload_offset as usize..(r.payload_offset + r.payload_size) as usize],
            &[0, 255]
        );
        if version == 9 {
            assert_eq!(r.region_id, Some(2));
            assert_eq!(r.unique_id, Some(7));
        }
    }
}
#[test]
fn all_truncated_record_prefixes_fail_without_panicking() {
    for version in [1, 2, 5, 7, 9] {
        let mut v = cache(version, 70450);
        let boundary = v.len();
        v.extend(record(version, 1, 7, 1, &[5; 10]));
        for n in boundary + 1..v.len() {
            assert!(
                parse_cache(&v[..n], InputKind::HotfixCache, 70450).is_err(),
                "version {version} prefix {n}"
            );
        }
    }
}
#[test]
fn unsupported_and_ambiguous_formats_are_explicit_not_empty_success() {
    for version in [0, 8, 10, u32::MAX] {
        let p = parse_cache(&cache(version, 70450), InputKind::HotfixCache, 70450).unwrap();
        assert!(p.unsupported.is_some());
        assert!(p.records.is_empty());
    }
    assert!(
        parse_cache(b"WQSTarbitrary", InputKind::QuestCache, 70450)
            .unwrap()
            .unsupported
            .is_some()
    );
    assert!(parse_cache(b"WQST", InputKind::CharacterSnapshot, 70450).is_err());
}
#[test]
fn no_cross_build_or_unknown_header_flags() {
    let mut v = cache(9, 70450);
    v.extend(record(9, 1, 7, 1, b"a"));
    assert!(parse_cache(&v, InputKind::HotfixCache, 70374).is_err());
    v[44 + 4 + 25] = 1;
    assert!(parse_cache(&v, InputKind::HotfixCache, 70450).is_err());
}
#[test]
fn duplicate_record_pushes_are_preserved_as_conflicts() {
    let mut v = cache(7, 70450);
    v.extend(record(7, 5, 8, 1, b"a"));
    v.extend(record(7, 5, 8, 1, b"b"));
    v.extend(record(7, 6, 8, 1, b"c"));
    let p = parse_cache(&v, InputKind::HotfixCache, 70450).unwrap();
    assert_eq!(p.records.len(), 3);
    assert_eq!(p.conflicting_records, vec![(123, 8, 5)]);
}
#[test]
fn empty_reviewed_cache_is_justified_but_truncated_extended_header_is_not() {
    let p = parse_cache(&cache(9, 70450), InputKind::HotfixCache, 70450).unwrap();
    assert!(p.unsupported.is_none());
    assert!(p.records.is_empty());
    assert!(parse_cache(&cache(9, 70450)[..43], InputKind::HotfixCache, 70450).is_err());
}
#[test]
fn malicious_sizes_and_ids_are_bounded() {
    for id in [0, u32::MAX] {
        let mut v = cache(7, 70450);
        v.extend(record(7, 0, id, 1, b"a"));
        assert!(parse_cache(&v, InputKind::HotfixCache, 70450).is_err());
    }
    let mut v = cache(7, 70450);
    v.extend(record(7, 0, 7, 1, b"a"));
    v[44 + 4 + 12..44 + 4 + 16].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(parse_cache(&v, InputKind::HotfixCache, 70450).is_err());
}

#[test]
fn record_capacity_is_enforced_before_an_unbounded_inventory_can_grow() {
    let mut bytes = cache(7, 70450);
    let row = record(7, 1, 7, 0, &[]);
    for _ in 0..=MAX_CACHE_RECORDS {
        bytes.extend(&row);
    }
    assert!(parse_cache(&bytes, InputKind::HotfixCache, 70450).is_err());
}
