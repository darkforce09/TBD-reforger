//! Procedural text layout helpers (T-152.1 / T-152.7 / T-152.13) — pack label strings into
//! world-space glyph quads. Pure data; GPU upload lives in `map-engine-render` (wasm32).

use map_engine_core::dem::peaks::{HeightLabel, declutter_height_labels, height_label_min_sep_m};
use map_engine_core::label::LabelSpec;
use map_engine_core::world::{
    LocationLabel, RoadLabelPlacement, declutter_town_labels, locations_to_label_specs,
};
use map_engine_core::world::{REF_ZOOM, pack_icon_instance, pack_rgba_u32, size_with_min_px};

use crate::text_font_table::{FONT_16X32, FONT_GLYPH_W};

// ── Atlas geometry (T-152.13) — the single source of truth for bake, CPU UV oracle, and the
// shader (threaded through the `TextUniforms.grid_cols/grid_rows` fields at atlas upload, so
// `vs_text` carries no hardcoded grid dims).
/// Atlas grid columns (glyph index → cell: `col = glyph % COLS`).
pub const TEXT_ATLAS_COLS: u32 = 16;
/// Atlas grid rows (96 cells for ASCII 32..=126 + tofu).
pub const TEXT_ATLAS_ROWS: u32 = 6;
/// Square atlas cell edge in pixels (glyph ink is 16×32, x-centered).
pub const TEXT_CELL_PX: u32 = 32;
/// Horizontal pen advance as a fraction of the cell: ink is half the cell wide.
pub const TEXT_GLYPH_ADVANCE_RATIO: f32 = 0.5;
/// Fallback cell — a visually-obvious `□`; committed label data must never hit it (G3).
pub const TOFU_GLYPH: u16 = 95;

/// One textured glyph instance in world meters (center of character cell).
#[derive(Clone, Debug)]
pub struct TextGlyphInstance {
    pub x: f32,
    pub y: f32,
    /// Cell half-extent (meters).
    pub half_m: f32,
    /// Atlas cell index: 0..=94 for printable ASCII 32..=126, 95 = tofu fallback.
    pub glyph: u16,
}

/// World meters per character cell at `deck_zoom` (16 px @ REF_ZOOM, min 14 px — T-152.13
/// readability floor; the old 6 px clamp rendered microscopic strokes at editor zooms).
#[must_use]
pub fn text_char_meters(deck_zoom: f64) -> f32 {
    let base = 16.0 / 2.0_f64.powf(REF_ZOOM);
    size_with_min_px(base, 14.0, deck_zoom) as f32
}

/// Map a character to its atlas cell: printable ASCII directly, anything else through the
/// accent-fold table, and unmappable input to the tofu cell (never silently skipped — L4).
#[must_use]
pub fn glyph_index_for_char(ch: char) -> u16 {
    let code = ch as u32;
    if (32..=126).contains(&code) {
        return (code - 32) as u16;
    }
    match fold_accent(ch) {
        Some(ascii) => u16::from(ascii - 32),
        None => TOFU_GLYPH,
    }
}

/// Accent-fold map for label text (é→e …). Covers the Latin-1 letters plus the œ/æ-free
/// subset seen on cartographic sources; extend here when new label data introduces a char.
#[must_use]
fn fold_accent(ch: char) -> Option<u8> {
    Some(match ch {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => b'a',
        'ç' => b'c',
        'è' | 'é' | 'ê' | 'ë' => b'e',
        'ì' | 'í' | 'î' | 'ï' => b'i',
        'ñ' => b'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => b'o',
        'ù' | 'ú' | 'û' | 'ü' => b'u',
        'ý' | 'ÿ' => b'y',
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => b'A',
        'Ç' => b'C',
        'È' | 'É' | 'Ê' | 'Ë' => b'E',
        'Ì' | 'Í' | 'Î' | 'Ï' => b'I',
        'Ñ' => b'N',
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => b'O',
        'Ù' | 'Ú' | 'Û' | 'Ü' => b'U',
        'Ý' => b'Y',
        '\u{2019}' | '\u{2018}' => b'\'', // curly apostrophes
        '\u{2013}' | '\u{2014}' => b'-',  // en/em dash
        _ => return None,
    })
}

