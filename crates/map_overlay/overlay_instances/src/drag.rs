//! Drag previews of slots and vehicles.
//!
//! **Role:** projects a dragged position ([`drag_projected`]), classifies a drag store
//! transition into the upload the GPU bridge makes ([`classify_drag_transition`]), and packs
//! the drag overlay instances of dragged slots and vehicles.
//! **Position:** `overlay_instances`; packs through [`crate::symbols`]; the map engine's slot
//! bridges call it on every drag transition.
//! **Signals & state:** none; pure functions into owned buffers.
//! **Invariants:** a drag start or restart is one overlay upload and a delta moves the overlay
//! by its shader offset alone; the hidden base rows are reported so the base lane can be
//! patched, never rebuilt.

use crate::symbols::SLOT_GLYPH_RING;
use crate::symbols::SLOT_ICON_STRIDE;
use crate::symbols::SLOT_SELECTED_PX;
use crate::symbols::SLOT_SELECTED_RGBA;
use crate::symbols::pack_slot_symbology;
use render_primitives::text::pack::pack_icon_instance;
use render_primitives::text::pack::pack_rgba_u32;

/// World-meter drag delta applied in the shader (anchor cancels: same in relative space).
#[must_use]
pub fn drag_projected(base_x: f64, base_y: f64, dx: f64, dy: f64) -> (f64, f64) {
    (base_x + dx, base_y + dy)
}

/// - `Start` / `Restart` → one overlay upload; `Delta` → `set_slot_drag_delta` only; `End` → clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragGpuPhase {
    /// No drag change the GPU needs to see: no drag before or after, or nothing moved.
    Idle,

    /// A drag begins: upload the drag overlay once and hide the dragged base rows.
    Start,

    /// The same ids moved: update the shader drag offset only, without an upload.
    Delta,

    /// The dragged ids changed mid-drag: upload the drag overlay again.
    Restart,

    /// The drag finished: clear the drag overlay.
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
/// use overlay_instances::drag::pack_vehicle_drag_preview;
///
/// let points = vec![
/// ("v-parked".to_string(), 10.0, 20.0),
/// ("v-dragged".to_string(), 30.0, 40.0),
/// ];
/// // A mixed selection: one slot id (which names no vehicle) and one vehicle id.
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

/// Pack the drag overlay as selected unit symbology (role glyph and heading per dragged row);
/// returns the bytes and the full-document rows hidden in the base lane. Ids not in `ids` are
/// skipped.
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
