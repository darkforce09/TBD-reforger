//! Role: formations.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::MissionDocCore;
use super::Out;
use super::formation_offsets;
use super::position_any_merged;
use super::read_id_array;
use super::read_position_map;
use super::read_str;
use yrs::Map;
use yrs::Transact;

impl MissionDocCore {
    /// **The leader does not move.** Eden's `ForceToFormation` re-forms the group AROUND its leader; moving the leader too would translate the whole squad and make the action a nobody-asked-for reposition. So the leader is the anchor and the members take the offsets.
    pub fn force_to_formation(&self, leader_slot_id: &str, formation: &str) -> usize {
        let members = self.squad_members_of_leader(leader_slot_id);
        if members.is_empty() {
            return 0;
        }
        let mut txn = self.begin();
        let Some(Out::YMap(leader)) = self.slots.get(&txn, leader_slot_id) else {
            return 0;
        };
        let anchor = read_position_map(&txn, &leader);
        let px = |k: &str| match anchor.get(k) {
            Some(Any::Number(n)) => *n,
            #[allow(clippy::cast_precision_loss)]
            Some(Any::BigInt(i)) => *i as f64,
            _ => 0.0,
        };
        let (lx, ly, heading) = (px("x"), px("y"), px("rotation"));
        let offsets = formation_offsets(formation, members.len());
        let (sin_h, cos_h) = heading.to_radians().sin_cos();
        let mut moved = 0usize;
        for (member, (ox, oy)) in members.iter().zip(offsets) {
            let Some(Out::YMap(slot)) = self.slots.get(&txn, member.as_str()) else {
                continue;
            };
            let existing = read_position_map(&txn, &slot);

            let wx = lx + ox.mul_add(cos_h, oy * sin_h);
            let wy = ly + oy.mul_add(cos_h, -(ox * sin_h));
            let z = match existing.get("z") {
                Some(Any::Number(n)) => *n,
                #[allow(clippy::cast_precision_loss)]
                Some(Any::BigInt(i)) => *i as f64,
                _ => 0.0,
            };
            slot.insert(
                &mut txn,
                "position",
                position_any_merged(existing, wx, wy, z, heading),
            );
            moved += 1;
        }
        moved
    }
}

impl MissionDocCore {
    /// Squad members of leader using the supplied domain data.
    #[must_use]
    pub(super) fn squad_members_of_leader(&self, leader_slot_id: &str) -> Vec<String> {
        if leader_slot_id.is_empty() {
            return Vec::new();
        }
        let txn = self.doc.transact();
        for (squad_id, out_v) in self.squads.iter(&txn) {
            let Out::YMap(sq) = out_v else { continue };
            if read_str(&txn, &sq, "leaderSlotId").as_deref() != Some(leader_slot_id) {
                continue;
            }
            return read_id_array(&txn, &self.squads, squad_id, "slotIds")
                .iter()
                .filter_map(|a| match a {
                    Any::String(s) if s.as_ref() != leader_slot_id => Some(s.to_string()),
                    _ => None,
                })
                .collect();
        }
        Vec::new()
    }
}
