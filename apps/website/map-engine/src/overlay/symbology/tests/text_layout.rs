//! Role: Domain regression cases for cartographic text layout.
//! Position: `overlay/symbology/tests` in the map engine.
//! Signals & state: glyph packing, atlas cell geometry, and committed label data.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.
//!
//! T-0xx Phase 2B.1: this is `renderers/text/layout/tests/{mod,cases_1}.rs` merged. The
//! `layout/mod.rs` that sat above them was a pure `pub use` hub with no consumer anywhere
//! outside this file, so it is gone rather than moved and the names below are imported where
//! they actually live. The three shader-scrub cases left for `website-graphics-engine` in the
//! Kind A commit.

use crate::overlay::symbology::labels::declutter::LabelSpec;
use crate::overlay::symbology::text_metrics::TEXT_GLYPH_ADVANCE_RATIO;
use crate::overlay::symbology::text_metrics::atlas::{
    TEXT_ATLAS_COLS, TEXT_ATLAS_ROWS, TEXT_CELL_PX, TEXT_HALO_RGBA, TEXT_INK_RGBA, TOFU_GLYPH,
    bake_ascii_atlas_rgba, glyph_cell_uv,
};
use crate::overlay::symbology::text_metrics::font::FONT_16X32;
use crate::overlay::symbology::text_metrics::glyph_index_for_char;
use crate::overlay::symbology::text_packing::pack_label_glyphs;

fn cell_px(px: &[u8], w: u32, gi: u32, dx: u32, dy: u32) -> [u8; 4] {
    let cell = TEXT_CELL_PX;
    let (col, row) = (gi % TEXT_ATLAS_COLS, gi / TEXT_ATLAS_COLS);
    let x = col * cell + dx;
    let y = row * cell + dy;
    let i = ((y * w + x) * 4) as usize;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

#[test]
fn pack_empty() {
    assert!(pack_label_glyphs(&[], 0.0, 10.0).is_empty());
}

#[test]
fn pack_three_digits() {
    let labels = [LabelSpec {
        id: 1,
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

// T-0xx Phase 1D: `width_declutter_drops_overlapping_long_names` moved to
// `website-graphics-engine` with `declutter_specs_by_width` — a width overlap between two
// boxes is geometry, and the `LabelSpec` importance column it was written against plays no
// part in it. See `graphics-engine/src/text/tests/layout_tests.rs`.

#[test]
fn atlas_size() {
    let (px, w, h) = bake_ascii_atlas_rgba();
    assert_eq!(w, TEXT_ATLAS_COLS * TEXT_CELL_PX);
    assert_eq!(h, TEXT_ATLAS_ROWS * TEXT_CELL_PX);
    assert_eq!((w, h), (512, 192));
    assert_eq!(px.len(), (w * h * 4) as usize);
    assert!(px.iter().any(|&c| c > 0));
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
fn g2_seven_is_top_heavy_in_atlas() {
    let (px, w, _h) = bake_ascii_atlas_rgba();
    assert_eq!(
        cell_px(&px, w, 23, 11, 6),
        TEXT_INK_RGBA,
        "top bar ink (ink col 3, dy=6)"
    );
    assert_eq!(
        cell_px(&px, w, 23, 14, 18),
        TEXT_INK_RGBA,
        "descender ink (ink col 6, dy=18)"
    );
    assert_eq!(
        cell_px(&px, w, 23, 17, 18),
        TEXT_HALO_RGBA,
        "U-mirror trap: cell x=17 at dy=18 is halo upright — a mirrored '7' puts ink here"
    );
    assert_eq!(
        cell_px(&px, w, 23, 11, 25),
        [0, 0, 0, 0],
        "V-flip trap: (11,25) is ≥3 px from all ink — a flip lands the top bar here"
    );
}

#[test]
fn g2_full_ascii_coverage_distinct_lowercase() {
    let tofu_idx = usize::from(TOFU_GLYPH);

    for c in 33..=126u8 {
        let rows = &FONT_16X32[usize::from(c - 32)];
        assert!(
            rows.iter().any(|&r| r != 0),
            "glyph '{}' (0x{c:02x}) has no ink",
            c as char
        );
    }
    assert!(
        FONT_16X32[tofu_idx].iter().all(|&r| r == 0),
        "table tofu slot must stay zeroed (baker-drawn)"
    );
    for c in b'a'..=b'z' {
        let lower = &FONT_16X32[usize::from(c - 32)];
        let upper = &FONT_16X32[usize::from(c.to_ascii_uppercase() - 32)];
        assert_ne!(
            lower, upper,
            "lowercase '{}' must not reuse the uppercase raster",
            c as char
        );
    }
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
fn g3_committed_label_data_no_tofu() {
    const LOCATIONS: &str =
        include_str!("../../../../../../../assets_v2/terrains/everon/locations.json");
    const ROAD_NAMES: &str =
        include_str!("../../../../../../../assets_v2/terrains/everon/road-names.json");

    fn collect_names(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::Array(a) => a.iter().for_each(|x| collect_names(x, out)),
            serde_json::Value::Object(o) => {
                if let Some(serde_json::Value::String(s)) = o.get("name") {
                    out.push(s.clone());
                }
                o.values().for_each(|x| collect_names(x, out));
            }
            _ => {}
        }
    }

    let mut names = Vec::new();
    for src in [LOCATIONS, ROAD_NAMES] {
        let v: serde_json::Value = serde_json::from_str(src).expect("valid label JSON");
        collect_names(&v, &mut names);
    }
    assert!(!names.is_empty(), "committed label data yields names");
    names.push("0123456789".into());

    let mut offenders = Vec::new();
    for name in &names {
        for ch in name.chars() {
            if glyph_index_for_char(ch) == TOFU_GLYPH {
                offenders.push(format!("{ch:?} in {name:?}"));
            }
        }
    }
    assert!(offenders.is_empty(), "tofu fallback hits: {offenders:?}");
}

#[test]
fn g3_fold_and_tofu_mapping() {
    assert_eq!(glyph_index_for_char('é'), u16::from(b'e' - 32));
    assert_eq!(glyph_index_for_char('Ü'), u16::from(b'U' - 32));
    assert_eq!(glyph_index_for_char('-'), u16::from(b'-' - 32));
    assert_eq!(glyph_index_for_char('日'), TOFU_GLYPH);
}
