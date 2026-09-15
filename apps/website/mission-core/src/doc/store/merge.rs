//! Role: merge.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Arc;
use super::ENTITY_IDS;
use super::HashMap;
use super::Map;
use super::MapPrelim;
use super::MergeOpts;
use super::MergeReport;
use super::MissionDocCore;
use super::RemintMap;
use super::SLOT_IDS;
use super::append_id;
use super::copy_row_fields_except;
use super::insert_empty_native;
use super::json_num;
use super::json_position;
use super::json_position_map;
use super::json_str;
use super::merge_shape_rows;
use super::offset_shape_any;
use super::replace_native;
use super::squad_or_faction_is_merged;
use super::value_to_any;

impl MissionDocCore {
    /// Merge mission payload using the supplied domain data.
    #[must_use]
    pub fn merge_mission_payload(
        &self,
        payload: &serde_json::Value,
        opts: MergeOpts,
    ) -> MergeReport {
        let mut report = MergeReport::default();
        let Some(obj) = payload.as_object() else {
            report
                .skipped
                .push(("payload".into(), String::new(), "not a JSON object".into()));
            return report;
        };
        let editor = obj.get("editor").and_then(serde_json::Value::as_object);

        let (dx, dy) = opts.offset.unwrap_or((0.0, 0.0));

        let entity_order = self.doc.get_or_insert_map("entityOrder");

        let (resident_factions, resident_squads_by_side, resident_ids) =
            self.merge_resident_index(&entity_order);

        let mut remint = RemintMap::with_reserved(resident_ids);

        let incoming_factions: Vec<&serde_json::Map<String, serde_json::Value>> = editor
            .and_then(|e| e.get("factions"))
            .and_then(serde_json::Value::as_array)
            .map(|a| a.iter().filter_map(serde_json::Value::as_object).collect())
            .unwrap_or_default();
        let mut incoming_fac_key: HashMap<String, String> = HashMap::new();
        for f in &incoming_factions {
            let Some(id) = json_str(f, "id") else {
                continue;
            };
            let key = json_str(f, "key");
            if let Some(k) = &key {
                incoming_fac_key.insert(id.clone(), k.clone());
            }
            let name = json_str(f, "name");
            if let (Some(k), Some(n)) = (&key, &name)
                && let Some(resident) = resident_factions.get(&(n.clone(), k.clone()))
            {
                remint.map_to_existing(&id, resident);
            }
        }

        let incoming_squads: Vec<&serde_json::Map<String, serde_json::Value>> = editor
            .and_then(|e| e.get("squads"))
            .and_then(serde_json::Value::as_array)
            .map(|a| a.iter().filter_map(serde_json::Value::as_object).collect())
            .unwrap_or_default();

        let mut squad_merged_into: HashMap<String, String> = HashMap::new();
        for s in &incoming_squads {
            let Some(id) = json_str(s, "id") else {
                continue;
            };
            let name = json_str(s, "name");

            let side = json_str(s, "factionId").and_then(|fid| {
                incoming_fac_key.get(&fid).cloned().or_else(|| {
                    remint.get(&fid).and_then(|rid| {
                        resident_factions
                            .iter()
                            .find_map(|((_n, k), v)| (v == &rid).then(|| k.clone()))
                    })
                })
            });
            if let (Some(n), Some(s_key)) = (name, side)
                && let Some(resident_sq) = resident_squads_by_side.get(&(n, s_key))
            {
                remint.map_to_existing(&id, resident_sq);
                squad_merged_into.insert(id, resident_sq.clone());
            }
        }

        for f in &incoming_factions {
            if let Some(id) = json_str(f, "id") {
                remint.ensure_fresh(&id);
            }
        }
        for s in &incoming_squads {
            if let Some(id) = json_str(s, "id") {
                remint.ensure_fresh(&id);
            }
        }
        for arr in [
            editor.and_then(|e| e.get("slots")),
            editor.and_then(|e| e.get("editorLayers")),
            obj.get("vehicles"),
            obj.get("entities"),
            obj.get("zones"),
            obj.get("compositions"),
            obj.get("triggers"),
            obj.get("markers"),
        ] {
            if let Some(rows) = arr.and_then(serde_json::Value::as_array) {
                for row in rows {
                    if let Some(id) = row.as_object().and_then(|m| json_str(m, "id")) {
                        remint.ensure_fresh(&id);
                    }
                }
            }
        }

        let mut txn = self.begin();

        for f in &incoming_factions {
            let Some(old_id) = json_str(f, "id") else {
                report
                    .skipped
                    .push(("faction".into(), String::new(), "missing id".into()));
                continue;
            };
            if squad_or_faction_is_merged(&remint, &old_id) {
                report.factions_merged += 1;
                continue;
            }
            let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
            let fac = self.factions.insert(
                &mut txn,
                new_id.as_str(),
                MapPrelim::from([("id", new_id.as_str())]),
            );
            copy_row_fields_except(&mut txn, &fac, f, &["id", "squadIds"]);

            fac.insert(&mut txn, "squadIds", Any::Array(Vec::new().into()));
            report.factions_created += 1;
        }

        for s in &incoming_squads {
            let Some(old_id) = json_str(s, "id") else {
                report
                    .skipped
                    .push(("squad".into(), String::new(), "missing id".into()));
                continue;
            };
            if squad_merged_into.contains_key(&old_id) {
                report.squads_merged += 1;
                continue;
            }
            let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
            let sq = self.squads.insert(
                &mut txn,
                new_id.as_str(),
                MapPrelim::from([("id", new_id.as_str())]),
            );

            copy_row_fields_except(
                &mut txn,
                &sq,
                s,
                &["id", "slotIds", "vehicleIds", "factionId", "leaderSlotId"],
            );
            insert_empty_native(&mut txn, &sq, SLOT_IDS);
            sq.insert(&mut txn, "vehicleIds", Any::Array(Vec::new().into()));

            if let Some(fid) = json_str(s, "factionId").and_then(|f| remint.get(&f)) {
                sq.insert(&mut txn, "factionId", fid.as_str());
                append_id(&mut txn, &self.factions, &fid, "squadIds", &new_id);
            }

            if let Some(lid) = json_str(s, "leaderSlotId").and_then(|l| remint.get(&l)) {
                sq.insert(&mut txn, "leaderSlotId", lid.as_str());
            }
            report.squads_created += 1;
        }

        let incoming_slots = editor
            .and_then(|e| e.get("slots"))
            .and_then(serde_json::Value::as_array);
        if let Some(rows) = incoming_slots {
            for row in rows {
                let Some(m) = row.as_object() else {
                    report
                        .skipped
                        .push(("slot".into(), String::new(), "not an object".into()));
                    continue;
                };
                let Some(old_id) = json_str(m, "id") else {
                    report
                        .skipped
                        .push(("slot".into(), String::new(), "missing id".into()));
                    continue;
                };
                let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
                let slot = self.slots.insert(
                    &mut txn,
                    new_id.as_str(),
                    MapPrelim::from([("id", new_id.as_str())]),
                );

                let squad_id = json_str(m, "squadId").and_then(|sid| remint.get(&sid));
                copy_row_fields_except(&mut txn, &slot, m, &["id", "squadId", "position"]);
                if let Some(sid) = &squad_id {
                    slot.insert(&mut txn, "squadId", sid.as_str());
                }

                let (px, py, pz, prot) = json_position(m);
                let mut pos = json_position_map(m);
                pos.insert("x".to_string(), Any::Number(px + dx));
                pos.insert("y".to_string(), Any::Number(py + dy));
                pos.insert("z".to_string(), Any::Number(pz));
                pos.insert("rotation".to_string(), Any::Number(prot));
                slot.insert(&mut txn, "position", Any::Map(Arc::new(pos)));
                if let Some(sid) = &squad_id {
                    append_id(&mut txn, &self.squads, sid, "slotIds", &new_id);
                }
                report.slots_added += 1;
            }
        }

        if let Some(rows) = editor
            .and_then(|e| e.get("editorLayers"))
            .and_then(serde_json::Value::as_array)
        {
            for row in rows {
                let Some(m) = row.as_object() else { continue };
                let Some(old_id) = json_str(m, "id") else {
                    continue;
                };
                let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
                let layer = self.editor_layers.insert(
                    &mut txn,
                    new_id.as_str(),
                    MapPrelim::from([("id", new_id.as_str())]),
                );
                copy_row_fields_except(&mut txn, &layer, m, &["id", "parentId", "entityIds"]);
                match json_str(m, "parentId").and_then(|p| remint.get(&p)) {
                    Some(pid) => layer.insert(&mut txn, "parentId", pid.as_str()),
                    None => layer.insert(&mut txn, "parentId", Any::Null),
                };
                let entity_ids: Vec<String> = m
                    .get("entityIds")
                    .and_then(serde_json::Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(serde_json::Value::as_str)
                            .filter_map(|s| remint.get(s))
                            .collect()
                    })
                    .unwrap_or_default();
                replace_native(&mut txn, &layer, ENTITY_IDS, &entity_ids);
            }
        }

