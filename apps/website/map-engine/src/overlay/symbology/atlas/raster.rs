//! Role: raster.
//! Position: `overlay/symbology/atlas` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.
//!
//! T-0xx Phase 1D — **this file STAYS, against the engine-split spec's own §1D table.**
//! That table sends `symbology/atlas/` wholesale to the renderer's `text/`; §2's acceptance
//! grep (`terrain|symbology|mission|orbat|arma`, case-insensitive, over
//! `graphics-engine/src`) forbids it, and the two cannot both be satisfied. The grep is the
//! one wired into CI, so the grep wins.
//!
//! It is also the right answer on the merits. The cells this module rasterises are ORBAT
//! vocabulary, not glyphs: [`SYMBOLOGY_CELL_COUNT`] is 15 because there are ten unit classes,
//! three vehicle silhouettes and one comment bubble; [`UNIT_CELL_BASE`],
//! [`VEHICLE_CELL_BASE`] and [`COMMENT_CELL`] are where each block starts;
//! [`extend_atlas_with_unit_glyphs`] appends them and [`WidenedSlotAtlas`] reports where they
//! landed. A renderer that held this would know what an infantry section looks like.
//!
//! What DID cross is the half that has no vocabulary in it: the texture upload, the uniform
//! buffer and the bind-group build now live in `website-graphics-engine`'s `text::gpu`, which
//! takes an already-packed uniform block and never reads a cell index.

/// Slot/cluster atlas dimensions — two 64 px cells side by side (ring | disc), the `slotAtlas.ts` contract the engine's UV table + pipeline were built against.
pub const SLOT_ATLAS_W: u32 = 128;

/// Canonical slot atlas h value.
pub const SLOT_ATLAS_H: u32 = 64;

/// Flat per-glyph UV table: minU,minV,maxU,maxV for ring (glyph 0) and disc (glyph 1).
pub const SLOT_ATLAS_UV: [f32; 8] = [0.0, 0.0, 0.5, 1.0, 0.5, 0.0, 1.0, 1.0];

/// Slot atlas.
pub struct SlotAtlas {
    /// `SLOT_ATLAS_W × SLOT_ATLAS_H` straight-alpha RGBA, white-on-alpha (tint multiplies).
    pub rgba: Vec<u8>,

    /// Width.
    pub width: u32,

    /// Height.
    pub height: u32,

    /// Uv.
    pub uv: [f32; 8],
}

/// Build the two-glyph atlas: glyph 0 = ring (outer r 24, inner r 10), glyph 1 = solid disc (r 26) — the `slotAtlas.ts` radii. 1 px analytic edge coverage stands in for canvas arc AA (visually equivalent at the 20–28 px render sizes).
#[must_use]
pub fn build_slot_atlas() -> SlotAtlas {
    let (w, h) = (SLOT_ATLAS_W as usize, SLOT_ATLAS_H as usize);
    let mut rgba = vec![0u8; w * h * 4];

    let cov = |d: f64, r: f64| (r + 0.5 - d).clamp(0.0, 1.0);
    for y in 0..h {
        for x in 0..w {
            let (cx, ring) = if x < 64 { (32.0, true) } else { (96.0, false) };
            let dx = x as f64 + 0.5 - cx;
            let dy = y as f64 + 0.5 - 32.0;
            let d = (dx * dx + dy * dy).sqrt();
            let a = if ring {
                cov(d, 24.0) - cov(d, 10.0)
            } else {
                cov(d, 26.0)
            };
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let a8 = (a.clamp(0.0, 1.0) * 255.0).round() as u8;
            let i = (y * w + x) * 4;
            rgba[i..i + 4].copy_from_slice(&[255, 255, 255, a8]);
        }
    }
    SlotAtlas {
        rgba,
        width: SLOT_ATLAS_W,
        height: SLOT_ATLAS_H,
        uv: SLOT_ATLAS_UV,
    }
}

/// Offset of the unselected unit block: cell `UNIT_CELL_BASE + (class as u16)`.
pub const UNIT_CELL_BASE: u16 = 0;

/// Offset of the SELECTED unit block — the same five role glyphs with the selection RING added around them, so "selection ring + heading + role" is one instance and the O(delta) row patch in `set_selection` keeps working (a second overlay instance per selected row would force a full re-pack on every click).
pub const UNIT_SELECTED_CELL_BASE: u16 = 5;

/// Offset of the vehicle silhouette block: cell `VEHICLE_CELL_BASE + (kind as u16)`.
pub const VEHICLE_CELL_BASE: u16 = 10;

/// Offset of the comment speech-bubble cell.
pub const COMMENT_CELL: u16 = 13;

