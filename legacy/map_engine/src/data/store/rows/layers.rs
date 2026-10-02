//! Role: layers.
//! Position: `doc/store` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::ENTITY_IDS;
use super::HashSet;
use super::MapPrelim;
use super::MissionDocCore;
use super::Out;
use super::ReadTxn;
use super::append_id;
use super::insert_empty_native;
use super::read_bool;
use super::read_field_ids;
use super::remove_id_from_all_layers;
use super::remove_slots_in_txn;
use super::set_slot_editor_hidden_in_txn;
use super::slot_is_transform_locked;
use yrs::Map;
use yrs::Transact;

impl MissionDocCore {
    /// Delete an Outliner folder AND its whole subtree — every nested folder plus all filed slots — in one transaction (mirrors `ydoc.removeEditorLayer` @500). No-op if the folder is absent or it is the only layer (keep ≥1). If the subtree was every layer, a fresh default layer is reseeded (JS mints `reseed_id`) so the editor is never layer-less.
    pub fn remove_editor_layer(&self, id: &str, reseed_id: &str) {
        let mut txn = self.begin();
        if self.editor_layers.get(&txn, id).is_none() || self.editor_layers.len(&txn) <= 1 {
            return;
        }

        let mut subtree: HashSet<String> = HashSet::new();
        subtree.insert(id.to_string());
        loop {
            let parents: Vec<(String, Option<String>)> = self
                .editor_layers
                .iter(&txn)
                .map(|(lid, out)| {
                    let pid = match out {
                        Out::YMap(l) => match l.get(&txn, "parentId") {
                            Some(Out::Any(Any::String(p))) => Some(p.to_string()),
                            _ => None,
                        },
                        _ => None,
                    };
                    (lid.to_string(), pid)
                })
                .collect();
            let mut added = false;
            for (lid, pid) in parents {
                if let Some(p) = pid
                    && subtree.contains(&p)
                    && !subtree.contains(&lid)
                {
                    subtree.insert(lid);
                    added = true;
                }
            }
            if !added {
                break;
            }
        }

        let mut slot_ids: Vec<String> = Vec::new();
        for lid in &subtree {
            if let Some(Out::YMap(layer)) = self.editor_layers.get(&txn, lid) {
                slot_ids.extend(read_field_ids(&txn, &layer, ENTITY_IDS));
            }
        }
        remove_slots_in_txn(
            &mut txn,
            &self.slots,
            &self.squads,
            &self.editor_layers,
            &slot_ids,
        );
        for lid in &subtree {
            self.editor_layers.remove(&mut txn, lid);
        }
        if self.editor_layers.len(&txn) == 0 {
            let layer = self.editor_layers.insert(
                &mut txn,
                reseed_id,
                MapPrelim::from([("id", reseed_id)]),
            );
            layer.insert(&mut txn, "name", "Default Layer");
            layer.insert(&mut txn, "parentId", Any::Null);
            insert_empty_native(&mut txn, &layer, ENTITY_IDS);
        }
    }
}

impl MissionDocCore {
    /// Create an Outliner folder (id + name computed JS-side). Mirrors `ydoc.addEditorLayer`.
    pub fn add_editor_layer(&self, id: &str, name: &str, parent_id: Option<String>) {
        let mut txn = self.begin();
        let layer = self
            .editor_layers
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        layer.insert(&mut txn, "name", name);
        match parent_id {
            Some(p) => layer.insert(&mut txn, "parentId", p),
            None => layer.insert(&mut txn, "parentId", Any::Null),
        };
        insert_empty_native(&mut txn, &layer, ENTITY_IDS);
    }
}

impl MissionDocCore {
    /// Rename an Outliner folder. Mirrors `ydoc.renameEditorLayer`.
    pub fn rename_editor_layer(&self, id: &str, name: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(layer)) = self.editor_layers.get(&txn, id) {
            layer.insert(&mut txn, "name", name);
        }
    }
}

impl MissionDocCore {
    /// This is the ATTR-FIELD-LYR-ENABLE-VIS writer, and it is deliberately a sibling of [`Self::rename_editor_layer`]: like `name`, `hidden` is a per-layer property that rides the layer row in the doc, so it PERSISTS with the mission and goes through [`Self::begin`] — a LOCAL flip is one undo step, exactly like a rename ([`Self::hidden_flag_is_one_undo_step`]).
    pub fn set_editor_layer_hidden(&self, id: &str, hidden: bool) {
        let mut txn = self.begin();
        if let Some(Out::YMap(layer)) = self.editor_layers.get(&txn, id) {
            if hidden {
                layer.insert(&mut txn, "hidden", true);
            } else {
                layer.remove(&mut txn, "hidden");
            }
        }
    }
}

