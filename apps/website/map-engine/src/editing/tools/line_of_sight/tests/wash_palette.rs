//! Role: the viewshed colour language, its rationale citation, and the raster encoder.
//! Position: `editing/tools/line_of_sight/tests` in the map engine.
//! Signals & state: explicit profiles, shots and rasters built in the test body.
//! Invariants: texture row 0 is the raster's north edge; the palette bytes are a contract and are pinned exactly.

use super::*;

use crate::spatial::los::terrain::viewshed::Viewshed;
use crate::spatial::los::terrain::viewshed::Visibility;

/// (`vs_textured`, uv = (x, 1.0 − unit.y)) maps texture ROW 0 to world MAX-Y (north); the
/// raster's row 0 is world MIN-Y (south). The encoder must therefore emit rows in REVERSE
/// (north first), exactly as the forest lane's pack_island_r8_yflip does. This pin plants one
/// Hidden cell at the raster's SOUTH-WEST corner (r=0, c=0) and asserts its bytes land in the
/// texture's LAST row, first column — the flipped offset. Against the unflipped encoder this
/// fails with the hidden bytes at offset 0.
#[test]
fn encoder_flips_rows_so_north_is_texture_row_zero() {
    let mut vs = crate::spatial::los::terrain::viewshed::Viewshed {
        cols: 3,
        rows: 2,
        cells: vec![crate::spatial::los::terrain::viewshed::Visibility::Visible; 6],
        min_x: 0.0,
        min_y: 0.0,
        max_x: 16.0,
        max_y: 8.0,
        obs_x: 0.0,
        obs_y: 0.0,
    };
    // South-west corner of the WORLD raster (row 0 = min_y).
    vs.cells[0] = crate::spatial::los::terrain::viewshed::Visibility::Hidden;
    let rgba = encode_viewshed_rgba(&vs);
    let px = |r: usize, c: usize| &rgba[(r * vs.cols + c) * 4..(r * vs.cols + c) * 4 + 4];
    assert_eq!(
        px(1, 0),
        &VIEWSHED_HIDDEN_RGBA,
        "world SW cell must land in the texture's LAST row (shader maps row 0 to north)"
    );
    assert_ne!(
        px(0, 0),
        &VIEWSHED_HIDDEN_RGBA,
        "texture row 0 col 0 is world NW here — it must NOT carry the SW cell's bytes"
    );
}

/// PALETTE CONSTANTS PIN (the ticket's required "palette constants + rationale pin"). The colour
/// language is a contract, so pin the exact bytes: HIDDEN is a desaturated dark near-neutral at
/// α0.38, VISIBLE is fully transparent (the untouched map — the conventional army answer), UNKNOWN
/// is the same dark but LIGHTER (α0.22). A change to any of these is a deliberate colour-language
/// decision that must update this pin.
#[test]
fn viewshed_palette_constants_are_pinned() {
    assert_eq!(VIEWSHED_HIDDEN_RGBA, [24, 26, 36, 97], "HIDDEN wash pinned");
    assert_eq!(VIEWSHED_VISIBLE_RGBA, [0, 0, 0, 0], "VISIBLE = transparent");
    assert_eq!(
        VIEWSHED_UNKNOWN_RGBA,
        [24, 26, 36, 56],
        "UNKNOWN wash pinned"
    );
    // The colour-language INVARIANTS the rationale rests on (checked, not just asserted in prose):
    // VISIBLE is fully transparent (no ink on seen ground — the whole conventional-answer point).
    assert_eq!(VIEWSHED_VISIBLE_RGBA[3], 0, "visible ground is never inked");
    // HIDDEN is more opaque than UNKNOWN (proven dead ground reads darker than a coverage hole),
    // and BOTH are translucent enough to let the α235 T-640 contours show through (α well under
    // the contour's 235). If HIDDEN's alpha ever climbed to/over the contour alpha the hairlines
    // would drown — the derivation in the module rationale.
    assert!(
        VIEWSHED_HIDDEN_RGBA[3] > VIEWSHED_UNKNOWN_RGBA[3],
        "hidden (dead ground) must read darker than unknown (a coverage hole)"
    );
    assert!(
        u32::from(VIEWSHED_HIDDEN_RGBA[3]) < 235,
        "the wash alpha must stay under the T-640 contour alpha (235) so contours show through"
    );
    // Near-neutral with a faint COOL cast (b ≥ r) so the wash can never be mistaken for the WARM
    // brown contour (`CONTOUR_RGBA = [188,150,100]`, r > g > b). This is the hue half of "don't
    // fight the contours".
    assert!(
        VIEWSHED_HIDDEN_RGBA[2] >= VIEWSHED_HIDDEN_RGBA[0],
        "the wash is cool/neutral (b ≥ r), never a warm brown like the contours"
    );
}