/// Offset of the SELECTED comment cell (bubble + selection ring).
pub const COMMENT_SELECTED_CELL: u16 = 14;

/// Total symbology cells [`extend_atlas_with_unit_glyphs`] appends.
pub const SYMBOLOGY_CELL_COUNT: usize = 15;

/// Cell edge in pixels — the slot/marker atlas convention (`build_slot_atlas`, `scene::build_marker_slot_atlas`). An incoming atlas that is not a horizontal strip of these cells is refused by [`extend_atlas_with_unit_glyphs`] rather than mangled.
pub const ATLAS_CELL_PX: u32 = 64;

/// Edge.
#[must_use]
pub(crate) fn edge(s: f64) -> f64 {
    (s + 0.5).clamp(0.0, 1.0)
}

/// Facing point cov.
#[must_use]
pub(crate) fn facing_point_cov(dx: f64, dy: f64) -> f64 {
    let t = ((dy + 30.0) / 20.0).clamp(0.0, 1.0);
    edge(dy + 30.0)
        .min(edge(-10.0 - dy))
        .min(edge(11.0f64.mul_add(t, -dx.abs())))
}

/// Role knockout cov.
#[must_use]
pub(crate) fn role_knockout_cov(class: u16, dx: f64, dy: f64) -> f64 {
    match class {
        0 => 0.0,

        1 => {
            let a = edge(3.5 - (dy + 2.0 + dx).abs());
            let b = edge(3.5 - (dy + 2.0 - dx).abs());
            a.max(b)
                .min(edge(dy + 10.0))
                .min(edge(9.0 - dy))
                .min(edge(13.0 - dx.abs()))
        }

        2 => {
            let v = edge(4.0 - dx.abs()).min(edge(12.0 - dy.abs()));
            let h = edge(4.0 - dy.abs()).min(edge(12.0 - dx.abs()));
            v.max(h)
        }

        3 => {
            let t = ((12.0 - dy) / 22.0).clamp(0.0, 1.0);
            edge(12.0 - dy)
                .min(edge(dy + 10.0))
                .min(edge(11.0f64.mul_add(t, -dx.abs())))
        }

        4 => {
            let top = edge(3.0 - (dy + 6.0).abs());
            let bot = edge(3.0 - (dy - 6.0).abs());
            top.max(bot).min(edge(12.0 - dx.abs()))
        }
        _ => 0.0,
    }
}

/// Selection ring cov.
#[must_use]
pub(crate) fn selection_ring_cov(d: f64) -> f64 {
    let cov = |r: f64| (r + 0.5 - d).clamp(0.0, 1.0);
    cov(30.0) - cov(25.0)
}

/// Vehicle silhouette cov.
#[must_use]
pub(crate) fn vehicle_silhouette_cov(kind: u16, dx: f64, dy: f64) -> f64 {
    let box_cov = |cx: f64, cy: f64, hx: f64, hy: f64| {
        edge(hx - (dx - cx).abs()).min(edge(hy - (dy - cy).abs()))
    };
    match kind {
        0 => {
            let hw = if dy < -8.0 {
                5.5f64.mul_add(-((-8.0 - dy) / 12.0), 9.0)
            } else {
                9.0
            };
            let hull = edge(20.0 - dy.abs()).min(edge(hw - dx.abs()));
            hull.max(box_cov(-13.0, -12.0, 4.0, 6.0))
                .max(box_cov(13.0, -12.0, 4.0, 6.0))
                .max(box_cov(-13.0, 12.0, 4.0, 6.0))
                .max(box_cov(13.0, 12.0, 4.0, 6.0))
        }

        1 => {
            let cab = edge(dy + 26.0)
                .min(edge(-14.0 - dy))
                .min(edge(10.0 - dx.abs()));
            let bed = edge(dy + 11.0)
                .min(edge(26.0 - dy))
                .min(edge(11.0 - dx.abs()));
            cab.max(bed)
                .max(box_cov(-14.0, -18.0, 3.5, 5.0))
                .max(box_cov(14.0, -18.0, 3.5, 5.0))
                .max(box_cov(-14.0, 10.0, 3.5, 5.0))
                .max(box_cov(14.0, 10.0, 3.5, 5.0))
                .max(box_cov(-14.0, 20.0, 3.5, 5.0))
                .max(box_cov(14.0, 20.0, 3.5, 5.0))
        }

        2 => {
            let hw = if dy < -10.0 {
                5.0f64.mul_add(-((-10.0 - dy) / 12.0), 11.0)
            } else {
                11.0
            };
            let hull = edge(dy + 22.0)
                .min(edge(22.0 - dy))
                .min(edge(hw - dx.abs()));
            let rail = edge(dy + 20.0)
                .min(edge(22.0 - dy))
                .min(edge(2.5 - (dx.abs() - 14.5).abs()));
            hull.max(rail)
        }
        _ => 0.0,
    }
}

