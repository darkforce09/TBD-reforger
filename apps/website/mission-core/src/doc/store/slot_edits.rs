//! Role: slot edits.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::MapPrelim;
use super::MissionDocCore;
use super::Out;
use super::json_str_to_any;
use super::position_any;
use super::remove_slots_in_txn;
use super::update_slot_in_txn;
use super::update_slot_object_in_txn;
use yrs::Map;

impl MissionDocCore {
    /// Remove one slot (mirrors `slots.delete(id)`; layer detach is out of the spike mutator set).
    pub fn remove_slot(&self, id: &str) {
        let mut txn = self.begin();
        self.slots.remove(&mut txn, id);
    }
}

impl MissionDocCore {
    /// Bulk-seed `n` random slots in ONE transaction — the browser-harness generator for the criterion-6 fps/zero-copy test. Deterministic LCG positions in `[0,w)×[0,h)`; not undo-granular (the whole seed is one step).
    pub fn seed_random(&self, n: u32, w: f64, h: f64, seed: u64) {
        let mut s = seed | 1;
        let mut txn = self.begin();
        for i in 0..n {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let x = (s >> 33) as f64 / f64::from(1u32 << 31) * w;
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let y = (s >> 33) as f64 / f64::from(1u32 << 31) * h;
            let id = format!("s{i}");
            let slot = self.slots.insert(
                &mut txn,
                id.as_str(),
                MapPrelim::from([("id", id.as_str())]),
            );
            slot.insert(&mut txn, "squadId", "sq");
            slot.insert(&mut txn, "role", "Rifleman");
            slot.insert(&mut txn, "stance", "stand");
            slot.insert(&mut txn, "position", position_any(x, y, 0.0, 0.0));
        }
    }
}

impl MissionDocCore {
    /// Patch scalar slot fields; `None` leaves a field unchanged. Mirrors `ydoc.updateSlot`.
    pub fn update_slot(
        &self,
        id: &str,
        role: Option<String>,
        tag: Option<String>,
        stance: Option<String>,
    ) {
        let mut txn = self.begin();
        update_slot_in_txn(&mut txn, &self.slots, id, role, tag, stance);
    }
}

impl MissionDocCore {
    /// B2 — mutate an existing slot's role/tag/character in place (ORBAT Apply mutate semantics: the slot id — and with it every downstream `uid` reference — survives the re-apply). `tag` / `asset_id`: `Some(non-empty)` sets, `None`/empty clears (library rows are authoritative on Apply). Position, stance and identity fields are deliberately untouched — an operator-moved slot stays where it was moved.
    pub fn update_slot_role_character(
        &self,
        id: &str,
        role: &str,
        tag: Option<String>,
        asset_id: Option<String>,
    ) {
        let mut txn = self.begin();
        if let Some(Out::YMap(slot)) = self.slots.get(&txn, id) {
            slot.insert(&mut txn, "role", role);
            match tag.filter(|s| !s.is_empty()) {
                Some(t) => {
                    slot.insert(&mut txn, "tag", t);
                }
                None => {
                    slot.remove(&mut txn, "tag");
                }
            }
            match asset_id.filter(|s| !s.is_empty()) {
                Some(a) => {
                    slot.insert(&mut txn, "assetId", a);
                }
                None => {
                    slot.remove(&mut txn, "assetId");
                }
            }
        }
    }
}

impl MissionDocCore {
    /// Update slot identity using the supplied domain data.
    pub fn update_slot_identity(&self, id: &str, callsign: Option<String>, rank: Option<String>) {
        let mut txn = self.begin();
        if let Some(Out::YMap(slot)) = self.slots.get(&txn, id) {
            match callsign.filter(|s| !s.is_empty()) {
                Some(c) => {
                    slot.insert(&mut txn, "callsign", c);
                }
                None => {
                    slot.remove(&mut txn, "callsign");
                }
            }
            match rank.filter(|s| !s.is_empty()) {
                Some(r) => {
                    slot.insert(&mut txn, "rank", r);
                }
                None => {
                    slot.remove(&mut txn, "rank");
                }
            }
        }
    }
}

impl MissionDocCore {
    /// Update slot object using the supplied domain data.
    pub fn update_slot_object(
        &self,
        id: &str,
        asset_id: Option<String>,
        description: Option<String>,
    ) {
        if asset_id.is_none() && description.is_none() {
            return;
        }
        let mut txn = self.begin();
        update_slot_object_in_txn(&mut txn, &self.slots, id, asset_id, description);
    }
}

impl MissionDocCore {
    /// Update slots attr batch using the supplied domain data.
    #[allow(clippy::too_many_arguments)]
    pub fn update_slots_attr_batch(
        &self,
        ids: &[String],
        slot_half: bool,
        role: Option<String>,
        tag: Option<String>,
        stance: Option<String>,
        asset_id: Option<String>,
        description: Option<String>,
    ) -> usize {
        if ids.is_empty() {
            return 0;
        }
        let mut txn = self.begin();
        for id in ids {
            if slot_half {
                update_slot_in_txn(
                    &mut txn,
                    &self.slots,
                    id,
                    role.clone(),
                    tag.clone(),
                    stance.clone(),
                );
            }

            update_slot_object_in_txn(
                &mut txn,
                &self.slots,
                id,
                asset_id.clone(),
                description.clone(),
            );
        }
        ids.len()
    }
}

impl MissionDocCore {
    /// Update slot loadout using the supplied domain data.
    pub fn update_slot_loadout(&self, id: &str, loadout_json: Option<String>) -> bool {
        let mut txn = self.begin();
        let Some(Out::YMap(slot)) = self.slots.get(&txn, id) else {
            return false;
        };
        match loadout_json.filter(|s| !s.is_empty()) {
            Some(json) => {
                slot.insert(&mut txn, "loadout", json_str_to_any(&json));
            }
            None => {
                slot.remove(&mut txn, "loadout");
            }
        }
        true
    }
}

impl MissionDocCore {
    /// Remove several slots and detach them from their squad's `slotIds` and every layer's `entityIds` (batched cascade). Mirrors `ydoc.removeEntities` (slots path). The cascade body lives in [`remove_slots_in_txn`] so `remove_editor_layer` can reuse it inside its own txn.
    pub fn remove_slots(&self, ids: Vec<String>) {
        let mut txn = self.begin();
        remove_slots_in_txn(
            &mut txn,
            &self.slots,
            &self.squads,
            &self.editor_layers,
            &ids,
        );
    }
}