/// Pack decluttered labels into monospaced glyph instances.
///
/// `char_m` = world meters per character cell at current zoom (caller scales).
#[must_use]
pub fn pack_label_glyphs(
    labels: &[LabelSpec],
    deck_zoom: f64,
    char_m: f32,
) -> Vec<TextGlyphInstance> {
    let drawn = map_engine_core::label::declutter(labels, deck_zoom);
    glyphs_from_specs(&drawn, char_m, pack_rgba_u32([220, 220, 215, 230]))
}

/// T-152.7 — height labels with 80 m declutter (not the 48 px town-label curve).
#[must_use]
pub fn pack_height_label_glyphs(
    labels: &[HeightLabel],
    deck_zoom: f64,
    char_m: f32,
) -> Vec<TextGlyphInstance> {
    let drawn = declutter_height_labels(labels, deck_zoom);
    let specs: Vec<LabelSpec> = map_engine_core::dem::peaks::height_labels_to_specs(&drawn);
    glyphs_from_specs(&specs, char_m, pack_rgba_u32([220, 220, 215, 230]))
}

/// T-152.8 — town names with A3 importance declutter + cartographic tint `#e8e4dc` @ α0.92.
#[must_use]
pub fn pack_town_label_glyphs(
    locations: &[LocationLabel],
    deck_zoom: f64,
    char_m: f32,
) -> Vec<TextGlyphInstance> {
    let drawn = declutter_town_labels(locations, deck_zoom);
    let specs = locations_to_label_specs(&drawn);
    glyphs_from_specs(&specs, char_m, pack_rgba_u32([232, 228, 220, 234]))
}

/// T-152.9 — road names tangent-aligned along polylines; tint `#d8d4cc` @ α0.88.
#[must_use]
pub fn pack_road_label_bytes(placements: &[RoadLabelPlacement], deck_zoom: f64) -> Vec<u8> {
    let char_m = text_char_meters(deck_zoom);
    let advance = char_m * TEXT_GLYPH_ADVANCE_RATIO;
    let tint = pack_rgba_u32([216, 212, 204, 224]);
    let mut out = Vec::new();
    for lab in placements {
        let chars: Vec<char> = lab.name.chars().collect();
        let n = chars.len() as f32;
        let rad = lab.angle_deg.to_radians();
        let cos_a = rad.cos() as f32;
        let sin_a = rad.sin() as f32;
        let cx = lab.x as f32;
        let cy = lab.y as f32;
        for (i, ch) in chars.into_iter().enumerate() {
            let along = ((i as f32) - (n - 1.0) * 0.5) * advance;
            let gx = cx + along * cos_a;
            let gy = cy + along * sin_a;
            let glyph = glyph_index_for_char(ch);
            pack_icon_instance(&mut out, gx, gy, char_m, lab.angle_deg, glyph, tint);
        }
    }
    out
}

fn glyphs_from_specs(specs: &[LabelSpec], char_m: f32, _tint: u32) -> Vec<TextGlyphInstance> {
    let mut out = Vec::new();
    let half = char_m * 0.5;
    let advance = char_m * TEXT_GLYPH_ADVANCE_RATIO;
    for lab in specs {
        let chars: Vec<char> = lab.text.chars().collect();
        let n = chars.len() as f32;
        let y = lab.y as f32;
        for (i, ch) in chars.into_iter().enumerate() {
            let glyph = glyph_index_for_char(ch);
            out.push(TextGlyphInstance {
                x: lab.x as f32 + ((i as f32) - (n - 1.0) * 0.5) * advance,
                y,
                half_m: half,
                glyph,
            });
        }
    }
    out
}

