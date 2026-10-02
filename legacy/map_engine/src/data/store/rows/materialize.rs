//! Role: materialize.
//! Position: `doc/store` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::ENTITY_IDS;
use super::HashMap;
use super::Interner;
use super::MissionDocCore;
use super::NONE_IDX;
use super::Out;
use super::SideKeyMemo;
use super::SlotSoa;
use super::layer_flag_effective;
use super::read_bool;
use super::read_field_ids;
use super::read_position;
use super::read_stance;
use super::read_str;
use super::resolve_slot_side_key;
use yrs::Map;
use yrs::Transact;

impl MissionDocCore {
    /// Materialize every slot into the columnar [`SlotSoa`] (criterion 1). Keyed by `ids[row]`.
    #[must_use]
    pub fn materialize(&self) -> SlotSoa {
        let txn = self.doc.transact();

        let mut slot_layer: HashMap<String, String> = HashMap::new();
        let mut hidden_layers: HashMap<String, bool> = HashMap::new();
        for (layer_id, out) in self.editor_layers.iter(&txn) {
            if let Out::YMap(layer) = out {
                for sid in read_field_ids(&txn, &layer, ENTITY_IDS) {
                    slot_layer
                        .entry(sid)
                        .or_insert_with(|| layer_id.to_string());
                }
                hidden_layers
                    .entry(layer_id.to_string())
                    .or_insert_with(|| {
                        layer_flag_effective(&txn, &self.editor_layers, layer_id, "hidden")
                    });
            }
        }

        let mut soa = SlotSoa::default();
        let mut roles = Interner::new();
        let mut tags = Interner::new();
        let mut squads = Interner::new();
        let mut layers = Interner::new();

        let mut per_call = HashMap::new();
        let mut memo_borrow = self.side_key_memo.as_ref().map(SideKeyMemo::entries);
        let memo: &mut HashMap<String, String> = match memo_borrow.as_deref_mut() {
            Some(m) => m,
            None => &mut per_call,
        };

        for (id, out) in self.slots.iter(&txn) {
            let Out::YMap(slot) = out else { continue };

            let layer_hidden = slot_layer
                .get(id)
                .is_some_and(|l| hidden_layers.get(l).copied().unwrap_or(false));
            if layer_hidden || read_bool(&txn, &slot, "editorHidden") {
                continue;
            }
            let (x, y, z, rot) = read_position(&txn, &slot);
            soa.ids.push(id.to_string());
            soa.xs.push(x as f32);
            soa.ys.push(y as f32);
            soa.xy.push(x as f32);
            soa.xy.push(y as f32);
            soa.zs.push(z as f32);
            soa.rotations.push(rot as f32);
            soa.stance.push(read_stance(&txn, &slot));
            soa.role_idx
                .push(roles.intern(read_str(&txn, &slot, "role").as_deref().unwrap_or("")));
            soa.tag_idx.push(match read_str(&txn, &slot, "tag") {
                Some(t) => tags.intern(&t),
                None => NONE_IDX,
            });
            let squad_id = read_str(&txn, &slot, "squadId").unwrap_or_default();
            soa.squad_idx.push(squads.intern(&squad_id));
            soa.layer_idx.push(match slot_layer.get(id) {
                Some(l) => layers.intern(l),
                None => NONE_IDX,
            });

            let side_key = match memo.get(&squad_id) {
                Some(k) => k.clone(),
                None => {
                    self.side_key_resolutions
                        .set(self.side_key_resolutions.get().saturating_add(1));
                    let resolved =
                        resolve_slot_side_key(&txn, &self.squads, &self.factions, &squad_id);
                    memo.insert(squad_id.clone(), resolved.clone());
                    resolved
                }
            };
            soa.side_keys.push(side_key);
        }

        soa.roles = roles.words;
        soa.tags = tags.words;
        soa.squads = squads.words;
        soa.layers = layers.words;
        soa
    }
}
