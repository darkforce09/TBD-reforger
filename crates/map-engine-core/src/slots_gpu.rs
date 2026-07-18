//! T-151.6 W6 — pure slot/cluster GPU packing + cluster gates (no wgpu).
//!
//! Mirrors Deck oracles:
//! - `useIconLayer.ts` — ring size 20 / selected 28, Aegis primary + tactical yellow
//! - `useClusterIconLayer.ts` — disc size `22 + min(26, log10(count)*12)`
//! - `state/constants.ts` — `CLUSTER_SLOT_THRESHOLD=500`, `ZOOM_CLUSTER_MAX=-4`

/// Icon instance stride (pos2 + size + yaw_i16 + glyph_u16 + tint_u32).
pub const SLOT_ICON_STRIDE: usize = 20;

/// Glyph index in the dedicated slot atlas (ring).
pub const SLOT_GLYPH_RING: u16 = 0;
/// Glyph index in the dedicated slot atlas (solid disc).
pub const SLOT_GLYPH_DISC: u16 = 1;

/// Base ring size in CSS pixels (`useIconLayer` getSize).
pub const SLOT_RING_PX: f32 = 20.0;
/// Selected ring size in CSS pixels.
pub const SLOT_SELECTED_PX: f32 = 28.0;

/// Aegis primary `#adc6ff` full alpha.
pub const SLOT_PRIMARY_RGBA: [u8; 4] = [173, 198, 255, 255];
/// Tactical yellow `#facc15` full alpha.
pub const SLOT_SELECTED_RGBA: [u8; 4] = [250, 204, 21, 255];
/// Cluster disc primary with Deck alpha 235.
pub const CLUSTER_DISC_RGBA: [u8; 4] = [173, 198, 255, 235];

/// T-065: cluster mode only when placed slots exceed this.
pub const CLUSTER_SLOT_THRESHOLD: u32 = 500;
/// T-065.2: cluster mode only at/below this deck zoom.
pub const ZOOM_CLUSTER_MAX: f64 = -4.0;

/// Pack RGBA8 as little-endian `u32` (r | g<<8 | b<<16 | a<<24).
#[must_use]
pub fn pack_rgba_u32(rgba: [u8; 4]) -> u32 {
    u32::from(rgba[0])
        | (u32::from(rgba[1]) << 8)
        | (u32::from(rgba[2]) << 16)
        | (u32::from(rgba[3]) << 24)
}

/// Cluster mode gate (T-065): `slot_len > 500 && zoom ≤ −4`.
#[must_use]
pub fn cluster_mode(slot_len: u32, deck_zoom: f64) -> bool {
    slot_len > CLUSTER_SLOT_THRESHOLD && deck_zoom <= ZOOM_CLUSTER_MAX
}

/// Disc pixel size from aggregated count (`useClusterIconLayer.discSize`).
#[must_use]
pub fn cluster_disc_size_px(count: u32) -> f32 {
    let c = count.max(1) as f64;
    let extra = (c.log10() * 12.0).min(26.0);
    (22.0 + extra) as f32
}

/// Pack one 20 B icon instance (WORLD meters for pos; size in **pixels** for slot atlas
/// with `px_to_m` uniform, or meters when `px_to_m = 1`).
pub fn pack_icon_instance(
    out: &mut Vec<u8>,
    pos_x: f32,
    pos_y: f32,
    size_px: f32,
    glyph: u16,
    tint: u32,
) {
    out.extend_from_slice(&pos_x.to_le_bytes());
    out.extend_from_slice(&pos_y.to_le_bytes());
    out.extend_from_slice(&size_px.to_le_bytes());
    out.extend_from_slice(&0_i16.to_le_bytes()); // yaw
    out.extend_from_slice(&glyph.to_le_bytes());
    out.extend_from_slice(&tint.to_le_bytes());
}

/// Pack slot rings from interleaved `xy` (`[x0,y0,…]`, length `2·n`).
/// `selected[i]` true → yellow + 28 px, else primary + 20 px.
///
/// # Panics
/// Never; short `selected` is treated as all unselected beyond its length.
#[must_use]
pub fn pack_slot_instances(xy: &[f32], selected: &[bool]) -> Vec<u8> {
    let n = xy.len() / 2;
    let mut out = Vec::with_capacity(n * SLOT_ICON_STRIDE);
    let primary = pack_rgba_u32(SLOT_PRIMARY_RGBA);
    let sel = pack_rgba_u32(SLOT_SELECTED_RGBA);
    for i in 0..n {
        let x = xy[i * 2];
        let y = xy[i * 2 + 1];
        let is_sel = selected.get(i).copied().unwrap_or(false);
        let (size, tint) = if is_sel {
            (SLOT_SELECTED_PX, sel)
        } else {
            (SLOT_RING_PX, primary)
        };
        pack_icon_instance(&mut out, x, y, size, SLOT_GLYPH_RING, tint);
    }
    out
}

