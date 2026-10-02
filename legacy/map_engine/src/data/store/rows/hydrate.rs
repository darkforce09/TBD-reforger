//! Role: hydrate.
//! Position: `doc/store` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::ENTITY_IDS;
use super::Map;
use super::MapPrelim;
use super::MissionDocCore;
use super::PENDING_BRIEFING_MARKERS;
use super::insert_empty_native;
use super::is_known_editor_payload_top_level;
use super::json_str_to_any;
use super::load_row;
use super::load_rows_ordered;
use super::migrate_legacy_id_lists;

impl MissionDocCore {
    /// Hydrate using the supplied domain data.
    pub fn hydrate(&self, payload_json: &str, default_layer_id: &str) {
        let Any::Map(payload) = json_str_to_any(payload_json) else {
            return;
        };

        let payload_extras = self.doc.get_or_insert_map("payloadExtras");
        let entity_order = self.doc.get_or_insert_map("entityOrder");

        let mut txn = self.begin();
        for m in [
            &self.slots,
            &self.squads,
            &self.factions,
            &self.editor_layers,
            &self.loadouts,
            &self.items,
            &self.objectives,
            &self.vehicles,
            &self.entities,
            &self.zones,
            &self.compositions,
            &self.triggers,
            &self.comments,
            &self.markers,
            &payload_extras,
            &entity_order,
        ] {
            m.clear(&mut txn);
        }

        self.meta.remove(&mut txn, "map");
        self.meta.remove(&mut txn, "schemaVersion");
        self.meta.remove(&mut txn, "title");

        self.meta.remove(&mut txn, PENDING_BRIEFING_MARKERS);

        if let Some(env) = payload.get("environment") {
            self.meta.insert(&mut txn, "environment", env.clone());
        }
        if let Some(sv) = payload.get("schemaVersion") {
            self.meta.insert(&mut txn, "schemaVersion", sv.clone());
        }

        if let Some(Any::String(title)) = payload.get("title") {
            let trimmed = title.trim();
            if !trimmed.is_empty() {
                self.meta.insert(&mut txn, "title", trimmed);
            }
        }
        if let Some(map_val) = payload.get("map") {
            self.meta.insert(&mut txn, "map", map_val.clone());
            if let Any::Map(map) = map_val
                && let Some(Any::String(terrain)) = map.get("terrain")
            {
                self.meta.insert(&mut txn, "terrain", terrain.as_ref());
            }
        }

        load_rows_ordered(
            &mut txn,
            &self.objectives,
            payload.get("objectives"),
            &entity_order,
            "objectives",
        );
        load_rows_ordered(
            &mut txn,
            &self.vehicles,
            payload.get("vehicles"),
            &entity_order,
            "vehicles",
        );
        load_rows_ordered(
            &mut txn,
            &self.entities,
            payload.get("entities"),
            &entity_order,
            "entities",
        );

        load_rows_ordered(
            &mut txn,
            &self.zones,
            payload.get("zones"),
            &entity_order,
            "zones",
        );

        load_rows_ordered(
            &mut txn,
            &self.compositions,
            payload.get("compositions"),
            &entity_order,
            "compositions",
        );

        load_rows_ordered(
            &mut txn,
            &self.triggers,
            payload.get("triggers"),
            &entity_order,
            "triggers",
        );

        load_rows_ordered(
            &mut txn,
            &self.comments,
            payload.get("comments"),
            &entity_order,
            "comments",
        );

        load_rows_ordered(
            &mut txn,
            &self.connections,
            payload.get("connections"),
            &entity_order,
            "connections",
        );
        load_rows_ordered(
            &mut txn,
            &self.markers,
            payload.get("markers"),
            &entity_order,
            "markers",
        );
        if let Some(Any::Map(lo)) = payload.get("loadouts") {
            for v in lo.values() {
                load_row(&mut txn, &self.loadouts, v);
            }
        }

        if let Some(Any::Map(editor)) = payload.get("editor") {
            load_rows_ordered(
                &mut txn,
                &self.factions,
                editor.get("factions"),
                &entity_order,
                "factions",
            );
            load_rows_ordered(
                &mut txn,
                &self.squads,
                editor.get("squads"),
                &entity_order,
                "squads",
            );
            load_rows_ordered(
                &mut txn,
                &self.slots,
                editor.get("slots"),
                &entity_order,
                "slots",
            );
            load_rows_ordered(
                &mut txn,
                &self.editor_layers,
                editor.get("editorLayers"),
                &entity_order,
                "editorLayers",
            );
        }

        migrate_legacy_id_lists(&mut txn, &self.squads, &self.editor_layers);

        for (k, v) in payload.iter() {
            if is_known_editor_payload_top_level(k) {
                continue;
            }
            payload_extras.insert(&mut txn, k.as_str(), v.clone());
        }

        if self.editor_layers.len(&txn) == 0 {
            let layer = self.editor_layers.insert(
                &mut txn,
                default_layer_id,
                MapPrelim::from([("id", default_layer_id)]),
            );
            layer.insert(&mut txn, "name", "Default Layer");
            layer.insert(&mut txn, "parentId", Any::Null);
            insert_empty_native(&mut txn, &layer, ENTITY_IDS);
        }
    }
}
