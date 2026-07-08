//! Pure W1 basemap-lane geometry — no wgpu/web types, so this module compiles natively and its
//! byte-level tests run under plain `cargo test` (plan §Verification, Class R). It mirrors the
//! Deck oracle exactly:
//!
//! - `grid_lines` ≡ `useBaseMapLayer.ts` (`GRID_STEP = 1000`, major every 5 km, the six
//!   `[173,198,255,α]` palettes, `x <= width` inclusive loop).
//! - `corner_uv` is the north-up texture contract: unit `y = 1` (world maxY = north) → `v = 0`
//!   (texture top). A Y-flip swaps NW/SW and the tests fail.
//! - `pack_offset` places pyramid tile `(tx, ty)` (south-first world) into the packed atlas with
//!   the northernmost tile at texture row 0.
//! - `world_rect_rel` is the anchor-relative-meters conversion shared with `scene::QuadInstance`.

use crate::scene::ANCHOR;
use bytemuck::{Pod, Zeroable};

/// One grid-line vertex: anchor-relative position (meters) + normalized-RGBA color. 24 B,
/// laid out for the Polyline vertex buffer (`@location(0) pos`, `@location(1) color`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct LineVertex {
    /// Anchor-relative [x, y], meters (world minus [`ANCHOR`]).
    pub pos: [f32; 2],
    /// RGBA, linear 0..1 (non-sRGB target — no transfer function).
    pub color: [f32; 4],
}

/// Meters between gridlines (1 km) — `useBaseMapLayer.ts:11`.
const GRID_STEP: u32 = 1000;
/// Major gridline every 5 km — `useBaseMapLayer.ts:49`.
const MAJOR_STEP: u32 = 5000;

// Aegis primary #adc6ff = rgb(173,198,255); faint minor, brighter every 5 km, brighter still on
// the border. The `_HS` palette (over the hillshade) boosts alpha so the grid stays legible over
// the ~40 %-gray overlay. Bytes are the exact tuples from `useBaseMapLayer.ts:16-21`.
const MINOR: [u8; 4] = [173, 198, 255, 28];
const MAJOR: [u8; 4] = [173, 198, 255, 60];
const BORDER: [u8; 4] = [173, 198, 255, 90];
const MINOR_HS: [u8; 4] = [173, 198, 255, 80];
const MAJOR_HS: [u8; 4] = [173, 198, 255, 150];
const BORDER_HS: [u8; 4] = [173, 198, 255, 210];

/// `[u8;4]` → normalized f32 RGBA, `c / 255` (luma.gl's u8-attribute normalization).
#[must_use]
fn norm(c: [u8; 4]) -> [f32; 4] {
    [
        f32::from(c[0]) / 255.0,
        f32::from(c[1]) / 255.0,
        f32::from(c[2]) / 255.0,
        f32::from(c[3]) / 255.0,
    ]
}

/// Convert a world point to anchor-relative meters, f32 (the `scene::QuadInstance` contract).
#[must_use]
fn rel(x: f64, y: f64) -> [f32; 2] {
    #[allow(clippy::cast_possible_truncation)]
    [(x - ANCHOR[0]) as f32, (y - ANCHOR[1]) as f32]
}

/// Build the procedural 1 km grid as a `LineList` vertex buffer (2 vertices per line), an exact
/// mirror of `useBaseMapLayer.ts:44-58`: verticals `x ∈ [0, width]` step 1000 (`x <= width`
/// inclusive) then horizontals `y ∈ [0, height]`; color is BORDER (`x == 0 || x >= width`) /
/// MAJOR (`x % 5000 == 0`) / MINOR (else), switching to the `_HS` palette over the hillshade.
///
/// Everon (12800) ⇒ 13 verticals (x = 0..12000) + 13 horizontals = 26 lines / 52 vertices; only
/// `x = 0` / `y = 0` are borders (12800 is never hit by the step-1000 loop — Deck's behavior).
#[must_use]
pub fn grid_lines(width: f64, height: f64, over_hillshade: bool) -> Vec<LineVertex> {
    let (minor, major, border) = if over_hillshade {
        (norm(MINOR_HS), norm(MAJOR_HS), norm(BORDER_HS))
    } else {
        (norm(MINOR), norm(MAJOR), norm(BORDER))
    };
    let mut out = Vec::new();

    let mut push_line = |a: [f32; 2], b: [f32; 2], color: [f32; 4]| {
        out.push(LineVertex { pos: a, color });
        out.push(LineVertex { pos: b, color });
    };

    // Verticals: x from 0 to width, step 1000 (inclusive, mirroring the JS `for` bound).
    let mut x: u32 = 0;
    while f64::from(x) <= width {
        let on_border = x == 0 || f64::from(x) >= width;
        let color = if on_border {
            border
        } else if x.is_multiple_of(MAJOR_STEP) {
            major
        } else {
            minor
        };
        push_line(rel(f64::from(x), 0.0), rel(f64::from(x), height), color);
        x += GRID_STEP;
    }
    // Horizontals: y from 0 to height, step 1000.
    let mut y: u32 = 0;
    while f64::from(y) <= height {
        let on_border = y == 0 || f64::from(y) >= height;
        let color = if on_border {
            border
        } else if y.is_multiple_of(MAJOR_STEP) {
            major
        } else {
            minor
        };
        push_line(rel(0.0, f64::from(y)), rel(width, f64::from(y)), color);
        y += GRID_STEP;
    }
    out
}

