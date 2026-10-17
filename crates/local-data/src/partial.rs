//! Frame-aware partial logical files (task V2.2d).
//!
//! A DB2 file whose later BLTE frames are encrypted with keys that are not
//! available locally is kept as its verified plain frames only. Encrypted frames
//! are never decoded, decrypted, zero-filled or synthesized: their byte ranges
//! are recorded as explicit gaps and hold no bytes at all. The decoder sees the
//! file through [`Input`], which refuses every access outside the plain ranges.
use crate::{
    metadata::Definition,
    tables::{self, DecodedTable, Input, TableHeader},
};

/// Upper bound on BLTE frames per logical file accepted for a partial read.
pub const MAX_FRAMES: usize = 1 << 20;
const RETAINED_MAGIC: &[u8] = b"NAVZEROTH-PARTIAL-ORIGINAL-V1\0";

/// One BLTE frame's logical byte range and whether its verified encoded bytes
/// are BLTE mode `E`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameLayout {
    pub start: u64,
    pub length: u32,
    pub encrypted: bool,
}

/// A maximal run of consecutive encrypted frames: frames
/// `first_frame..first_frame + frames`, logical bytes `offset..offset + length`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnavailableRange {
    pub first_frame: u32,
    pub frames: u32,
    pub offset: usize,
    pub length: usize,
}
impl UnavailableRange {
    /// Explicit gap naming the frames and the logical byte range they cover.
    pub fn gap(&self) -> String {
        format!(
            "unavailable_encrypted_frames:{}-{}:bytes:{}+{}",
            self.first_frame,
            self.first_frame + self.frames - 1,
            self.offset,
            self.length
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AssembleError<E> {
    /// The frame layout is not a contiguous, bounded cover of the logical file,
    /// has no plain or no encrypted frame, or a plain run read back short.
    Layout(&'static str),
    /// Reading a plain run failed; the cause is passed through unchanged.
    Read(E),
}

/// Verified plain ranges of one logical file plus its explicit encrypted gaps.
#[derive(Debug)]
pub struct PartialFile {
    length: usize,
    /// Merged runs of consecutive plain frames, in order, never adjacent.
    segments: Vec<(usize, Vec<u8>)>,
    unavailable: Vec<UnavailableRange>,
}

impl PartialFile {
    /// Build from a frame layout. `read(offset, length)` must strictly read and
    /// verify exactly that run of plain frames; it is never called for an
    /// encrypted frame.
    pub fn assemble<E>(
        length: usize,
        frames: &[FrameLayout],
        mut read: impl FnMut(usize, usize) -> Result<Vec<u8>, E>,
    ) -> Result<Self, AssembleError<E>> {
        use AssembleError::Layout;
        if frames.is_empty() || frames.len() > MAX_FRAMES {
            return Err(Layout("frame_count_outside_bounds"));
        }
        // (encrypted, first frame, frame count, offset, length)
        let mut runs: Vec<(bool, u32, u32, usize, usize)> = Vec::new();
        let mut at = 0usize;
        for (index, frame) in frames.iter().enumerate() {
            let start =
                usize::try_from(frame.start).map_err(|_| Layout("frame_offset_overflow"))?;
            let size = frame.length as usize;
            if start != at {
                return Err(Layout("noncontiguous_frames"));
            }
            if size == 0 {
                return Err(Layout("empty_frame"));
            }
            at = at
                .checked_add(size)
                .filter(|end| *end <= length)
                .ok_or(Layout("frames_exceed_logical_length"))?;
            match runs.last_mut() {
                Some(run) if run.0 == frame.encrypted => {
                    run.2 += 1;
                    run.4 += size;
                }
                _ => runs.push((frame.encrypted, index as u32, 1, start, size)),
            }
        }
        if at != length {
            return Err(Layout("frames_do_not_cover_logical_length"));
        }
        if !runs.iter().any(|r| r.0) {
            return Err(Layout("no_encrypted_frames"));
        }
        if !runs.iter().any(|r| !r.0) {
            return Err(Layout("no_plain_frames"));
        }
        let mut segments = Vec::new();
        let mut unavailable = Vec::new();
        for (encrypted, first_frame, count, offset, size) in runs {
            if encrypted {
                unavailable.push(UnavailableRange {
                    first_frame,
                    frames: count,
                    offset,
                    length: size,
                });
            } else {
                let bytes = read(offset, size).map_err(AssembleError::Read)?;
                if bytes.len() != size {
                    return Err(Layout("plain_run_length_mismatch"));
                }
                segments.push((offset, bytes));
            }
        }
        Ok(Self {
            length,
            segments,
            unavailable,
        })
    }
    /// Declared logical length, including unavailable ranges.
    pub fn len(&self) -> usize {
        self.length
    }
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
    pub fn unavailable(&self) -> &[UnavailableRange] {
        &self.unavailable
    }
    pub fn plain_bytes(&self) -> usize {
        self.segments.iter().map(|(_, b)| b.len()).sum()
    }
    pub fn input(&self) -> Input<'_> {
        Input::partial(
            self.length,
            self.segments
                .iter()
                .map(|(o, b)| (*o, b.as_slice()))
                .collect(),
        )
        .expect("assembled segments are sorted, non-empty and inside the length")
    }
    pub fn header(&self) -> Result<TableHeader, &'static str> {
        tables::header_of(&self.input())
    }
    /// Decode the plain sections. The table's gaps name every unavailable range
    /// and every section that was not decoded.
    pub fn decode(&self, definition: &Definition) -> Result<DecodedTable, &'static str> {
        let mut decoded = tables::decode_input(&self.input(), definition)?;
        decoded
            .gaps
            .extend(self.unavailable.iter().map(UnavailableRange::gap));
        decoded.gaps.sort();
        Ok(decoded)
    }
    /// Deterministic private retained form: the logical length, every range with
    /// its state and frames, then the plain bytes only. No encrypted byte is
    /// present, and nothing marks a filler.
    pub fn retained_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(RETAINED_MAGIC.len() + 64 + self.plain_bytes());
        out.extend_from_slice(RETAINED_MAGIC);
        out.extend_from_slice(&(self.length as u64).to_le_bytes());
        let mut ranges: Vec<(usize, usize, u8, u32, u32)> = self
            .unavailable
            .iter()
            .map(|r| (r.offset, r.length, 2, r.first_frame, r.frames))
            .collect();
        ranges.extend(
            self.segments
                .iter()
                .map(|(o, b)| (*o, b.len(), 1, u32::MAX, 0)),
        );
        ranges.sort();
        out.extend_from_slice(&(ranges.len() as u32).to_le_bytes());
        for (offset, length, state, first_frame, frames) in ranges {
            // State 1: verified plain bytes follow below. State 2: unavailable
            // encrypted frames, no bytes. Plain runs carry no frame numbers.
            out.push(state);
            out.extend_from_slice(&(offset as u64).to_le_bytes());
            out.extend_from_slice(&(length as u64).to_le_bytes());
            out.extend_from_slice(&first_frame.to_le_bytes());
            out.extend_from_slice(&frames.to_le_bytes());
        }
        for (_, bytes) in &self.segments {
            out.extend_from_slice(bytes);
        }
        out
    }
}
