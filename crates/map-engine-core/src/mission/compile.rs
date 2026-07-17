//! Mission editor compile — Rust port of `compile.ts` (`compileMission` / `assemblePayload`) and
//! `exportSchema.ts` (`toMissionExport`). Turns the doc's by-id JSON (from
//! `MissionDocCore::small_maps_json` and `slots_json`) into the `MissionPayload` superset the backend
//! `/versions` route validates against `mission-editor-payload.schema.json`, plus the camelCase
//! `MissionExport` download envelope.
//!
//! Save Version omits `orbat` (the server re-derives it via `parse_orbat_template`); Export includes
//! it via `derive_orbat_from_editor`. The transforms are pure (`&str`/`Value` in → `Value` out, no
//! live doc), so they unit-test natively and are reused unchanged behind the wasm editor. Output uses
//! serde_json's default `Map` (BTreeMap → sorted keys), so a given doc compiles byte-deterministically;
//! byte-order vs the React blob is **not** a parity target (the backend validator is order-agnostic and
//! the T-159 Class R contract is semantic).
//!
//! @contract mission-editor-payload.schema.json (payload); exportSchema.ts MissionExport (envelope)

use serde_json::{Map, Value, json};

use crate::mission::orbat::derive_orbat_from_editor;

/// Terrain world bounds `[minX, minY, maxX, maxY]` — mirror of `coords/terrains.ts` `TERRAINS`
/// (`getTerrain(id).bounds`). Unknown / `custom` terrain falls back to Everon `12800²`, matching
/// React's `getTerrain(terrainId)` default.
#[must_use]
pub fn terrain_bounds(terrain: &str) -> [f64; 4] {
    match terrain {
        "arland" => [0.0, 0.0, 4096.0, 4096.0],
        // everon + custom + anything unknown → 12800²
        _ => [0.0, 0.0, 12_800.0, 12_800.0],
    }
}

/// `Object.values(obj[key])` — the by-id map's values as an array. Missing / non-object → `[]`.
/// serde_json `Map` iteration is key-sorted, so the array order is deterministic (id-sorted).
fn values_of(obj: &Value, key: &str) -> Vec<Value> {
    obj.get(key)
        .and_then(Value::as_object)
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default()
}

/// `{ ...obj[key] }` — the by-id map itself as an object (React keeps `loadouts` object-shaped).
/// Missing / non-object → `{}`.
fn object_of(obj: &Value, key: &str) -> Value {
    obj.get(key)
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()))
}

/// Compile the doc's by-id JSON into the `MissionPayload` superset (`compileMission` /
/// `compileMissionWithProgress` + `assemblePayload`). `include_orbat` = the Export path (orbat
/// derived + injected); `false` = the Save path (orbat key entirely absent — the server re-derives).
///
/// `small_maps_json` is [`MissionDocCore::small_maps_json`] (has `meta` + the small by-id maps);
/// `slots_json` is [`MissionDocCore::slots_json`] (`slotsById`). `meta == null` → React defaults
/// (`terrain "everon"`, `environment {}`).
#[must_use]
pub fn compile_payload(small_maps_json: &str, slots_json: &str, include_orbat: bool) -> Value {
    let small: Value = serde_json::from_str(small_maps_json).unwrap_or_else(|_| json!({}));
    let slots: Value = serde_json::from_str(slots_json).unwrap_or_else(|_| json!({}));
    let meta = small.get("meta").cloned().unwrap_or(Value::Null);

    // terrain = meta.terrain ?? 'everon'; map.bounds = [0, 0, width, height] (integer, like React).
    let terrain = meta
        .get("terrain")
        .and_then(Value::as_str)
        .unwrap_or("everon")
        .to_string();
    let b = terrain_bounds(&terrain);
    let bounds = json!([b[0] as i64, b[1] as i64, b[2] as i64, b[3] as i64]);

    // environment = { ...(meta.environment ?? {}) }.
    let environment = meta
        .get("environment")
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));

    // editor.slots = Object.values(slotsById) (the full exact-f64 slot dicts).
    let slots_vec: Vec<Value> = slots
        .as_object()
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default();

    let mut payload = json!({
        "schemaVersion": 1,
        "map": { "terrain": terrain, "bounds": bounds },
        "environment": environment,
        "loadouts": object_of(&small, "loadoutsById"),
        "objectives": values_of(&small, "objectivesById"),
        "vehicles": values_of(&small, "vehiclesById"),
        "markers": values_of(&small, "markersById"),
        "editor": {
            "factions": values_of(&small, "factionsById"),
            "squads": values_of(&small, "squadsById"),
            "slots": slots_vec,
            "editorLayers": values_of(&small, "editorLayersById"),
        },
    });

    // Export path: derive orbat from the just-built editor graph and inject it (spread
    // `...(orbat ? { orbat } : {})` → key present only here; absent on Save).
    if include_orbat {
        let bytes = serde_json::to_vec(&payload).unwrap_or_default();
        let orbat = derive_orbat_from_editor(&bytes);
        let orbat_val = serde_json::to_value(orbat).unwrap_or_else(|_| Value::Array(vec![]));
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("orbat".to_string(), orbat_val);
        }
    }
    payload
}