/// North-up UV for a textured quad: unit `(x, y)` over the world rect → texture `(u, v)`. Unit
/// `y = 1` is the world maxY (north); the texture's top row (`v = 0`) is north (Deck BitmapLayer,
/// TBDS block-row-0-north, Rust hillshade row-0-north all agree), so `v = 1 - unit.y`.
#[must_use]
pub fn corner_uv(unit: [f32; 2]) -> [f32; 2] {
    [unit[0], 1.0 - unit[1]]
}

/// Packed-atlas pixel offset for pyramid tile `(tx, ty)` (south-first world tiles) given the
/// visible tile range's `tx_min` and `ty_max`. The northernmost tile (`ty == ty_max`) lands at
/// texture row 0 so the atlas is north-up before the `corner_uv` flip. Tile size is 256 px.
#[must_use]
pub fn pack_offset(tx: u32, ty: u32, tx_min: u32, ty_max: u32) -> (u32, u32) {
    ((tx - tx_min) * 256, (ty_max - ty) * 256)
}

/// Anchor-relative-meters `[minX, minY, maxX, maxY]` (f32) for a world rect — the textured-quad
/// instance geometry, matching the `scene::QuadInstance` anchor contract.
#[must_use]
pub fn world_rect_rel(min: [f64; 2], max: [f64; 2]) -> [f32; 4] {
    let a = rel(min[0], min[1]);
    let b = rel(max[0], max[1]);
    [a[0], a[1], b[0], b[1]]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Class R: north-up UV. NW (unit y=1) → texture top (v=0); SW (unit y=0) → bottom (v=1).
    /// A sign flip on the `1 - y` swaps these two and fails.
    #[test]
    fn corner_uv_is_north_up() {
        assert_eq!(corner_uv([0.0, 1.0]), [0.0, 0.0]); // NW world → texture top-left
        assert_eq!(corner_uv([1.0, 1.0]), [1.0, 0.0]); // NE → top-right
        assert_eq!(corner_uv([0.0, 0.0]), [0.0, 1.0]); // SW → bottom-left
        assert_eq!(corner_uv([1.0, 0.0]), [1.0, 1.0]); // SE → bottom-right
    }

    /// Class R: pyramid pack offsets. Northernmost visible tile (ty = ty_max) → row 0.
    #[test]
    fn pack_offset_places_north_at_top() {
        // Visible range tx ∈ [2,4], ty ∈ [5,7] (ty_max = 7).
        assert_eq!(pack_offset(2, 7, 2, 7), (0, 0)); // NW corner tile → atlas origin
        assert_eq!(pack_offset(4, 7, 2, 7), (512, 0)); // east edge, top row
        assert_eq!(pack_offset(2, 5, 2, 7), (0, 512)); // west edge, bottom row (southmost)
        assert_eq!(pack_offset(3, 6, 2, 7), (256, 256)); // interior
    }

    /// Class R: world→anchor-relative meters (Everon full extent → [-6400,6400]²).
    #[test]
    fn world_rect_rel_is_anchor_relative() {
        assert_eq!(
            world_rect_rel([0.0, 0.0], [12_800.0, 12_800.0]),
            [-6400.0, -6400.0, 6400.0, 6400.0]
        );
    }

    /// Class R pinned-bytes: the Everon grid geometry + colors, mirroring `useBaseMapLayer.ts`.
    #[test]
    fn grid_lines_everon_pinned() {
        let lines = grid_lines(12_800.0, 12_800.0, false);
        // 13 verticals (x = 0,1000,…,12000) + 13 horizontals, 2 vertices each.
        assert_eq!(lines.len(), 52);

        // First vertical is x=0 (border): source [0,0] → rel [-6400,-6400], BORDER color.
        assert_eq!(lines[0].pos, [-6400.0, -6400.0]);
        assert_eq!(lines[0].color, norm(BORDER));
        // Its target [0,height] → rel [-6400, 6400].
        assert_eq!(lines[1].pos, [-6400.0, 6400.0]);

        // The 6th vertical is x=5000 (major): index 2*5 = 10, source [5000,0] → rel [-1400,-6400].
        assert_eq!(lines[10].pos, [-1400.0, -6400.0]);
        assert_eq!(lines[10].color, norm(MAJOR));

        // The 2nd vertical is x=1000 (minor): index 2, rel [-5400,-6400].
        assert_eq!(lines[2].pos, [-5400.0, -6400.0]);
        assert_eq!(lines[2].color, norm(MINOR));

        // First horizontal (index 26) is y=0 (border): source [0,0] → rel [-6400,-6400].
        assert_eq!(lines[26].pos, [-6400.0, -6400.0]);
        assert_eq!(lines[26].color, norm(BORDER));
        // Its target [width,0] → rel [6400,-6400].
        assert_eq!(lines[27].pos, [6400.0, -6400.0]);
    }

    /// Class R: the over-hillshade palette swaps in (boosted alphas).
    #[test]
    fn grid_lines_hs_palette() {
        let lines = grid_lines(12_800.0, 12_800.0, true);
        assert_eq!(lines[0].color, norm(BORDER_HS));
        assert_eq!(lines[10].color, norm(MAJOR_HS));
        assert_eq!(lines[2].color, norm(MINOR_HS));
    }
}
