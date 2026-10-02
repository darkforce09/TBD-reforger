//! Role: lookup.
//! Position: `doc/store` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Arc;
use super::HashMap;
use super::INIT_ORIGIN;
use super::LOCAL_ORIGIN;
use super::Map;
use super::MapRef;
use super::MissionDocCore;
use super::Out;
use super::SquadMembership;
use super::StateVector;
use super::Update;
use super::ordered_rows;
use super::pending_briefing_markers_map;
use super::read_id_array;
use super::read_str;
use super::resolve_slot_side_key;
use yrs::types::ToJson;
use yrs::updates::decoder::Decode;
use yrs::{ReadTxn, Transact};

impl MissionDocCore {
    /// Client id using the supplied domain data.
    #[must_use]
    pub fn client_id(&self) -> u64 {
        self.doc.client_id().get()
    }
}

impl MissionDocCore {
    /// Toggle init-mode: while true, mutators stamp `INIT` (untracked). The JS wrapper brackets boot / hydrate / default-seeding with `set_origin_init(true)` … `set_origin_init(false)` so a load is never an undo step (mirrors `ydoc.ts` running those under `INIT_ORIGIN`).
    pub fn set_origin_init(&self, on: bool) {
        self.init_mode.set(on);
    }
}

impl MissionDocCore {
    /// Begin using the supplied domain data.
    pub(super) fn begin(&self) -> yrs::TransactionMut<'_> {
        let origin = if self.init_mode.get() {
            INIT_ORIGIN
        } else {
            LOCAL_ORIGIN
        };
        self.doc.transact_mut_with(origin)
    }
}

impl MissionDocCore {
    /// Apply a Yjs-wire (v1) update byte-stream — the exact bytes `Y.encodeStateAsUpdate(doc)` emits.
    pub fn apply_update(&self, bytes: &[u8]) -> Result<(), String> {
        let update = Update::decode_v1(bytes).map_err(|e| e.to_string())?;
        let me = self.doc.client_id();
        let my_clock = self.doc.transact().state_vector().get(&me);
        let claimed = update.state_vector().get(&me);

        if my_clock != 0 && claimed > my_clock {
            return Err(format!(
                "client id collision: incoming update carries blocks authored by client {me} up to \
                 clock {claimed}, but that is this document's own id and it has only issued {my_clock}. \
                 Another writer is authoring as us; applying this would interleave two authors into \
                 one history. Give every peer its own id — MissionDocCore::new() mints one."
            ));
        }

        let mut txn = self.doc.transact_mut_with(INIT_ORIGIN);
        txn.apply_update(update).map_err(|e| e.to_string())
    }
}

impl MissionDocCore {
    /// Encode state using the supplied domain data.
    #[must_use]
    pub fn encode_state(&self) -> Vec<u8> {
        self.doc
            .transact()
            .encode_state_as_update_v1(&StateVector::default())
    }
}

impl MissionDocCore {
    /// Small maps json using the supplied domain data.
    #[must_use]
    pub fn small_maps_json(&self) -> String {
        let meta = self.doc.get_or_insert_map("meta");
        let payload_extras = self.doc.get_or_insert_map("payloadExtras");
        let entity_order = self.doc.get_or_insert_map("entityOrder");
        let named: [(&str, MapRef); 14] = [
            ("factionsById", self.doc.get_or_insert_map("factions")),
            ("squadsById", self.doc.get_or_insert_map("squads")),
            ("loadoutsById", self.doc.get_or_insert_map("loadouts")),
            ("itemsById", self.doc.get_or_insert_map("items")),
            ("objectivesById", self.doc.get_or_insert_map("objectives")),
            ("vehiclesById", self.doc.get_or_insert_map("vehicles")),
            ("entitiesById", self.doc.get_or_insert_map("entities")),
            ("zonesById", self.doc.get_or_insert_map("zones")),
            ("markersById", self.doc.get_or_insert_map("markers")),
            (
                "editorLayersById",
                self.doc.get_or_insert_map("editorLayers"),
            ),
            (
                "compositionsById",
                self.doc.get_or_insert_map("compositions"),
            ),
            ("triggersById", self.doc.get_or_insert_map("triggers")),
            ("commentsById", self.doc.get_or_insert_map("comments")),
            ("connectionsById", self.doc.get_or_insert_map("connections")),
        ];

        let txn = self.doc.transact();
        let mut root: HashMap<String, Any> = HashMap::new();
        root.insert(
            "meta".to_string(),
            if meta.len(&txn) == 0 {
                Any::Null
            } else {
                meta.to_json(&txn)
            },
        );
        for (key, map) in &named {
            root.insert((*key).to_string(), map.to_json(&txn));
        }

        let zone_rows = ordered_rows(&txn, &self.zones, &entity_order, "zones");

        let comp_rows = ordered_rows(&txn, &self.compositions, &entity_order, "compositions");

        let trigger_rows = ordered_rows(&txn, &self.triggers, &entity_order, "triggers");

        let comment_rows = ordered_rows(&txn, &self.comments, &entity_order, "comments");

        let connection_rows = ordered_rows(&txn, &self.connections, &entity_order, "connections");

        if payload_extras.len(&txn) > 0
            || !zone_rows.is_empty()
            || !comp_rows.is_empty()
            || !trigger_rows.is_empty()
            || !comment_rows.is_empty()
            || !connection_rows.is_empty()
        {
            let mut extras: HashMap<String, Any> = match payload_extras.to_json(&txn) {
                Any::Map(m) => (*m).clone(),
                _ => HashMap::new(),
            };
            if zone_rows.is_empty() {
                extras.remove("zones");
            } else {
                extras.insert("zones".to_string(), Any::Array(zone_rows.into()));
            }

            if comp_rows.is_empty() {
                extras.remove("compositions");
            } else {
                extras.insert("compositions".to_string(), Any::Array(comp_rows.into()));
            }

            if trigger_rows.is_empty() {
                extras.remove("triggers");
            } else {
                extras.insert("triggers".to_string(), Any::Array(trigger_rows.into()));
            }

            if comment_rows.is_empty() {
                extras.remove("comments");
            } else {
                extras.insert("comments".to_string(), Any::Array(comment_rows.into()));
            }

            if connection_rows.is_empty() {
                extras.remove("connections");
            } else {
                extras.insert(
                    "connections".to_string(),
                    Any::Array(connection_rows.into()),
                );
            }
            if !extras.is_empty() {
                root.insert("payloadExtras".to_string(), Any::Map(Arc::new(extras)));
            }
        }
        if entity_order.len(&txn) > 0 {
            root.insert("entityOrder".to_string(), entity_order.to_json(&txn));
        }

        let mut buf = String::new();
        Any::Map(Arc::new(root)).to_json(&mut buf);
        buf
    }
}

