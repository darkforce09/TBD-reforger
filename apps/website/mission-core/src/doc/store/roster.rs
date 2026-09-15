//! Role: roster.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::HashSet;
use super::MapPrelim;
use super::MissionDocCore;
use super::Out;
use super::SLOT_IDS;
use super::append_id;
use super::ensure_leader_invariant_in_txn;
use super::garbage_collect_squad_in_txn;
use super::insert_empty_native;
use super::position_any;
use super::promote_pending_briefing_markers;
use super::read_id_array;
use super::remove_slots_in_txn;
use super::retain_in;
use super::rewrite_slot_indices;
use super::set_leader_in_txn;
use yrs::Map;

impl MissionDocCore {
    /// Add slot using the supplied domain data.
    #[allow(clippy::too_many_arguments)]
    pub fn add_slot(
        &self,
        id: &str,
        squad_id: &str,
        layer_id: &str,
        index: u32,
        role: &str,
        tag: Option<String>,
        asset_id: Option<String>,
        x: f64,
        y: f64,
        z: f64,
        rotation: f64,
    ) {
        let mut txn = self.begin();
        let slot = self
            .slots
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        slot.insert(&mut txn, "squadId", squad_id);
        slot.insert(&mut txn, "index", Any::BigInt(i64::from(index)));
        slot.insert(&mut txn, "role", role);
        if let Some(t) = tag.filter(|s| !s.is_empty()) {
            slot.insert(&mut txn, "tag", t);
        }
        if let Some(a) = asset_id.filter(|s| !s.is_empty()) {
            slot.insert(&mut txn, "assetId", a);
        }
        slot.insert(&mut txn, "position", position_any(x, y, z, rotation));
        slot.insert(&mut txn, "stance", "stand");
        slot.insert(&mut txn, "loadoutId", Any::Null);
        append_id(&mut txn, &self.squads, squad_id, "slotIds", id);
        append_id(&mut txn, &self.editor_layers, layer_id, "entityIds", id);
    }
}

impl MissionDocCore {
    /// Create a faction (mirrors `ydoc.addFaction` and `ensureDefaultSquad`'s faction — JS supplies `key`/`name`). Writes `{id, key, name, squadIds:[]}`.
    pub fn add_faction(&self, id: &str, key: &str, name: &str) {
        let mut txn = self.begin();
        let f = self
            .factions
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        f.insert(&mut txn, "key", key);
        f.insert(&mut txn, "name", name);
        f.insert(&mut txn, "squadIds", Any::Array(Vec::new().into()));

        promote_pending_briefing_markers(&mut txn, &self.meta, &f, id);
    }
}

impl MissionDocCore {
    /// Set faction name using the supplied domain data.
    pub fn set_faction_name(&self, faction_id: &str, name: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(f)) = self.factions.get(&txn, faction_id) {
            f.insert(&mut txn, "name", name);
        }
    }
}

impl MissionDocCore {
    /// Add squad using the supplied domain data.
    pub fn add_squad(&self, id: &str, faction_id: &str, name: &str, callsign: Option<String>) {
        let mut txn = self.begin();
        let sq = self
            .squads
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        sq.insert(&mut txn, "factionId", faction_id);
        if let Some(c) = callsign {
            sq.insert(&mut txn, "callsign", c);
        }
        sq.insert(&mut txn, "name", name);
        insert_empty_native(&mut txn, &sq, SLOT_IDS);
        sq.insert(&mut txn, "vehicleIds", Any::Array(Vec::new().into()));
        append_id(&mut txn, &self.factions, faction_id, "squadIds", id);
    }
}

impl MissionDocCore {
    /// Set leader using the supplied domain data.
    pub fn set_leader(&self, squad_id: &str, slot_id: &str) {
        let mut txn = self.begin();
        set_leader_in_txn(&mut txn, &self.squads, squad_id, slot_id);
    }
}

impl MissionDocCore {
    /// Rename squad using the supplied domain data.
    pub fn rename_squad(&self, squad_id: &str, name: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(sq)) = self.squads.get(&txn, squad_id) {
            sq.insert(&mut txn, "name", name);
        }
    }
}

impl MissionDocCore {
    /// Reorder squads using the supplied domain data.
    pub fn reorder_squads(&self, faction_id: &str, squad_ids: &[String]) {
        let mut txn = self.begin();
        if self.factions.get(&txn, faction_id).is_none() {
            return;
        }
        let mut next: Vec<Any> = Vec::with_capacity(squad_ids.len());
        for sid in squad_ids {
            if let Some(Out::YMap(sq)) = self.squads.get(&txn, sid.as_str())
                && let Some(Out::Any(Any::String(fid))) = sq.get(&txn, "factionId")
                && fid.as_ref() == faction_id
            {
                next.push(Any::String(sid.as_str().into()));
            }
        }
        if let Some(Out::YMap(f)) = self.factions.get(&txn, faction_id) {
            f.insert(&mut txn, "squadIds", Any::Array(next.into()));
        }
    }
}

