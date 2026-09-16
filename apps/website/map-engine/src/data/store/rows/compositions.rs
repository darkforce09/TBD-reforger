//! Role: compositions.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Arc;
use super::ENTITY_IDS;
use super::HashMap;
use super::Map;
use super::MapPrelim;
use super::MissionDocCore;
use super::any_to_f64;
use super::append_id;
use super::comment_row;
use super::json_str_to_any;
use super::position_any;
use super::read_composition_map;
use yrs::Transact;
use yrs::types::ToJson;

impl MissionDocCore {
    /// Add composition using the supplied domain data.
    pub fn add_composition(&self, id: &str, row_json: &str) {
        let Any::Map(fields) = json_str_to_any(row_json) else {
            return;
        };
        let mut map: HashMap<String, Any> = (*fields).clone();
        map.insert("id".to_string(), Any::String(id.into()));
        let mut txn = self.begin();
        self.compositions
            .insert(&mut txn, id, Any::Map(Arc::new(map)));
    }
}

impl MissionDocCore {
    /// Set composition title using the supplied domain data.
    pub fn set_composition_title(&self, id: &str, title: &str) {
        self.set_composition_field(id, "title", title);
    }
}

impl MissionDocCore {
    /// Set composition category using the supplied domain data.
    pub fn set_composition_category(&self, id: &str, category: &str) {
        self.set_composition_field(id, "category", category);
    }
}

impl MissionDocCore {
    /// Set composition author using the supplied domain data.
    pub fn set_composition_author(&self, id: &str, author: &str) {
        self.set_composition_field(id, "author", author);
    }
}

impl MissionDocCore {
    /// Set composition field using the supplied domain data.
    pub(super) fn set_composition_field(&self, id: &str, key: &str, value: &str) {
        let mut txn = self.begin();
        let Some(mut row) = read_composition_map(&txn, &self.compositions, id) else {
            return;
        };
        row.insert(key.to_string(), Any::String(value.into()));
        self.compositions
            .insert(&mut txn, id, Any::Map(Arc::new(row)));
    }
}

impl MissionDocCore {
    /// Remove composition using the supplied domain data.
    pub fn remove_composition(&self, id: &str) {
        let mut txn = self.begin();
        self.compositions.remove(&mut txn, id);
    }
}

impl MissionDocCore {
    /// Compositions json using the supplied domain data.
    #[must_use]
    pub fn compositions_json(&self) -> String {
        let txn = self.doc.transact();
        let mut buf = String::new();
        self.compositions.to_json(&txn).to_json(&mut buf);
        buf
    }
}

impl MissionDocCore {
    /// Composition count using the supplied domain data.
    #[must_use]
    pub fn composition_count(&self) -> usize {
        self.compositions.len(&self.doc.transact()) as usize
    }
}