/// Pack a single slot instance at world `(x,y)` with selection flag.
#[must_use]
pub fn pack_one_slot(x: f32, y: f32, selected: bool) -> [u8; SLOT_ICON_STRIDE] {
    let mut v = Vec::with_capacity(SLOT_ICON_STRIDE);
    let (size, tint) = if selected {
        (SLOT_SELECTED_PX, pack_rgba_u32(SLOT_SELECTED_RGBA))
    } else {
        (SLOT_RING_PX, pack_rgba_u32(SLOT_PRIMARY_RGBA))
    };
    pack_icon_instance(&mut v, x, y, size, SLOT_GLYPH_RING, tint);
    let mut arr = [0u8; SLOT_ICON_STRIDE];
    arr.copy_from_slice(&v);
    arr
}

/// Pack cluster disc markers: parallel `xs`/`ys`/`counts` (world meters).
#[must_use]
pub fn pack_cluster_instances(xs: &[f64], ys: &[f64], counts: &[u32]) -> Vec<u8> {
    let n = xs.len().min(ys.len()).min(counts.len());
    let mut out = Vec::with_capacity(n * SLOT_ICON_STRIDE);
    let tint = pack_rgba_u32(CLUSTER_DISC_RGBA);
    for i in 0..n {
        #[allow(clippy::cast_possible_truncation)]
        let x = xs[i] as f32;
        #[allow(clippy::cast_possible_truncation)]
        let y = ys[i] as f32;
        let size = cluster_disc_size_px(counts[i]);
        pack_icon_instance(&mut out, x, y, size, SLOT_GLYPH_DISC, tint);
    }
    out
}

/// World-meter drag delta applied in the shader (anchor cancels: same in relative space).
#[must_use]
pub fn drag_projected(base_x: f64, base_y: f64, dx: f64, dy: f64) -> (f64, f64) {
    (base_x + dx, base_y + dy)
}

/// Meters per CSS pixel at deck zoom (`scale = 2^zoom` → m/px = `2^(-zoom)`).
#[must_use]
pub fn px_to_m_at_zoom(deck_zoom: f64) -> f32 {
    if !deck_zoom.is_finite() {
        return 1.0;
    }
    #[allow(clippy::cast_possible_truncation)]
    {
        2.0_f64.powf(-deck_zoom) as f32
    }
}

/// Drag GPU phase for the slot overlay lane (T-151.7.1 / T-151.7.3).
///
/// - `Start` / `Restart` → one overlay upload; `Delta` → `set_slot_drag_delta` only; `End` → clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragGpuPhase {
    Idle,
    Start,
    Delta,
    Restart,
    End,
}

/// Classify a drag store transition for the GPU bridge (pure; mirrors W7.1 TS helper).
#[must_use]
pub fn classify_drag_transition(
    had: bool,
    has: bool,
    ids_changed: bool,
    delta_changed: bool,
) -> DragGpuPhase {
    if !had && has {
        return DragGpuPhase::Start;
    }
    if had && !has {
        return DragGpuPhase::End;
    }
    if had && has && ids_changed {
        return DragGpuPhase::Restart;
    }
    if had && has && delta_changed {
        return DragGpuPhase::Delta;
    }
    DragGpuPhase::Idle
}

/// Pack only selected slot rings (cluster short-lane / selection-only path).
/// Full-doc row index is **not** preserved — output is dense k selected instances.
#[must_use]
pub fn pack_selection_only(xy: &[f32], selected: &[bool]) -> Vec<u8> {
    let n = xy.len() / 2;
    let mut out = Vec::new();
    let tint = pack_rgba_u32(SLOT_SELECTED_RGBA);
    for i in 0..n {
        if !selected.get(i).copied().unwrap_or(false) {
            continue;
        }
        let x = xy[i * 2];
        let y = xy[i * 2 + 1];
        pack_icon_instance(&mut out, x, y, SLOT_SELECTED_PX, SLOT_GLYPH_RING, tint);
    }
    out
}

/// 12 B hide patch for base-lane size/yaw/glyph/tint at instance offset+8 (alpha 0 tint).
#[must_use]
pub fn hide_slot_row_patch() -> [u8; 12] {
    let mut hide = [0u8; 12];
    hide[0..4].copy_from_slice(&SLOT_SELECTED_PX.to_le_bytes());
    // yaw i16 = 0, glyph u16 = 0, tint u32 = 0 (alpha 0)
    hide
}

