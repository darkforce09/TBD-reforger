//! Role: the vehicles a mission places, and who is boarded in them.
//! Position: `editing/hosted_commands` in the map engine.
//! Signals & state: none of its own; the document comes from the host.
//! Invariants: every vehicle read here comes from [`vehicle_rows`], the one `vehiclesById` reader.
//! Vehicles sit off the slot SoA deliberately, so "which ids are vehicles", "where are they" and
//! "who is crewed" are all derived from that single row set rather than from parallel flags that
//! could drift. Every mutator runs exactly one post-change tail.

use std::collections::HashSet;

use crate::data::store::operations::entity as entity_ops;
use crate::editing::history::after_local_edit;
use crate::editing::host::with_doc;

use super::document_edit::commit_document_edit;

/// One placed vehicle, as the docks need it.
pub use crate::data::store::operations::entity::{PlacedSlotChoice, VehicleCargoRow, VehicleRow};

/// Every placed vehicle, sorted by id — the docks' row order.
#[must_use]
pub fn vehicle_rows() -> Vec<VehicleRow> {
    with_doc(entity_ops::vehicle_rows).unwrap_or_default()
}

/// The slots sitting in a vehicle seat right now. The one answer to "who is boarded" — a surface
/// that needs it reads here rather than growing a second flag on the slot row.
#[must_use]
pub fn crewed_slot_ids() -> HashSet<String> {
    vehicle_rows()
        .into_iter()
        .flat_map(|v| v.crew.into_values())
        .filter(|id| !id.is_empty())
        .collect()
}

/// Every placed vehicle that has a position, as `(id, x, y)` — the map pick's vehicle lane.
#[must_use]
pub fn vehicle_points() -> Vec<(String, f64, f64)> {
    vehicle_rows()
        .into_iter()
        .filter_map(|v| v.xy.map(|(x, y)| (v.id, x, y)))
        .collect()
}

/// Whether `id` names a placed vehicle rather than a slot.
#[must_use]
pub fn is_vehicle_id(id: &str) -> bool {
    vehicle_rows().iter().any(|v| v.id == id)
}

/// The placed slots a seat picker can offer, labelled for a menu row.
#[must_use]
pub fn placed_slot_choices() -> Vec<PlacedSlotChoice> {
    with_doc(entity_ops::placed_slot_choices).unwrap_or_default()
}

/// Replace a vehicle's authored cargo rows.
pub fn set_vehicle_cargo(vehicle_id: String, rows: Vec<VehicleCargoRow>) -> bool {
    let did =
        with_doc(|core| entity_ops::set_vehicle_cargo(core, vehicle_id, rows)).unwrap_or(false);
    if did {
        after_local_edit();
    }
    did
}

/// Turn a vehicle to `heading_deg`.
pub fn set_vehicle_heading(vehicle_id: String, heading_deg: f64) -> bool {
    let did = with_doc(|core| entity_ops::set_vehicle_heading(core, vehicle_id, heading_deg))
        .unwrap_or(false);
    if did {
        after_local_edit();
    }
    did
}

/// Shift every vehicle in `ids` by `(dx, dy)` metres. A move of nothing, or of zero distance,
/// never touches the document — so a drag that ended where it began is not an undo step.
pub fn move_vehicles(ids: Vec<String>, dx: f64, dy: f64) -> bool {
    if ids.is_empty() || (dx == 0.0 && dy == 0.0) {
        return false;
    }
    commit_document_edit(|core| core.move_vehicles(&ids, dx, dy))
}

/// Delete a placed vehicle, freeing whoever was seated in it.
pub fn remove_vehicle(vehicle_id: String) -> bool {
    commit_document_edit(|core| core.remove_vehicle(&vehicle_id))
}

/// Seat `slot_id` in `seat_id` of `vehicle_id`.
pub fn assign_crew_seat(vehicle_id: String, seat_id: String, slot_id: String) -> bool {
    commit_document_edit(|core| core.assign_crew_seat(&vehicle_id, &seat_id, &slot_id))
}

/// Empty one seat of a vehicle.
pub fn clear_crew_seat(vehicle_id: String, seat_id: String) -> bool {
    commit_document_edit(|core| core.clear_crew_seat(&vehicle_id, &seat_id))
}