impl MissionDocCore {
    /// The `slots` map as a JSON object (`slotsById`) — full, **exact-f64** `Slot`s for the non-render readers (compile / persistence / the store mirror). Together with `small_maps_json` this reproduces the entire `MapSnapshot`. O(n) JSON — a one-shot (save), never the render hot path, which reads the f32 SoA (positions there are f32-truncated, fine for pixels, lossy for compile).
    #[must_use]
    pub fn slots_json(&self) -> String {
        let txn = self.doc.transact();
        let mut buf = String::new();
        self.slots.to_json(&txn).to_json(&mut buf);
        buf
    }
}

impl MissionDocCore {
    /// One `MapRef::contains_key` on the raw `slots` root map, under a read transaction. It is the existence answer, and the ONLY correct one:.
    #[must_use]
    pub fn slot_exists(&self, id: &str) -> bool {
        let txn = self.doc.transact();
        self.slots.contains_key(&txn, id)
    }
}

impl MissionDocCore {
    /// Raw membership for authoring operations, including layer-hidden and editor-hidden slots. Like [`Self::slot_exists`], this reads one row without filtering through the render SoA or serialising every slot. Missing slots or missing memberships return `None`.
    #[must_use]
    pub fn slot_squad_id(&self, id: &str) -> Option<String> {
        let txn = self.doc.transact();
        let Out::YMap(slot) = self.slots.get(&txn, id)? else {
            return None;
        };
        match slot.get(&txn, "squadId")? {
            Out::Any(Any::String(squad)) => Some(squad.to_string()),
            _ => None,
        }
    }
}

impl MissionDocCore {
    /// Side key resolution count using the supplied domain data.
    #[must_use]
    pub fn side_key_resolution_count(&self) -> u64 {
        self.side_key_resolutions.get()
    }
}

impl MissionDocCore {
    /// Squad link inputs using the supplied domain data.
    #[must_use]
    pub fn squad_link_inputs(&self) -> Vec<SquadMembership> {
        let txn = self.doc.transact();
        let mut out = Vec::new();
        for (squad_id, out_v) in self.squads.iter(&txn) {
            let Out::YMap(sq) = out_v else {
                continue;
            };
            let leader_slot_id = read_str(&txn, &sq, "leaderSlotId").unwrap_or_default();
            let member_slot_ids: Vec<String> =
                read_id_array(&txn, &self.squads, squad_id, "slotIds")
                    .iter()
                    .filter_map(|a| match a {
                        Any::String(s) => Some(s.to_string()),
                        _ => None,
                    })
                    .collect();
            let side = resolve_slot_side_key(&txn, &self.squads, &self.factions, squad_id);
            out.push(SquadMembership {
                leader_slot_id,
                member_slot_ids,
                side,
            });
        }
        out
    }
}

impl MissionDocCore {
    /// Number of slots currently in the document.
    #[must_use]
    pub fn slot_count(&self) -> usize {
        self.slots.len(&self.doc.transact()) as usize
    }
}

impl MissionDocCore {
    /// True if the doc holds authored content beyond seeded defaults — any faction / slot / objective / vehicle / entity / zone / marker. Backs `useMissionEditor.hasLocalContent` (the warm-session / conflict gate).
    #[must_use]
    pub fn has_content(&self) -> bool {
        let txn = self.doc.transact();
        self.factions.len(&txn) > 0
            || self.slots.len(&txn) > 0
            || self.objectives.len(&txn) > 0
            || self.vehicles.len(&txn) > 0
            || self.entities.len(&txn) > 0
            || self.zones.len(&txn) > 0
            || self.compositions.len(&txn) > 0
            || self.comments.len(&txn) > 0
            || self.markers.len(&txn) > 0
            || !pending_briefing_markers_map(&txn, &self.meta).is_empty()
    }
}