        if let Some(rows) = obj.get("vehicles").and_then(serde_json::Value::as_array) {
            for row in rows {
                let Some(m) = row.as_object() else {
                    report
                        .skipped
                        .push(("vehicle".into(), String::new(), "not an object".into()));
                    continue;
                };
                let Some(old_id) = json_str(m, "id") else {
                    report
                        .skipped
                        .push(("vehicle".into(), String::new(), "missing id".into()));
                    continue;
                };
                let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
                let v = self.vehicles.insert(
                    &mut txn,
                    new_id.as_str(),
                    MapPrelim::from([("id", new_id.as_str())]),
                );
                copy_row_fields_except(
                    &mut txn,
                    &v,
                    m,
                    &["id", "squadId", "factionId", "crew", "position"],
                );
                if let Some(sid) = json_str(m, "squadId").and_then(|s| remint.get(&s)) {
                    v.insert(&mut txn, "squadId", sid.as_str());
                    append_id(&mut txn, &self.squads, &sid, "vehicleIds", &new_id);
                }
                if let Some(fid) = json_str(m, "factionId").and_then(|f| remint.get(&f)) {
                    v.insert(&mut txn, "factionId", fid.as_str());
                }
                if m.contains_key("position") {
                    let (px, py, pz, prot) = json_position(m);
                    let mut pos = json_position_map(m);
                    pos.insert("x".to_string(), Any::Number(px + dx));
                    pos.insert("y".to_string(), Any::Number(py + dy));
                    pos.insert("z".to_string(), Any::Number(pz));
                    pos.insert("rotation".to_string(), Any::Number(prot));
                    v.insert(&mut txn, "position", Any::Map(Arc::new(pos)));
                }

                if let Some(crew) = m.get("crew").and_then(serde_json::Value::as_object) {
                    let mut seats: HashMap<String, Any> = HashMap::new();
                    for (seat, occ) in crew {
                        if let Some(sid) = occ.as_str().and_then(|s| remint.get(s)) {
                            seats.insert(seat.clone(), Any::String(sid.into()));
                        }
                    }
                    if !seats.is_empty() {
                        v.insert(&mut txn, "crew", Any::Map(Arc::new(seats)));
                    }
                }
                report.vehicles_added += 1;
            }
        }

