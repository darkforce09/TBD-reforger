//! Role: merge index.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::HashMap;
use super::HashSet;
use super::Map;
use super::MapRef;
use super::MissionDocCore;
use super::NamedSideIndex;
use super::any_map_str;
use super::ordered_rows;
use yrs::Transact;

impl MissionDocCore {
    /// Merge resident index using the supplied domain data.
    pub(super) fn merge_resident_index(
        &self,
        entity_order: &MapRef,
    ) -> (NamedSideIndex, NamedSideIndex, HashSet<String>) {
        let txn = self.doc.transact();
        let mut resident_ids: HashSet<String> = HashSet::new();
        for map in [
            &self.slots,
            &self.squads,
            &self.factions,
            &self.editor_layers,
            &self.vehicles,
            &self.entities,
            &self.zones,
            &self.triggers,
            &self.compositions,
            &self.markers,
        ] {
            for (id, _out) in map.iter(&txn) {
                resident_ids.insert(id.to_string());
            }
        }
        let fac_rows = ordered_rows(&txn, &self.factions, entity_order, "factions");

        let mut fac_index: HashMap<(String, String), String> = HashMap::new();
        let mut fac_key_by_id: HashMap<String, String> = HashMap::new();
        for row in &fac_rows {
            if let Any::Map(m) = row {
                let id = any_map_str(m, "id");
                let key = any_map_str(m, "key");
                let name = any_map_str(m, "name");
                if let Some(id) = &id {
                    if let Some(k) = &key {
                        fac_key_by_id.insert(id.clone(), k.clone());
                    }
                    if let (Some(k), Some(n)) = (&key, &name) {
                        fac_index
                            .entry((n.clone(), k.clone()))
                            .or_insert(id.clone());
                    }
                }
            }
        }
        let sq_rows = ordered_rows(&txn, &self.squads, entity_order, "squads");

        let mut sq_index: HashMap<(String, String), String> = HashMap::new();
        for row in &sq_rows {
            if let Any::Map(m) = row {
                let id = any_map_str(m, "id");
                let name = any_map_str(m, "name");
                let side =
                    any_map_str(m, "factionId").and_then(|fid| fac_key_by_id.get(&fid).cloned());
                if let (Some(id), Some(n), Some(s)) = (id, name, side) {
                    sq_index.entry((n, s)).or_insert(id);
                }
            }
        }
        (fac_index, sq_index, resident_ids)
    }
}