/// Pack drag overlay instances for the given drag ids (lookup by id → row in `ids`/`xy`).
/// Returns packed bytes + parallel full-doc row indices that were hidden (for base patches).
#[must_use]
pub fn pack_drag_overlay(drag_ids: &[String], ids: &[String], xy: &[f32]) -> (Vec<u8>, Vec<usize>) {
    let mut id_to_row: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::with_capacity(ids.len());
    for (i, id) in ids.iter().enumerate() {
        id_to_row.insert(id.as_str(), i);
    }
    let tint = pack_rgba_u32(SLOT_SELECTED_RGBA);
    let mut out = Vec::with_capacity(drag_ids.len() * SLOT_ICON_STRIDE);
    let mut rows = Vec::with_capacity(drag_ids.len());
    for id in drag_ids {
        let Some(&row) = id_to_row.get(id.as_str()) else {
            continue;
        };
        let x = xy.get(row * 2).copied().unwrap_or(0.0);
        let y = xy.get(row * 2 + 1).copied().unwrap_or(0.0);
        pack_icon_instance(&mut out, x, y, SLOT_SELECTED_PX, SLOT_GLYPH_RING, tint);
        rows.push(row);
    }
    (out, rows)
}

/// Build a dense `selected[i]` mask from SoA ids + selected id set.
#[must_use]
pub fn selected_mask(ids: &[String], selected: &std::collections::HashSet<String>) -> Vec<bool> {
    ids.iter().map(|id| selected.contains(id)).collect()
}

/// Slot/cluster atlas dimensions — two 64 px cells side by side (ring | disc), the
/// `slotAtlas.ts` contract the engine's UV table + pipeline were built against.
pub const SLOT_ATLAS_W: u32 = 128;
pub const SLOT_ATLAS_H: u32 = 64;
/// Flat per-glyph UV table: minU,minV,maxU,maxV for ring (glyph 0) and disc (glyph 1).
pub const SLOT_ATLAS_UV: [f32; 8] = [0.0, 0.0, 0.5, 1.0, 0.5, 0.0, 1.0, 1.0];

/// Procedurally built slot atlas pixels (T-172 B4). The engine's `ensure_slot_atlas` takes
/// caller-built RGBA — the React app built this on a 2D canvas; the Leptos host builds it here.
pub struct SlotAtlas {
    /// `SLOT_ATLAS_W × SLOT_ATLAS_H` straight-alpha RGBA, white-on-alpha (tint multiplies).
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub uv: [f32; 8],
}

