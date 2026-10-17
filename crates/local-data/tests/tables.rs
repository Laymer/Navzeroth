use azeroth_local_data::{
    metadata::{self, Definition},
    tables::{self, Cell},
};
use wowdbdefs_rs::{Layout, Version};

fn definition(columns: &str, entries: &str) -> Definition {
    let text = format!(
        "COLUMNS\nint ID\n{columns}\nLAYOUT 11223344\nBUILD 1.60.1.70425\n$noninline,id$ID<32>\n{entries}\n"
    );
    let file = wowdbdefs_rs::load_file_from_string(&text, "Synthetic.dbd").unwrap();
    let layout = file
        .definitions
        .iter()
        .find(|d| {
            d.versions.contains(&Version::new(1, 60, 1, 70425))
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
#[derive(Clone)]
struct Field {
    width: u16,
    offset: u16,
    kind: u32,
    a: u32,
    b: u32,
    c: u32,
    extra: Vec<u8>,
}
fn field(width: u16, offset: u16) -> Field {
    Field {
        width,
        offset,
        kind: 0,
        a: 0,
        b: 0,
        c: 0,
        extra: vec![],
    }
}

fn fixture(
    fields: &[Field],
    data: &[u8],
    count: u32,
    strings: &[u8],
    copies: &[(u32, u32)],
    relations: &[(u32, u32)],
    sparse: bool,
) -> Vec<u8> {
    let header = 204;
    let section = header;
    let meta = section + 40;
    let storage = meta + fields.len() * 4;
    let mut extra = Vec::new();
    for kind in [3, 4, 2] {
        for f in fields.iter().filter(|f| f.kind == kind) {
            extra.extend_from_slice(&f.extra);
        }
    }
    let start = storage + fields.len() * 24 + extra.len();
    let relation_bytes = if relations.is_empty() {
        0
    } else {
        12 + relations.len() * 8
    };
    let sparse_bytes = if sparse { 10 * count as usize } else { 0 };
    let mut bytes = vec![
        0;
        start
            + data.len()
            + strings.len()
            + count as usize * 4
            + copies.len() * 8
            + relation_bytes
            + sparse_bytes
    ];
    bytes[..4].copy_from_slice(b"WDC5");
    put32(&mut bytes, 4, 5);
    let b = 136;
    put32(&mut bytes, b, count);
    put32(&mut bytes, b + 4, fields.len() as u32);
    put32(
        &mut bytes,
        b + 8,
        (data.len() / count.max(1) as usize) as u32,
    );
    put32(&mut bytes, b + 12, strings.len() as u32);
    put32(&mut bytes, b + 16, 0xaabbccdd);
    put32(&mut bytes, b + 20, 0x11223344);
    put16(&mut bytes, b + 36, if sparse { 1 } else { 0 });
    put32(&mut bytes, b + 40, fields.len() as u32);
    put32(&mut bytes, b + 52, fields.len() as u32 * 24);
    put32(
        &mut bytes,
        b + 56,
        fields
            .iter()
            .filter(|f| f.kind == 2)
            .map(|f| f.extra.len() as u32)
            .sum(),
    );
    put32(
        &mut bytes,
        b + 60,
        fields
            .iter()
            .filter(|f| matches!(f.kind, 3 | 4))
            .map(|f| f.extra.len() as u32)
            .sum(),
    );
    put32(&mut bytes, b + 64, 1);
    put32(&mut bytes, section + 8, start as u32);
    put32(&mut bytes, section + 12, count);
    put32(&mut bytes, section + 16, strings.len() as u32);
    put32(&mut bytes, section + 20, (start + data.len()) as u32);
    put32(&mut bytes, section + 24, count * 4);
    put32(&mut bytes, section + 28, relation_bytes as u32);
    put32(&mut bytes, section + 32, if sparse { count } else { 0 });
    put32(&mut bytes, section + 36, copies.len() as u32);
    for (i, f) in fields.iter().enumerate() {
        put16(&mut bytes, meta + i * 4, (32i16 - f.width as i16) as u16);
        let p = storage + i * 24;
        put16(&mut bytes, p, f.offset);
        put16(&mut bytes, p + 2, f.width);
        put32(&mut bytes, p + 4, f.extra.len() as u32);
        put32(&mut bytes, p + 8, f.kind);
        put32(&mut bytes, p + 12, f.a);
        put32(&mut bytes, p + 16, f.b);
        put32(&mut bytes, p + 20, f.c);
    }
    bytes[storage + fields.len() * 24..start].copy_from_slice(&extra);
    bytes[start..start + data.len()].copy_from_slice(data);
    let mut at = start + data.len();
    bytes[at..at + strings.len()].copy_from_slice(strings);
    at += strings.len();
    for i in 0..count {
        put32(&mut bytes, at, i + 1);
        at += 4;
    }
    for (d, s) in copies {
        put32(&mut bytes, at, *d);
        put32(&mut bytes, at + 4, *s);
        at += 8;
    }
    if sparse {
        assert_eq!(count, 1);
        put32(&mut bytes, at, start as u32);
        put16(&mut bytes, at + 4, data.len() as u16);
        at += 6;
    }
    if !relations.is_empty() {
        put32(&mut bytes, at, relations.len() as u32);
        at += 12;
        for (id, index) in relations {
            put32(&mut bytes, at, *id);
            put32(&mut bytes, at + 4, *index);
            at += 8;
        }
    }
    if sparse {
        put32(&mut bytes, at, 1);
    }
    bytes
}

#[test]
fn retained_definitions_support_exact_large_builds_without_remapping() {
    // 70450 is admitted alongside 70374/70425 as development input only.
    assert_eq!(metadata::ADMITTED_BUILDS, [70374, 70425, 70450]);
    for (table, layout) in [("Map", 0xd43afac3)] {
        for &build in metadata::ADMITTED_BUILDS {
            let d = metadata::definition(table, build, layout).unwrap();
            assert!(!d.layout.entries.is_empty());
        }
        // Neighbouring and unknown builds are refused, never mapped to the nearest.
        for build in [70373, 70426, 70449, 70451, 0, u32::MAX] {
            assert_eq!(
                metadata::definition(table, build, layout).err(),
                Some("unsupported_build")
            );
        }
        assert!(metadata::definition(table, 70425, 0).is_err());
        assert!(metadata::definition(table, 70450, 0).is_err());
    }
    assert!(metadata::candidates().len() > 1024);
}

#[test]
fn signed_zero_strings_copy_and_missing_relationship_are_distinct() {
    let def = definition(
        "int Value\nstring Text\nint Parent",
        "Value<32>\nText\n$noninline,relation$Parent<32>",
    );
    let mut row = vec![0; 8];
    put32(&mut row, 0, u32::MAX);
    put32(&mut row, 4, 4);
    let bytes = fixture(
        &[field(32, 0), field(32, 32)],
        &row,
        1,
        b"hello\0",
        &[(2, 1)],
        &[],
        false,
    );
    let decoded = tables::decode(&bytes, &def).unwrap();
    assert_eq!(decoded.rows.len(), 2);
    assert_eq!(decoded.rows[0].fields["Value"], Cell::Signed(-1));
    assert_eq!(decoded.rows[0].fields["Text"], Cell::Text("hello".into()));
    assert_eq!(decoded.rows[0].fields["Parent"], Cell::Unknown);
    assert_eq!(decoded.rows[1].copied_from, Some(1));
    row.fill(0);
    let bytes = fixture(
        &[field(32, 0), field(32, 32)],
        &row,
        1,
        b"\0",
        &[],
        &[(0, 0)],
        false,
    );
    let decoded = tables::decode(&bytes, &def).unwrap();
    assert_eq!(decoded.rows[0].fields["Value"], Cell::Signed(0));
    assert_eq!(decoded.rows[0].fields["Text"], Cell::Text("".into()));
    assert_eq!(decoded.rows[0].fields["Parent"], Cell::Unsigned(0));
}

#[test]
fn bitpacked_signed_common_and_pallet_values_are_bounded() {
    let def = definition(
        "int Signed\nint Common\nint Choice",
        "Signed<32>\nCommon<u32>\nChoice<u32>",
    );
    let mut common = field(32, 0);
    common.kind = 2;
    common.a = 0;
    common.extra = [1u32.to_le_bytes(), 42u32.to_le_bytes()].concat();
    let mut packed = field(8, 0);
    packed.kind = 5;
    packed.b = 5;
    let mut pallet = field(32, 5);
    pallet.kind = 3;
    pallet.a = 5;
    pallet.b = 1;
    pallet.extra = [0u32.to_le_bytes(), 99u32.to_le_bytes()].concat();
    let bytes = fixture(
        &[packed, common, pallet],
        &[0b0011_1111],
        1,
        &[],
        &[],
        &[],
        false,
    );
    let row = &tables::decode(&bytes, &def).unwrap().rows[0];
    assert_eq!(row.fields["Signed"], Cell::Signed(-1));
    assert_eq!(row.fields["Common"], Cell::Unsigned(42));
    assert_eq!(row.fields["Choice"], Cell::Unsigned(99));
    assert!(tables::bits(&[0], 7, 2).is_err());
    assert!(tables::bits(&[0], 0, 65).is_err());
}

#[test]
fn sparse_inline_strings_and_unsupported_storage_remain_explicit() {
    let def = definition("string Text\nint Value", "Text\nValue<u16>");
    let bytes = fixture(
        &[field(32, 0), field(16, 32)],
        b"hi\0\x00\x00",
        1,
        &[],
        &[],
        &[],
        true,
    );
    let decoded = tables::decode(&bytes, &def).unwrap();
    assert_eq!(decoded.rows[0].fields["Text"], Cell::Text("hi".into()));
    assert_eq!(decoded.rows[0].fields["Value"], Cell::Unsigned(0));
}

#[test]
fn truncation_overflow_invalid_offsets_copy_cycles_and_unterminated_strings_refuse() {
    let def = definition("int Value", "Value<u32>");
    let valid = fixture(&[field(32, 0)], &[0; 4], 1, &[], &[], &[], false);
    for len in 0..valid.len() {
        assert!(
            std::panic::catch_unwind(|| tables::decode(&valid[..len], &def))
                .unwrap()
                .is_err(),
            "{len}"
        );
    }
    let mut changed = valid.clone();
    put32(&mut changed, 136, u32::MAX);
    assert!(tables::decode(&changed, &def).is_err());
    let mut changed = valid.clone();
    put32(&mut changed, 204 + 8, 0);
    assert!(tables::decode(&changed, &def).is_err());
    let cyclic = fixture(
        &[field(32, 0)],
        &[0; 4],
        1,
        &[],
        &[(2, 3), (3, 2)],
        &[],
        false,
    );
    assert!(tables::decode(&cyclic, &def).is_err());
    let duplicate = fixture(&[field(32, 0)], &[0; 4], 1, &[], &[(1, 1)], &[], false);
    assert!(tables::decode(&duplicate, &def).is_err());
    let def = definition("string Text", "Text");
    let bytes = fixture(
        &[field(32, 0)],
        &4u32.to_le_bytes(),
        1,
        b"bad",
        &[],
        &[],
        false,
    );
    assert!(tables::decode(&bytes, &def).is_err());
}

#[test]
fn zero_record_id_is_a_real_identity_not_a_missing_record() {
    let def = definition("int Value", "Value<u32>");
    let mut bytes = fixture(&[field(32, 0)], &[0; 4], 1, &[], &[], &[], false);
    let at = bytes.len() - 4;
    put32(&mut bytes, at, 0);
    let decoded = tables::decode(&bytes, &def).unwrap();
    assert_eq!(decoded.rows[0].id, 0);
    assert_eq!(decoded.rows[0].fields["Value"], Cell::Unsigned(0));
}

#[test]
fn research_rows_exceed_1024_without_truncation() {
    let def = definition("int Value", "Value<u32>");
    let mut records = Vec::new();
    for i in 0..1537u32 {
        records.extend_from_slice(&i.to_le_bytes());
    }
    let bytes = fixture(&[field(32, 0)], &records, 1537, &[], &[], &[], false);
    let decoded = tables::decode(&bytes, &def).unwrap();
    assert_eq!(decoded.rows.len(), 1537);
    assert_eq!(decoded.rows[1536].fields["Value"], Cell::Unsigned(1536));
}

#[test]
fn long_copy_chains_resolve_once_and_array_cardinality_is_bounded() {
    let def = definition("int Value", "Value<u32>");
    let copies: Vec<_> = (2..=2048).map(|id| (id, id - 1)).collect();
    let bytes = fixture(
        &[field(32, 0)],
        &7u32.to_le_bytes(),
        1,
        &[],
        &copies,
        &[],
        false,
    );
    let decoded = tables::decode(&bytes, &def).unwrap();
    assert_eq!(decoded.rows.len(), 2048);
    assert_eq!(decoded.rows.last().unwrap().copied_from, Some(2047));
    assert_eq!(
        decoded.rows.last().unwrap().fields["Value"],
        Cell::Unsigned(7)
    );
    let mut def = def;
    def.layout.entries[1].array_size = Some(257);
    assert!(tables::decode(&bytes, &def).is_err());
}

#[test]
fn signed_compression_preserves_unsigned_logical_bit_patterns() {
    let def = definition("int Value", "Value<u32>");
    let mut packed = field(8, 0);
    packed.kind = 5;
    packed.b = 5;
    let bytes = fixture(&[packed], &[0x1f], 1, &[], &[], &[], false);
    let decoded = tables::decode(&bytes, &def).unwrap();
    assert_eq!(
        decoded.rows[0].fields["Value"],
        Cell::Unsigned(u32::MAX.into())
    );
}

#[test]
fn decoded_cell_capacity_is_exactly_four_million() {
    assert_eq!(tables::MAX_CELLS, 4_000_000);
    // 1 ID cell + 249 array cells = 250 cells per row; 16,000 rows (one record
    // plus copies) is exactly the bound, one more row is over it.
    let def = definition("int Value", "Value<u8>[249]");
    let record: Vec<u8> = (0..249u32).map(|i| i as u8).collect();
    let rows = |n: u32| {
        let copies: Vec<_> = (2..=n).map(|id| (id, 1)).collect();
        fixture(&[field(8, 0)], &record, 1, &[], &copies, &[], false)
    };
    let decoded = tables::decode(&rows(16_000), &def).unwrap();
    assert_eq!(decoded.rows.len(), 16_000);
    assert_eq!(
        decoded.rows[15_999].fields["Value"],
        Cell::Array((0..249u64).map(Cell::Unsigned).collect())
    );
    drop(decoded);
    assert_eq!(
        tables::decode(&rows(16_001), &def).unwrap_err(),
        "decoded_cell_capacity"
    );
}
