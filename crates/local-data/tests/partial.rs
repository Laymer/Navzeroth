//! Frame-aware partial reads (task V2.2d) over synthetic WDC5 files. Encrypted
//! frames are modelled only by their layout: no payload is ever produced for
//! them, and the reader must never be asked for their bytes.
use azeroth_local_data::{
    metadata::Definition,
    partial::{AssembleError, FrameLayout, PartialFile},
    tables::{self, Cell, Input, UNAVAILABLE_RANGE},
};
use wowdbdefs_rs::{Layout, Version};

/// Marker written into every value of the second section. It must never appear
/// in decoded rows or the retained form when that section is unavailable.
const MARKER: u32 = 0xA5A5_A5A5;

fn definition() -> Definition {
    let text = "COLUMNS\nint ID\nint Value\nLAYOUT 11223344\nBUILD 1.60.1.70450\n$noninline,id$ID<32>\nValue<32>\n";
    let file = wowdbdefs_rs::load_file_from_string(text, "Synthetic.dbd").unwrap();
    let layout = file
        .definitions
        .iter()
        .find(|d| {
            d.versions.contains(&Version::new(1, 60, 1, 70450))
                && d.layouts.contains(&Layout::new(0x11223344))
        })
        .unwrap()
        .clone();
    Definition { file, layout }
}
fn put16(b: &mut [u8], at: usize, v: u16) {
    b[at..at + 2].copy_from_slice(&v.to_le_bytes());
}
fn put32(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

/// Two-section WDC5 file: section 0 holds ids 1..=n0 with values 10, 20, ...;
/// section 1 (tact key `key1`) holds ids 101.. with `MARKER` values. Returns the
/// bytes and the offset where section 1 starts.
fn two_sections(n0: u32, n1: u32, key1: u64) -> (Vec<u8>, usize) {
    let sections = 204;
    let meta = sections + 80;
    let storage = meta + 4;
    let mut at = storage + 24;
    let encrypted_ids = if key1 != 0 { 4 + n1 as usize * 4 } else { 0 };
    let id_list = at;
    at += encrypted_ids;
    let s0 = at;
    let s1 = s0 + n0 as usize * 8;
    let end = s1 + n1 as usize * 8;
    let mut b = vec![0; end];
    b[..4].copy_from_slice(b"WDC5");
    put32(&mut b, 4, 5);
    let h = 136;
    put32(&mut b, h, n0 + n1);
    put32(&mut b, h + 4, 1);
    put32(&mut b, h + 8, 4);
    put32(&mut b, h + 16, 0xaabbccdd);
    put32(&mut b, h + 20, 0x11223344);
    put32(&mut b, h + 40, 1);
    put32(&mut b, h + 52, 24);
    put32(&mut b, h + 64, 2);
    for (i, (key, offset, count)) in [(0u64, s0, n0), (key1, s1, n1)].into_iter().enumerate() {
        let e = sections + i * 40;
        b[e..e + 8].copy_from_slice(&key.to_le_bytes());
        put32(&mut b, e + 8, offset as u32);
        put32(&mut b, e + 12, count);
        put32(&mut b, e + 20, (offset + count as usize * 4) as u32);
        put32(&mut b, e + 24, count * 4);
    }
    put16(&mut b, meta, 0);
    put16(&mut b, storage, 0);
    put16(&mut b, storage + 2, 32);
    if key1 != 0 {
        put32(&mut b, id_list, n1);
        for i in 0..n1 {
            put32(&mut b, id_list + 4 + i as usize * 4, 101 + i);
        }
    }
    for i in 0..n0 as usize {
        put32(&mut b, s0 + i * 4, 10 * (i as u32 + 1));
        put32(&mut b, s0 + n0 as usize * 4 + i * 4, i as u32 + 1);
    }
    for i in 0..n1 as usize {
        put32(&mut b, s1 + i * 4, MARKER);
        put32(&mut b, s1 + n1 as usize * 4 + i * 4, 101 + i as u32);
    }
    (b, s1)
}

/// Frames of `size` bytes over `bytes`; frames starting at or after
/// `encrypted_from` are flagged encrypted.
fn frames(length: usize, size: usize, encrypted_from: usize) -> Vec<FrameLayout> {
    (0..length.div_ceil(size))
        .map(|i| FrameLayout {
            start: (i * size) as u64,
            length: (size.min(length - i * size)) as u32,
            encrypted: i * size >= encrypted_from,
        })
        .collect()
}

/// Assemble, reading plain runs from `bytes` and failing the test if a read
/// touches any encrypted frame.
fn assemble(bytes: &[u8], layout: &[FrameLayout]) -> PartialFile {
    let encrypted: Vec<_> = layout
        .iter()
        .filter(|f| f.encrypted)
        .map(|f| (f.start as usize, f.start as usize + f.length as usize))
        .collect();
    PartialFile::assemble(bytes.len(), layout, |offset, length| {
        assert!(
            encrypted
                .iter()
                .all(|(s, e)| offset + length <= *s || offset >= *e),
            "reader asked for encrypted bytes"
        );
        Ok::<_, ()>(bytes[offset..offset + length].to_vec())
    })
    .unwrap()
}

fn values(decoded: &tables::DecodedTable) -> Vec<(u32, Cell)> {
    decoded
        .rows
        .iter()
        .map(|r| (r.id, r.fields["Value"].clone()))
        .collect()
}

#[test]
fn plain_sections_decode_and_encrypted_trailing_frames_stay_gaps() {
    let (bytes, s1) = two_sections(3, 2, 0x1122_3344_5566_7788);
    // Frame boundary exactly at the encrypted section, as on the 70450 client.
    let layout = vec![
        FrameLayout {
            start: 0,
            length: s1 as u32,
            encrypted: false,
        },
        FrameLayout {
            start: s1 as u64,
            length: (bytes.len() - s1) as u32,
            encrypted: true,
        },
    ];
    let partial = assemble(&bytes, &layout);
    assert_eq!(partial.len(), bytes.len());
    assert_eq!(partial.plain_bytes(), s1);
    let decoded = partial.decode(&definition()).unwrap();
    assert_eq!(
        values(&decoded),
        vec![
            (1, Cell::Signed(10)),
            (2, Cell::Signed(20)),
            (3, Cell::Signed(30))
        ]
    );
    assert_eq!(
        decoded.gaps,
        vec![
            "encrypted_section_records:2".to_owned(),
            format!(
                "unavailable_encrypted_frames:1-1:bytes:{s1}+{}",
                bytes.len() - s1
            ),
        ]
    );
    // The whole file decodes the same plain rows; only the read gap differs.
    let whole = tables::decode(&bytes, &definition()).unwrap();
    assert_eq!(values(&whole), values(&decoded));
    assert_eq!(whole.gaps, vec!["encrypted_section_records:2".to_owned()]);
    // The retained form holds the plain bytes and range table only.
    let retained = partial.retained_bytes();
    assert!(retained.starts_with(b"NAVZEROTH-PARTIAL-ORIGINAL-V1\0"));
    assert_eq!(retained.len(), 30 + 8 + 4 + 2 * 25 + s1);
    assert!(
        !retained
            .windows(4)
            .any(|w| w == MARKER.to_le_bytes().as_slice())
    );
    assert_eq!(&retained[retained.len() - s1..], &bytes[..s1]);
    assert_eq!(partial.retained_bytes(), retained, "deterministic");
}

#[test]
fn an_unflagged_section_in_an_encrypted_frame_is_a_distinct_gap() {
    // Section 1 has no tact key, but its bytes are in an encrypted frame.
    let (bytes, s1) = two_sections(2, 3, 0);
    let partial = assemble(&bytes, &frames(bytes.len(), s1, s1));
    let decoded = partial.decode(&definition()).unwrap();
    assert_eq!(values(&decoded).len(), 2);
    assert!(
        decoded
            .rows
            .iter()
            .all(|r| r.fields["Value"] != Cell::Signed(MARKER as i32 as i64))
    );
    assert!(
        decoded
            .gaps
            .contains(&"unavailable_frame_section_records:3".to_owned())
    );
    assert!(!decoded.gaps.iter().any(|g| g.starts_with("encrypted_")));
    // A frame boundary inside section 0 makes it unavailable too, never partial.
    let partial = assemble(&bytes, &frames(bytes.len(), s1 - 4, s1 - 4));
    let decoded = partial.decode(&definition()).unwrap();
    assert!(decoded.rows.is_empty());
    assert!(
        decoded
            .gaps
            .contains(&"unavailable_frame_section_records:5".to_owned())
    );
}

#[test]
fn table_metadata_in_an_encrypted_frame_refuses_the_table() {
    let (bytes, _) = two_sections(2, 2, 7);
    for boundary in [64, 210, 290] {
        let partial = assemble(&bytes, &frames(bytes.len(), boundary, boundary));
        let header = partial.header();
        let decoded = partial.decode(&definition());
        if boundary < 204 {
            assert_eq!(header.err(), Some(UNAVAILABLE_RANGE));
        } else {
            assert!(header.is_ok());
        }
        assert_eq!(
            decoded.err(),
            Some(UNAVAILABLE_RANGE),
            "boundary {boundary}"
        );
    }
}

#[test]
fn frame_layouts_must_cover_the_file_with_plain_and_encrypted_frames() {
    let frame = |start: u64, length: u32, encrypted: bool| FrameLayout {
        start,
        length,
        encrypted,
    };
    let read = |_: usize, length: usize| Ok::<_, &str>(vec![1; length]);
    let cases: [(&[FrameLayout], &str); 7] = [
        (&[], "frame_count_outside_bounds"),
        (
            &[frame(0, 4, false), frame(5, 4, true)],
            "noncontiguous_frames",
        ),
        (
            &[frame(0, 4, false), frame(4, 3, true)],
            "frames_do_not_cover_logical_length",
        ),
        (
            &[frame(0, 4, false), frame(4, 9, true)],
            "frames_exceed_logical_length",
        ),
        (
            &[frame(0, 4, false), frame(4, 4, false)],
            "no_encrypted_frames",
        ),
        (&[frame(0, 4, true), frame(4, 4, true)], "no_plain_frames"),
        (&[frame(0, 0, false), frame(0, 8, true)], "empty_frame"),
    ];
    for (layout, reason) in cases {
        assert_eq!(
            PartialFile::assemble(8, layout, read).err(),
            Some(AssembleError::Layout(reason))
        );
    }
    let layout = [frame(0, 4, false), frame(4, 4, true)];
    assert_eq!(
        PartialFile::assemble(8, &layout, |_, _| Err::<Vec<u8>, _>("io")).err(),
        Some(AssembleError::Read("io"))
    );
    assert_eq!(
        PartialFile::assemble(8, &layout, |_, _| Ok::<_, ()>(vec![0; 3])).err(),
        Some(AssembleError::Layout("plain_run_length_mismatch"))
    );
}

#[test]
fn interleaved_runs_merge_and_every_encrypted_run_is_named() {
    let layout: Vec<_> = [false, false, true, true, false, true]
        .iter()
        .enumerate()
        .map(|(i, encrypted)| FrameLayout {
            start: i as u64 * 4,
            length: 4,
            encrypted: *encrypted,
        })
        .collect();
    let bytes: Vec<u8> = (0..24).collect();
    let partial = assemble(&bytes, &layout);
    let gaps: Vec<_> = partial.unavailable().iter().map(|r| r.gap()).collect();
    assert_eq!(
        gaps,
        [
            "unavailable_encrypted_frames:2-3:bytes:8+8",
            "unavailable_encrypted_frames:5-5:bytes:20+4"
        ]
    );
    assert_eq!(partial.plain_bytes(), 12);
    let input = partial.input();
    assert!(input.available(0, 8));
    assert!(input.available(16, 20));
    assert!(!input.available(4, 12));
    assert!(!input.available(20, 24));
}

fn string_definition() -> Definition {
    let text = "COLUMNS\nint ID\nstring Value\nLAYOUT 11223344\nBUILD 1.60.1.70450\n$noninline,id$ID<32>\nValue\n";
    let file = wowdbdefs_rs::load_file_from_string(text, "Synthetic.dbd").unwrap();
    let layout = file.definitions[0].clone();
    Definition { file, layout }
}

/// One section's records (one u32 field each), string pool and copies.
struct Spec<'a> {
    key: u64,
    ids: &'a [u32],
    values: &'a [u32],
    strings: &'a [u8],
    copies: &'a [(u32, u32)],
}

