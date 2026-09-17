//! Z drag for editor overlays.
use super::*;

thread_local! {
    static Z_DRAG_READOUT: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };

    static Z_DRAG_READOUT_GEN: std::cell::RefCell<Option<ArcRwSignal<u32>>> =
        const { std::cell::RefCell::new(None) };
}

fn z_drag_readout_generation() -> ArcRwSignal<u32> {
    Z_DRAG_READOUT_GEN.with(|c| {
        c.borrow_mut()
            .get_or_insert_with(|| ArcRwSignal::new(0))
            .clone()
    })
}

/// Stores the active elevation drag label and invalidates subscribers.
pub(crate) fn set_z_drag_readout(readout: Option<String>) {
    Z_DRAG_READOUT.with(|c| *c.borrow_mut() = readout);
    let generation = z_drag_readout_generation();
    generation.set(generation.get_untracked().wrapping_add(1));
}

/// Reads the current elevation drag label reactively.
pub(crate) fn read_z_drag_readout() -> Option<String> {
    let _ = z_drag_readout_generation().get();
    Z_DRAG_READOUT.with(|c| c.borrow().clone())
}

/// Converts pointer movement into a finite snapped elevation delta.
pub(crate) fn z_drag_elevation_delta(py: f64, start_y: f64, scale: f64, step: f64) -> f64 {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let raw = crate::v2::apps::editor::bridge::gizmo_z::dy_to_elevation(py - start_y, scale);
    if !raw.is_finite() {
        return 0.0;
    }
    crate::v2::apps::editor::bridge::gizmo_z::snap_elevation(raw, step)
}

/// Returns the active elevation snap increment for a drag.
pub(crate) fn z_drag_snap_step(snap: transform::SnapState, shift: bool) -> f64 {
    let rung = if shift {
        0
    } else {
        snap.effective_translate_rung()
    };
    transform::TRANSLATE_LADDER_M
        .get(rung)
        .copied()
        .unwrap_or(0.0)
}

/// State captured at the start of an elevation drag.
#[derive(Clone, Debug)]
pub(crate) struct ZDrag {
    pub(crate) pointer_id: i32,
    pub(crate) start_y: f64,
    pub(crate) scale: f64,
    slots: Vec<(String, f64)>,
    vehicles: Vec<(String, f64)>,
}

#[cfg(any(target_arch = "wasm32", test))]
impl ZDrag {
    /// Starts an elevation drag from pointer and camera state.
    pub(crate) fn begin(
        core: &website_map_engine::data::store::MissionDocCore,
        ids: &[String],
        pointer_id: i32,
        start_y: f64,
        scale: f64,
    ) -> Option<Self> {
        let rows: serde_json::Value = serde_json::from_str(&core.slots_json()).ok()?;
        let maps: serde_json::Value = serde_json::from_str(&core.small_maps_json()).ok()?;
        let mut slots = Vec::new();
        let mut vehicles = Vec::new();
        for id in ids {
            let (row, target) = if let Some(row) = rows.get(id) {
                (row, &mut slots)
            } else if let Some(row) = maps["vehiclesById"].get(id) {
                (row, &mut vehicles)
            } else {
                continue;
            };
            if let Some(position) = row.get("position") {
                let z = position
                    .get("z")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0);
                if z.is_finite() {
                    target.push((id.clone(), z));
                }
            }
        }
        (!slots.is_empty() || !vehicles.is_empty()).then_some(Self {
            pointer_id,
            start_y,
            scale,
            slots,
            vehicles,
        })
    }

    /// Returns the resulting height for a drag delta.
    pub(crate) fn height(&self, delta: f64) -> f64 {
        self.slots
            .first()
            .or_else(|| self.vehicles.first())
            .map_or(0.0, |(_, z)| *z)
            + delta
    }

    /// Writes the final snapped elevation to the document.
    pub(crate) fn commit(
        self,
        core: &mut website_map_engine::data::store::MissionDocCore,
        delta: f64,
    ) -> bool {
        if delta == 0.0 || !delta.is_finite() {
            return false;
        }
        let maps: serde_json::Value =
            serde_json::from_str(&core.small_maps_json()).unwrap_or_default();
        core.begin_group();
        let (slot_ids, zs): (Vec<_>, Vec<_>) = self
            .slots
            .into_iter()
            .map(|(id, z)| (id, z + delta))
            .unzip();
        if !slot_ids.is_empty() {
            core.move_entities(slot_ids, 0.0, 0.0, zs);
        }
        for (id, z) in self.vehicles {
            if let Some(p) = maps["vehiclesById"]
                .get(&id)
                .and_then(|row| row.get("position"))
            {
                core.set_vehicle_position(
                    &id,
                    p["x"].as_f64().unwrap_or(0.0),
                    p["y"].as_f64().unwrap_or(0.0),
                    z + delta,
                    p["rotation"].as_f64().unwrap_or(0.0),
                );
            }
        }
        core.end_group();
        true
    }
}

/// Consumes a matching active elevation drag on pointer release.
pub(crate) fn take_z_drag(drag: &mut Option<ZDrag>, pointer_id: i32) -> Option<ZDrag> {
    if drag
        .as_ref()
        .is_some_and(|arm| arm.pointer_id == pointer_id)
    {
        drag.take()
    } else {
        None
    }
}
