//! Bounded BLP2 top-mip decoding. Original implementation, no game data retained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Unsupported,
    Invalid,
    Capacity,
}
const MAX_PIXELS: usize = 4_194_304;
const MAX_BYTES: usize = 32 * 1024 * 1024;
fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, Refusal> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or(Refusal::Invalid)?
            .try_into()
            .unwrap(),
    ))
}
fn rgb565(v: u16) -> [u8; 4] {
    let r = (v >> 11) as u8;
    let g = ((v >> 5) & 63) as u8;
    let b = (v & 31) as u8;
    [
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
        255,
    ]
}
fn colors(block: &[u8], force_four: bool) -> [[u8; 4]; 16] {
    let a = u16::from_le_bytes([block[0], block[1]]);
    let b = u16::from_le_bytes([block[2], block[3]]);
    let mut palette = [rgb565(a), rgb565(b), [0; 4], [0; 4]];
    let first = palette[0];
    let second = palette[1];
    if a > b || force_four {
        for (c, (left, right)) in first[..3].iter().zip(&second[..3]).enumerate() {
            palette[2][c] = ((2 * *left as u16 + *right as u16) / 3) as u8;
            palette[3][c] = ((*left as u16 + 2 * *right as u16) / 3) as u8;
        }
        palette[2][3] = 255;
        palette[3][3] = 255;
    } else {
        for (target, (left, right)) in palette[2][..3]
            .iter_mut()
            .zip(first[..3].iter().zip(&second[..3]))
        {
            *target = ((*left as u16 + *right as u16) / 2) as u8;
        }
        palette[2][3] = 255;
    }
    let indices = u32::from_le_bytes(block[4..8].try_into().unwrap());
    std::array::from_fn(|i| palette[((indices >> (2 * i)) & 3) as usize])
}
/// Supports BLP2 paletted alpha 0/1/4/8, BGRA8 and BC1/BC2/BC3 (DXT1/3/5).
/// JPEG, BLP1 and unknown encodings are explicit unsupported formats.
pub fn decode(bytes: &[u8]) -> Result<Texture, Refusal> {
    if bytes.len() > MAX_BYTES {
        return Err(Refusal::Capacity);
    }
    if bytes.get(..4) != Some(b"BLP2") {
        return Err(Refusal::Unsupported);
    }
    if bytes.len() < 148 {
        return Err(Refusal::Invalid);
    }
    if u32_at(bytes, 4)? != 1 {
        return Err(Refusal::Unsupported);
    }
    let encoding = bytes[8];
    let alpha = bytes[9];
    let alpha_encoding = bytes[10];
    if bytes[11] > 1 {
        return Err(Refusal::Invalid);
    }
    let width = u32_at(bytes, 12)?;
    let height = u32_at(bytes, 16)?;
    if width == 0 || height == 0 || !width.is_power_of_two() || !height.is_power_of_two() {
        return Err(Refusal::Invalid);
    }
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .filter(|p| *p <= MAX_PIXELS)
        .ok_or(Refusal::Capacity)?;
    let header = if encoding == 1 { 1172 } else { 148 };
    if bytes.len() < header {
        return Err(Refusal::Invalid);
    }
    let mut ranges = Vec::new();
    let mut seen_gap = false;
    for i in 0..16 {
        let offset = u32_at(bytes, 20 + 4 * i)? as usize;
        let length = u32_at(bytes, 84 + 4 * i)? as usize;
        if offset == 0 && length == 0 {
            seen_gap = true;
            continue;
        }
        if seen_gap
            || offset < header
            || length == 0
            || offset
                .checked_add(length)
                .is_none_or(|end| end > bytes.len())
        {
            return Err(Refusal::Invalid);
        }
        if i > width.max(height).ilog2() as usize || (bytes[11] == 0 && i > 0) {
            return Err(Refusal::Invalid);
        }
        if ranges
            .iter()
            .any(|(start, end)| offset < *end && offset + length > *start)
        {
            return Err(Refusal::Invalid);
        }
        let mip_width = ((width as usize) >> i).max(1);
        let mip_height = ((height as usize) >> i).max(1);
        let mip_pixels = mip_width * mip_height;
        let expected = match encoding {
            1 if matches!(alpha, 0 | 1 | 4 | 8) => {
                mip_pixels + (mip_pixels * alpha as usize).div_ceil(8)
            }
            3 if alpha == 8 => mip_pixels * 4,
            2 => {
                let size = match (alpha, alpha_encoding) {
                    (0 | 1, 0) => 8,
                    (4 | 8, 1 | 7) => 16,
                    _ => return Err(Refusal::Unsupported),
                };
                mip_width.div_ceil(4) * mip_height.div_ceil(4) * size
            }
            _ => return Err(Refusal::Unsupported),
        };
        if length != expected {
            return Err(Refusal::Invalid);
        }
        ranges.push((offset, offset + length));
    }
    let &(start, end) = ranges.first().ok_or(Refusal::Invalid)?;
    let data = &bytes[start..end];
    let mut rgba = vec![0; pixels * 4];
    match encoding {
        1 if matches!(alpha, 0 | 1 | 4 | 8) => {
            let alpha_len = (pixels * alpha as usize).div_ceil(8);
            if data.len() != pixels + alpha_len {
                return Err(Refusal::Invalid);
            }
            for i in 0..pixels {
                let palette = 148 + data[i] as usize * 4;
                rgba[i * 4..i * 4 + 3].copy_from_slice(&[
                    bytes[palette + 2],
                    bytes[palette + 1],
                    bytes[palette],
                ]);
                rgba[i * 4 + 3] = match alpha {
                    0 => 255,
                    1 => ((data[pixels + i / 8] >> (i % 8)) & 1) * 255,
                    4 => ((data[pixels + i / 2] >> (4 * (i % 2))) & 15) * 17,
                    8 => data[pixels + i],
                    _ => unreachable!(),
                };
            }
        }
        3 if alpha == 8 => {
            if data.len() != pixels * 4 {
                return Err(Refusal::Invalid);
            }
            for (dst, src) in rgba
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(data.as_chunks::<4>().0)
            {
                dst.copy_from_slice(&[src[2], src[1], src[0], src[3]]);
            }
        }
        2 => {
            let block_size = match (alpha, alpha_encoding) {
                (0 | 1, 0) => 8,
                (4 | 8, 1 | 7) => 16,
                _ => return Err(Refusal::Unsupported),
            };
            let blocks_x = (width as usize).div_ceil(4);
            let blocks_y = (height as usize).div_ceil(4);
            if data.len() != blocks_x * blocks_y * block_size {
                return Err(Refusal::Invalid);
            }
            for (index, block) in data.chunks_exact(block_size).enumerate() {
                let mut decoded = colors(&block[block_size - 8..], block_size == 16);
                if block_size == 8 && alpha == 0 {
                    for pixel in &mut decoded {
                        pixel[3] = 255;
                    }
                }
                if block_size == 16 && alpha_encoding == 1 {
                    let bits = u64::from_le_bytes(block[..8].try_into().unwrap());
                    for (i, p) in decoded.iter_mut().enumerate() {
                        p[3] = ((bits >> (i * 4)) & 15) as u8 * 17;
                    }
                } else if block_size == 16 {
                    let mut palette = [0u8; 8];
                    palette[0] = block[0];
                    palette[1] = block[1];
                    if palette[0] > palette[1] {
                        for i in 1..=6 {
                            palette[i + 1] = (((7 - i) as u16 * palette[0] as u16
                                + i as u16 * palette[1] as u16)
                                / 7) as u8;
                        }
                    } else {
                        for i in 1..=4 {
                            palette[i + 1] = (((5 - i) as u16 * palette[0] as u16
                                + i as u16 * palette[1] as u16)
                                / 5) as u8;
                        }
                        palette[6] = 0;
                        palette[7] = 255;
                    }
                    let mut bits = 0u64;
                    for (i, b) in block[2..8].iter().enumerate() {
                        bits |= (*b as u64) << (8 * i);
                    }
                    for (i, p) in decoded.iter_mut().enumerate() {
                        p[3] = palette[((bits >> (3 * i)) & 7) as usize];
                    }
                }
                for (i, p) in decoded.iter().enumerate() {
                    let x = index % blocks_x * 4 + i % 4;
                    let y = index / blocks_x * 4 + i / 4;
                    if x < width as usize && y < height as usize {
                        rgba[(y * width as usize + x) * 4..(y * width as usize + x + 1) * 4]
                            .copy_from_slice(p);
                    }
                }
            }
        }
        _ => return Err(Refusal::Unsupported),
    }
    Ok(Texture {
        width,
        height,
        rgba,
    })
}
