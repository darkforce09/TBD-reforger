//! Role: selection.
//! Position: `doc/picking` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use crate::data::store::{MissionDocCore, SlotSoa};

impl MissionDocCore {
    /// Canonical pick radius px value.
    pub const PICK_RADIUS_PX: f64 = 4.0;
}

impl MissionDocCore {
    /// Canonical grid cell m value.
    pub const GRID_CELL_M: f64 = 256.0;
}

impl MissionDocCore {
    /// Resolve a spatial slot row to its authored identifier.
    pub fn pick_slot(soa: &SlotSoa, row: Option<u32>) -> Option<String> {
        row.map(|h| soa.ids[h as usize].clone())
    }
}

impl MissionDocCore {
    /// Resolve a spatial vehicle row to its authored identifier.
    pub fn pick_vehicle(points: &[(String, f64, f64)], row: Option<usize>) -> Option<String> {
        row.map(|h| points[h].0.clone())
    }
}

impl MissionDocCore {
    /// Resolve spatial hits, preferring slots when world distances tie.
    pub fn pick_slot_or_vehicle(
        soa: &SlotSoa,
        vehicle_points: &[(String, f64, f64)],
        qx: f64,
        qy: f64,
        slot_row: Option<u32>,
        vehicle_row: Option<usize>,
    ) -> Option<String> {
        let slot = Self::pick_slot(soa, slot_row);
        let veh = Self::pick_vehicle(vehicle_points, vehicle_row);
        match (slot, veh) {
            (None, v) => v,
            (s, None) => s,
            (Some(s), Some(v)) => {
                let slot_d2 = soa
                    .ids
                    .iter()
                    .position(|id| *id == s)
                    .map(|i| {
                        let dx = f64::from(soa.xs[i]) - qx;
                        let dy = f64::from(soa.ys[i]) - qy;
                        dx * dx + dy * dy
                    })
                    .unwrap_or(f64::INFINITY);
                let veh_d2 = vehicle_points
                    .iter()
                    .find(|(id, _, _)| *id == v)
                    .map(|(_, x, y)| {
                        let dx = x - qx;
                        let dy = y - qy;
                        dx * dx + dy * dy
                    })
                    .unwrap_or(f64::INFINITY);
                if veh_d2 < slot_d2 { Some(v) } else { Some(s) }
            }
        }
    }
}

impl MissionDocCore {
    /// Resolve ordered spatial rows without changing query traversal order.
    pub fn marquee_slot_ids(soa: &SlotSoa, rows: &[u32]) -> Vec<String> {
        rows.iter().map(|&h| soa.ids[h as usize].clone()).collect()
    }
}

impl MissionDocCore {
    /// Resolve ordered vehicle rows to authored identifiers.
    pub fn marquee_vehicle_ids(points: &[(String, f64, f64)], rows: &[usize]) -> Vec<String> {
        rows.iter().map(|&h| points[h].0.clone()).collect()
    }
}

impl MissionDocCore {
    /// Append vehicle selection after slot selection.
    pub fn marquee_ids_with_vehicles(
        soa: &SlotSoa,
        vehicle_points: &[(String, f64, f64)],
        slot_rows: &[u32],
        vehicle_rows: &[usize],
    ) -> Vec<String> {
        let mut ids = Self::marquee_slot_ids(soa, slot_rows);
        ids.extend(Self::marquee_vehicle_ids(vehicle_points, vehicle_rows));
        ids
    }
}
