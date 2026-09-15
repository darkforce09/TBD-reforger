//! Role: vehicles.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Arc;
use super::HashMap;
use super::HashSet;
use super::Map;
use super::MapPrelim;
use super::MissionDocCore;
use super::Out;
use super::append_id;
use super::position_any;
use super::read_crew_map;
use super::read_id_array;
use super::read_position;
use super::retain_ids;
use super::write_crew_map;
use yrs::Transact;

impl MissionDocCore {
    /// Add vehicle using the supplied domain data.
    pub fn add_vehicle(
        &self,
        id: &str,
        resource_name: &str,
        x: Option<f64>,
        y: Option<f64>,
        z: Option<f64>,
        rotation: Option<f64>,
    ) {
        let mut txn = self.begin();
        let v = self
            .vehicles
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        v.insert(&mut txn, "resourceName", resource_name);
        if let (Some(x), Some(y)) = (x, y) {
            v.insert(
                &mut txn,
                "position",
                position_any(x, y, z.unwrap_or(0.0), rotation.unwrap_or(0.0)),
            );
        }
    }
}

impl MissionDocCore {
    /// Set vehicle faction using the supplied domain data.
    pub fn set_vehicle_faction(&self, vehicle_id: &str, faction_id: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) {
            v.insert(&mut txn, "factionId", faction_id);
        }
    }
}

impl MissionDocCore {
    /// Malformed rows are **dropped, not written**: an empty `item` violates `minLength: 1` and a `qty < 1` violates `minimum: 1`, so writing either would produce a document that cannot validate — strictly worse than one that lost a row it could never have shipped.
    pub fn set_vehicle_cargo(&self, vehicle_id: &str, rows: &[(String, i64)]) {
        let mut txn = self.begin();
        let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) else {
            return;
        };
        let kept: Vec<Any> = rows
            .iter()
            .filter(|(item, qty)| !item.trim().is_empty() && *qty >= 1)
            .map(|(item, qty)| {
                Any::Map(Arc::new(HashMap::from([
                    ("item".to_string(), Any::String(item.as_str().into())),
                    ("qty".to_string(), Any::BigInt(*qty)),
                ])))
            })
            .collect();
        if kept.is_empty() {
            v.remove(&mut txn, "cargo");
        } else {
            v.insert(&mut txn, "cargo", Any::Array(kept.into()));
        }
    }
}

impl MissionDocCore {
    /// Attach vehicle using the supplied domain data.
    pub fn attach_vehicle(&self, squad_id: &str, vehicle_id: &str) {
        let mut txn = self.begin();
        if self.squads.get(&txn, squad_id).is_none()
            || self.vehicles.get(&txn, vehicle_id).is_none()
        {
            return;
        }
        let existing = read_id_array(&txn, &self.squads, squad_id, "vehicleIds");
        if existing
            .iter()
            .any(|a| matches!(a, Any::String(s) if s.as_ref() == vehicle_id))
        {
        } else {
            append_id(&mut txn, &self.squads, squad_id, "vehicleIds", vehicle_id);
        }
        if let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) {
            v.insert(&mut txn, "squadId", squad_id);
        }
    }
}

impl MissionDocCore {
    /// B2 — delete a vehicle row entirely: detach from its squad (if any) and remove it from `vehiclesById` ([`Self::detach_vehicle`] alone leaves the row orphaned).
    pub fn remove_vehicle(&self, vehicle_id: &str) {
        let mut txn = self.begin();
        let squad_id = match self.vehicles.get(&txn, vehicle_id) {
            Some(Out::YMap(v)) => match v.get(&txn, "squadId") {
                Some(Out::Any(Any::String(s))) => Some(s.to_string()),
                _ => None,
            },
            _ => return,
        };
        if let Some(sid) = squad_id
            && let Some(Out::YMap(sq)) = self.squads.get(&txn, &sid)
        {
            let arr = read_id_array(&txn, &self.squads, &sid, "vehicleIds");
            let remove: HashSet<&str> = HashSet::from([vehicle_id]);
            let kept = retain_ids(&arr, &remove);
            sq.insert(&mut txn, "vehicleIds", Any::Array(kept.into()));
        }
        self.vehicles.remove(&mut txn, vehicle_id);
    }
}

