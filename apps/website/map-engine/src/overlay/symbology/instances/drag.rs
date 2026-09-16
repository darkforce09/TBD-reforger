//! Role: drag.
//! Position: `overlay/symbology/instances` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::overlay::symbology::instances::packing::pack_icon_instance;
use crate::overlay::symbology::instances::packing::pack_rgba_u32;
use crate::overlay::symbology::instances::symbols::SLOT_GLYPH_RING;
use crate::overlay::symbology::instances::symbols::SLOT_ICON_STRIDE;
use crate::overlay::symbology::instances::symbols::SLOT_SELECTED_PX;
use crate::overlay::symbology::instances::symbols::SLOT_SELECTED_RGBA;
use crate::overlay::symbology::instances::symbols::pack_slot_symbology;

/// World-meter drag delta applied in the shader (anchor cancels: same in relative space).
#[must_use]
pub fn drag_projected(base_x: f64, base_y: f64, dx: f64, dy: f64) -> (f64, f64) {
    (base_x + dx, base_y + dy)
}

/// - `Start` / `Restart` → one overlay upload; `Delta` → `set_slot_drag_delta` only; `End` → clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragGpuPhase {
    /// Idle.
    Idle,

    /// Start.
    Start,

    /// Delta.
    Delta,

    /// Restart.
    Restart,

    /// End.
    End,
}

/// Classify a drag store transition for the GPU bridge.
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

/// Pack drag overlay instances for the given drag ids (lookup by id → row in `ids`/`xy`). Returns packed bytes + parallel full-doc row indices that were hidden (for base patches).
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

/// Offset only selected vehicles while preserving parked vehicles and ignoring other entity IDs.
///
/// ```
/// use website_map_engine::overlay::symbology::instances::drag::pack_vehicle_drag_preview;
///
/// let points = vec![
/// ("v-parked".to_string(), 10.0, 20.0),
/// ("v-dragged".to_string(), 30.0, 40.0),
/// ];
/// // The mixed selection that was broken: one slot id (which names no vehicle) + one vehicle id.
/// let drag = vec!["slot-7".to_string(), "v-dragged".to_string()];
///
/// assert_eq!(
/// pack_vehicle_drag_preview(&drag, &points, 7.5, -3.25),
/// vec![10.0_f32, 20.0, 37.5, 36.75],
/// "the dragged vehicle moves, the parked one does not, and the slot id resolves to no row"
/// );
/// ```
#[must_use]
pub fn pack_vehicle_drag_preview(
    drag_ids: &[String],
    points: &[(String, f64, f64)],
    dx: f64,
    dy: f64,
) -> Vec<f32> {
    let dragged: std::collections::HashSet<&str> = drag_ids.iter().map(String::as_str).collect();
    let mut out = Vec::with_capacity(points.len() * 2);
    for (id, x, y) in points {
        let (wx, wy) = if dragged.contains(id.as_str()) {
            (x + dx, y + dy)
        } else {
            (*x, *y)
        };
        #[allow(clippy::cast_possible_truncation)]
        {
            out.push(wx as f32);
            out.push(wy as f32);
        }
    }
    out
}

/// Pack drag overlay symbology.
#[must_use]
pub fn pack_drag_overlay_symbology(
    drag_ids: &[String],
    ids: &[String],
    xy: &[f32],
    roles: &[String],
    headings_deg: &[f32],
    m_per_px: f32,
    glyph_base: u16,
) -> (Vec<u8>, Vec<usize>) {
    let mut id_to_row: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::with_capacity(ids.len());
    for (i, id) in ids.iter().enumerate() {
        id_to_row.insert(id.as_str(), i);
    }
    let mut out = Vec::with_capacity(drag_ids.len() * SLOT_ICON_STRIDE);
    let mut rows = Vec::with_capacity(drag_ids.len());
    for id in drag_ids {
        let Some(&row) = id_to_row.get(id.as_str()) else {
            continue;
        };
        let x = xy.get(row * 2).copied().unwrap_or(0.0);
        let y = xy.get(row * 2 + 1).copied().unwrap_or(0.0);

        let role = roles.get(row).cloned().unwrap_or_default();
        let h = headings_deg.get(row).copied().unwrap_or(0.0);
        out.extend_from_slice(&pack_slot_symbology(
            &[x, y],
            &[true],
            &[],
            std::slice::from_ref(&role),
            &[h],
            m_per_px,
            glyph_base,
        ));
        rows.push(row);
    }
    (out, rows)
}
