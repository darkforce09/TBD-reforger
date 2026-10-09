//! Role: the viewshed colour language, its rationale citation, and the raster encoder.
//! Position: `line_of_sight/tests` in `map_editing_tools`.
//! Signals & state: explicit profiles, shots and rasters built in the test body.
//! Invariants: texture row 0 is the raster's north edge; the palette bytes are a contract and are pinned exactly.

use super::*;

use terrain_line_of_sight::viewshed::Viewshed;
use terrain_line_of_sight::viewshed::Visibility;

/// (`vs_textured`, uv = (x, 1.0 − unit.y)) maps texture ROW 0 to world MAX-Y (north); the
/// raster's row 0 is world MIN-Y (south). The encoder must therefore emit rows in REVERSE
/// (north first), exactly as the forest lane's pack_island_r8_yflip does. This pin plants one
/// Hidden cell at the raster's SOUTH-WEST corner (r=0, c=0) and asserts its bytes land in the
/// texture's LAST row, first column — the flipped offset. Against the unflipped encoder this
/// fails with the hidden bytes at offset 0.
#[test]
fn encoder_flips_rows_so_north_is_texture_row_zero() {
    let mut vs = terrain_line_of_sight::viewshed::Viewshed {
        cols: 3,
        rows: 2,
        cells: vec![terrain_line_of_sight::viewshed::Visibility::Visible; 6],
        min_x: 0.0,
        min_y: 0.0,
        max_x: 16.0,
        max_y: 8.0,
        obs_x: 0.0,
        obs_y: 0.0,
    };
    // South-west corner of the WORLD raster (row 0 = min_y).
    vs.cells[0] = terrain_line_of_sight::viewshed::Visibility::Hidden;
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