/// Build the two-glyph atlas: glyph 0 = ring (outer r 24, inner r 10), glyph 1 = solid disc
/// (r 26) — the `slotAtlas.ts` radii. 1 px analytic edge coverage stands in for canvas arc AA
/// (visually equivalent at the 20–28 px render sizes).
#[must_use]
pub fn build_slot_atlas() -> SlotAtlas {
    let (w, h) = (SLOT_ATLAS_W as usize, SLOT_ATLAS_H as usize);
    let mut rgba = vec![0u8; w * h * 4];
    // Coverage of a disc of radius `r` at distance `d`, with a 1 px linear edge.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn icon_stride_is_20() {
        assert_eq!(SLOT_ICON_STRIDE, 20);
        let one = pack_one_slot(1.5, -2.5, false);
        assert_eq!(one.len(), 20);
    }

    #[test]
    fn pack_count_matches_xy() {
        let xy = [0.0_f32, 0.0, 100.0, 200.0, 300.0, 400.0];
        let sel = [false, true, false];
        let bytes = pack_slot_instances(&xy, &sel);
        assert_eq!(bytes.len(), 3 * SLOT_ICON_STRIDE);
        // row 1 selected → size 28
        let size1 = f32::from_le_bytes(bytes[20 + 8..20 + 12].try_into().unwrap());
        assert!((size1 - SLOT_SELECTED_PX).abs() < 1e-6);
        let tint1 = u32::from_le_bytes(bytes[20 + 16..20 + 20].try_into().unwrap());
        assert_eq!(tint1, pack_rgba_u32(SLOT_SELECTED_RGBA));
        // row 0 primary → size 20
        let size0 = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
        assert!((size0 - SLOT_RING_PX).abs() < 1e-6);
        assert_eq!(
            u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
            pack_rgba_u32(SLOT_PRIMARY_RGBA)
        );
    }

    #[test]
    fn cluster_gate_truth_table() {
        assert!(!cluster_mode(0, -6.0));
        assert!(!cluster_mode(500, -6.0)); // not >
        assert!(!cluster_mode(501, -3.9));
        assert!(cluster_mode(501, -4.0));
        assert!(cluster_mode(10_000, -6.0));
        assert!(!cluster_mode(10_000, -2.0));
    }

    #[test]
    fn cluster_disc_size_formula() {
        assert!((cluster_disc_size_px(1) - 22.0).abs() < 1e-5);
        // log10(1000)=3 → 22+min(26,36)=22+26=48
        assert!((cluster_disc_size_px(1000) - 48.0).abs() < 1e-5);
    }

    #[test]
    fn drag_delta_math() {
        let (x, y) = drag_projected(100.0, 200.0, 3.5, -1.25);
        assert!((x - 103.5).abs() < 1e-12);
        assert!((y - 198.75).abs() < 1e-12);
    }

    #[test]
    fn px_to_m_at_default_zoom() {
        // zoom -2 → 2^2 = 4 m/px
        assert!((px_to_m_at_zoom(-2.0) - 4.0).abs() < 1e-6);
        assert!((px_to_m_at_zoom(0.0) - 1.0).abs() < 1e-6);
        assert!((px_to_m_at_zoom(3.0) - 0.125).abs() < 1e-6);
    }

    #[test]
    fn pack_cluster_instances_count() {
        let xs = [10.0, 20.0];
        let ys = [30.0, 40.0];
        let counts = [5u32, 100];
        let b = pack_cluster_instances(&xs, &ys, &counts);
        assert_eq!(b.len(), 2 * SLOT_ICON_STRIDE);
        assert_eq!(
            u16::from_le_bytes(b[14..16].try_into().unwrap()),
            SLOT_GLYPH_DISC
        );
    }

    #[test]
    fn classify_drag_transition_truth_table() {
        assert_eq!(
            classify_drag_transition(false, true, true, false),
            DragGpuPhase::Start
        );
        assert_eq!(
            classify_drag_transition(true, true, false, true),
            DragGpuPhase::Delta
        );
        assert_eq!(
            classify_drag_transition(true, false, true, true),
            DragGpuPhase::End
        );
        assert_eq!(
            classify_drag_transition(true, true, true, false),
            DragGpuPhase::Restart
        );
        assert_eq!(
            classify_drag_transition(true, true, false, false),
            DragGpuPhase::Idle
        );
        assert_eq!(
            classify_drag_transition(false, false, false, true),
            DragGpuPhase::Idle
        );
    }

    #[test]
    fn pack_selection_only_dense_k() {
        let xy = [0.0_f32, 0.0, 100.0, 200.0, 300.0, 400.0];
        let sel = [false, true, true];
        let bytes = pack_selection_only(&xy, &sel);
        assert_eq!(bytes.len(), 2 * SLOT_ICON_STRIDE);
        let size0 = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
        assert!((size0 - SLOT_SELECTED_PX).abs() < 1e-6);
        let x0 = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
        assert!((x0 - 100.0).abs() < 1e-6);
    }

    #[test]
    fn selected_mask_from_set() {
        let ids = vec!["a".into(), "b".into(), "c".into()];
        let mut set = HashSet::new();
        set.insert("b".into());
        assert_eq!(selected_mask(&ids, &set), vec![false, true, false]);
    }

    #[test]
    fn pack_drag_overlay_rows() {
        let ids = vec!["a".into(), "b".into()];
        let xy = [1.0_f32, 2.0, 3.0, 4.0];
        let drag = vec!["b".into()];
        let (bytes, rows) = pack_drag_overlay(&drag, &ids, &xy);
        assert_eq!(rows, vec![1]);
        assert_eq!(bytes.len(), SLOT_ICON_STRIDE);
        let x = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
        assert!((x - 3.0).abs() < 1e-6);
    }

    #[test]
    fn slot_atlas_shape_and_uv() {
        let a = build_slot_atlas();
        assert_eq!(a.rgba.len(), 128 * 64 * 4);
        assert_eq!((a.width, a.height), (128, 64));
        assert_eq!(a.uv, [0.0, 0.0, 0.5, 1.0, 0.5, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn slot_atlas_ring_and_disc_probes() {
        let a = build_slot_atlas();
        let alpha = |x: usize, y: usize| a.rgba[(y * 128 + x) * 4 + 3];
        // Ring cell (center 32,32): hollow center, opaque band at r≈17, transparent outside r≈24.
        assert_eq!(alpha(32, 32), 0, "ring center must be hollow");
        assert_eq!(alpha(32 + 17, 32), 255, "ring band must be opaque");
        assert_eq!(alpha(32 + 30, 32), 0, "outside ring must be transparent");
        // Disc cell (center 96,32): opaque center and mid, transparent outside r≈26.
        assert_eq!(alpha(96, 32), 255, "disc center must be opaque");
        assert_eq!(alpha(96 + 20, 32), 255, "disc mid must be opaque");
        assert_eq!(alpha(96 + 30, 32), 0, "outside disc must be transparent");
        // White-on-alpha everywhere (tint multiplies).
        assert_eq!(&a.rgba[0..3], &[255, 255, 255]);
    }
}