impl MissionDocCore {
    /// Set editor layer locked using the supplied domain data.
    pub fn set_editor_layer_locked(&self, id: &str, locked: bool) {
        let mut txn = self.begin();
        if let Some(Out::YMap(layer)) = self.editor_layers.get(&txn, id) {
            if locked {
                layer.insert(&mut txn, "locked", true);
            } else {
                layer.remove(&mut txn, "locked");
            }
        }
    }
}

impl MissionDocCore {
    /// Slot layer is locked using the supplied domain data.
    #[must_use]
    pub fn slot_layer_is_locked(&self, slot_id: &str) -> bool {
        let txn = self.doc.transact();
        slot_is_transform_locked(&txn, &self.editor_layers, slot_id)
    }
}

impl MissionDocCore {
    /// Set slot editor hidden using the supplied domain data.
    pub fn set_slot_editor_hidden(&self, id: &str, hidden: bool) {
        let mut txn = self.begin();
        set_slot_editor_hidden_in_txn(&mut txn, &self.slots, id, hidden);
    }
}

impl MissionDocCore {
    /// Set slots editor hidden using the supplied domain data.
    pub fn set_slots_editor_hidden(&self, ids: &[String], hidden: bool) {
        let mut txn = self.begin();
        for id in ids {
            set_slot_editor_hidden_in_txn(&mut txn, &self.slots, id, hidden);
        }
    }
}

impl MissionDocCore {
    /// Clear all editor hidden using the supplied domain data.
    pub fn clear_all_editor_hidden(&self) -> usize {
        let mut txn = self.begin();
        let hidden_ids: Vec<String> = self
            .slots
            .iter(&txn)
            .filter_map(|(id, out)| match out {
                Out::YMap(slot) if read_bool(&txn, &slot, "editorHidden") => Some(id.to_string()),
                _ => None,
            })
            .collect();
        for id in &hidden_ids {
            set_slot_editor_hidden_in_txn(&mut txn, &self.slots, id, false);
        }
        hidden_ids.len()
    }
}

impl MissionDocCore {
    /// Reparent an Outliner folder; rejects cycles (dropping it into its own subtree). Mirrors `ydoc.reparentEditorLayer`.
    pub fn reparent_editor_layer(&self, id: &str, new_parent_id: Option<String>) {
        let mut txn = self.begin();
        if self.editor_layers.get(&txn, id).is_none() {
            return;
        }
        if let Some(p) = new_parent_id.as_deref()
            && (p == id || self.is_layer_descendant(&txn, id, p))
        {
            return;
        }
        if let Some(Out::YMap(layer)) = self.editor_layers.get(&txn, id) {
            match new_parent_id {
                Some(p) => layer.insert(&mut txn, "parentId", p),
                None => layer.insert(&mut txn, "parentId", Any::Null),
            };
        }
    }
}

impl MissionDocCore {
    /// Refile a slot into a different Outliner folder (workflow-only; squad unchanged): detach from every folder holding it, then append to the target. Mirrors `ydoc.moveSlotToLayer`.
    pub fn move_slot_to_layer(&self, slot_id: &str, target_layer_id: &str) {
        let mut txn = self.begin();
        if self.editor_layers.get(&txn, target_layer_id).is_none() {
            return;
        }

        remove_id_from_all_layers(&mut txn, &self.editor_layers, slot_id);
        append_id(
            &mut txn,
            &self.editor_layers,
            target_layer_id,
            ENTITY_IDS,
            slot_id,
        );
    }
}

impl MissionDocCore {
    /// Is layer descendant using the supplied domain data.
    pub(super) fn is_layer_descendant<T: ReadTxn>(
        &self,
        txn: &T,
        ancestor_id: &str,
        node_id: &str,
    ) -> bool {
        let mut cur = Some(node_id.to_string());
        while let Some(c) = cur {
            if c == ancestor_id {
                return true;
            }
            cur = match self.editor_layers.get(txn, &c) {
                Some(Out::YMap(layer)) => match layer.get(txn, "parentId") {
                    Some(Out::Any(Any::String(p))) => Some(p.to_string()),
                    _ => None,
                },
                _ => None,
            };
        }
        false
    }
}