        if let Some(rows) = obj.get("entities").and_then(serde_json::Value::as_array) {
            for row in rows {
                let Some(m) = row.as_object() else {
                    report
                        .skipped
                        .push(("entity".into(), String::new(), "not an object".into()));
                    continue;
                };
                let Some(old_id) = json_str(m, "id") else {
                    report
                        .skipped
                        .push(("entity".into(), String::new(), "missing id".into()));
                    continue;
                };
                let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
                let e = self.entities.insert(
                    &mut txn,
                    new_id.as_str(),
                    MapPrelim::from([("id", new_id.as_str())]),
                );
                copy_row_fields_except(&mut txn, &e, m, &["id", "position"]);
                if m.contains_key("position") {
                    let (px, py, pz, prot) = json_position(m);
                    let mut pos = json_position_map(m);
                    pos.insert("x".to_string(), Any::Number(px + dx));
                    pos.insert("y".to_string(), Any::Number(py + dy));
                    pos.insert("z".to_string(), Any::Number(pz));
                    pos.insert("rotation".to_string(), Any::Number(prot));
                    e.insert(&mut txn, "position", Any::Map(Arc::new(pos)));
                }
                report.entities_added += 1;
            }
        }

        report.zones_added += merge_shape_rows(
            &mut txn,
            &self.zones,
            obj.get("zones"),
            &remint,
            dx,
            dy,
            "zone",
            &mut report.skipped,
        );

