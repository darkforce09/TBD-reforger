//! Role: atlas.
//! Position: `text` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::text::font::FONT_16X32;
use crate::text::font::FONT_GLYPH_W;

/// Atlas grid columns (glyph index → cell: `col = glyph % COLS`).
pub const TEXT_ATLAS_COLS: u32 = 16;

/// Atlas grid rows (96 cells for ASCII 32..=126 + tofu).
pub const TEXT_ATLAS_ROWS: u32 = 6;

/// Square atlas cell edge in pixels (glyph ink is 16×32, x-centered).
pub const TEXT_CELL_PX: u32 = 32;

/// Fallback cell — a visually-obvious `□`; committed label data must never hit it (G3).
pub const TOFU_GLYPH: u16 = 95;

/// Canonical text halo px value.
pub const TEXT_HALO_PX: u32 = 2;

/// Glyph ink color (warm off-white; lanes modulate via instance tint).
pub const TEXT_INK_RGBA: [u8; 4] = [240, 240, 230, 255];

/// Halo color — near-black blue-grey so glyphs stay legible over any background.
pub const TEXT_HALO_RGBA: [u8; 4] = [16, 21, 29, 255];

/// The atlas is authored y-down (`bake_ascii_atlas_rgba` paints cell row 0 at the texture top), while quad `unit.y = 1` is the world/screen **top** of the glyph. Correct sampling therefore flips V: `uv = mix((u0,v0), (u1,v1), (unit_x, 1 − unit_y))` — the same convention as `vs_textured` ("North-up: unit.y=1 → v=0 (texture top)").
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

/// Bake ascii atlas rgba.
#[must_use]
pub fn bake_ascii_atlas_rgba() -> (Vec<u8>, u32, u32) {
    const CELL: u32 = TEXT_CELL_PX;
    const COLS: u32 = TEXT_ATLAS_COLS;
    const ROWS: u32 = TEXT_ATLAS_ROWS;
    const HALO: i32 = TEXT_HALO_PX as i32;
    let w = COLS * CELL;
    let h = ROWS * CELL;
    let ink_x0 = (CELL - FONT_GLYPH_W) / 2;
    let mut px = vec![0u8; (w * h * 4) as usize];
    for gi in 0..96u32 {
        let ox = (gi % COLS) * CELL;
        let oy = (gi / COLS) * CELL;

        let mut ink = [0u32; CELL as usize];
        if gi == u32::from(TOFU_GLYPH) {
            for y in 4..28u32 {
                for x in 10..22u32 {
                    if !(12..20).contains(&x) || !(6..26).contains(&y) {
                        ink[y as usize] |= 1 << x;
                    }
                }
            }
        } else {
            let rows = &FONT_16X32[gi as usize];
            for (dy, bits) in rows.iter().enumerate() {
                for dx in 0..FONT_GLYPH_W {
                    if (bits >> (15 - dx)) & 1 == 1 {
                        ink[dy] |= 1 << (ink_x0 + dx);
                    }
                }
            }
        }

        let mut dilated = [0u32; CELL as usize];
        for y in 0..CELL as i32 {
            let r = ink[y as usize];
            let mut spread = r;
            for s in 1..=TEXT_HALO_PX {
                spread |= r << s | r >> s;
            }
            for dy in -HALO..=HALO {
                let ty = y + dy;
                if (0..CELL as i32).contains(&ty) {
                    dilated[ty as usize] |= spread;
                }
            }
        }
        for y in 0..CELL {
            for x in 0..CELL {
                let on_ink = (ink[y as usize] >> x) & 1 == 1;
                let on_halo = !on_ink && (dilated[y as usize] >> x) & 1 == 1;
                if !(on_ink || on_halo) {
                    continue;
                }
                let rgba = if on_ink {
                    TEXT_INK_RGBA
                } else {
                    TEXT_HALO_RGBA
                };
                let i = (((oy + y) * w + ox + x) * 4) as usize;
                px[i..i + 4].copy_from_slice(&rgba);
            }
        }
    }
    (px, w, h)
}
