//! Role: packing.
//! Position: `renderers/text` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::environment::locations::peaks::HeightLabel;
use crate::environment::locations::peaks::declutter_height_labels;
use crate::environment::locations::route_placement::RoadLabelPlacement;
use crate::environment::locations::towns::locations_to_label_specs;
use crate::renderers::text::metrics::TEXT_GLYPH_ADVANCE_RATIO;
use crate::renderers::text::metrics::TextGlyphInstance;
use crate::renderers::text::metrics::glyph_index_for_char;
use crate::renderers::text::metrics::text_char_meters;
use crate::symbology::labels::declutter::LabelSpec;
use crate::symbology::labels::glyph_math::pack_icon_instance;
use crate::symbology::labels::glyph_math::pack_rgba_u32;
use crate::symbology::labels::importance::LocationLabel;
use crate::symbology::labels::importance::declutter_town_labels;
use crate::symbology::labels::importance::town_label_fade_alpha;

/// Pack decluttered labels into monospaced glyph instances.
#[must_use]
pub fn pack_label_glyphs(
    labels: &[LabelSpec],
    deck_zoom: f64,
    char_m: f32,
) -> Vec<TextGlyphInstance> {
    let drawn = crate::symbology::labels::declutter::declutter(labels, deck_zoom);
    glyphs_from_specs(&drawn, char_m, pack_rgba_u32([220, 220, 215, 230]))
}

/// Pack height label glyphs.
#[must_use]
pub fn pack_height_label_glyphs(
    labels: &[HeightLabel],
    deck_zoom: f64,
    char_m: f32,
) -> Vec<TextGlyphInstance> {
    let drawn = declutter_height_labels(labels, deck_zoom);
    let specs: Vec<LabelSpec> =
        crate::environment::locations::peaks::height_labels_to_specs(&drawn);
    let kept = declutter_specs_by_width(&specs, char_m);
    glyphs_from_specs(&kept, char_m, pack_rgba_u32([220, 220, 215, 230]))
}

/// Declutter specs by width.
pub(crate) fn declutter_specs_by_width(specs: &[LabelSpec], char_m: f32) -> Vec<LabelSpec> {
    let advance = char_m * TEXT_GLYPH_ADVANCE_RATIO;
    let half_h = char_m;
    let mut kept: Vec<(f32, f32, f32)> = Vec::new();
    let mut out = Vec::new();
    for s in specs {
        let half_w = s.text.chars().count() as f32 * advance * 0.5;
        let sx = s.x as f32;
        let sy = s.y as f32;
        let overlaps = kept.iter().any(|&(kx, ky, kw)| {
            (sx - kx).abs() < half_w + kw + advance && (sy - ky).abs() < half_h
        });
        if !overlaps {
            kept.push((sx, sy, half_w));
            out.push(s.clone());
        }
    }
    out
}

/// Pack town label glyphs.
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

/// Pack road label bytes.
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

/// Glyphs from specs.
pub(crate) fn glyphs_from_specs(
    specs: &[LabelSpec],
    char_m: f32,
    _tint: u32,
) -> Vec<TextGlyphInstance> {
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

/// Pack town label bytes.
#[must_use]
pub fn pack_town_label_bytes(locations: &[LocationLabel], deck_zoom: f64) -> Vec<u8> {
    const BASE_ALPHA: f64 = 234.0;
    let char_m = text_char_meters(deck_zoom);
    let glyphs = pack_town_label_glyphs(locations, deck_zoom, char_m);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let alpha = (BASE_ALPHA * town_label_fade_alpha(deck_zoom)).round() as u8;
    pack_text_icon_bytes_tint(&glyphs, deck_zoom, pack_rgba_u32([232, 228, 220, alpha]))
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
    let _ = char_m;
    out
}
