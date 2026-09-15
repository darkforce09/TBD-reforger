//! Role: vehicles.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

/// Read every `vehiclesById` row for the docks. Off `small_maps_json`, like [`squad_rows`] — vehicles are deliberately off the slot SoA, so there is no columnar reader to use instead.
#[must_use]
pub fn vehicle_rows() -> Vec<VehicleRow> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        website_mission_core::doc::operations::entity::vehicle_rows(core)
    })
}

/// Derived from [`vehicle_rows`]; the same set `mission_editor::crewed_slot_ids` computes from `small_maps_json`. Docks / diagnostics that need "who is boarded right now" read here — never invent a parallel flag on the slot row.
#[must_use]
pub fn crewed_slot_ids() -> std::collections::HashSet<String> {
    vehicle_rows()
        .into_iter()
        .flat_map(|v| v.crew.into_values())
        .filter(|id| !id.is_empty())
        .collect()
}

/// Set vehicle cargo using the supplied domain data.
pub fn set_vehicle_cargo(vehicle_id: String, rows: Vec<VehicleCargoRow>) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        website_mission_core::doc::operations::entity::set_vehicle_cargo(core, vehicle_id, rows)
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Vehicle points using the supplied domain data.
#[must_use]
pub fn vehicle_points() -> Vec<(String, f64, f64)> {
    vehicle_rows()
        .into_iter()
        .filter_map(|v| v.xy.map(|(x, y)| (v.id, x, y)))
        .collect()
}

/// Set vehicle heading using the supplied domain data.
pub fn set_vehicle_heading(vehicle_id: String, heading_deg: f64) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        website_mission_core::doc::operations::entity::set_vehicle_heading(
            core,
            vehicle_id,
            heading_deg,
        )
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Move vehicles using the supplied domain data.
pub fn move_vehicles(ids: Vec<String>, dx: f64, dy: f64) -> bool {
    if ids.is_empty() || (dx == 0.0 && dy == 0.0) {
        return false;
    }
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.move_vehicles(&ids, dx, dy);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Is vehicle id using the supplied domain data.
#[must_use]
pub fn is_vehicle_id(id: &str) -> bool {
    vehicle_rows().iter().any(|v| v.id == id)
}

/// Remove vehicle using the supplied domain data.
pub fn remove_vehicle(vehicle_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.remove_vehicle(&vehicle_id);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Placed slot choices using the supplied domain data.
#[must_use]
pub fn placed_slot_choices() -> Vec<PlacedSlotChoice> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        website_mission_core::doc::operations::entity::placed_slot_choices(core)
    })
}

/// Assign crew seat using the supplied domain data.
pub fn assign_crew_seat(vehicle_id: String, seat_id: String, slot_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.assign_crew_seat(&vehicle_id, &seat_id, &slot_id);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Clear crew seat using the supplied domain data.
pub fn clear_crew_seat(vehicle_id: String, seat_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.clear_crew_seat(&vehicle_id, &seat_id);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}
