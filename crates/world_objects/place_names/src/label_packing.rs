//! Packing of the terrain's place-name labels (spot heights, towns, road names) into glyph
//! instances and instance bytes.
//!
//! **Role:** turns decluttered spot heights, town labels and placed road names into the text
//! glyphs the label lanes draw.
//! **Position:** reads the declutter of [`crate::peaks`], [`crate::towns`] and
//! [`crate::route_placement`], the generic glyph packing of `label_layout::text_packing` and the
//! glyph metrics of `render_primitives::text`; read by the map engine's label loader.
//! **Signals & state:** none; pure functions over their arguments.
//! **Invariants:** only labels the declutter keeps are packed; a road name's glyphs are spaced
//! one glyph advance apart along its placement angle, centred on its anchor; the town fade only
//! scales the tint's alpha.

use label_layout::declutter::LabelSpec;
use label_layout::importance::LocationLabel;
use label_layout::importance::declutter_town_labels;
use label_layout::importance::town_label_fade_alpha;
use label_layout::text_packing::glyphs_from_specs;
use label_layout::text_packing::to_glyph_specs;
use render_primitives::text::layout::pack_text_icon_bytes_tint;
use render_primitives::text::metrics::TEXT_GLYPH_ADVANCE_RATIO;
use render_primitives::text::metrics::TextGlyphInstance;
use render_primitives::text::metrics::glyph_index_for_char;
use render_primitives::text::metrics::text_char_meters;
use render_primitives::text::pack::pack_icon_instance_yaw as pack_icon_instance;
use render_primitives::text::pack::pack_rgba_u32;

use crate::peaks::HeightLabel;
use crate::peaks::declutter_height_labels;
use crate::peaks::height_labels_to_specs;
use crate::route_placement::RoadLabelPlacement;
use crate::towns::locations_to_label_specs;

/// Glyph instances of the spot heights that draw at `deck_zoom`: the height declutter (nothing
/// outside the height label zoom band), then a declutter by measured text width, each character
/// a `char_m`-metre cell in a pale grey tint.
#[must_use]
pub fn pack_height_label_glyphs(
    labels: &[HeightLabel],
    deck_zoom: f64,
    char_m: f32,
) -> Vec<TextGlyphInstance> {
    let drawn = declutter_height_labels(labels, deck_zoom);
    let specs: Vec<LabelSpec> = height_labels_to_specs(&drawn);
    let kept = render_primitives::text::layout::declutter_specs_by_width(
        &to_glyph_specs(&specs, char_m),
        char_m,
    );
    render_primitives::text::layout::glyphs_from_specs(
        &kept,
        char_m,
        pack_rgba_u32([220, 220, 215, 230]),
    )
}

/// Glyph instances of the town labels the town declutter keeps at `deck_zoom`, each character a
/// `char_m`-metre cell in an off-white tint; a name that trims to nothing is skipped.
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

/// Instance bytes of already decluttered road name placements: one glyph per character, sized by
/// `text_char_meters` at `deck_zoom`, rotated to the placement angle and spaced one glyph
/// advance apart along it, centred on the anchor.
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

/// Instance bytes of [`pack_town_label_glyphs`] at `deck_zoom`, the character size from
/// `text_char_meters` and the tint's alpha (234 at full) scaled by the town label fade.
#[must_use]
pub fn pack_town_label_bytes(locations: &[LocationLabel], deck_zoom: f64) -> Vec<u8> {
    const BASE_ALPHA: f64 = 234.0;
    let char_m = text_char_meters(deck_zoom);
    let glyphs = pack_town_label_glyphs(locations, deck_zoom, char_m);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let alpha = (BASE_ALPHA * town_label_fade_alpha(deck_zoom)).round() as u8;
    pack_text_icon_bytes_tint(&glyphs, deck_zoom, pack_rgba_u32([232, 228, 220, alpha]))
}
