//! Bounded WDC research decoding. No gameplay availability or hotfix inference.
//! Format cross-check: pinned DBCD e732093f (development oracle only).
use crate::metadata::Definition;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use wowdbdefs_rs::{RawEntry, RawType};

const MAX_RECORDS: usize = 1_000_000;
/// Fixed bound on decoded scalar/array cells per table, copies included. Sized so
/// build 70450 ItemSparse (2,000,024 cells in its plain sections) decodes; see
/// docs/data/LOCAL_EXTRACTION.md for the memory reasoning.
pub const MAX_CELLS: usize = 4_000_000;
const MAX_TEXT_BYTES: usize = 64 * 1024 * 1024;
type Result<T> = std::result::Result<T, &'static str>;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TableHeader {
    pub format: String,
    pub records: u32,
    pub fields: u32,
    pub record_bytes: u32,
    pub table_hash: u32,
    pub layout_hash: u32,
    pub locale_mask: u32,
    pub sections: u32,
}
/// Decoder reason for any access that touches bytes outside the verified plain
/// ranges of a partially available logical file. Such bytes do not exist here:
/// they were never read, filled or synthesized.
pub const UNAVAILABLE_RANGE: &str = "unavailable_encrypted_frame_range";

/// The decoder's view of one logical file: its declared length and the verified
/// plain byte ranges that are actually present. A whole file is one range. Every
/// access must lie inside one range; anything else is refused, never zero-read.
pub struct Input<'a> {
    length: usize,
    segments: Vec<(usize, &'a [u8])>,
}
impl<'a> Input<'a> {
    pub fn whole(bytes: &'a [u8]) -> Self {
        Self {
            length: bytes.len(),
            segments: vec![(0, bytes)],
        }
    }
    /// Accepts only sorted, non-empty, non-overlapping ranges inside `length`.
    pub fn partial(length: usize, segments: Vec<(usize, &'a [u8])>) -> Result<Self> {
        let mut end = 0usize;
        for (start, bytes) in &segments {
            let stop = start
                .checked_add(bytes.len())
                .ok_or("table_offset_overflow")?;
            if bytes.is_empty() || *start < end || stop > length {
                return Err("invalid_partial_layout");
            }
            end = stop;
        }
        Ok(Self { length, segments })
    }
    pub fn len(&self) -> usize {
        self.length
    }
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
    /// Whether `[start, end)` lies entirely inside one verified range.
    pub fn available(&self, start: usize, end: usize) -> bool {
        self.find(start, end).is_some()
    }
    fn find(&self, start: usize, end: usize) -> Option<&'a [u8]> {
        if end < start || end > self.length {
            return None;
        }
        if start == end {
            return Some(&[]);
        }
        let index = self
            .segments
            .partition_point(|(s, _)| *s <= start)
            .checked_sub(1)?;
        let (s, bytes) = self.segments[index];
        (end <= s + bytes.len()).then(|| &bytes[start - s..end - s])
    }
    fn get(&self, start: usize, length: usize, outside: &'static str) -> Result<&'a [u8]> {
        let end = start.checked_add(length).ok_or("table_offset_overflow")?;
        if end > self.length {
            return Err(outside);
        }
        self.find(start, end).ok_or(UNAVAILABLE_RANGE)
    }
}
fn u16_at(bytes: &Input<'_>, at: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        bytes.get(at, 2, "truncated_table")?.try_into().unwrap(),
    ))
}
fn u32_at(bytes: &Input<'_>, at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes.get(at, 4, "truncated_table")?.try_into().unwrap(),
    ))
}
fn u64_at(bytes: &Input<'_>, at: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        bytes.get(at, 8, "truncated_table")?.try_into().unwrap(),
    ))
}
fn range<'a>(bytes: &Input<'a>, start: usize, length: usize) -> Result<&'a [u8]> {
    bytes.get(start, length, "table_offset_outside_input")
}
fn base(bytes: &Input<'_>) -> Result<usize> {
    let magic = match bytes.get(0, 4, "unsupported_table_format") {
        Ok(magic) => magic,
        Err(UNAVAILABLE_RANGE) => return Err(UNAVAILABLE_RANGE),
        Err(_) => return Err("unsupported_table_format"),
    };
    match magic {
        b"WDC3" | b"WDC4" => Ok(4),
        b"WDC5" => {
            if u32_at(bytes, 4)? != 5 {
                return Err("unsupported_wdc_schema");
            }
            Ok(136)
        }
        _ => Err("unsupported_table_format"),
    }
}
pub fn header(bytes: &[u8]) -> Result<TableHeader> {
    header_of(&Input::whole(bytes))
}
/// Header of a whole or partially available file. The header itself must lie in
/// a verified plain range.
pub fn header_of(bytes: &Input<'_>) -> Result<TableHeader> {
    let b = base(bytes)?;
    range(bytes, b, 68)?;
    let h = TableHeader {
        format: std::str::from_utf8(range(bytes, 0, 4)?).unwrap().into(),
        records: u32_at(bytes, b)?,
        fields: u32_at(bytes, b + 4)?,
        record_bytes: u32_at(bytes, b + 8)?,
        table_hash: u32_at(bytes, b + 16)?,
        layout_hash: u32_at(bytes, b + 20)?,
        locale_mask: u32_at(bytes, b + 32)?,
        sections: u32_at(bytes, b + 64)?,
    };
    if h.records as usize > MAX_RECORDS
        || h.fields > 512
        || h.record_bytes > 65536
        || h.sections > 1024
    {
        return Err("table_capacity");
    }
    if h.records > 0 && (h.record_bytes == 0 || h.sections == 0) {
        return Err("invalid_table_header");
    }
    Ok(h)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub enum Cell {
    Unsigned(u64),
    Signed(i64),
    Float(f64),
    Text(String),
    Array(Vec<Cell>),
    Unknown,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchRow {
    pub id: u32,
    pub fields: BTreeMap<String, Cell>,
    pub copied_from: Option<u32>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecodedTable {
    pub format_version: u32,
    pub header: TableHeader,
    pub rows: Vec<ResearchRow>,
    pub gaps: Vec<String>,
}
fn text_bytes(cell: &Cell) -> usize {
    match cell {
        Cell::Text(value) => value.len(),
        Cell::Array(values) => values.iter().map(text_bytes).sum(),
        _ => 0,
    }
}

struct Column {
    offset: usize,
    width: usize,
    kind: u32,
    a: u32,
    b: u32,
    c: u32,
    pallet: Vec<u32>,
    common: BTreeMap<u32, u32>,
}
#[derive(Clone)]
struct Section {
    key: u64,
    offset: usize,
    count: usize,
    strings: usize,
    end: usize,
    ids: usize,
    relations: usize,
    sparse: usize,
    copies: usize,
}

pub fn bits(bytes: &[u8], start: usize, width: usize) -> Result<u64> {
    if width == 0
        || width > 64
        || start
            .checked_add(width)
            .is_none_or(|end| end > bytes.len().saturating_mul(8))
    {
        return Err("invalid_bit_field");
    }
    let mut value = 0u64;
    for bit in 0..width {
        value |= (((bytes[(start + bit) / 8] >> ((start + bit) % 8)) & 1) as u64) << bit;
    }
    Ok(value)
}
fn signed(value: u64, width: usize) -> i64 {
    if width == 64 {
        value as i64
    } else {
        ((value << (64 - width)) as i64) >> (64 - width)
    }
}

fn raw_values(
    column: &Column,
    row: &[u8],
    id: u32,
    count: usize,
    sparse_cursor: Option<&mut usize>,
) -> Result<Vec<(u64, usize, usize)>> {
    if count == 0 || count > 256 {
        return Err("invalid_array_width");
    }
    let offset = if let Some(cursor) = sparse_cursor {
        let offset = *cursor;
        *cursor = cursor
            .checked_add(column.width * count)
            .ok_or("table_offset_overflow")?;
        offset
    } else {
        // RecordOffset is absolute in the record; immediate/pallet 'a' is
        // relative to PackedDataOffset and must not address the unpacked prefix.
        column.offset
    };
    match column.kind {
        0 | 1 | 5 => {
            let width = if column.kind == 0 {
                column.width
            } else {
                column.b as usize
            };
            (0..count)
                .map(|i| {
                    let raw = bits(row, offset + i * width, width)?;
                    Ok((
                        if column.kind == 5 {
                            signed(raw, width) as u64
                        } else {
                            raw
                        },
                        width,
                        offset + i * width,
                    ))
                })
                .collect()
        }
        2 if count == 1 => Ok(vec![(
            u64::from(*column.common.get(&id).unwrap_or(&column.a)),
            32,
            offset,
        )]),
        3 | 4 => {
            let index = bits(row, offset, column.b as usize)? as usize;
            let cardinality = if column.kind == 3 {
                1
            } else {
                column.c as usize
            };
            if cardinality != count {
                return Err("pallet_cardinality_mismatch");
            }
            let begin = index
                .checked_mul(cardinality)
                .ok_or("table_offset_overflow")?;
            (0..count)
                .map(|i| {
                    Ok((
                        u64::from(*column.pallet.get(begin + i).ok_or("invalid_pallet_index")?),
                        32,
                        offset,
                    ))
                })
                .collect()
        }
        _ => Err("unsupported_field_storage"),
    }
}

#[derive(Clone, Copy)]
struct StringContext<'a> {
    row: &'a [u8],
    global_record: usize,
    total_records: usize,
    record_size: usize,
    pools: &'a [(usize, &'a [u8])],
    sparse: bool,
}
fn value(
    raw: (u64, usize, usize),
    entry: &RawEntry,
    ty: RawType,
    context: &StringContext<'_>,
    cursor: &mut usize,
) -> Result<Cell> {
    let StringContext {
        row,
        global_record,
        total_records,
        record_size,
        pools,
        sparse,
    } = *context;
    let (v, width, offset) = raw;
    Ok(match ty {
        RawType::Int if entry.unsigned => {
            let width = usize::from(
                entry
                    .integer_width
                    .ok_or("definition_integer_width_unknown")?,
            );
            if width == 0 || width > 64 {
                return Err("unsupported_integer_width");
            }
            Cell::Unsigned(if width == 64 {
                v
            } else {
                v & ((1u64 << width) - 1)
            })
        }
        RawType::Int => {
            let declared = usize::from(
                entry
                    .integer_width
                    .ok_or("definition_integer_width_unknown")?,
            );
            if declared == 0 || declared > 64 {
                return Err("unsupported_integer_width");
            }
            Cell::Signed(signed(v, declared.min(width)))
        }
        RawType::Float => {
            if width > 32 {
                return Err("invalid_float_width");
            }
            let value = f32::from_bits(v as u32) as f64;
            if !value.is_finite() {
                return Err("nonfinite_table_float");
            }
            Cell::Float(value)
        }
        RawType::String | RawType::LocString => {
            if sparse {
                let start = offset / 8;
                let tail = row.get(start..).ok_or("invalid_string_offset")?;
                let length = tail
                    .iter()
                    .position(|b| *b == 0)
                    .ok_or("unterminated_table_string")?;
                if length > 64 * 1024 {
                    return Err("table_string_capacity");
                }
                *cursor = (start + length + 1) * 8;
                Cell::Text(
                    std::str::from_utf8(&tail[..length])
                        .map_err(|_| "invalid_string_utf8")?
                        .into(),
                )
            } else if v == 0 {
                Cell::Text(String::new())
            } else {
                // WDC string pointers use the virtual contiguous record/string
                // address space, including preceding sections, not filesystem offsets.
                let address = (global_record as i64 - total_records as i64) * record_size as i64
                    + offset as i64 / 8
                    + v as u32 as i32 as i64;
                if address < 0 {
                    return Err("invalid_string_offset");
                }
                let address = address as usize;
                let index = pools
                    .partition_point(|(start, _)| *start <= address)
                    .checked_sub(1)
                    .ok_or("invalid_string_offset")?;
                let (start, pool) = pools
                    .get(index)
                    .filter(|(start, pool)| address < *start + pool.len())
                    .ok_or("invalid_string_offset")?;
                let tail = &pool[address - start..];
                let length = tail
                    .iter()
                    .position(|b| *b == 0)
                    .ok_or("unterminated_table_string")?;
                if length > 64 * 1024 {
                    return Err("table_string_capacity");
                }
                Cell::Text(
                    std::str::from_utf8(&tail[..length])
                        .map_err(|_| "invalid_string_utf8")?
                        .into(),
                )
            }
        }
    })
}

