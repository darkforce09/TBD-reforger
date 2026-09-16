//! Role: text metrics.
//! Position: `text` in the graphics engine.
//! Signals & state: cell size in world meters, and the character → atlas-cell map.
//! Invariants: one character in, one cell index out. No word, no language, no place.

use crate::text::atlas::TOFU_GLYPH;
use crate::text::scale::{REF_ZOOM, size_with_min_px};

/// Horizontal pen advance as a fraction of the cell: ink is half the cell wide.
pub const TEXT_GLYPH_ADVANCE_RATIO: f32 = 0.5;

/// One textured glyph instance in world meters (center of character cell).
#[derive(Clone, Debug)]
pub struct TextGlyphInstance {
    /// X.
    pub x: f32,

    /// Y.
    pub y: f32,

    /// Cell half-extent (meters).
    pub half_m: f32,

    /// Atlas cell index: 0..=94 for printable ASCII 32..=126, 95 = tofu fallback.
    pub glyph: u16,
}

/// World meters per character cell at `deck_zoom` (24 px @ REF_ZOOM, min 20 px).
#[must_use]
pub fn text_char_meters(deck_zoom: f64) -> f32 {
    let base = 24.0 / 2.0_f64.powf(REF_ZOOM);
    size_with_min_px(base, 20.0, deck_zoom) as f32
}

/// Map a character to its atlas cell: printable ASCII directly, anything else through the accent-fold table, and unmappable input to the tofu cell (never silently skipped — L4).
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

/// Fold accent.
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
        '\u{2019}' | '\u{2018}' => b'\'',
        '\u{2013}' | '\u{2014}' => b'-',
        _ => return None,
    })
}