/// The palette rationale CITES the live contour RGBA from `dem_vectors.rs` — pin that the exact
/// values the rationale quotes still match the source of truth, so the citation can't rot. Reads
/// the scrubbed `dem_vectors.rs` for the literal `[188, 150, 100, 235]` (base) and
/// `[174, 145, 123, 235]` (summit). If T-640's contour colour is retuned, THIS fails and forces
/// the viewshed alpha rationale to be re-derived against the new contour alpha.
#[test]
fn viewshed_rationale_cites_live_contour_rgba() {
    let relief_host = include_str!("../../../../world/terrain/relief/host.rs");
    assert!(
        relief_host.contains("[188, 150, 100, 235]"),
        "the wash rationale cites CONTOUR_RGBA = [188,150,100,235]; dem_vectors.rs must still define \
         it (retuning the contour colour must re-derive the viewshed wash alpha)"
    );
    assert!(
        relief_host.contains("[174, 145, 123, 235]"),
        "the wash rationale cites CONTOUR_SUMMIT_RGBA = [174,145,123,235]; dem_vectors.rs must still \
         define it"
    );
    // And the los_tool rationale block actually quotes them (guards against the comment being
    // dropped in a future edit while the constants stay).
    let los = include_str!("../wash_palette.rs");
    assert!(
        los.contains("CONTOUR_RGBA = [188, 150, 100, 235]")
            && los.contains("CONTOUR_SUMMIT_RGBA = [174, 145, 123, 235]"),
        "the viewshed palette rationale must cite the contour RGBA values by number"
    );
}

/// The per-cell encoder maps each class to its palette byte-for-byte, and the whole-raster encode
/// produces `cols*rows*4` bytes in row-major order.
#[test]
fn viewshed_encoder_maps_classes_and_sizes() {
    assert_eq!(
        viewshed_cell_rgba(Visibility::Visible),
        VIEWSHED_VISIBLE_RGBA
    );
    assert_eq!(viewshed_cell_rgba(Visibility::Hidden), VIEWSHED_HIDDEN_RGBA);
    assert_eq!(
        viewshed_cell_rgba(Visibility::Unknown),
        VIEWSHED_UNKNOWN_RGBA
    );
    // A 2×2 raster: V H / U V → 16 bytes, each cell's 4 bytes in order.
    let vs = Viewshed {
        cols: 2,
        rows: 2,
        cells: vec![
            Visibility::Visible,
            Visibility::Hidden,
            Visibility::Unknown,
            Visibility::Visible,
        ],
        min_x: 0.0,
        min_y: 0.0,
        max_x: 8.0,
        max_y: 8.0,
        obs_x: 0.0,
        obs_y: 0.0,
    };
    let rgba = encode_viewshed_rgba(&vs);
    assert_eq!(rgba.len(), 2 * 2 * 4, "cols*rows*4 bytes");
    // Rows emit NORTH-FIRST (the shader's row-0 = max_y contract; wave-110 BLOCKER-1 fix):
    // texture row 0 carries world row 1 (U V), texture row 1 carries world row 0 (V H).
    assert_eq!(
        &rgba[0..4],
        &VIEWSHED_UNKNOWN_RGBA,
        "tex row 0 col 0 = world NW = unknown"
    );
    assert_eq!(
        &rgba[4..8],
        &VIEWSHED_VISIBLE_RGBA,
        "tex row 0 col 1 = world NE = visible"
    );
    assert_eq!(
        &rgba[8..12],
        &VIEWSHED_VISIBLE_RGBA,
        "tex row 1 col 0 = world SW = visible"
    );
    assert_eq!(
        &rgba[12..16],
        &VIEWSHED_HIDDEN_RGBA,
        "tex row 1 col 1 = world SE = hidden"
    );
}

// ── T-644 — the LoS sub-mode toggle (Ray ⇆ Viewshed) ─────────────────────────────────────────
