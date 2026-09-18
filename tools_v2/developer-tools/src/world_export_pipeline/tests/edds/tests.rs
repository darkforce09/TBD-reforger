use super::*;

/// The vendored bc7.test.mjs golden, ported verbatim: a varied 4×4 BC7 block from
/// Eden_1174 mip0; EXPECT_RGB produced by an INDEPENDENT decoder (Pillow) — guards the
/// decoder against wrong layout, not a circular self-check.
#[test]
fn bc7_decodes_known_block() {
    let block: Vec<u8> = (0..16)
        .map(|i| {
            u8::from_str_radix(&"c05ae575293dfeff0d726b56cf79ef7b"[i * 2..i * 2 + 2], 16).unwrap()
        })
        .collect();
    #[rustfmt::skip]
    let expect_rgb: [u8; 48] = [
        81, 76, 57, 107, 95, 75, 98, 88, 69, 77, 73, 54,
        60, 60, 43, 81, 76, 57, 81, 76, 57, 86, 79, 61,
        43, 47, 31, 56, 57, 40, 69, 67, 49, 77, 73, 54,
        43, 47, 31, 47, 50, 34, 60, 60, 43, 77, 73, 54,
    ];
    let rgba = decode_bc7(&block, 4, 4).unwrap();
    assert_eq!(rgba.len(), 64, "4x4 RGBA = 64 bytes");
    let rgb: Vec<u8> = (0..16)
        .flat_map(|i| rgba[i * 4..i * 4 + 3].to_vec())
        .collect();
    assert_eq!(rgb, expect_rgb);
}

#[test]
fn bc7_rejects_bad_dims() {
    assert!(decode_bc7(&[0u8; 16], 3, 4).is_err());
}

#[test]
fn lz4_roundtrip_simple() {
    // literal-only block: token 0x50 (5 literals, no match at end)
    let src = [0x50, b'h', b'e', b'l', b'l', b'o'];
    assert_eq!(lz4_block(&src, 5).unwrap(), b"hello");
}
