//! Role: payload.
//! Position: `mission/compiler/payload` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{Map, Value, derive_orbat_from_editor, json, terrain_bounds};

/// Canonical known editor payload top level keys value.
pub const KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS: &[&str] = &[
    "schemaVersion",
    "map",
    "environment",
    "title",
    "loadouts",
    "objectives",
    "vehicles",
    "entities",
    "markers",
    "editor",
    "orbat",
    "payloadExtras",
];

/// Is known editor payload top level using the supplied domain data.
#[must_use]
pub fn is_known_editor_payload_top_level(key: &str) -> bool {
    KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS.contains(&key)
}

/// Values of ordered using the supplied domain data.
pub(super) fn values_of_ordered(small: &Value, by_id_key: &str, order_key: &str) -> Vec<Value> {
    let Some(map) = small.get(by_id_key).and_then(Value::as_object) else {
        return Vec::new();
    };
    let Some(order) = small
        .get("entityOrder")
        .and_then(|o| o.get(order_key))
        .and_then(Value::as_array)
    else {
        return map.values().cloned().collect();
    };
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(map.len());
    for id_val in order {
        let Some(id) = id_val.as_str() else {
            continue;
        };
        if let Some(row) = map.get(id) {
            out.push(row.clone());
            seen.insert(id.to_string());
        }
    }
    for (id, row) in map {
        if !seen.contains(id) {
            out.push(row.clone());
        }
    }
    out
}

/// Object of using the supplied domain data.
pub(super) fn object_of(obj: &Value, key: &str) -> Value {
    obj.get(key)
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()))
}

/// Meta title nonblank using the supplied domain data.
pub(super) fn meta_title_nonblank(meta: &Value) -> Option<&str> {
    meta.get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// Compile the doc's by-id JSON into the `MissionPayload` superset (`compileMission` / `compileMissionWithProgress` + `assemblePayload`). `include_orbat` = the Export path (orbat derived + injected); `false` = the Save path (orbat key entirely absent — the server re-derives).
#[must_use]
pub fn compile_payload(small_maps_json: &str, slots_json: &str, include_orbat: bool) -> Value {
    let small: Value = serde_json::from_str(small_maps_json).unwrap_or_else(|_| json!({}));
    let slots: Value = serde_json::from_str(slots_json).unwrap_or_else(|_| json!({}));
    let meta = small.get("meta").cloned().unwrap_or(Value::Null);

    let terrain = meta
        .get("terrain")
        .and_then(Value::as_str)
        .unwrap_or("everon")
        .to_string();
    let b = terrain_bounds(&terrain);
    let default_bounds = json!([b[0] as i64, b[1] as i64, b[2] as i64, b[3] as i64]);

    let mut map_obj: Map<String, Value> = meta
        .get("map")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    map_obj.insert("terrain".to_string(), json!(terrain));
    if !map_obj.contains_key("bounds") {
        map_obj.insert("bounds".to_string(), default_bounds);
    }

    let schema_version = meta
        .get("schemaVersion")
        .cloned()
        .unwrap_or_else(|| json!(1));

    let environment = meta
        .get("environment")
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));

    let slots_vec: Vec<Value> = {
        let order = small
            .get("entityOrder")
            .and_then(|o| o.get("slots"))
            .and_then(Value::as_array);
        match (slots.as_object(), order) {
            (Some(map), Some(ord)) => {
                let mut seen = std::collections::HashSet::new();
                let mut out = Vec::with_capacity(map.len());
                for id_val in ord {
                    let Some(id) = id_val.as_str() else {
                        continue;
                    };
                    if let Some(row) = map.get(id) {
                        out.push(row.clone());
                        seen.insert(id.to_string());
                    }
                }
                for (id, row) in map {
                    if !seen.contains(id) {
                        out.push(row.clone());
                    }
                }
                out
            }
            (Some(map), None) => map.values().cloned().collect(),
            _ => Vec::new(),
        }
    };

    let mut payload = json!({
        "schemaVersion": schema_version,
        "map": Value::Object(map_obj),
        "environment": environment,
        "loadouts": object_of(&small, "loadoutsById"),
        "objectives": values_of_ordered(&small, "objectivesById", "objectives"),
        "vehicles": values_of_ordered(&small, "vehiclesById", "vehicles"),
        "entities": values_of_ordered(&small, "entitiesById", "entities"),
        "markers": values_of_ordered(&small, "markersById", "markers"),
        "editor": {

            "factions": values_of_ordered(&small, "factionsById", "factions"),
            "squads": values_of_ordered(&small, "squadsById", "squads"),
            "slots": slots_vec,
            "editorLayers": values_of_ordered(&small, "editorLayersById", "editorLayers"),
        },
    });

    if let Some(title) = meta_title_nonblank(&meta)
        && let Some(obj) = payload.as_object_mut()
    {
        obj.insert("title".to_string(), json!(title));
    }

    if include_orbat {
        let bytes = serde_json::to_vec(&payload).unwrap_or_default();
        let orbat = derive_orbat_from_editor(&bytes);
        let orbat_val = serde_json::to_value(orbat).unwrap_or_else(|_| Value::Array(vec![]));
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("orbat".to_string(), orbat_val);
        }
    }

    if let Some(obj) = payload.as_object_mut() {
        crate::data::scenario::extensions::copy_authored_blocks(&environment, obj);
    }

    if let Some(extras) = small.get("payloadExtras").and_then(Value::as_object)
        && let Some(obj) = payload.as_object_mut()
    {
        for (k, v) in extras {
            if is_known_editor_payload_top_level(k)
                || crate::data::scenario::extensions::is_authored_block(k)
                || obj.contains_key(k)
            {
                continue;
            }
            obj.insert(k.clone(), v.clone());
        }
    }

    payload
}

/// **`briefing` here is SINGULAR and it is not the per-faction briefing block.** The two were conflated once and the conflation is worth naming, because both names live one keystroke apart:.
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

    let title = meta_title_nonblank(&meta)
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

    let briefing = meta
        .get("briefing")
        .and_then(Value::as_str)
        .unwrap_or_default()
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
        "briefing": briefing,
        "armory": [],
        "payload": payload,
        "exportedAt": exported_at,
    })
}
