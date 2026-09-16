//! Role: transforms.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::EntityTransformPatch;
use super::MissionDocCore;
use super::Out;
use super::move_entities_in_txn;
use super::move_vehicles_in_txn;
use super::position_any_merged;
use super::read_position_map;
use super::update_slot_position_in_txn;
use super::update_vehicle_position_in_txn;
use yrs::Map;

impl MissionDocCore {
    /// Set vehicle position using the supplied domain data.
    pub fn set_vehicle_position(&self, vehicle_id: &str, x: f64, y: f64, z: f64, rotation: f64) {
        let mut txn = self.begin();
        if let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) {
            let existing = read_position_map(&txn, &v);
            v.insert(
                &mut txn,
                "position",
                position_any_merged(existing, x, y, z, rotation),
            );
        }
    }
}

impl MissionDocCore {
    /// Move vehicles using the supplied domain data.
    pub fn move_vehicles(&self, ids: &[String], dx: f64, dy: f64) {
        let mut txn = self.begin();
        move_vehicles_in_txn(&mut txn, &self.vehicles, ids, dx, dy);
    }
}

impl MissionDocCore {
    /// let doc = MissionDocCore::new(); doc.set_origin_init(true); doc.add_slot("s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0); doc.add_vehicle("v0", "Prefab/Vehicle.et", Some(300.0), Some(400.0), Some(0.0), Some(45.0)); doc.set_origin_init(false);.
    ///
    /// ```
    /// use website_map_engine::data::store::MissionDocCore;
    ///
    /// let doc = MissionDocCore::new();
    /// doc.set_origin_init(true);
    /// doc.add_slot("s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0);
    /// doc.add_vehicle("v0", "Prefab/Vehicle.et", Some(300.0), Some(400.0), Some(0.0), Some(45.0));
    /// doc.set_origin_init(false);
    ///
    /// doc.move_entities_and_vehicles(vec!["s0".into()], &["v0".into()], 10.0, 20.0, vec![1.5]);
    ///
    /// let slots = doc.materialize(); // one slot, so row 0 is s0
    /// assert_eq!((slots.xs[0], slots.ys[0], slots.zs[0]), (110.0, 220.0, 1.5), "the slot moved");
    /// assert_eq!(doc.vehicle_xy_flat(), vec![310.0, 420.0], "the vehicle moved too");
    /// assert_eq!(doc.undo_depth(), 1, "…and both inside ONE transaction");
    /// ```
    pub fn move_entities_and_vehicles(
        &self,
        slot_ids: Vec<String>,
        vehicle_ids: &[String],
        dx: f64,
        dy: f64,
        zs: Vec<f64>,
    ) {
        let mut txn = self.begin();
        move_entities_in_txn(
            &mut txn,
            &self.slots,
            &self.editor_layers,
            &slot_ids,
            dx,
            dy,
            &zs,
        );
        move_vehicles_in_txn(&mut txn, &self.vehicles, vehicle_ids, dx, dy);
    }
}

impl MissionDocCore {
    /// Set slot position using the supplied domain data.
    pub fn set_slot_position(&self, id: &str, x: f64, y: f64, z: f64, rotation: f64) {
        let mut txn = self.begin();
        if let Some(Out::YMap(slot)) = self.slots.get(&txn, id) {
            let existing = read_position_map(&txn, &slot);
            slot.insert(
                &mut txn,
                "position",
                position_any_merged(existing, x, y, z, rotation),
            );
        }
    }
}

impl MissionDocCore {
    /// Edit a slot's transform (Attributes Transform tab). `x`/`y` clamp to `[0,width]×[0,height]`, `rotation` normalizes to `[0,360)`, and the z-policy matches `ydoc.updateSlotPosition` (manual z sticks; an x/y edit terrain-follows → 0 here, DEM sampled JS-side). `None` = leave the axis.
    #[allow(clippy::too_many_arguments)]
    pub fn update_slot_position(
        &self,
        id: &str,
        x: Option<f64>,
        y: Option<f64>,
        z: Option<f64>,
        rotation: Option<f64>,
        width: f64,
        height: f64,
    ) {
        let mut txn = self.begin();
        let _ = update_slot_position_in_txn(
            &mut txn,
            &self.slots,
            &self.editor_layers,
            id,
            x,
            y,
            z,
            rotation,
            width,
            height,
        );
    }
}

impl MissionDocCore {
    /// Update entity transforms using the supplied domain data.
    pub fn update_entity_transforms(
        &self,
        patches: &[EntityTransformPatch],
        width: f64,
        height: f64,
    ) -> usize {
        if patches.is_empty() {
            return 0;
        }
        let mut txn = self.begin();
        let mut n = 0usize;
        for p in patches {
            let applied = if p.is_slot {
                update_slot_position_in_txn(
                    &mut txn,
                    &self.slots,
                    &self.editor_layers,
                    &p.id,
                    p.x,
                    p.y,
                    p.z,
                    p.rotation,
                    width,
                    height,
                )
            } else {
                update_vehicle_position_in_txn(
                    &mut txn,
                    &self.vehicles,
                    &p.id,
                    p.x,
                    p.y,
                    p.z,
                    p.rotation,
                )
            };
            if applied {
                n += 1;
            }
        }
        n
    }
}

impl MissionDocCore {
    /// Thin wrapper over [`Self::update_entity_transforms`] for the multi-select Shift-rotate / orient path. Each `(id, is_slot, degrees)` writes rotation only; x/y/z stay put. Returns how many entities rotated.
    pub fn rotate_entities(&self, items: &[(String, bool, f64)], width: f64, height: f64) -> usize {
        let patches: Vec<EntityTransformPatch> = items
            .iter()
            .map(|(id, is_slot, deg)| EntityTransformPatch {
                id: id.clone(),
                is_slot: *is_slot,
                x: None,
                y: None,
                z: None,
                rotation: Some(*deg),
            })
            .collect();
        self.update_entity_transforms(&patches, width, height)
    }
}

impl MissionDocCore {
    /// Move several slots by a shared world delta (drag release). `zs[i]` is the JS-sampled DEM elevation at slot `ids[i]`'s new position (0 when the DEM is not ready — the vitest case, which keeps byte-parity). Mirrors `ydoc.moveEntities` (`z = terrainZ(newX, newY)`).
    pub fn move_entities(&self, ids: Vec<String>, dx: f64, dy: f64, zs: Vec<f64>) {
        let mut txn = self.begin();
        move_entities_in_txn(
            &mut txn,
            &self.slots,
            &self.editor_layers,
            &ids,
            dx,
            dy,
            &zs,
        );
    }
}