/// Two-section WDC5 file from specs. Returns the bytes and section 1's offset.
fn build(specs: [Spec<'_>; 2]) -> (Vec<u8>, usize) {
    let sections = 204;
    let meta = sections + 80;
    let storage = meta + 4;
    let id_list = storage + 24;
    let listed: usize = specs
        .iter()
        .filter(|s| s.key != 0)
        .map(|s| 4 + s.ids.len() * 4)
        .sum();
    let mut offsets = Vec::new();
    let mut at = id_list + listed;
    for s in &specs {
        offsets.push(at);
        at += s.ids.len() * 8 + s.strings.len() + s.copies.len() * 8;
    }
    let mut b = vec![0; at];
    b[..4].copy_from_slice(b"WDC5");
    put32(&mut b, 4, 5);
    let h = 136;
    let records: usize = specs.iter().map(|s| s.ids.len()).sum();
    put32(&mut b, h, records as u32);
    put32(&mut b, h + 4, 1);
    put32(&mut b, h + 8, 4);
    put32(
        &mut b,
        h + 12,
        specs.iter().map(|s| s.strings.len() as u32).sum(),
    );
    put32(&mut b, h + 16, 0xaabbccdd);
    put32(&mut b, h + 20, 0x11223344);
    put32(&mut b, h + 40, 1);
    put32(&mut b, h + 52, 24);
    put32(&mut b, h + 64, 2);
    put16(&mut b, meta, 0);
    put16(&mut b, storage, 0);
    put16(&mut b, storage + 2, 32);
    let mut list = id_list;
    for (i, (s, offset)) in specs.iter().zip(&offsets).enumerate() {
        let n = s.ids.len();
        let e = sections + i * 40;
        b[e..e + 8].copy_from_slice(&s.key.to_le_bytes());
        put32(&mut b, e + 8, *offset as u32);
        put32(&mut b, e + 12, n as u32);
        put32(&mut b, e + 16, s.strings.len() as u32);
        put32(&mut b, e + 20, (offset + n * 4 + s.strings.len()) as u32);
        put32(&mut b, e + 24, n as u32 * 4);
        put32(&mut b, e + 36, s.copies.len() as u32);
        if s.key != 0 {
            put32(&mut b, list, n as u32);
            for (j, id) in s.ids.iter().enumerate() {
                put32(&mut b, list + 4 + j * 4, *id);
            }
            list += 4 + n * 4;
        }
        let mut at = *offset;
        for v in s.values {
            put32(&mut b, at, *v);
            at += 4;
        }
        b[at..at + s.strings.len()].copy_from_slice(s.strings);
        at += s.strings.len();
        for id in s.ids {
            put32(&mut b, at, *id);
            at += 4;
        }
        for (d, src) in s.copies {
            put32(&mut b, at, *d);
            put32(&mut b, at + 4, *src);
            at += 8;
        }
    }
    (b, offsets[1])
}

#[test]
fn strings_in_sections_that_are_not_decoded_are_reported_as_unavailable() {
    // Record 1 points at section 1's pool: (record - total) * 4 + v = 2 (pool 0
    // is 2 bytes long), so v = 2 + 2 * 4. Record 101 has an empty string.
    let specs = |key| {
        [
            Spec {
                key: 0,
                ids: &[1],
                values: &[10],
                strings: b"a\0",
                copies: &[],
            },
            Spec {
                key,
                ids: &[101],
                values: &[0],
                strings: b"zz\0",
                copies: &[],
            },
        ]
    };
    let (bytes, _) = build(specs(0));
    let whole = tables::decode(&bytes, &string_definition()).unwrap();
    assert_eq!(whole.rows[0].fields["Value"], Cell::Text("zz".into()));
    // Encrypted section 1: its pool is never read.
    let (bytes, _) = build(specs(7));
    assert_eq!(
        tables::decode(&bytes, &string_definition()).err(),
        Some(tables::STRING_IN_UNAVAILABLE_SECTION)
    );
    // Unflagged section 1 in an encrypted frame.
    let (bytes, s1) = build(specs(0));
    let partial = assemble(&bytes, &frames(bytes.len(), s1, s1));
    assert_eq!(
        partial.decode(&string_definition()).err(),
        Some(tables::STRING_IN_UNAVAILABLE_SECTION)
    );
}

#[test]
fn copies_of_records_in_sections_that_are_not_decoded() {
    let specs = |key, copies| {
        [
            Spec {
                key: 0,
                ids: &[1],
                values: &[10],
                strings: &[],
                copies,
            },
            Spec {
                key,
                ids: &[101],
                values: &[MARKER],
                strings: &[],
                copies: &[],
            },
        ]
    };
    // Source 101 is listed for an encrypted section: the chain 101 -> 200 -> 201
    // is an explicit gap and never materialized.
    let chain: &[(u32, u32)] = &[(200, 101), (201, 200)];
    let (bytes, s1) = build(specs(7, chain));
    for decoded in [
        tables::decode(&bytes, &definition()).unwrap(),
        assemble(&bytes, &frames(bytes.len(), s1, s1))
            .decode(&definition())
            .unwrap(),
    ] {
        assert_eq!(values(&decoded), vec![(1, Cell::Signed(10))]);
        assert!(
            decoded
                .gaps
                .contains(&"encrypted_copy_source_records:2".to_owned())
        );
    }
    // Unflagged section 1 in an encrypted frame: its IDs are unknown.
    let (bytes, s1) = build(specs(0, &[(200, 101)]));
    assert_eq!(
        assemble(&bytes, &frames(bytes.len(), s1, s1))
            .decode(&definition())
            .err(),
        Some(tables::COPY_SOURCE_IN_UNAVAILABLE_SECTION)
    );
    let whole = tables::decode(&bytes, &definition()).unwrap();
    assert_eq!(whole.rows.len(), 3);
    // A source that exists nowhere is still a refusal.
    let (bytes, _) = build(specs(7, &[(200, 999)]));
    assert_eq!(
        tables::decode(&bytes, &definition()).err(),
        Some("cyclic_or_missing_copy_source")
    );
}

#[test]
fn partial_inputs_refuse_overlaps_and_accesses_across_ranges() {
    let a = [1u8; 4];
    assert!(Input::partial(8, vec![(0, &a[..]), (2, &a[..])]).is_err());
    assert!(Input::partial(6, vec![(4, &a[..])]).is_err());
    assert!(Input::partial(8, vec![(0, &a[..0])]).is_err());
    // Adjacent ranges are accepted, but an access must lie inside one of them.
    let input = Input::partial(8, vec![(0, &a[..]), (4, &a[..])]).unwrap();
    assert!(input.available(0, 4) && input.available(4, 8));
    assert!(!input.available(2, 6));
    assert!(!input.available(0, 9));
}
