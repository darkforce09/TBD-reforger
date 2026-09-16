//! Role: briefings.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Arc;
use super::Map;
use super::MissionDocCore;
use super::Out;
use super::any_to_f64;
use super::briefing_markers;
use super::marker_any;
use super::marker_row_id;
use super::pending_briefing_markers_map;
use super::read_any_map;
use super::remove_pending_briefing_marker;
use super::upsert_pending_briefing_marker;
use yrs::Transact;

impl MissionDocCore {
    /// Doc: `{id, x, z, icon, label}`. Wire (`$defs/marker`): `{x, z, icon, label}` with `additionalProperties: false`. **The doc shape and the wire shape deliberately differ by exactly this one key** — the first place in this document that they do, which is why it is recorded here, on the writer, rather than left to be inferred by whoever reads next.
    pub fn set_faction_briefing_marker(
        &self,
        faction_id: &str,
        marker_id: &str,
        x: f64,
        z: f64,
        icon: &str,
        label: &str,
    ) {
        let mut txn = self.begin();
        let row = marker_any(marker_id, x, z, icon, label);

        let Some(Out::YMap(f)) = self.factions.get(&txn, faction_id) else {
            upsert_pending_briefing_marker(&mut txn, &self.meta, faction_id, marker_id, row);
            return;
        };
        let mut briefing = read_any_map(&txn, &f, "briefing");
        let mut markers = briefing_markers(&briefing);
        match markers
            .iter()
            .position(|m| marker_row_id(m) == Some(marker_id))
        {
            Some(i) => markers[i] = row,
            None => markers.push(row),
        }
        briefing.insert("markers".to_string(), Any::Array(markers.into()));
        f.insert(&mut txn, "briefing", Any::Map(Arc::new(briefing)));
    }
}

impl MissionDocCore {
    /// Delete one marker from a faction's briefing by its doc-internal id, leaving the prose fields and the sibling markers untouched.
    pub fn remove_faction_briefing_marker(&self, faction_id: &str, marker_id: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(f)) = self.factions.get(&txn, faction_id) {
            let mut briefing = read_any_map(&txn, &f, "briefing");
            let mut markers = briefing_markers(&briefing);
            let before = markers.len();
            markers.retain(|m| marker_row_id(m) != Some(marker_id));
            if markers.len() != before {
                briefing.insert("markers".to_string(), Any::Array(markers.into()));
                f.insert(&mut txn, "briefing", Any::Map(Arc::new(briefing)));
                return;
            }
        }

        remove_pending_briefing_marker(&mut txn, &self.meta, faction_id, marker_id);
    }
}

impl MissionDocCore {
    /// Rows with no doc-internal `id` are SKIPPED. Such a row is still compiled (see [`marker_row_id`]) — it is simply not addressable, so offering it in a list whose every verb takes an id would produce controls that silently do nothing.
    #[must_use]
    pub fn briefing_marker_rows_json(&self) -> String {
        let txn = self.doc.transact();
        let mut rows: Vec<serde_json::Value> = Vec::new();
        for (faction_id, out) in self.factions.iter(&txn) {
            let Out::YMap(f) = out else { continue };
            let briefing = read_any_map(&txn, &f, "briefing");
            for row in briefing_markers(&briefing) {
                let Some(id) = marker_row_id(&row) else {
                    continue;
                };
                let Any::Map(fields) = &row else { continue };
                let text = |k: &str| match fields.get(k) {
                    Some(Any::String(s)) => s.to_string(),
                    _ => String::new(),
                };
                rows.push(serde_json::json!({
                    "factionId": faction_id,
                    "id": id,
                    "x": fields.get("x").map_or(0.0, any_to_f64),
                    "z": fields.get("z").map_or(0.0, any_to_f64),
                    "icon": text("icon"),
                    "label": text("label"),
                }));
            }
        }

        for (faction_id, markers) in pending_briefing_markers_map(&txn, &self.meta) {
            for row in markers {
                let Some(id) = marker_row_id(&row) else {
                    continue;
                };
                let Any::Map(fields) = &row else { continue };
                let text = |k: &str| match fields.get(k) {
                    Some(Any::String(s)) => s.to_string(),
                    _ => String::new(),
                };
                rows.push(serde_json::json!({
                    "factionId": faction_id,
                    "id": id,
                    "x": fields.get("x").map_or(0.0, any_to_f64),
                    "z": fields.get("z").map_or(0.0, any_to_f64),
                    "icon": text("icon"),
                    "label": text("label"),
                }));
            }
        }

        rows.sort_by(|a, b| a["factionId"].as_str().cmp(&b["factionId"].as_str()));
        serde_json::Value::Array(rows).to_string()
    }
}

impl MissionDocCore {
    /// Writes `factionsById[faction_id].briefing.{situation,mission,execution}`, the three prose keys `mission.schema.json` `$defs/briefing` declares beside `markers`. `additionalProperties: false` there makes those four the only legal shape, so this is the whole prose surface. No-op on an unknown `faction_id`, exactly as the marker mutators: orders need a side to be given to.
    pub fn set_faction_briefing(
        &self,
        faction_id: &str,
        situation: &str,
        mission: &str,
        execution: &str,
    ) {
        let mut txn = self.begin();
        let Some(Out::YMap(f)) = self.factions.get(&txn, faction_id) else {
            return;
        };
        let before = read_any_map(&txn, &f, "briefing");
        let mut briefing = before.clone();
        for (key, text) in [
            ("situation", situation),
            ("mission", mission),
            ("execution", execution),
        ] {
            if text.is_empty() {
                briefing.remove(key);
            } else {
                briefing.insert(key.to_string(), Any::String(text.into()));
            }
        }
        if briefing == before {
            return;
        }
        f.insert(&mut txn, "briefing", Any::Map(Arc::new(briefing)));
    }
}
