//! Role: text layout.
//! Position: `text` in the graphics engine.
//! Signals & state: a placed run of characters, and the 20-byte instances it packs into.
//! Invariants: the caller decides WHICH strings exist and WHERE they sit; this lays the
//! characters of an already-chosen string along a row and packs the bytes. The only collision
//! rule here is the width overlap of two boxes — importance, zoom fade and place names are the
//! caller's.

use crate::text::metrics::{TEXT_GLYPH_ADVANCE_RATIO, TextGlyphInstance, glyph_index_for_char};
use crate::text::pack::{pack_icon_instance_yaw, pack_rgba_u32};

/// One placed string, in world meters.
///
/// The renderer's stand-in for whatever richer record the caller decluttered: an id to keep the
/// ordering stable, an anchor, the characters, and the cell size the caller sized it at.
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphSpec {
    /// Caller's stable id — carried through so a kept set can be traced back.
    pub id: u32,

    /// Anchor X, world meters.
    pub x: i32,

    /// Anchor Y, world meters.
    pub y: i32,

    /// The characters to lay out.
    pub text: String,

    /// Cell size the caller sized this spec at, world meters.
    pub size_m: f32,
}

/// Drop specs whose laid-out boxes overlap, keeping the first of each overlapping group.
///
/// Distinct from the caller's importance declutter: that one runs on anchors before the text is
/// measured, this one runs on the width the characters actually occupy.
#[must_use]
pub fn declutter_specs_by_width(specs: &[GlyphSpec], char_m: f32) -> Vec<GlyphSpec> {
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

/// Lay each spec's characters out along its row, centred on the anchor.
#[must_use]
pub fn glyphs_from_specs(specs: &[GlyphSpec], char_m: f32, _tint: u32) -> Vec<TextGlyphInstance> {
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
    let char_m = crate::text::metrics::text_char_meters(deck_zoom);
    let mut out = Vec::with_capacity(glyphs.len() * 20);
    for g in glyphs {
        let size = g.half_m * 2.0;
        pack_icon_instance_yaw(&mut out, g.x, g.y, size, 0.0, g.glyph, tint);
    }
    let _ = char_m;
    out
}

#[cfg(test)]
#[path = "tests/layout_tests.rs"]
mod tests;