pub fn decode(bytes: &[u8], definition: &Definition) -> Result<DecodedTable> {
    decode_input(&Input::whole(bytes), definition)
}

/// Why a section's records are not decoded. Each is an explicit gap.
#[derive(Clone, Copy, PartialEq)]
enum Skip {
    /// The section's tact-key lookup is non-zero: encrypted, never recovered.
    Encrypted,
    /// The section is not key-flagged, but some of its bytes lie outside the
    /// verified plain ranges of a partially available file.
    Unavailable,
}

/// Decode a whole or partially available file. Header, field, pallet, common
/// and section-directory bytes must be available, or the table is refused with
/// [`UNAVAILABLE_RANGE`]. A section is decoded only when all of its records,
/// strings, IDs, copies, sparse entries and relationships are available.
pub fn decode_input(bytes: &Input<'_>, definition: &Definition) -> Result<DecodedTable> {
    let h = header_of(bytes)?;
    let b = base(bytes)?;
    let field_count = h.fields as usize;
    let record_size = h.record_bytes as usize;
    let cells_per_record = definition
        .layout
        .entries
        .iter()
        .try_fold(0usize, |sum, e| {
            let count = e.array_size.unwrap_or(1);
            if count == 0 || count > 256 {
                return Err("invalid_array_width");
            }
            sum.checked_add(count).ok_or("decoded_cell_capacity")
        })?;
    if u32_at(bytes, b + 40)? != h.fields || u32_at(bytes, b + 52)? as usize != field_count * 24 {
        return Err("unsupported_column_layout");
    }
    let flags = u16_at(bytes, b + 36)?;
    if flags & !0x1f != 0 {
        return Err("unsupported_table_flags");
    }
    let sparse = flags & 1 != 0;
    let mut sections = Vec::new();
    let mut section_records = 0usize;
    for i in 0..h.sections as usize {
        let at = b + 68 + i * 40;
        range(bytes, at, 40)?;
        let section = Section {
            key: u64_at(bytes, at)?,
            offset: u32_at(bytes, at + 8)? as usize,
            count: u32_at(bytes, at + 12)? as usize,
            strings: u32_at(bytes, at + 16)? as usize,
            end: u32_at(bytes, at + 20)? as usize,
            ids: u32_at(bytes, at + 24)? as usize,
            relations: u32_at(bytes, at + 28)? as usize,
            sparse: u32_at(bytes, at + 32)? as usize,
            copies: u32_at(bytes, at + 36)? as usize,
        };
        section_records = section_records
            .checked_add(section.count)
            .ok_or("table_offset_overflow")?;
        if section.count > MAX_RECORDS
            || section.copies > MAX_RECORDS
            || section.sparse > MAX_RECORDS
            || !section.ids.is_multiple_of(4)
        {
            return Err("invalid_section_counts");
        }
        sections.push(section);
    }
    if section_records != h.records as usize {
        return Err("section_record_count_mismatch");
    }
    let fields = b + 68 + sections.len() * 40;
    let storage = fields + field_count * 4;
    range(bytes, fields, field_count * 28)?;
    let mut columns = Vec::new();
    for i in 0..field_count {
        let at = storage + i * 24;
        let bits = u16_at(bytes, fields + i * 4)? as i16;
        let kind = u32_at(bytes, at + 8)?;
        let width = if kind == 0 {
            32i32 - i32::from(bits)
        } else if kind == 2 {
            32
        } else {
            u32_at(bytes, at + 16)? as i32
        };
        if !(1..=64).contains(&width) {
            return Err("invalid_column_width");
        }
        columns.push(Column {
            offset: u16_at(bytes, at)? as usize,
            width: width as usize,
            kind: u32_at(bytes, at + 8)?,
            a: u32_at(bytes, at + 12)?,
            b: u32_at(bytes, at + 16)?,
            c: u32_at(bytes, at + 20)?,
            pallet: Vec::new(),
            common: BTreeMap::new(),
        });
    }
    let mut at = storage + field_count * 24;
    let pallet_start = at;
    for kind in [3, 4] {
        for (i, col) in columns
            .iter_mut()
            .enumerate()
            .filter(|(_, c)| c.kind == kind)
        {
            let size = u32_at(bytes, storage + i * 24 + 4)? as usize;
            if !size.is_multiple_of(4) {
                return Err("invalid_pallet_size");
            }
            range(bytes, at, size)?;
            col.pallet = (0..size / 4)
                .map(|i| u32_at(bytes, at + i * 4))
                .collect::<Result<_>>()?;
            at += size;
        }
    }
    if at - pallet_start != u32_at(bytes, b + 60)? as usize {
        return Err("pallet_size_mismatch");
    }
    let common_start = at;
    for (i, col) in columns.iter_mut().enumerate().filter(|(_, c)| c.kind == 2) {
        let size = u32_at(bytes, storage + i * 24 + 4)? as usize;
        if !size.is_multiple_of(8) {
            return Err("invalid_common_size");
        }
        range(bytes, at, size)?;
        for i in 0..size / 8 {
            if col
                .common
                .insert(u32_at(bytes, at + i * 8)?, u32_at(bytes, at + i * 8 + 4)?)
                .is_some()
            {
                return Err("duplicate_common_record");
            }
        }
        at += size;
    }
    if at - common_start != u32_at(bytes, b + 56)? as usize {
        return Err("common_size_mismatch");
    }
    // WDC4/5 carry encrypted ID lists after common/pallet data. We never recover
    // those sections; still bound their directory before accepting section offsets.
    if matches!(h.format.as_str(), "WDC4" | "WDC5") {
        for section in &sections {
            if section.key != 0 {
                let count = u32_at(bytes, at)? as usize;
                if count > MAX_RECORDS {
                    return Err("encrypted_id_capacity");
                }
                range(bytes, at + 4, count * 4)?;
                at += 4 + count * 4;
            }
        }
    }
    let metadata_end = at;
    let mut pools = Vec::new();
    let mut pool_offset = 0usize;
    let mut occupied = Vec::new();
    let mut skips = Vec::with_capacity(sections.len());
    for section in &sections {
        if section.offset < metadata_end {
            return Err("overlapping_table_sections");
        }
        if section.key != 0 {
            pool_offset = pool_offset
                .checked_add(section.strings)
                .ok_or("table_offset_overflow")?;
            skips.push(Some(Skip::Encrypted));
            continue;
        }
        let length = if sparse {
            section
                .end
                .checked_sub(section.offset)
                .ok_or("invalid_sparse_end")?
        } else {
            section
                .count
                .checked_mul(record_size)
                .ok_or("table_offset_overflow")?
        };
        let tail = section
            .offset
            .checked_add(length)
            .and_then(|n| n.checked_add(section.strings))
            .ok_or("table_offset_overflow")?;
        let extra = section
            .ids
            .checked_add(section.copies * 8)
            .and_then(|n| n.checked_add(section.sparse * 10))
            .and_then(|n| n.checked_add(section.relations))
            .ok_or("table_offset_overflow")?;
        let end = tail.checked_add(extra).ok_or("table_offset_overflow")?;
        if end > bytes.len() {
            return Err("table_offset_outside_input");
        }
        // The declared extent stays structurally checked and claimed even when its
        // bytes are unavailable, so overlap validation is unchanged.
        occupied.push((section.offset, end));
        if !bytes.available(section.offset, end) {
            pool_offset = pool_offset
                .checked_add(section.strings)
                .ok_or("table_offset_overflow")?;
            skips.push(Some(Skip::Unavailable));
            continue;
        }
        let strings = range(bytes, section.offset + length, section.strings)?;
        pools.push((pool_offset, strings));
        pool_offset += strings.len();
        skips.push(None);
    }
    occupied.sort();
    if occupied.windows(2).any(|w| w[0].1 > w[1].0) {
        return Err("overlapping_table_sections");
    }
    // The cell budget bounds what is materialized: records of skipped (encrypted
    // or unavailable) sections are never decoded and hold no cells.
    let decodable: usize = sections
        .iter()
        .zip(&skips)
        .filter(|(_, skip)| skip.is_none())
        .map(|(s, _)| s.count)
        .sum();
    if decodable
        .checked_mul(cells_per_record)
        .is_none_or(|v| v > MAX_CELLS)
    {
        return Err("decoded_cell_capacity");
    }
    let inline: Vec<_> = definition
        .layout
        .entries
        .iter()
        .filter(|e| e.inline)
        .collect();
    if inline.len() != columns.len() {
        return Err("definition_column_count_mismatch");
    }
    let id_entry = definition
        .layout
        .entries
        .iter()
        .find(|e| e.primary_key)
        .ok_or("definition_id_missing")?;
    let mut rows = BTreeMap::new();
    let mut copies = Vec::new();
    let mut global_record = 0usize;
    let mut gaps = Vec::new();
    let (mut encrypted_records, mut unavailable_records) = (None, None);
    let mut allocated_text = 0usize;
    for (section, skip) in sections.into_iter().zip(skips) {
        if let Some(skip) = skip {
            // One total per reason: equal per-section counts must not collapse.
            let total = match skip {
                Skip::Encrypted => &mut encrypted_records,
                Skip::Unavailable => &mut unavailable_records,
            };
            *total = Some(total.unwrap_or(0) + section.count);
            global_record += section.count;
            continue;
        }
        let data_length = if sparse {
            section.end - section.offset
        } else {
            section.count * record_size
        };
        let mut at = section.offset + data_length + section.strings;
        let mut ids: Vec<u32> = (0..section.ids / 4)
            .map(|i| u32_at(bytes, at + i * 4))
            .collect::<Result<_>>()?;
        at += section.ids;
        for i in 0..section.copies {
            copies.push((u32_at(bytes, at + i * 8)?, u32_at(bytes, at + i * 8 + 4)?));
        }
        at += section.copies * 8;
        let mut sparse_records = Vec::new();
        for i in 0..section.sparse {
            sparse_records.push((
                u32_at(bytes, at + i * 6)? as usize,
                u16_at(bytes, at + i * 6 + 4)? as usize,
            ));
        }
        at += section.sparse * 6;
        if flags & 2 != 0 && section.sparse > 0 {
            ids = (0..section.sparse)
                .map(|i| u32_at(bytes, at + i * 4))
                .collect::<Result<_>>()?;
            at += section.sparse * 4;
        }
        let mut relations = BTreeMap::new();
        if section.relations > 0 {
            if section.relations < 12 {
                return Err("invalid_relationship_size");
            }
            let count = u32_at(bytes, at)? as usize;
            if count > MAX_RECORDS || count * 8 + 12 != section.relations {
                return Err("invalid_relationship_count");
            }
            for i in 0..count {
                if relations
                    .insert(
                        u32_at(bytes, at + 16 + i * 8)?,
                        u32_at(bytes, at + 12 + i * 8)?,
                    )
                    .is_some()
                {
                    return Err("duplicate_relationship");
                }
            }
            at += section.relations;
        }
        if flags & 2 == 0 && section.sparse > 0 {
            ids = (0..section.sparse)
                .map(|i| u32_at(bytes, at + i * 4))
                .collect::<Result<_>>()?;
        }
        if (!id_entry.inline || sparse) && ids.len() != section.count {
            return Err("missing_noninline_ids");
        }
        if !ids.is_empty() && ids.len() != section.count {
            return Err("invalid_id_count");
        }
        if sparse && (sparse_records.len() != section.count || columns.iter().any(|c| c.kind != 0))
        {
            return Err("unsupported_sparse_storage");
        }
        let mut sparse_occupied = Vec::new();
        // Normal tables have no sparse-record vector; its length cannot drive
        // iteration. The checked section count drives both storage modes.
        #[allow(clippy::needless_range_loop)]
        for i in 0..section.count {
            let row = if sparse {
                let (offset, size) = sparse_records[i];
                if offset < section.offset
                    || offset.checked_add(size).is_none_or(|e| e > section.end)
                {
                    return Err("invalid_sparse_record");
                }
                sparse_occupied.push((offset, offset + size));
                range(bytes, offset, size)?
            } else {
                range(bytes, section.offset + i * record_size, record_size)?
            };
            let mut id = ids.get(i).copied().unwrap_or(0);
            if id_entry.inline {
                let col_index = inline
                    .iter()
                    .position(|e| e.primary_key)
                    .ok_or("inline_id_missing")?;
                if sparse && col_index != 0 {
                    return Err("unsupported_sparse_id_position");
                }
                let values = raw_values(&columns[col_index], row, 0, 1, None)?;
                let parsed = u32::try_from(values[0].0).map_err(|_| "invalid_record_id")?;
                if ids.get(i).is_some_and(|stored| *stored != parsed) {
                    return Err("record_id_mismatch");
                }
                id = parsed;
            }
            let mut values = BTreeMap::new();
            let mut cursor = 0;
            for (entry, column) in inline.iter().zip(&columns) {
                let ty = definition
                    .file
                    .columns
                    .get(&entry.name)
                    .ok_or("definition_field_missing")?
                    .ty;
                let count = entry.array_size.unwrap_or(1);
                let string = matches!(ty, RawType::String | RawType::LocString);
                // Sparse strings are variable-size and must be read before advancing.
                let raw = if sparse && string {
                    (0..count)
                        .map(|_| Ok((0, 32, cursor)))
                        .collect::<Result<Vec<_>>>()?
                } else {
                    raw_values(
                        column,
                        row,
                        id,
                        count,
                        if sparse { Some(&mut cursor) } else { None },
                    )?
                };
                let mut cells = Vec::new();
                for mut raw in raw {
                    if sparse && string {
                        raw.2 = cursor;
                    }
                    cells.push(value(
                        raw,
                        entry,
                        ty,
                        &StringContext {
                            row,
                            global_record: global_record + i,
                            total_records: h.records as usize,
                            record_size,
                            pools: &pools,
                            sparse,
                        },
                        &mut cursor,
                    )?);
                }
                let cell = if entry.array_size.is_some() {
                    Cell::Array(cells)
                } else {
                    cells.pop().ok_or("missing_field")?
                };
                allocated_text = allocated_text
                    .checked_add(text_bytes(&cell))
                    .ok_or("decoded_text_capacity")?;
                if allocated_text > MAX_TEXT_BYTES {
                    return Err("decoded_text_capacity");
                }
                if values.insert(entry.name.clone(), cell).is_some() {
                    return Err("duplicate_definition_field");
                }
            }
            for entry in definition.layout.entries.iter().filter(|e| !e.inline) {
                let cell = if entry.primary_key {
                    if entry.unsigned {
                        Cell::Unsigned(id.into())
                    } else {
                        Cell::Signed(id.into())
                    }
                } else if entry.relation {
                    let key = if flags & 2 != 0 { id } else { i as u32 };
                    relations
                        .get(&key)
                        .map(|v| Cell::Unsigned(u64::from(*v)))
                        .unwrap_or(Cell::Unknown)
                } else {
                    return Err("unsupported_noninline_field");
                };
                values.insert(entry.name.clone(), cell);
            }
            if rows
                .insert(
                    id,
                    ResearchRow {
                        id,
                        fields: values,
                        copied_from: None,
                    },
                )
                .is_some()
            {
                return Err("duplicate_record_id");
            }
        }
        sparse_occupied.sort();
        if sparse_occupied.windows(2).any(|w| w[0].1 > w[1].0) {
            return Err("overlapping_sparse_records");
        }
        global_record += section.count;
    }
    if let Some(count) = encrypted_records {
        gaps.push(format!("encrypted_section_records:{count}"));
    }
    if let Some(count) = unavailable_records {
        gaps.push(format!("unavailable_frame_section_records:{count}"));
    }
    let mut pending: BTreeMap<u32, u32> = BTreeMap::new();
    for (destination, source) in copies {
        if rows.contains_key(&destination) || pending.insert(destination, source).is_some() {
            return Err("invalid_copy_id");
        }
    }
    if rows.len() + pending.len() > MAX_RECORDS
        || (rows.len() + pending.len())
            .checked_mul(cells_per_record)
            .is_none_or(|n| n > MAX_CELLS)
    {
        return Err("decoded_cell_capacity");
    }
    let mut dependents = BTreeMap::<u32, Vec<u32>>::new();
    for (destination, source) in pending {
        dependents.entry(source).or_default().push(destination);
    }
    let mut ready: VecDeque<_> = dependents
        .keys()
        .filter(|source| rows.contains_key(source))
        .copied()
        .collect();
    while let Some(source) = ready.pop_front() {
        for destination in dependents.remove(&source).unwrap_or_default() {
            let original = rows.get(&source).ok_or("missing_copy_source")?;
            allocated_text = allocated_text
                .checked_add(original.fields.values().map(text_bytes).sum::<usize>())
                .ok_or("decoded_text_capacity")?;
            if allocated_text > MAX_TEXT_BYTES {
                return Err("decoded_text_capacity");
            }
            let mut row = rows.get(&source).unwrap().clone();
            row.id = destination;
            row.copied_from = Some(source);
            row.fields.insert(
                id_entry.name.clone(),
                if id_entry.unsigned {
                    Cell::Unsigned(destination.into())
                } else {
                    Cell::Signed(destination.into())
                },
            );
            rows.insert(destination, row);
            ready.push_back(destination);
        }
    }
    if !dependents.is_empty() {
        return Err("cyclic_or_missing_copy_source");
    }
    Ok(DecodedTable {
        format_version: 1,
        header: h,
        rows: rows.into_values().collect(),
        gaps,
    })
}
