//! Role: the 256-byte row padding and the upload-ready texture payload.
//! Position: `editing/tools/line_of_sight/tests` in the map engine.
//! Signals & state: explicit profiles, shots and rasters built in the test body.
//! Invariants: an already-aligned width is copied through unpadded; every other width is right-padded with transparent bytes.

use super::super::wash_palette::*;
use super::*;

use crate::spatial::los::terrain::viewshed::Viewshed;
use crate::spatial::los::terrain::viewshed::Visibility;

// ── T-644 — the engine texture payload (256-row-pad) ─────────────────────────────────────────

/// A width whose byte-row is ALREADY 256-aligned is copied through unchanged (no padding).
/// 64 cols × 4 = 256 bytes → stride 256, length == tight length.
#[test]
fn pack_rgba_256_aligned_width_is_unpadded() {
    let cols = 64;
    let rows = 3;
    let tight = vec![7u8; cols * 4 * rows];
    let (padded, stride) = pack_rgba_256(&tight, cols, rows);
    assert_eq!(stride, 256, "64*4 is exactly 256");
    assert_eq!(padded.len(), tight.len(), "no padding added");
    assert_eq!(padded, tight);
}

/// A NON-aligned width is right-padded per row to the next 256 multiple, and the original bytes
/// land at the row starts (the pad is trailing zero texels). 51 cols × 4 = 204 → stride 256.
#[test]
fn pack_rgba_256_pads_each_row_to_stride() {
    let cols = 51; // 51*4 = 204, not a multiple of 256
    let rows = 2;
    // Distinct per-cell bytes so we can prove the row copy is correct.
    let mut tight = Vec::with_capacity(cols * 4 * rows);
    for i in 0..(cols * rows) {
        let b = (i % 251) as u8;
        tight.extend_from_slice(&[b, b, b, 255]);
    }
    let (padded, stride_u32) = pack_rgba_256(&tight, cols, rows);
    let stride = stride_u32 as usize;
    assert_eq!(stride, 256, "204 rounds up to 256");
    assert_eq!(padded.len(), 256 * rows, "stride * rows");
    // Row 0 and row 1 original bytes are at the row starts; the tail [204..256] is zero.
    for r in 0..rows {
        let row_bytes = cols * 4;
        assert_eq!(
            &padded[r * stride..r * stride + row_bytes],
            &tight[r * row_bytes..(r + 1) * row_bytes],
            "row {r} payload preserved at the row start"
        );
        assert!(
            padded[r * stride + row_bytes..(r + 1) * stride]
                .iter()
                .all(|&b| b == 0),
            "row {r} pad is zero (transparent) texels"
        );
    }
}

/// `viewshed_texture_payload` carries the raster's world rect + dims and a 256-aligned stride,
/// with the palette-encoded bytes — the whole hand-off the host gives `engine.viewshed_upload`.
#[test]
fn viewshed_texture_payload_is_engine_ready() {
    // 2×2 raster, world rect [0,0]..[8,8], one hidden cell.
    let vs = Viewshed {
        cols: 2,
        rows: 2,
        cells: vec![
            Visibility::Visible,
            Visibility::Hidden,
            Visibility::Visible,
            Visibility::Unknown,
        ],
        min_x: 0.0,
        min_y: 0.0,
        max_x: 8.0,
        max_y: 8.0,
        obs_x: 4.0,
        obs_y: 4.0,
    };
    let tex = viewshed_texture_payload(&vs);
    assert_eq!((tex.tex_w, tex.tex_h), (2, 2));
    assert_eq!(
        (tex.min_x, tex.min_y, tex.max_x, tex.max_y),
        (0.0, 0.0, 8.0, 8.0)
    );
    // 2*4 = 8 bytes/row → padded to 256.
    assert_eq!(tex.stride_bytes, 256);
    assert_eq!(tex.rgba.len(), 256 * 2, "stride * rows");
    // The engine's length invariant: rgba.len() == stride * tex_h.
    assert_eq!(
        tex.rgba.len(),
        tex.stride_bytes as usize * tex.tex_h as usize
    );
    // North-first rows (wave-110 BLOCKER-1 fix): texture row 0 = world row 1 (V U).
    assert_eq!(
        &tex.rgba[0..4],
        &VIEWSHED_VISIBLE_RGBA,
        "tex row 0 = world north row"
    );
    assert_eq!(&tex.rgba[4..8], &VIEWSHED_UNKNOWN_RGBA);
    // World row 0 (V H) lands in texture row 1, after the 256-byte stride.
    assert_eq!(&tex.rgba[256..260], &VIEWSHED_VISIBLE_RGBA);
    assert_eq!(
        &tex.rgba[260..264],
        &VIEWSHED_HIDDEN_RGBA,
        "world SE hidden in tex row 1"
    );
}