/// Town label GPU bytes with cartographic tint.
#[must_use]
pub fn pack_town_label_bytes(locations: &[LocationLabel], deck_zoom: f64) -> Vec<u8> {
    let char_m = text_char_meters(deck_zoom);
    let glyphs = pack_town_label_glyphs(locations, deck_zoom, char_m);
    pack_text_icon_bytes_tint(&glyphs, deck_zoom, pack_rgba_u32([232, 228, 220, 234]))
}

/// Pack glyph instances into 20 B icon instances for the text atlas lane (WORLD coords).
#[must_use]
pub fn pack_text_icon_bytes(glyphs: &[TextGlyphInstance], deck_zoom: f64) -> Vec<u8> {
    pack_text_icon_bytes_tint(glyphs, deck_zoom, pack_rgba_u32([220, 220, 215, 230]))
}

/// Height labels use the default tint; town labels pass cartographic `#e8e4dc` @ α0.92.
#[must_use]
pub fn pack_text_icon_bytes_tint(
    glyphs: &[TextGlyphInstance],
    deck_zoom: f64,
    tint: u32,
) -> Vec<u8> {
    let char_m = text_char_meters(deck_zoom);
    let mut out = Vec::with_capacity(glyphs.len() * 20);
    for g in glyphs {
        let size = g.half_m * 2.0;
        pack_icon_instance(&mut out, g.x, g.y, size, 0.0, g.glyph, tint);
    }
    let _ = char_m; // size already baked into glyph half_m
    out
}

/// G4 oracle for height labels (re-export for tests).
#[must_use]
pub fn height_label_sep_m(deck_zoom: f64) -> f64 {
    height_label_min_sep_m(deck_zoom)
}

/// T-152.12 — CPU oracle for the `vs_text` UV mapping (Class R vs the WGSL source).
///
/// The atlas is authored y-down (`bake_ascii_atlas_rgba` paints cell row 0 at the texture top),
/// while quad `unit.y = 1` is the world/screen **top** of the glyph. Correct sampling therefore
/// flips V: `uv = mix((u0,v0), (u1,v1), (unit_x, 1 − unit_y))` — the same convention as
/// `vs_textured` ("North-up: unit.y=1 → v=0 (texture top)").
#[must_use]
pub fn glyph_cell_uv(glyph: u16, unit_x: f32, unit_y: f32) -> (f32, f32) {
    let cols = TEXT_ATLAS_COLS as f32;
    let rows = TEXT_ATLAS_ROWS as f32;
    let col = f32::from(glyph % TEXT_ATLAS_COLS as u16);
    let row = f32::from(glyph / TEXT_ATLAS_COLS as u16);
    let u0 = col / cols;
    let v0 = row / rows;
    let u1 = (col + 1.0) / cols;
    let v1 = (row + 1.0) / rows;
    (u0 + (u1 - u0) * unit_x, v0 + (v1 - v0) * (1.0 - unit_y))
}