/// Wrap a compiled payload in the camelCase `MissionExport` download envelope (`toMissionExport`).
/// `exported_at` is injected (the core never reads the wall clock — the editor passes
/// `js_sys::Date::new_0().to_iso_string()`; the smoke passes a fixed value for determinism).
/// `mission_id` is the route `:id`, used for `missionId` when `meta.id` is absent.
#[must_use]
pub fn compile_export(
    payload: &Value,
    small_maps_json: &str,
    mission_id: &str,
    version: &str,
    exported_at: &str,
) -> Value {
    let small: Value = serde_json::from_str(small_maps_json).unwrap_or_else(|_| json!({}));
    let meta = small.get("meta").cloned().unwrap_or(Value::Null);

    let mission_id_field = meta
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or(mission_id)
        .to_string();
    let title = meta
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Untitled Mission")
        .to_string();
    let terrain = meta
        .get("terrain")
        .and_then(Value::as_str)
        .unwrap_or("everon")
        .to_string();
    let env = meta.get("environment").cloned().unwrap_or(Value::Null);
    let weather = env
        .get("weather")
        .and_then(Value::as_str)
        .unwrap_or("clear")
        .to_string();
    let time_of_day = env
        .get("time")
        .and_then(Value::as_str)
        .unwrap_or("06:00")
        .to_string();

    json!({
        "exportFormatVersion": 1,
        "missionId": mission_id_field,
        "title": title,
        "terrain": terrain,
        "gameMode": "",
        "weather": weather,
        "timeOfDay": time_of_day,
        "maxPlayers": 0,
        "version": version,
        "briefing": "",
        "armory": [],
        "payload": payload,
        "exportedAt": exported_at,
    })
}