impl MissionDocCore {
    /// Each entry's world position is `drop + (dx, dz)` — the RELATIVE-OFFSET entries are re-anchored so the composition's captured centroid lands under the cursor, clamped to the terrain `width`/`height` (the `paste_slots` clamp). `ids[i]` is the pre-minted id for `entities[i]` (minted by `editor_ops`, which proves uniqueness against the live doc); the caller passes as many ids as there are entities.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn place_composition(
        &self,
        entities_json: &str,
        ids: &[String],
        side: &str,
        layer_id: &str,
        drop_x: f64,
        drop_y: f64,
        width: f64,
        height: f64,
    ) -> Vec<String> {
        let Any::Array(entities) = json_str_to_any(entities_json) else {
            return Vec::new();
        };
        let mut written = Vec::new();
        let mut txn = self.begin();

        let faction_id = format!("faction-{side}");
        if self.factions.get(&txn, &faction_id).is_none() {
            let f = self.factions.insert(
                &mut txn,
                faction_id.as_str(),
                MapPrelim::from([("id", faction_id.as_str())]),
            );
            f.insert(&mut txn, "key", side);
            f.insert(&mut txn, "name", side);
            f.insert(&mut txn, "squadIds", Any::Array(Vec::new().into()));
        }

        let g_str = |m: &HashMap<String, Any>, k: &str| match m.get(k) {
            Some(Any::String(s)) => s.to_string(),
            _ => String::new(),
        };
        let g_num = |m: &HashMap<String, Any>, k: &str| m.get(k).map_or(0.0, any_to_f64);

        for (i, ent) in entities.iter().enumerate() {
            let Some(id) = ids.get(i) else { break };
            let Any::Map(fields) = ent else { continue };
            let kind = g_str(fields, "kind");
            let wx = (drop_x + g_num(fields, "dx")).clamp(0.0, width);
            let wy = (drop_y + g_num(fields, "dz")).clamp(0.0, height);
            let rot = g_num(fields, "rotation");

            let elev = g_num(fields, "elevation");
            match kind.as_str() {
                "slot" => {
                    let slot = self.slots.insert(
                        &mut txn,
                        id.as_str(),
                        MapPrelim::from([("id", id.as_str())]),
                    );

                    slot.insert(&mut txn, "index", Any::BigInt(0));
                    let role = g_str(fields, "role");
                    slot.insert(
                        &mut txn,
                        "role",
                        if role.is_empty() {
                            "Rifleman"
                        } else {
                            role.as_str()
                        },
                    );
                    let tag = g_str(fields, "tag");
                    if !tag.is_empty() {
                        slot.insert(&mut txn, "tag", tag.as_str());
                    }
                    let asset = g_str(fields, "assetId");
                    if !asset.is_empty() {
                        slot.insert(&mut txn, "assetId", asset.as_str());
                    }
                    let stance = g_str(fields, "stance");
                    slot.insert(
                        &mut txn,
                        "stance",
                        if stance.is_empty() {
                            "stand"
                        } else {
                            stance.as_str()
                        },
                    );
                    slot.insert(&mut txn, "loadoutId", Any::Null);
                    if let Some(l) = fields.get("loadout").filter(|l| !matches!(l, Any::Null)) {
                        slot.insert(&mut txn, "loadout", l.clone());
                    }
                    slot.insert(&mut txn, "position", position_any(wx, wy, elev, rot));
                    append_id(&mut txn, &self.editor_layers, layer_id, ENTITY_IDS, id);
                    written.push(id.clone());
                }
                "vehicle" => {
                    let resource = g_str(fields, "resourceName");
                    if resource.is_empty() {
                        continue;
                    }
                    let v = self.vehicles.insert(
                        &mut txn,
                        id.as_str(),
                        MapPrelim::from([("id", id.as_str())]),
                    );
                    v.insert(&mut txn, "resourceName", resource.as_str());
                    v.insert(&mut txn, "position", position_any(wx, wy, elev, rot));
                    v.insert(&mut txn, "factionId", faction_id.as_str());

                    if fields.get("crewed") == Some(&Any::Bool(false)) {
                        v.insert(&mut txn, "crewed", false);
                    }

                    if let Some(Any::Map(crew)) = fields.get("crew")
                        && !crew.is_empty()
                    {
                        v.insert(&mut txn, "crew", Any::Map(crew.clone()));
                    }
                    written.push(id.clone());
                }
                "object" => {
                    let resource = g_str(fields, "resourceName");
                    let alias = g_str(fields, "alias");
                    if resource.is_empty() || alias.is_empty() {
                        continue;
                    }
                    let e = self.entities.insert(
                        &mut txn,
                        id.as_str(),
                        MapPrelim::from([("id", id.as_str())]),
                    );
                    e.insert(&mut txn, "alias", alias.as_str());
                    e.insert(&mut txn, "resourceName", resource.as_str());
                    e.insert(&mut txn, "position", position_any(wx, wy, elev, rot));
                    let faction = g_str(fields, "faction");
                    if !faction.is_empty() {
                        e.insert(&mut txn, "faction", faction.as_str());
                    }
                    written.push(id.clone());
                }

                "comment" => {
                    let title = g_str(fields, "title");
                    let tooltip = g_str(fields, "tooltip");
                    self.comments.insert(
                        &mut txn,
                        id.as_str(),
                        Any::Map(Arc::new(comment_row(id, &title, &tooltip, wx, wy))),
                    );
                    append_id(&mut txn, &self.editor_layers, layer_id, ENTITY_IDS, id);
                    written.push(id.clone());
                }
                _ => {}
            }
        }
        written
    }
}