/// Build the 16×6-cell (96 glyph) RGBA text atlas — 32×32 px cells, Spleen 16×32 ink
/// x-centered in each cell (T-152.13), tofu `□` painted procedurally in cell 95.
/// Image size 512×192 (16·32 × 6·32).
#[must_use]
pub fn bake_ascii_atlas_rgba() -> (Vec<u8>, u32, u32) {
    const CELL: u32 = TEXT_CELL_PX;
    const COLS: u32 = TEXT_ATLAS_COLS;
    const ROWS: u32 = TEXT_ATLAS_ROWS;
    let w = COLS * CELL;
    let h = ROWS * CELL;
    let ink_x0 = (CELL - FONT_GLYPH_W) / 2;
    let mut px = vec![0u8; (w * h * 4) as usize];
    let mut set = |x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        px[i] = 240;
        px[i + 1] = 240;
        px[i + 2] = 230;
        px[i + 3] = 255;
    };
    for gi in 0..96u32 {
        let ox = (gi % COLS) * CELL;
        let oy = (gi / COLS) * CELL;
        if gi == u32::from(TOFU_GLYPH) {
            // Hollow □ over the ink box (2 px stroke) — unmapped chars must be obvious, not blobs.
            for y in 4..28u32 {
                for x in 10..22u32 {
                    let edge = !(12..20).contains(&x) || !(6..26).contains(&y);
                    if edge {
                        set(ox + x, oy + y);
                    }
                }
            }
            continue;
        }
        let rows = &FONT_16X32[gi as usize];
        for (dy, bits) in rows.iter().enumerate() {
            for dx in 0..FONT_GLYPH_W {
                if (bits >> (15 - dx)) & 1 == 1 {
                    set(ox + ink_x0 + dx, oy + dy as u32);
                }
            }
        }
    }
    (px, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use map_engine_core::label::LabelSpec;

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
        // T-152.13 advance: chars step by char_m × ratio, centered on the anchor.
        let advance = 10.0 * TEXT_GLYPH_ADVANCE_RATIO;
        assert!((g[1].x - 100.0).abs() < 1e-4);
        assert!((g[2].x - g[1].x - advance).abs() < 1e-4);
    }

    #[test]
    fn atlas_size() {
        let (px, w, h) = bake_ascii_atlas_rgba();
        assert_eq!(w, TEXT_ATLAS_COLS * TEXT_CELL_PX);
        assert_eq!(h, TEXT_ATLAS_ROWS * TEXT_CELL_PX);
        assert_eq!((w, h), (512, 192));
        assert_eq!(px.len(), (w * h * 4) as usize);
        assert!(px.iter().any(|&c| c > 0));
    }

    // ── T-152.12 G1 — WGSL source guards ─────────────────────────────────────────────────────
    // The vec3-padded TextUniforms (32 B vs the 16 B binding) killed the whole text pipeline at
    // tags T-152.7–.10; the missing V-flip drew every glyph upside-down. Lock both in source.

    const SHADER_SRC: &str = include_str!("shader.wgsl");

    fn text_uniforms_block() -> &'static str {
        let start = SHADER_SRC
            .find("struct TextUniforms")
            .expect("TextUniforms struct present");
        let end = SHADER_SRC[start..].find('}').expect("struct closes") + start;
        &SHADER_SRC[start..end]
    }

    fn vs_text_body() -> &'static str {
        // Paren-anchored: a bare "fn vs_text" prefix-matches `vs_textured` further up the file.
        let start = SHADER_SRC.find("fn vs_text(").expect("vs_text present");
        let end = SHADER_SRC[start..]
            .find("fn fs_text(")
            .expect("fs_text follows vs_text")
            + start;
        &SHADER_SRC[start..end]
    }

    #[test]
    fn g1_text_uniforms_is_16_bytes_no_vec3() {
        let block = text_uniforms_block();
        assert!(
            !block.contains("vec3"),
            "TextUniforms must not use vec3 padding (align-16 makes the struct 32 B \
             against the 16 B min_binding_size — dead text pipeline)"
        );
        // Exactly four scalar f32 fields = 16 B.
        assert_eq!(
            block.matches(": f32").count(),
            4,
            "TextUniforms must stay exactly 4×f32 (16 B contract)"
        );
    }

    #[test]
    fn g1_vs_text_has_v_flip() {
        let body = vs_text_body();
        assert!(
            body.contains("1.0 - in.unit.y"),
            "vs_text must flip V (world-top → atlas cell top) like vs_textured"
        );
    }

    // ── T-152.13 L2 — grid dims must come from the uniform, not WGSL literals ───────────────
    #[test]
    fn l2_vs_text_grid_from_uniform() {
        let body = vs_text_body();
        assert!(
            body.contains("text_u.grid_cols") && body.contains("text_u.grid_rows"),
            "vs_text must read atlas grid dims from TextUniforms"
        );
        assert!(
            !body.contains("/ 16.0") && !body.contains("/ 6.0") && !body.contains("% 16u"),
            "vs_text must not hardcode the atlas grid (16/6 remnants)"
        );
    }

    // ── T-152.12 G2 — UV corner proof against the y-down atlas ──────────────────────────────
    #[test]
    fn g2_glyph_cell_uv_corners_upright() {
        let cols = TEXT_ATLAS_COLS as f32;
        let rows = TEXT_ATLAS_ROWS as f32;
        let eps = 1e-6;
        // World-top-left (unit 0,1) → atlas cell top-left (u0, v0).
        let (u, v) = glyph_cell_uv(0, 0.0, 1.0);
        assert!((u - 0.0).abs() < eps && (v - 0.0).abs() < eps);
        // World-bottom-left (unit 0,0) → atlas cell bottom-left (u0, v1).
        let (u, v) = glyph_cell_uv(0, 0.0, 0.0);
        assert!((u - 0.0).abs() < eps && (v - 1.0 / rows).abs() < eps);
        // World-top-right (unit 1,1) → atlas cell top-right (u1, v0).
        let (u, v) = glyph_cell_uv(0, 1.0, 1.0);
        assert!((u - 1.0 / cols).abs() < eps && (v - 0.0).abs() < eps);
        // U is NOT mirrored: unit_x=0 stays at the cell's left edge for any glyph.
        let (u, _) = glyph_cell_uv(23, 0.0, 0.5);
        assert!((u - 7.0 / cols).abs() < eps, "glyph 23 = col 7 left edge");
    }

    /// Alpha at (dx, dy) inside glyph `gi`'s atlas cell.
    fn cell_lit(px: &[u8], w: u32, gi: u32, dx: u32, dy: u32) -> bool {
        let cell = TEXT_CELL_PX;
        let (col, row) = (gi % TEXT_ATLAS_COLS, gi / TEXT_ATLAS_COLS);
        let x = col * cell + dx;
        let y = row * cell + dy;
        px[((y * w + x) * 4 + 3) as usize] > 0
    }

    #[test]
    fn g2_seven_is_top_heavy_in_atlas() {
        // Cross-check the probe geometry used by `text_self_check` against the Spleen '7'
        // (glyph 23): ink is 16 px wide at cell x-offset 8. Row dy=6 carries the top bar
        // (ink cols 2..=13); rows dy≥18 only the descender at ink cols 6..=7.
        let (px, w, _h) = bake_ascii_atlas_rgba();
        assert!(cell_lit(&px, w, 23, 11, 6), "top bar lit (ink col 3, dy=6)");
        assert!(
            cell_lit(&px, w, 23, 14, 18),
            "descender lit (ink col 6, dy=18)"
        );
        assert!(
            !cell_lit(&px, w, 23, 17, 18),
            "U-mirror trap: cell x=17 (ink col 9) empty at dy=18"
        );
        assert!(
            !cell_lit(&px, w, 23, 11, 25),
            "V-flip trap: dy=25 empty at ink col 3 (flip would land the top bar here)"
        );
    }

    // ── T-152.13 G2 — full printable-ASCII coverage with distinct lowercase ─────────────────
    #[test]
    fn g2_full_ascii_coverage_distinct_lowercase() {
        let tofu_idx = usize::from(TOFU_GLYPH);
        // Every printable char except space has real ink; the table's tofu slot stays zeroed
        // (the baker paints the box) so "≠ fallback" means a non-empty raster of its own.
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
        assert!(cell_lit(&px, w, gi, 10, 4), "tofu top-left stroke");
        assert!(cell_lit(&px, w, gi, 21, 27), "tofu bottom-right stroke");
        assert!(!cell_lit(&px, w, gi, 16, 16), "tofu interior hollow");
    }

    // ── T-152.13 G3 — no-blob gate over committed label data ────────────────────────────────
    // Every character actually shipped in locations.json / road-names.json (plus height-label
    // numerals) must resolve to a real glyph after accent folding — zero tofu hits.
    #[test]
    fn g3_committed_label_data_no_tofu() {
        const LOCATIONS: &str = include_str!("../../../packages/map-assets/everon/locations.json");
        const ROAD_NAMES: &str =
            include_str!("../../../packages/map-assets/everon/road-names.json");

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
        names.push("0123456789".into()); // height-label numerals (peaks.rs value_m.to_string())

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
}