        if let Some(rows) = obj.get("triggers").and_then(serde_json::Value::as_array) {
            for row in rows {
                let Some(m) = row.as_object() else {
                    report
                        .skipped
                        .push(("trigger".into(), String::new(), "not an object".into()));
                    continue;
                };
                let Some(old_id) = json_str(m, "id") else {
                    report
                        .skipped
                        .push(("trigger".into(), String::new(), "missing id".into()));
                    continue;
                };
                let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
                let t = self.triggers.insert(
                    &mut txn,
                    new_id.as_str(),
                    MapPrelim::from([("id", new_id.as_str())]),
                );
                copy_row_fields_except(&mut txn, &t, m, &["id", "ownerId", "shape"]);
                if let Some(oid) = json_str(m, "ownerId").and_then(|o| remint.get(&o)) {
                    t.insert(&mut txn, "ownerId", oid.as_str());
                }
                if let Some(shape) = m.get("shape") {
                    t.insert(&mut txn, "shape", offset_shape_any(shape, dx, dy));
                }
                report.triggers_added += 1;
            }
        }

        if let Some(rows) = obj
            .get("compositions")
            .and_then(serde_json::Value::as_array)
        {
            for row in rows {
                let Some(m) = row.as_object() else {
                    report.skipped.push((
                        "composition".into(),
                        String::new(),
                        "not an object".into(),
                    ));
                    continue;
                };
                let Some(old_id) = json_str(m, "id") else {
                    report
                        .skipped
                        .push(("composition".into(), String::new(), "missing id".into()));
                    continue;
                };
                let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
                let c = self.compositions.insert(
                    &mut txn,
                    new_id.as_str(),
                    MapPrelim::from([("id", new_id.as_str())]),
                );

                for (k, v) in m {
                    if k != "id" {
                        c.insert(&mut txn, k.as_str(), value_to_any(v));
                    }
                }
                report.compositions_added += 1;
            }
        }

        if let Some(rows) = obj.get("markers").and_then(serde_json::Value::as_array) {
            for row in rows {
                let Some(m) = row.as_object() else {
                    report
                        .skipped
                        .push(("marker".into(), String::new(), "not an object".into()));
                    continue;
                };
                let Some(old_id) = json_str(m, "id") else {
                    report
                        .skipped
                        .push(("marker".into(), String::new(), "missing id".into()));
                    continue;
                };
                let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
                let mk = self.markers.insert(
                    &mut txn,
                    new_id.as_str(),
                    MapPrelim::from([("id", new_id.as_str())]),
                );
                for (k, v) in m {
                    match k.as_str() {
                        "id" => {}
                        "x" => {
                            mk.insert(&mut txn, "x", Any::Number(json_num(m, "x") + dx));
                        }
                        "z" => {
                            mk.insert(&mut txn, "z", Any::Number(json_num(m, "z") + dy));
                        }
                        _ => {
                            mk.insert(&mut txn, k.as_str(), value_to_any(v));
                        }
                    }
                }
                report.markers_added += 1;
            }
        }

        report
    }
}
