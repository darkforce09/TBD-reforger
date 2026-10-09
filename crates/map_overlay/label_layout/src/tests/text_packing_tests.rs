//! Cases of the label glyph packing measured against the baked ASCII atlas: glyph advance,
//! atlas cell geometry, the tofu fallback, and every name in the committed Everon label data
//! drawing without it.

use crate::declutter::LabelSpec;
use crate::label_ids::LabelId;
use crate::text_packing::pack_label_glyphs;
use render_primitives::text::atlas::{
    TEXT_ATLAS_COLS, TEXT_ATLAS_ROWS, TEXT_CELL_PX, TEXT_INK_RGBA, TOFU_GLYPH,
    bake_ascii_atlas_rgba, glyph_cell_uv,
};
use render_primitives::text::metrics::TEXT_GLYPH_ADVANCE_RATIO;
use render_primitives::text::metrics::glyph_index_for_char;

fn cell_px(px: &[u8], w: u32, gi: u32, dx: u32, dy: u32) -> [u8; 4] {
    let cell = TEXT_CELL_PX;
    let (col, row) = (gi % TEXT_ATLAS_COLS, gi / TEXT_ATLAS_COLS);
    let x = col * cell + dx;
    let y = row * cell + dy;
    let i = ((y * w + x) * 4) as usize;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

#[test]
fn pack_three_digits() {
    let labels = [LabelSpec {
        id: LabelId::new(1),
        x: 100,
        y: 200,
        importance: 10,
        text: "170".into(),
    }];
    let g = pack_label_glyphs(&labels, 0.0, 10.0);
    assert_eq!(g.len(), 3);
    assert_eq!(g[0].glyph, (b'1' - 32) as u16);

    let advance = 10.0 * TEXT_GLYPH_ADVANCE_RATIO;
    assert!((g[1].x - 100.0).abs() < 1e-4);
    assert!((g[2].x - g[1].x - advance).abs() < 1e-4);
}

#[test]
fn g2_glyph_cell_uv_corners_upright() {
    let cols = TEXT_ATLAS_COLS as f32;
    let rows = TEXT_ATLAS_ROWS as f32;
    let eps = 1e-6;

    let (u, v) = glyph_cell_uv(0, 0.0, 1.0);
    assert!((u - 0.0).abs() < eps && (v - 0.0).abs() < eps);

    let (u, v) = glyph_cell_uv(0, 0.0, 0.0);
    assert!((u - 0.0).abs() < eps && (v - 1.0 / rows).abs() < eps);

    let (u, v) = glyph_cell_uv(0, 1.0, 1.0);
    assert!((u - 1.0 / cols).abs() < eps && (v - 0.0).abs() < eps);

    let (u, _) = glyph_cell_uv(23, 0.0, 0.5);
    assert!((u - 7.0 / cols).abs() < eps, "glyph 23 = col 7 left edge");
}

#[test]
fn g2_tofu_cell_is_painted_box() {
    let (px, w, _h) = bake_ascii_atlas_rgba();
    let gi = u32::from(TOFU_GLYPH);
    assert_eq!(
        cell_px(&px, w, gi, 10, 4),
        TEXT_INK_RGBA,
        "tofu top-left stroke"
    );
    assert_eq!(
        cell_px(&px, w, gi, 21, 27),
        TEXT_INK_RGBA,
        "tofu bottom-right stroke"
    );
    assert_eq!(
        cell_px(&px, w, gi, 16, 16),
        [0, 0, 0, 0],
        "tofu interior hollow (≥3 px from strokes — outside the halo)"
    );
}

#[test]
fn g3_fold_and_tofu_mapping() {
    assert_eq!(glyph_index_for_char('é'), u16::from(b'e' - 32));
    assert_eq!(glyph_index_for_char('Ü'), u16::from(b'U' - 32));
    assert_eq!(glyph_index_for_char('-'), u16::from(b'-' - 32));
    assert_eq!(glyph_index_for_char('日'), TOFU_GLYPH);
}