impl MissionDocCore {
    /// Emptying the source squad **deletes it** — the row, its place in `faction.squadIds`, and every vehicle attached to it (see [`garbage_collect_squad_in_txn`]). That is the drag-refile contract and every existing caller depends on it; a batch reassign wants the opposite and takes [`Self::move_slot_to_squad_keep_source`].
    pub fn move_slot_to_squad(&self, slot_id: &str, dest_squad_id: &str) {
        self.move_slot_between_squads(slot_id, dest_squad_id, false);
    }
}

impl MissionDocCore {
    /// The one thing the kept squad does *not* keep is a now-dangling `leaderSlotId`: the leader left with the last slot, and a squad pointing at a member it no longer has is the state [`ensure_leader_invariant_in_txn`] exists to prevent. An empty squad with no leader key is exactly what [`Self::add_squad`] mints, so the kept row lands back in that shape.
    pub fn move_slot_to_squad_keep_source(&self, slot_id: &str, dest_squad_id: &str) {
        self.move_slot_between_squads(slot_id, dest_squad_id, true);
    }
}

impl MissionDocCore {
    /// Move slot between squads using the supplied domain data.
    pub(super) fn move_slot_between_squads(
        &self,
        slot_id: &str,
        dest_squad_id: &str,
        keep_source: bool,
    ) {
        let mut txn = self.begin();
        if self.squads.get(&txn, dest_squad_id).is_none() {
            return;
        }
        let Some(Out::YMap(slot)) = self.slots.get(&txn, slot_id) else {
            return;
        };
        let Some(Out::Any(Any::String(src))) = slot.get(&txn, "squadId") else {
            return;
        };
        let source_squad_id = src.to_string();
        if source_squad_id == dest_squad_id {
            return;
        }

        let source_ids = read_id_array(&txn, &self.squads, &source_squad_id, "slotIds");
        if !source_ids
            .iter()
            .any(|a| matches!(a, Any::String(s) if s.as_ref() == slot_id))
        {
            return;
        }

        let was_leader = matches!(
            self.squads.get(&txn, source_squad_id.as_str()).and_then(|o| match o {
                Out::YMap(sq) => sq.get(&txn, "leaderSlotId"),
                _ => None,
            }),
            Some(Out::Any(Any::String(l))) if l.as_ref() == slot_id
        );

        let kept: Vec<Any> = source_ids
            .iter()
            .filter(|a| !matches!(a, Any::String(s) if s.as_ref() == slot_id))
            .cloned()
            .collect();
        if let Some(Out::YMap(src_sq)) = self.squads.get(&txn, source_squad_id.as_str()) {
            retain_in(&mut txn, &src_sq, SLOT_IDS, &HashSet::from([slot_id]));
        }

        append_id(&mut txn, &self.squads, dest_squad_id, "slotIds", slot_id);
        if let Some(Out::YMap(slot)) = self.slots.get(&txn, slot_id) {
            slot.insert(&mut txn, "squadId", dest_squad_id);
        }

        rewrite_slot_indices(&mut txn, &self.slots, &self.squads, &source_squad_id);
        rewrite_slot_indices(&mut txn, &self.slots, &self.squads, dest_squad_id);

        if kept.is_empty() && keep_source {
            if let Some(Out::YMap(src_sq)) = self.squads.get(&txn, source_squad_id.as_str()) {
                src_sq.remove(&mut txn, "leaderSlotId");
            }
        } else if kept.is_empty() {
            garbage_collect_squad_in_txn(
                &mut txn,
                &self.squads,
                &self.factions,
                &self.vehicles,
                &source_squad_id,
            );
        } else if was_leader && let Some(Any::String(next)) = kept.first() {
            set_leader_in_txn(&mut txn, &self.squads, &source_squad_id, next.as_ref());
        }

        ensure_leader_invariant_in_txn(
            &mut txn,
            &self.squads,
            &self.factions,
            &self.vehicles,
            dest_squad_id,
        );
    }
}

impl MissionDocCore {
    /// Remove squad using the supplied domain data.
    pub fn remove_squad(&self, squad_id: &str) {
        let mut txn = self.begin();
        let slot_ids: Vec<String> = read_id_array(&txn, &self.squads, squad_id, "slotIds")
            .iter()
            .filter_map(|a| match a {
                Any::String(s) => Some(s.to_string()),
                _ => None,
            })
            .collect();
        remove_slots_in_txn(
            &mut txn,
            &self.slots,
            &self.squads,
            &self.editor_layers,
            &slot_ids,
        );
        garbage_collect_squad_in_txn(
            &mut txn,
            &self.squads,
            &self.factions,
            &self.vehicles,
            squad_id,
        );
    }
}
