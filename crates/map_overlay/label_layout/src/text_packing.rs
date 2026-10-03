//! Generic label packing: decluttered [`LabelSpec`]s into monospaced glyph instances.
//!
//! **Role:** the `LabelSpec` adapter over the renderer's glyph layout, and the declutter-then-pack
//! of a generic label set.
//! **Position:** `label_layout`; reads [`crate::declutter`] and
//! `render_primitives::text::layout`; read by the map engine's marker captions and place-name
//! packers.
//! **Signals & state:** none; pure functions over their arguments.
//! **Invariants:** the importance column of a [`LabelSpec`] never reaches the renderer; the
//! module names no peak, town or road.

use crate::declutter::{LabelSpec, declutter};
use render_primitives::text::layout::{GlyphSpec, GlyphSpecId};
use render_primitives::text::metrics::TextGlyphInstance;
use render_primitives::text::pack::pack_rgba_u32;

/// A decluttered label, as the renderer wants it: an anchor, characters and a cell size.
///
/// The importance column is dropped on purpose — it decided which labels reach here, and the
/// renderer must not be able to consult it afterwards.
#[must_use]
pub fn to_glyph_specs(specs: &[LabelSpec], char_m: f32) -> Vec<GlyphSpec> {
    specs
        .iter()
        .map(|s| GlyphSpec {
            id: GlyphSpecId(s.id.get()),
            x: s.x,
            y: s.y,
            text: s.text.clone(),
            size_m: char_m,
        })
        .collect()
}

/// Pack decluttered labels into monospaced glyph instances.
#[must_use]
pub fn pack_label_glyphs(
    labels: &[LabelSpec],
    deck_zoom: f64,
    char_m: f32,
) -> Vec<TextGlyphInstance> {
    let drawn = declutter(labels, deck_zoom);
    glyphs_from_specs(&drawn, char_m, pack_rgba_u32([220, 220, 215, 230]))
}

/// Glyph instances of already-decluttered label specs, tinted `tint`.
///
/// The `LabelSpec` adapter over the renderer's `glyphs_from_specs`: the caller keeps the
/// importance column, the renderer never sees it.
#[must_use]
pub fn glyphs_from_specs(specs: &[LabelSpec], char_m: f32, tint: u32) -> Vec<TextGlyphInstance> {
    render_primitives::text::layout::glyphs_from_specs(&to_glyph_specs(specs, char_m), char_m, tint)
}

#[cfg(test)]
#[path = "tests/text_packing_tests.rs"]
mod tests;
