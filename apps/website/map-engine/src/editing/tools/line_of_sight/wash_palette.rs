//! Role: the viewshed wash colour language and its raster encoder.
//! Position: `editing/tools/line_of_sight` in the map engine.
//! Signals & state: session-local measurement state; never the authored document.
//! Invariants: hidden ground carries the ink and visible ground stays untouched; off-coverage reads lighter than proven dead ground and never as visible. Texture row 0 is the raster's north edge.

use crate::spatial::los::terrain::viewshed::Viewshed;
use crate::spatial::los::terrain::viewshed::Visibility;

// ── T-644 — the viewshed COLOUR LANGUAGE (the hard part; palette + written rationale) ────────────
//
// A viewshed wash shares the SAME map surface as two lanes that were already tuned for legibility on
// this basemap, and it must not fight either:
//
//   * T-640's TWO-TONE BROWN CONTOURS (`world_assets/dem_vectors.rs`). Cited verbatim so this
//     rationale is checkable, not vibes: the base contour is `CONTOUR_RGBA = [188, 150, 100, 235]`
//     and the per-peak summit ring `CONTOUR_SUMMIT_RGBA = [174, 145, 123, 235]`. Both are WARM
//     (r > g > b), 1 px, and drawn at α ≈ 0.92 (235/255) — effectively opaque hairlines that carry
//     the terrain's shape.
//   * the LANDCOVER / forest-density washes, which are GREENS and greys over the same hillshade.
//
// THE CONVENTIONAL ARMY ANSWER, and the one this ticket adopts: shade what the observer CANNOT see;
// leave what it CAN see as the untouched map. "Dead ground" is the thing a planner scans for, so the
// ink goes on the HIDDEN cells and VISIBLE cells stay pristine (no green/brown is added over ground
// the operator can already read). That immediately settles the hue question — the wash must be a
// NEUTRAL DESATURATED DARK, not a colour, so it reads as "shadow / no-data" rather than as a third
// thematic layer competing with the warm contours and the green landcover.
//
//   HIDDEN  = a desaturated dark wash at LOW alpha. Near-neutral (a hair of cool blue so it never
//             reads as "brown contour" and never as "green forest"), dark, and TRANSLUCENT so the
//             hillshade relief + the brown contour hairlines show straight THROUGH it. Alpha is the
//             whole game: too high and the α0.92 contours drown; too low and the dead-ground read is
//             lost. Chosen α = 0.38. Rationale for that number, against the cited contour values: a
//             1 px contour at α0.92 composited UNDER a full-cell wash at α0.38 keeps
//             `0.62 × 235 ≈ 146` of its 235 source alpha showing through — the contour stays a clearly
//             visible hairline (well above the ~α0.3/​luma-155 floor T-175 A3 set for contour
//             legibility on both basemaps), while the wash is still solid enough over a multi-cell
//             dead-ground pocket to read as a distinct dark region. A HIDDEN cell darkens the map by
//             ~38%; a lone contour pixel crossing it dips only where it crosses, so the line reads
//             continuous.
//   VISIBLE = the untouched map — α 0 (fully transparent). No ink at all: the conventional answer,
//             and it guarantees zero fight with contours/landcover on the ground that matters most.
//   UNKNOWN = off-coverage (constraint 1). Rendered as HIDDEN but a shade LIGHTER (α 0.22) so a
//             coverage hole is visually distinct from proven dead ground without ever masquerading as
//             visible — an honest "can't tell", never a fake CLEAR (the em-dash policy, in pixels).
//
// The wash is a per-cell RGBA raster uploaded as ONE texture over the world rect (the engine's
// texture-lane shape), so it is a single translucent quad the GPU blends over the map in one draw —
// it never touches the contour or landcover geometry, only composites above them.

/// HIDDEN-cell wash colour, straight RGBA8 `[r, g, b, a]`. A desaturated dark near-neutral (faint
/// cool cast, `b > r` by 12 so it can never be mistaken for the warm brown contour) at α 0.38 — dark
/// enough to read as dead ground, translucent enough that the α0.92 T-640 contours + the hillshade
/// show through (see the module rationale for the alpha derivation against `CONTOUR_RGBA`).
pub const VIEWSHED_HIDDEN_RGBA: [u8; 4] = [24, 26, 36, 97]; // 97/255 ≈ 0.38

/// VISIBLE-cell colour: fully transparent — the untouched map (the conventional army answer). No ink
/// on ground the observer can see, so the wash never competes with contours/landcover where it matters.
pub const VIEWSHED_VISIBLE_RGBA: [u8; 4] = [0, 0, 0, 0];

/// UNKNOWN-cell (off-coverage) colour: the same neutral dark as HIDDEN but a shade LIGHTER (α 0.22),
/// so a coverage hole is distinct from proven dead ground yet still clearly NOT visible — the honest
/// "can't tell" (constraint 1: off-coverage renders hidden-ish, never fake-visible).
pub const VIEWSHED_UNKNOWN_RGBA: [u8; 4] = [24, 26, 36, 56]; // 56/255 ≈ 0.22

/// Map one [`Visibility`] class to its wash RGBA8 (the palette above). Pure + native-tested so the
/// colour language is proved without a GPU: Visible → transparent, Hidden → the dark wash, Unknown →
/// the lighter dark wash.
#[must_use]
pub fn viewshed_cell_rgba(v: Visibility) -> [u8; 4] {
    match v {
        Visibility::Visible => VIEWSHED_VISIBLE_RGBA,
        Visibility::Hidden => VIEWSHED_HIDDEN_RGBA,
        Visibility::Unknown => VIEWSHED_UNKNOWN_RGBA,
    }
}

/// Encode a computed [`Viewshed`] into a row-major RGBA8 byte buffer (`cols * rows * 4`) via
/// [`viewshed_cell_rgba`], ready to upload as one texture over the viewshed's world rect. Pure (no
/// GPU, no wasm) so the encoding is native-testable; the wasm host hands the bytes + the world rect
/// straight to the engine's viewshed texture lane. Texture row 0 is the raster's `max_y` (north)
/// edge — the shader's `uv = (x, 1.0 − unit.y)` contract — so rows emit in reverse, exactly as the
/// forest-density lane's `pack_island_r8_yflip` does. (The previous claim here that `flip_y:false`
/// puts world-min at row 0 was false on both counts and shipped a north-south mirrored wash.)
#[must_use]
pub fn encode_viewshed_rgba(vs: &Viewshed) -> Vec<u8> {
    // ROWS EMIT IN REVERSE — north first. The shader (`vs_textured`, uv = (x, 1.0 − unit.y)) maps
    // texture row 0 to world MAX-Y, and the raster's row 0 is world MIN-Y; emitting in natural
    // order mirrored the wash north-south (wave-110 verifier BLOCKER-1 — dead ground computed
    // north of a ridge shaded SOUTH on screen). Same flip pack_island_r8_yflip does for the
    // forest lane. The bridge pin below (`encoder_flips_rows_so_north_is_texture_row_zero`) is
    // what was missing: it ties encoder row order to the shader's UV contract.
    let mut out = Vec::with_capacity(vs.cols * vs.rows * 4);
    for r in (0..vs.rows).rev() {
        let base = r * vs.cols;
        for c in 0..vs.cols {
            out.extend_from_slice(&viewshed_cell_rgba(vs.cells[base + c]));
        }
    }
    out
}

#[cfg(test)]
#[path = "tests/wash_palette.rs"]
mod tests;
