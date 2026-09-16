//! Role: paste.
//! Position: `doc/store` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Arc;
use super::ENTITY_IDS;
use super::HashMap;
use super::Map;
use super::MapPrelim;
use super::MissionDocCore;
use super::PASTE_KNOWN_SLOT_KEYS;
use super::SLOT_IDS;
use super::append_id;
use super::json_str_to_any;
use super::read_id_array;

impl MissionDocCore {
    /// Paste slots using the supplied domain data.
    #[allow(clippy::too_many_arguments)]
    pub fn paste_slots(
        &self,
        ids: Vec<String>,
        squad_ids: Vec<String>,
        layer_ids: Vec<String>,
        src_x: Vec<f64>,
        src_y: Vec<f64>,
        src_rot: Vec<f64>,
        zs: Vec<f64>,
        roles: Vec<String>,
        tags: Vec<String>,
        asset_ids: Vec<String>,
        stances: Vec<String>,
        loadouts: Vec<String>,
        extras_json: Vec<String>,
        anchor_x: Option<f64>,
        anchor_y: Option<f64>,
        width: f64,
        height: f64,
    ) {
        let n = ids.len();
        if n == 0 {
            return;
        }

        let cx = src_x.iter().sum::<f64>() / n as f64;
        let cy = src_y.iter().sum::<f64>() / n as f64;

        let (dx, dy) = match (anchor_x, anchor_y) {
            (Some(ax), Some(ay)) => (ax - cx, ay - cy),
            _ => (0.0, 0.0),
        };

        let mut txn = self.begin();
        for i in 0..n {
            let squad_id = &squad_ids[i];
            let layer_id = &layer_ids[i];
            let index = read_id_array(&txn, &self.squads, squad_id, SLOT_IDS).len() as i64;
            let px = (src_x[i] + dx).clamp(0.0, width);
            let py = (src_y[i] + dy).clamp(0.0, height);
            let id = ids[i].as_str();
            let slot = self
                .slots
                .insert(&mut txn, id, MapPrelim::from([("id", id)]));
            slot.insert(&mut txn, "squadId", squad_id.as_str());
            slot.insert(&mut txn, "index", Any::BigInt(index));
            slot.insert(&mut txn, "role", roles[i].as_str());
            if !tags[i].is_empty() {
                slot.insert(&mut txn, "tag", tags[i].as_str());
            }
            if !asset_ids[i].is_empty() {
                slot.insert(&mut txn, "assetId", asset_ids[i].as_str());
            }
            let z = zs.get(i).copied().unwrap_or(0.0);

            let mut pos = HashMap::new();
            pos.insert("x".to_string(), Any::Number(px));
            pos.insert("y".to_string(), Any::Number(py));
            pos.insert("z".to_string(), Any::Number(z));
            pos.insert("rotation".to_string(), Any::Number(src_rot[i]));
            slot.insert(&mut txn, "stance", stances[i].as_str());
            slot.insert(&mut txn, "loadoutId", Any::Null);

            if let Some(lj) = loadouts.get(i).filter(|s| !s.is_empty()) {
                slot.insert(&mut txn, "loadout", json_str_to_any(lj));
            }

            if let Some(extra) = extras_json.get(i).filter(|s| !s.is_empty())
                && let Any::Map(fields) = json_str_to_any(extra)
            {
                for (k, v) in fields.iter() {
                    if PASTE_KNOWN_SLOT_KEYS.contains(&k.as_str()) {
                        if k == "position"
                            && let Any::Map(sub) = v
                        {
                            for (pk, pv) in sub.iter() {
                                if !matches!(pk.as_str(), "x" | "y" | "z" | "rotation") {
                                    pos.insert(pk.clone(), pv.clone());
                                }
                            }
                        }
                        continue;
                    }
                    slot.insert(&mut txn, k.as_str(), v.clone());
                }
            }
            slot.insert(&mut txn, "position", Any::Map(Arc::new(pos)));
            append_id(&mut txn, &self.squads, squad_id, SLOT_IDS, id);
            append_id(&mut txn, &self.editor_layers, layer_id, ENTITY_IDS, id);
        }
    }
}