/// Comment bubble cov.
#[must_use]
pub(crate) fn comment_bubble_cov(dx: f64, dy: f64) -> f64 {
    let rrect = |hx: f64, hy: f64, r: f64| {
        let qx = (dx.abs() - hx).max(0.0);
        let qy = ((dy + 6.0).abs() - hy).max(0.0);
        edge(r - qx.hypot(qy))
    };
    let outline = (rrect(16.0, 8.0, 6.0) - rrect(12.0, 4.0, 5.0)).clamp(0.0, 1.0);

    let t = ((16.0 - dy) / 14.0).clamp(0.0, 1.0);
    let tail = edge(dy - 1.0)
        .min(edge(16.0 - dy))
        .min(edge(5.0f64.mul_add(t, -(dx + 9.0).abs())));
    outline.max(tail)
}

/// Symbology cell coverage.
#[must_use]
pub(crate) fn symbology_cell_coverage(offset: u16, px: f64, py: f64) -> f64 {
    let dx = px + 0.5 - 32.0;
    let dy = py + 0.5 - 32.0;
    let d = dx.hypot(dy);
    let body = (20.5 - d).clamp(0.0, 1.0);
    match offset {
        0..=9 => {
            let class = offset % 5;
            let unit = (body - role_knockout_cov(class, dx, dy))
                .clamp(0.0, 1.0)
                .max(facing_point_cov(dx, dy));
            if offset >= UNIT_SELECTED_CELL_BASE {
                unit.max(selection_ring_cov(d))
            } else {
                unit
            }
        }
        10..=12 => vehicle_silhouette_cov(offset - VEHICLE_CELL_BASE, dx, dy),
        COMMENT_CELL => comment_bubble_cov(dx, dy),
        COMMENT_SELECTED_CELL => comment_bubble_cov(dx, dy).max(selection_ring_cov(d)),
        _ => 0.0,
    }
}

/// Widened slot atlas.
pub struct WidenedSlotAtlas {
    /// Straight-alpha RGBA8, white-on-alpha (the instance tint multiplies).
    pub rgba: Vec<u8>,

    /// Width.
    pub width: u32,

    /// Height.
    pub height: u32,

    /// Flat `[minU,minV,maxU,maxV]·N` table over ALL cells (base + symbology).
    pub uv: Vec<f32>,

    /// Base cells.
    pub base_cells: u16,
}

/// Append the [`SYMBOLOGY_CELL_COUNT`] unit / vehicle / comment cells to a caller-built slot atlas.
#[must_use]
pub fn extend_atlas_with_unit_glyphs(
    base_rgba: &[u8],
    base_w: u32,
    base_h: u32,
) -> Option<WidenedSlotAtlas> {
    let cell = ATLAS_CELL_PX as usize;
    if base_h != ATLAS_CELL_PX || base_w == 0 || !base_w.is_multiple_of(ATLAS_CELL_PX) {
        return None;
    }
    let bw = base_w as usize;
    if base_rgba.len() != bw * cell * 4 {
        return None;
    }
    let base_cells = bw / cell;
    let n = base_cells + SYMBOLOGY_CELL_COUNT;
    let w = n * cell;
    let mut rgba = vec![0u8; w * cell * 4];
    for y in 0..cell {
        let src = y * bw * 4;
        let dst = y * w * 4;
        rgba[dst..dst + bw * 4].copy_from_slice(&base_rgba[src..src + bw * 4]);
        for c in 0..SYMBOLOGY_CELL_COUNT {
            let cx0 = (base_cells + c) * cell;
            for x in 0..cell {
                #[allow(clippy::cast_precision_loss)]
                let a = symbology_cell_coverage(c as u16, x as f64, y as f64);
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let a8 = (a.clamp(0.0, 1.0) * 255.0).round() as u8;
                let i = (y * w + cx0 + x) * 4;
                rgba[i..i + 4].copy_from_slice(&[255, 255, 255, a8]);
            }
        }
    }
    let mut uv = Vec::with_capacity(n * 4);
    #[allow(clippy::cast_precision_loss)]
    for c in 0..n {
        uv.extend_from_slice(&[c as f32 / n as f32, 0.0, (c + 1) as f32 / n as f32, 1.0]);
    }
    #[allow(clippy::cast_possible_truncation)]
    Some(WidenedSlotAtlas {
        rgba,
        width: w as u32,
        height: cell as u32,
        uv,
        base_cells: base_cells as u16,
    })
}