impl MissionDocCore {
    /// Detach vehicle using the supplied domain data.
    pub fn detach_vehicle(&self, squad_id: &str, vehicle_id: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(sq)) = self.squads.get(&txn, squad_id) {
            let arr = read_id_array(&txn, &self.squads, squad_id, "vehicleIds");
            let remove: HashSet<&str> = HashSet::from([vehicle_id]);
            let kept = retain_ids(&arr, &remove);
            sq.insert(&mut txn, "vehicleIds", Any::Array(kept.into()));
        }
        if let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) {
            v.remove(&mut txn, "squadId");
        }
    }
}

impl MissionDocCore {
    /// **One slot occupies at most ONE seat across ALL vehicles** — enforced HERE, not in the UI: the same soldier cannot be two places at once, and a rule the caller can forget to apply is not a rule. Before writing, `slot_id` is cleared from every other seat of every vehicle (its own included), so a re-board is a move, never a duplicate. Assigning a slot already in this exact seat is idempotent.
    pub fn assign_crew_seat(&self, vehicle_id: &str, seat_id: &str, slot_id: &str) {
        if seat_id.is_empty() || slot_id.is_empty() {
            return;
        }
        let mut txn = self.begin();
        if self.vehicles.get(&txn, vehicle_id).is_none() {
            return;
        }

        let mut evict: Vec<(String, String)> = Vec::new();
        for (vid, out_v) in self.vehicles.iter(&txn) {
            let Out::YMap(v) = out_v else { continue };
            for (sid, occ) in read_crew_map(&txn, &v) {
                if matches!(occ, Any::String(ref s) if s.as_ref() == slot_id)
                    && !(vid == vehicle_id && sid == seat_id)
                {
                    evict.push((vid.to_string(), sid));
                }
            }
        }
        for (vid, sid) in evict {
            if let Some(Out::YMap(v)) = self.vehicles.get(&txn, &vid) {
                let mut crew = read_crew_map(&txn, &v);
                crew.remove(&sid);

                write_crew_map(&mut txn, &v, crew);
            }
        }

        let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) else {
            return;
        };
        let mut crew = read_crew_map(&txn, &v);
        crew.insert(seat_id.to_string(), Any::String(slot_id.into()));
        write_crew_map(&mut txn, &v, crew);
    }
}

impl MissionDocCore {
    /// Set vehicle crewed using the supplied domain data.
    pub fn set_vehicle_crewed(&self, vehicle_id: &str, crewed: bool) {
        let mut txn = self.begin();
        if let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) {
            if crewed {
                v.remove(&mut txn, "crewed");
            } else {
                v.insert(&mut txn, "crewed", false);
            }
        }
    }
}

impl MissionDocCore {
    /// Place vehicle with crew stamp using the supplied domain data.
    #[allow(clippy::too_many_arguments)]
    pub fn place_vehicle_with_crew_stamp(
        &self,
        id: &str,
        resource_name: &str,
        x: f64,
        y: f64,
        z: f64,
        rotation: f64,
        faction_id: &str,
        crewed: bool,
    ) {
        let mut txn = self.begin();
        let v = self
            .vehicles
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        v.insert(&mut txn, "resourceName", resource_name);
        v.insert(&mut txn, "position", position_any(x, y, z, rotation));
        v.insert(&mut txn, "factionId", faction_id);
        if !crewed {
            v.insert(&mut txn, "crewed", false);
        }
    }
}

impl MissionDocCore {
    /// Clear crew seat using the supplied domain data.
    pub fn clear_crew_seat(&self, vehicle_id: &str, seat_id: &str) {
        let mut txn = self.begin();

        if let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) {
            let mut crew = read_crew_map(&txn, &v);
            if crew.remove(seat_id).is_some() {
                write_crew_map(&mut txn, &v, crew);
            }
        }
    }
}

impl MissionDocCore {
    /// Vehicle xy flat using the supplied domain data.
    #[must_use]
    pub fn vehicle_xy_flat(&self) -> Vec<f32> {
        let txn = self.doc.transact();
        let mut out = Vec::new();
        for (_id, out_v) in self.vehicles.iter(&txn) {
            let Out::YMap(v) = out_v else {
                continue;
            };
            if v.get(&txn, "position").is_none() {
                continue;
            }
            let (x, y, _, _) = read_position(&txn, &v);
            #[allow(clippy::cast_possible_truncation)]
            {
                out.push(x as f32);
                out.push(y as f32);
            }
        }
        out
    }
}
