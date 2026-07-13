//! Procedural text layout helpers (T-152.1 / T-152.7) — pack label strings into world-space glyph quads.
//! Pure data; GPU upload lives in `map-engine-render` (wasm32).

use map_engine_core::dem::peaks::{HeightLabel, declutter_height_labels, height_label_min_sep_m};
use map_engine_core::label::LabelSpec;
use map_engine_core::world::{
    LocationLabel, RoadLabelPlacement, declutter_town_labels, locations_to_label_specs,
};
use map_engine_core::world::{REF_ZOOM, pack_icon_instance, pack_rgba_u32, size_with_min_px};

/// One textured glyph instance in world meters (center of character cell).
#[derive(Clone, Debug)]
pub struct TextGlyphInstance {
    pub x: f32,
    pub y: f32,
    /// Cell half-extent (meters).
    pub half_m: f32,
    /// Atlas cell index 0..95 for printable ASCII 32..126.
    pub glyph: u16,
}

/// World meters per character cell at `deck_zoom` (12 px @ REF_ZOOM, min 6 px).
#[must_use]
pub fn text_char_meters(deck_zoom: f64) -> f32 {
    let base = 12.0 / 2.0_f64.powf(REF_ZOOM);
    size_with_min_px(base, 6.0, deck_zoom) as f32
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
    let half = char_m * 0.5;
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
        let start = -((n - 1.0) * char_m) * 0.5;
        for (i, ch) in chars.into_iter().enumerate() {
            let code = ch as u32;
            if !(32..127).contains(&code) {
                continue;
            }
            let along = start + (i as f32) * char_m;
            let gx = cx + along * cos_a;
            let gy = cy + along * sin_a;
            let glyph = (code - 32) as u16;
            pack_icon_instance(&mut out, gx, gy, char_m, lab.angle_deg, glyph, tint);
        }
        let _ = half; // size baked via char_m
    }
    out
}

