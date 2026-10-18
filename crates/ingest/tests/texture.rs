use azeroth_ingest::texture::{Refusal, decode};
fn fixture(encoding: u8, alpha: u8, mode: u8, width: u32, height: u32, data: &[u8]) -> Vec<u8> {
    let header = if encoding == 1 { 1172 } else { 148 };
    let mut b = vec![0; header];
    b[..4].copy_from_slice(b"BLP2");
    b[4] = 1;
    b[8] = encoding;
    b[9] = alpha;
    b[10] = mode;
    b[12..16].copy_from_slice(&width.to_le_bytes());
    b[16..20].copy_from_slice(&height.to_le_bytes());
    b[20..24].copy_from_slice(&(header as u32).to_le_bytes());
    b[84..88].copy_from_slice(&(data.len() as u32).to_le_bytes());
    b.extend(data);
    b
}
#[test]
fn palette_alpha_bit_order_and_bgra_are_exact() {
    for (alpha, data, expected) in [
        (0, vec![0, 0], vec![255, 255]),
        (1, vec![0, 0, 1], vec![255, 0]),
        (4, vec![0, 0, 0x3f], vec![255, 51]),
        (8, vec![0, 0, 0, 127], vec![0, 127]),
    ] {
        let mut b = fixture(1, alpha, 0, 2, 1, &data);
        b[148..152].copy_from_slice(&[1, 2, 3, 99]);
        let t = decode(&b).unwrap();
        assert_eq!(t.rgba, vec![3, 2, 1, expected[0], 3, 2, 1, expected[1]]);
    }
    assert_eq!(
        decode(&fixture(3, 8, 0, 1, 1, &[1, 2, 3, 0])).unwrap().rgba,
        [3, 2, 1, 0]
    );
}
#[test]
fn bc1_transparency_and_bc2_bc3_alpha_are_decoded() {
    let b = fixture(2, 1, 0, 1, 1, &[0, 0, 255, 255, 3, 0, 0, 0]);
    assert_eq!(decode(&b).unwrap().rgba, [0, 0, 0, 0]);
    assert_eq!(
        decode(&fixture(2, 0, 0, 1, 1, &[0, 0, 255, 255, 3, 0, 0, 0]))
            .unwrap()
            .rgba,
        [0, 0, 0, 255]
    );
    let mut block = vec![0; 16];
    block[0] = 15;
    block[8..12].copy_from_slice(&[0, 248, 0, 0]);
    assert_eq!(
        decode(&fixture(2, 4, 1, 1, 1, &block)).unwrap().rgba,
        [255, 0, 0, 255]
    );
    block[0] = 120;
    block[1] = 60;
    block[2] = 2;
    assert_eq!(
        decode(&fixture(2, 8, 7, 1, 1, &block)).unwrap().rgba,
        [255, 0, 0, 111]
    );
}
#[test]
fn ranges_dimensions_truncation_and_unsupported_are_distinct() {
    let b = fixture(3, 8, 0, 1, 1, &[1, 2, 3, 4]);
    for end in 4..b.len() {
        assert!(decode(&b[..end]).is_err());
    }
    let mut bad = b.clone();
    bad[20..24].copy_from_slice(&20u32.to_le_bytes());
    assert_eq!(decode(&bad), Err(Refusal::Invalid));
    bad = b.clone();
    bad[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(decode(&bad), Err(Refusal::Invalid));
    bad = b.clone();
    bad[12..16].copy_from_slice(&8192u32.to_le_bytes());
    bad[16..20].copy_from_slice(&8192u32.to_le_bytes());
    assert_eq!(decode(&bad), Err(Refusal::Capacity));
    assert_eq!(decode(b"BLP1"), Err(Refusal::Unsupported));
    assert_eq!(
        decode(&fixture(7, 8, 0, 1, 1, &[0])),
        Err(Refusal::Unsupported)
    );
}
#[test]
fn opaque_bc1_keeps_three_color_mode_for_equal_and_reversed_endpoints() {
    for endpoints in [[0, 0, 255, 255], [0, 248, 0, 248]] {
        let mut data = endpoints.to_vec();
        data.extend([14, 0, 0, 0]);
        let t = decode(&fixture(2, 0, 0, 2, 1, &data)).unwrap();
        assert_eq!(&t.rgba[4..8], &[0, 0, 0, 255]);
        assert_eq!(
            &t.rgba[..4],
            if endpoints[0] == 0 && endpoints[1] == 0 {
                &[127, 127, 127, 255]
            } else {
                &[255, 0, 0, 255]
            }
        );
    }
}
#[test]
fn mip_payloads_must_be_complete_nonoverlapping_and_dimension_bound() {
    let mut b = fixture(3, 8, 0, 2, 2, &[0; 16]);
    b[11] = 1;
    let next = b.len() as u32;
    b[24..28].copy_from_slice(&next.to_le_bytes());
    b[88..92].copy_from_slice(&4u32.to_le_bytes());
    b.extend([1, 2, 3, 4]);
    assert!(decode(&b).is_ok());
    let mut bad = b.clone();
    bad[88..92].copy_from_slice(&3u32.to_le_bytes());
    assert_eq!(decode(&bad), Err(Refusal::Invalid));
    bad = b.clone();
    bad[24..28].copy_from_slice(&148u32.to_le_bytes());
    assert_eq!(decode(&bad), Err(Refusal::Invalid));
    bad = b.clone();
    bad[11] = 0;
    assert_eq!(decode(&bad), Err(Refusal::Invalid));
    bad = b.clone();
    bad[28..32].copy_from_slice(&next.to_le_bytes());
    bad[92..96].copy_from_slice(&4u32.to_le_bytes());
    assert_eq!(decode(&bad), Err(Refusal::Invalid));
}
#[test]
fn block_rows_and_bc3_implicit_alpha_extremes_have_correct_positions() {
    let red = [0, 248, 0, 0, 0, 0, 0, 0];
    let blue = [31, 0, 0, 0, 0, 0, 0, 0];
    let mut data = red.to_vec();
    data.extend(blue);
    let t = decode(&fixture(2, 0, 0, 4, 8, &data)).unwrap();
    assert_eq!(&t.rgba[..4], &[255, 0, 0, 255]);
    assert_eq!(&t.rgba[4 * 4 * 4..4 * 4 * 4 + 4], &[0, 0, 255, 255]);
    let mut block = vec![0; 16];
    block[0] = 10;
    block[1] = 20;
    block[2] = 62;
    block[8..16].copy_from_slice(&red);
    let t = decode(&fixture(2, 8, 7, 2, 1, &block)).unwrap();
    assert_eq!(t.rgba[3], 0);
    assert_eq!(t.rgba[7], 255);
}
