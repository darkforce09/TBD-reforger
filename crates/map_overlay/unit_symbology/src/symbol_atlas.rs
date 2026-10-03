//! The symbol atlas: the slot ring and disc cells, and the unit, vehicle and comment cells
//! appended to a slot atlas.
//!
//! **Role:** rasterises the two-cell slot atlas ([`build_slot_atlas`]) and appends the
//! [`SYMBOLOGY_CELL_COUNT`] symbology cells to a caller-built slot atlas
//! ([`extend_atlas_with_unit_glyphs`]): five unit-role glyphs unselected from
//! [`UNIT_CELL_BASE`] and selected from [`UNIT_SELECTED_CELL_BASE`], three vehicle silhouettes
//! from [`VEHICLE_CELL_BASE`], and the comment bubble unselected ([`COMMENT_CELL`]) and
//! selected ([`COMMENT_SELECTED_CELL`]); [`WidenedSlotAtlas`] reports where they landed.
//! **Position:** `unit_symbology`; the overlay instance packers index these cells, and the map
//! engine's slot atlas bridge uploads the pixels through the graphics engine's `frame::atlas`,
//! which takes a packed uniform block and never reads a cell index.
//! **Signals & state:** none; pure rasterisation into owned buffers.
//! **Invariants:** the cells are ORBAT vocabulary, so they live with the map, never in the
//! renderer; every cell is [`ATLAS_CELL_PX`] square, white on alpha, so the instance tint
//! multiplies; an incoming atlas that is not a horizontal strip of such cells is refused.

/// Slot/cluster atlas dimensions — two 64 px cells side by side (ring | disc), the layout the engine's UV table and pipeline are built against.
pub const SLOT_ATLAS_W: u32 = 128;

/// Canonical slot atlas h value.
pub const SLOT_ATLAS_H: u32 = 64;

/// Flat per-glyph UV table: minU,minV,maxU,maxV for ring (glyph 0) and disc (glyph 1).
pub const SLOT_ATLAS_UV: [f32; 8] = [0.0, 0.0, 0.5, 1.0, 0.5, 0.0, 1.0, 1.0];

/// The two-cell slot atlas [`build_slot_atlas`] rasterises: ring (glyph 0) and solid disc
/// (glyph 1), with their UV table.
pub struct SlotAtlas {
    /// `SLOT_ATLAS_W × SLOT_ATLAS_H` straight-alpha RGBA, white-on-alpha (tint multiplies).
    pub rgba: Vec<u8>,

    /// Atlas width in pixels, [`SLOT_ATLAS_W`] (two 64-pixel cells).
    pub width: u32,

    /// Atlas height in pixels, [`SLOT_ATLAS_H`] (one cell row).
    pub height: u32,

    /// Per-glyph `[minU, minV, maxU, maxV]` for ring then disc, [`SLOT_ATLAS_UV`].
    pub uv: [f32; 8],
}

/// Build the two-glyph atlas: glyph 0 = ring (outer r 24, inner r 10), glyph 1 = solid disc (r 26) — the slot ring and disc radii. 1 px analytic edge coverage stands in for canvas arc AA (visually equivalent at the 20–28 px render sizes).
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

/// Cell edge in pixels — the slot/marker atlas convention ([`build_slot_atlas`], [`crate::markers::build_marker_slot_atlas`]). An incoming atlas that is not a horizontal strip of these cells is refused by [`extend_atlas_with_unit_glyphs`] rather than mangled.
pub const ATLAS_CELL_PX: u32 = 64;

/// Anti-aliased coverage in `[0, 1]` of a signed distance `s` in pixels (positive inside),
/// ramping over one pixel centred on the edge.
#[must_use]
pub(crate) fn edge(s: f64) -> f64 {
    (s + 0.5).clamp(0.0, 1.0)
}

/// Coverage of the heading pointer: a triangle above the unit body with its apex 30 pixels
/// above the cell centre and its 22-pixel base 10 pixels above it (`dx`, `dy` from the centre,
/// `dy` growing downwards).
#[must_use]
pub(crate) fn facing_point_cov(dx: f64, dy: f64) -> f64 {
    let t = ((dy + 30.0) / 20.0).clamp(0.0, 1.0);
    edge(dy + 30.0)
        .min(edge(-10.0 - dy))
        .min(edge(11.0f64.mul_add(t, -dx.abs())))
}

/// Coverage of the role symbol knocked out of the unit body for a
/// [`crate::classification::UnitRoleClass`] discriminant: none (rifleman), chevron (leader),
/// plus (medic), down-triangle (anti-tank), twin bars (machine gun); 0 for any other class.
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

/// Coverage of the selection ring around a unit or comment glyph: radii 25 to 30 pixels from
/// the cell centre at distance `d`.
#[must_use]
pub(crate) fn selection_ring_cov(d: f64) -> f64 {
    let cov = |r: f64| (r + 0.5 - d).clamp(0.0, 1.0);
    cov(30.0) - cov(25.0)
}

/// Coverage of a top-down vehicle silhouette, nose up, for a
/// [`crate::classification::VehicleKind`] discriminant: light wheeled hull with four wheels,
/// truck cab and bed with six wheels, or tracked hull with track rails; 0 for any other kind.
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

/// Coverage of the comment speech bubble: a rounded-rectangle outline with a tail at the
/// lower left (`dx`, `dy` in pixels from the cell centre).
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

/// Coverage of pixel (`px`, `py`) in symbology cell `offset` (relative to the first symbology
/// cell): unit glyphs 0..=9 (selected from [`UNIT_SELECTED_CELL_BASE`] with the ring), vehicles
/// 10..=12, comment 13 and selected comment 14; 0 for any other offset.
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

/// A caller's slot atlas strip with the [`SYMBOLOGY_CELL_COUNT`] symbology cells appended
/// after it, as [`extend_atlas_with_unit_glyphs`] returns it.
pub struct WidenedSlotAtlas {
    /// Straight-alpha RGBA8, white-on-alpha (the instance tint multiplies).
    pub rgba: Vec<u8>,

    /// Atlas width in pixels: (`base_cells` + [`SYMBOLOGY_CELL_COUNT`]) × [`ATLAS_CELL_PX`].
    pub width: u32,

    /// Atlas height in pixels, one cell row: [`ATLAS_CELL_PX`].
    pub height: u32,

    /// Flat `[minU,minV,maxU,maxV]·N` table over ALL cells (base + symbology).
    pub uv: Vec<f32>,

    /// Number of cells in the incoming base atlas, which is the glyph index of the first
    /// symbology cell (the `glyph_base` the overlay instance packers add cell offsets to).
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