/// The Save Version POST body: `{ semver, editor_notes, payload }` (React `buildVersionBlob`;
/// the FE `notes` arg maps to the wire key `editor_notes`). Backend `CreateVersionInput`.
#[must_use]
pub fn version_body(semver: &str, editor_notes: &str, payload: &Value) -> Value {
    json!({
        "semver": semver,
        "editor_notes": editor_notes,
        "payload": payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A two-faction editor graph with squads and index-shuffled slots — enough to prove the
    /// orbat traversal order the seed doc (slots-only, empty factions) cannot exercise.
    fn small_maps() -> String {
        json!({
            "meta": Value::Null,
            "factionsById": {
                "fa": { "key": "BLUFOR", "squadIds": ["s1"] },
                "fb": { "key": "OPFOR",  "squadIds": ["s2"] }
            },
            "squadsById": {
                "s1": { "id": "s1", "callsign": "Alpha", "name": "1st", "slotIds": ["z2", "z1"] },
                "s2": { "id": "s2", "callsign": "Bravo", "name": "2nd", "slotIds": ["z3"] }
            },
            "loadoutsById": {},
            "itemsById": {},
            "objectivesById": {},
            "vehiclesById": {},
            "markersById": {},
            "editorLayersById": {}
        })
        .to_string()
    }

    fn slots() -> String {
        json!({
            "z1": { "id": "z1", "index": 5, "role": "SL",       "tag": "CMD" },
            "z2": { "id": "z2", "index": 1, "role": "Rifleman", "tag": "" },
            "z3": { "id": "z3", "index": 0, "role": "MED",      "tag": "MED" }
        })
        .to_string()
    }

    #[test]
    fn save_payload_omits_orbat_and_has_editor_shape() {
        let p = compile_payload(&small_maps(), &slots(), false);
        assert!(p.get("orbat").is_none(), "Save payload must omit orbat");
        assert_eq!(p["schemaVersion"], json!(1));
        assert_eq!(p["map"]["terrain"], json!("everon"));
        assert_eq!(p["map"]["bounds"], json!([0, 0, 12800, 12800]));
        assert_eq!(p["editor"]["slots"].as_array().unwrap().len(), 3);
        assert_eq!(p["editor"]["factions"].as_array().unwrap().len(), 2);
        assert_eq!(p["editor"]["squads"].as_array().unwrap().len(), 2);
        assert_eq!(p["editor"]["editorLayers"], json!([]));
        assert!(p["loadouts"].is_object());
        assert!(p["environment"].is_object());
        assert_eq!(p["objectives"], json!([]));
        assert_eq!(p["vehicles"], json!([]));
        assert_eq!(p["markers"], json!([]));
    }

    #[test]
    fn export_orbat_is_faction_then_squad_then_index_sorted() {
        let p = compile_payload(&small_maps(), &slots(), true);
        // faction array order (fa=BLUFOR, fb=OPFOR) → each squad → slots sorted by index asc.
        assert_eq!(
            p["orbat"],
            json!([
                {
                    "faction": "BLUFOR", "callsign": "Alpha", "squad": "1st",
                    "slots": [
                        { "role": "Rifleman", "loadout": "", "tag": "" },
                        { "role": "SL",       "loadout": "", "tag": "CMD" }
                    ]
                },
                {
                    "faction": "OPFOR", "callsign": "Bravo", "squad": "2nd",
                    "slots": [ { "role": "MED", "loadout": "", "tag": "MED" } ]
                }
            ])
        );
    }

    #[test]
    fn null_meta_defaults_to_everon_and_empty_environment() {
        let p = compile_payload(r#"{"meta":null}"#, "{}", false);
        assert_eq!(p["map"]["terrain"], json!("everon"));
        assert_eq!(p["map"]["bounds"], json!([0, 0, 12800, 12800]));
        assert_eq!(p["environment"], json!({}));
        assert_eq!(p["editor"]["slots"], json!([]));
    }

    #[test]
    fn arland_terrain_yields_4096_bounds() {
        let small = json!({ "meta": { "terrain": "arland" } }).to_string();
        let p = compile_payload(&small, "{}", false);
        assert_eq!(p["map"]["terrain"], json!("arland"));
        assert_eq!(p["map"]["bounds"], json!([0, 0, 4096, 4096]));
    }

    #[test]
    fn version_body_shape() {
        let payload = json!({ "schemaVersion": 1 });
        let body = version_body("0.1.0", "note", &payload);
        assert_eq!(
            body,
            json!({ "semver": "0.1.0", "editor_notes": "note", "payload": { "schemaVersion": 1 } })
        );
    }

    #[test]
    fn export_envelope_defaults_and_wraps_payload() {
        let payload = compile_payload(r#"{"meta":null}"#, "{}", true);
        let doc = compile_export(
            &payload,
            r#"{"meta":null}"#,
            "smoke",
            "0.1.0",
            "1970-01-01T00:00:00.000Z",
        );
        assert_eq!(doc["exportFormatVersion"], json!(1));
        assert_eq!(doc["missionId"], json!("smoke"));
        assert_eq!(doc["title"], json!("Untitled Mission"));
        assert_eq!(doc["terrain"], json!("everon"));
        assert_eq!(doc["weather"], json!("clear"));
        assert_eq!(doc["timeOfDay"], json!("06:00"));
        assert_eq!(doc["gameMode"], json!(""));
        assert_eq!(doc["maxPlayers"], json!(0));
        assert_eq!(doc["exportedAt"], json!("1970-01-01T00:00:00.000Z"));
        assert_eq!(doc["payload"]["orbat"], json!([]));
    }
}