fn glyphs_from_specs(specs: &[LabelSpec], char_m: f32, _tint: u32) -> Vec<TextGlyphInstance> {
    let mut out = Vec::new();
    let half = char_m * 0.5;
    for lab in specs {
        let chars: Vec<char> = lab.text.chars().collect();
        let n = chars.len() as f32;
        let origin_x = lab.x as f32 - (n * char_m) * 0.5;
        let y = lab.y as f32;
        for (i, ch) in chars.into_iter().enumerate() {
            let code = ch as u32;
            if !(32..127).contains(&code) {
                continue;
            }
            let glyph = (code - 32) as u16;
            out.push(TextGlyphInstance {
                x: origin_x + (i as f32) * char_m + half,
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
    let col = f32::from(glyph % 16);
    let row = f32::from(glyph / 16);
    let u0 = col / 16.0;
    let v0 = row / 6.0;
    let u1 = (col + 1.0) / 16.0;
    let v1 = (row + 1.0) / 6.0;
    (u0 + (u1 - u0) * unit_x, v0 + (v1 - v0) * (1.0 - unit_y))
}

/// Build a 16×6 cell (96 glyphs) 8×8 px RGBA atlas for printable ASCII — baked bitmap font.
/// Image size 128×48 (16*8 × 6*8).
#[must_use]
pub fn bake_ascii_atlas_rgba() -> (Vec<u8>, u32, u32) {
    const CELL: u32 = 8;
    const COLS: u32 = 16;
    const ROWS: u32 = 6;
    let w = COLS * CELL;
    let h = ROWS * CELL;
    let mut px = vec![0u8; (w * h * 4) as usize];
    for gi in 0..96u32 {
        let col = gi % COLS;
        let row = gi / COLS;
        let ox = col * CELL;
        let oy = row * CELL;
        // Very simple 5×7 stroke pattern from character code — enough for height numerals.
        let ch = (gi + 32) as u8;
        for dy in 1..7 {
            for dx in 1..7 {
                let on = glyph_pixel(ch, dx - 1, dy - 1);
                if on {
                    let x = ox + dx;
                    let y = oy + dy;
                    let i = ((y * w + x) * 4) as usize;
                    px[i] = 240;
                    px[i + 1] = 240;
                    px[i + 2] = 230;
                    px[i + 3] = 255;
                }
            }
        }
    }
    (px, w, h)
}

/// Tiny patterns for digits + A–Z — 5×7 bitfields packed in u64 (35 bits used).
fn glyph_pixel(ch: u8, x: u32, y: u32) -> bool {
    if x >= 5 || y >= 7 {
        return false;
    }
    let ch = if ch.is_ascii_lowercase() {
        ch.to_ascii_uppercase()
    } else {
        ch
    };
    let bits: u64 = match ch {
        b'0' => 0b0_01110_10001_10011_10101_11001_10001_01110,
        b'1' => 0b0_00100_01100_00100_00100_00100_00100_01110,
        b'2' => 0b0_01110_10001_00001_00010_00100_01000_11111,
        b'3' => 0b0_01110_10001_00001_00110_00001_10001_01110,
        b'4' => 0b0_00010_00110_01010_10010_11111_00010_00010,
        b'5' => 0b0_11111_10000_11110_00001_00001_10001_01110,
        b'6' => 0b0_00110_01000_10000_11110_10001_10001_01110,
        b'7' => 0b0_11111_00001_00010_00100_01000_01000_01000,
        b'8' => 0b0_01110_10001_10001_01110_10001_10001_01110,
        b'9' => 0b0_01110_10001_10001_01111_00001_00010_01100,
        b'm' | b'M' => 0b0_10001_11011_10101_10101_10001_10001_10001,
        b' ' => 0,
        b'A'..=b'Z' => letter_glyph_bits(ch),
        _ => 0b0_01110_10001_10001_10001_10001_10001_01110,
    };
    let bit = 34u32.saturating_sub(y * 5 + x);
    ((bits >> bit) & 1) == 1
}

/// Public-domain 5×7 uppercase alphabet (tom-thumb style).
fn letter_glyph_bits(ch: u8) -> u64 {
    const GLYPHS: [u64; 26] = [
        0b0_01110_10001_10001_11111_10001_10001_10001, // A
        0b0_11110_10001_10001_11110_10001_10001_11110, // B
        0b0_01110_10001_10000_10000_10001_10001_01110, // C
        0b0_11110_10001_10001_10001_10001_10001_11110, // D
        0b0_11111_10000_11110_10000_10000_10000_11111, // E
        0b0_11111_10000_11110_10000_10000_10000_10000, // F
        0b0_01110_10001_10000_10111_10001_10001_01110, // G
        0b0_10001_10001_10001_11111_10001_10001_10001, // H
        0b0_01110_00100_00100_00100_00100_00100_01110, // I
        0b0_00111_00010_00010_00010_10010_10010_01100, // J
        0b0_10001_10010_10100_11000_10100_10010_10001, // K
        0b0_10000_10000_10000_10000_10000_10000_11111, // L
        0b0_10001_11011_10101_10101_10001_10001_10001, // M
        0b0_10001_11001_10101_10011_10001_10001_10001, // N
        0b0_01110_10001_10001_10001_10001_10001_01110, // O
        0b0_11110_10001_10001_11110_10000_10000_10000, // P
        0b0_01110_10001_10001_10001_10101_10010_01101, // Q
        0b0_11110_10001_10001_11110_10100_10010_10001, // R
        0b0_01110_10001_10000_01110_00001_10001_01110, // S
        0b0_11111_00100_00100_00100_00100_00100_00100, // T
        0b0_10001_10001_10001_10001_10001_10001_01110, // U
        0b0_10001_10001_10001_10001_10001_01010_00100, // V
        0b0_10001_10001_10001_10101_10101_10101_01010, // W
        0b0_10001_10001_01010_00100_01010_10001_10001, // X
        0b0_10001_10001_10001_01010_00100_00100_00100, // Y
        0b0_11111_00001_00010_00100_01000_10000_11111, // Z
    ];
    let idx = (ch - b'A') as usize;
    GLYPHS.get(idx).copied().unwrap_or(0)
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
    }

    #[test]
    fn atlas_size() {
        let (px, w, h) = bake_ascii_atlas_rgba();
        assert_eq!(w, 128);
        assert_eq!(h, 48);
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
        let start = SHADER_SRC.find("fn vs_text").expect("vs_text present");
        let end = SHADER_SRC[start..]
            .find("fn fs_text")
            .expect("fs_text follows vs_text")
            + start;
        let body = &SHADER_SRC[start..end];
        assert!(
            body.contains("1.0 - in.unit.y"),
            "vs_text must flip V (world-top → atlas cell top) like vs_textured"
        );
    }

    // ── T-152.12 G2 — UV corner proof against the y-down atlas ──────────────────────────────
    #[test]
    fn g2_glyph_cell_uv_corners_upright() {
        // Glyph 0 → cell (col 0, row 0): u ∈ [0, 1/16], v ∈ [0, 1/6].
        let eps = 1e-6;
        // World-top-left (unit 0,1) → atlas cell top-left (u0, v0).
        let (u, v) = glyph_cell_uv(0, 0.0, 1.0);
        assert!((u - 0.0).abs() < eps && (v - 0.0).abs() < eps);
        // World-bottom-left (unit 0,0) → atlas cell bottom-left (u0, v1).
        let (u, v) = glyph_cell_uv(0, 0.0, 0.0);
        assert!((u - 0.0).abs() < eps && (v - 1.0 / 6.0).abs() < eps);
        // World-top-right (unit 1,1) → atlas cell top-right (u1, v0).
        let (u, v) = glyph_cell_uv(0, 1.0, 1.0);
        assert!((u - 1.0 / 16.0).abs() < eps && (v - 0.0).abs() < eps);
        // U is NOT mirrored: unit_x=0 stays at the cell's left edge for any glyph.
        let (u, _) = glyph_cell_uv(23, 0.0, 0.5);
        assert!((u - 7.0 / 16.0).abs() < eps, "glyph 23 = col 7 left edge");
    }

    #[test]
    fn g2_seven_is_top_heavy_in_atlas() {
        // Cross-check the probe geometry used by `text_self_check`: for '7' (glyph 23) the
        // atlas cell's row dy=1 is the full-width top bar and row dy=6 is lit only at dx=2.
        let (px, w, _h) = bake_ascii_atlas_rgba();
        let cell = 8u32;
        let (col, row) = (23u32 % 16, 23u32 / 16);
        let lit = |dx: u32, dy: u32| -> bool {
            let x = col * cell + dx;
            let y = row * cell + dy;
            px[((y * w + x) * 4 + 3) as usize] > 0
        };
        assert!(lit(1, 1) && lit(3, 1) && lit(5, 1), "top bar row lit");
        assert!(lit(2, 6), "bottom stroke at dx=2");
        assert!(!lit(4, 6), "bottom row dx=4 empty (upright probe target)");
    }
}
