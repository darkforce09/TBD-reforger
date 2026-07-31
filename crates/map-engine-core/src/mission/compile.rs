//! Mission editor compile — Rust port of `compile.ts` (`compileMission` / `assemblePayload`) and
//! `exportSchema.ts` (`toMissionExport`). Turns the doc's by-id JSON (from
//! `MissionDocCore::small_maps_json` and `slots_json`) into the `MissionPayload` superset the backend
//! `/versions` route validates against `mission-editor-payload.schema.json`, plus the camelCase
//! `MissionExport` download envelope.
//!
//! Save Version omits `orbat` (the server re-derives it via `parse_orbat_template`); Export includes
//! it via `derive_orbat_from_editor`. The transforms are pure (`&str`/`Value` in → `Value` out, no
//! live doc), so they unit-test natively and are reused unchanged behind the wasm editor.
//!
//! **T-220 — order.** `serde_json` is built with `preserve_order` (IndexMap), so object key order and
//! `Object.values`-style arrays follow insertion order from the doc getters rather than re-sorting
//! by key. Semantic equality is still what the backend validator checks; authored order is what a
//! hydrate→compile round trip must not silently rewrite.
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

/// Top-level keys [`compile_payload`] itself authors (and `orbat` on the Export path), plus the
/// reserved `payloadExtras` side-channel name (T-432 / T-219 — never park or re-emit that key
/// onto the wire). Everything else is a T-219 passthrough candidate, carried through the doc as
/// `payloadExtras` (see `MissionDocCore::hydrate` / `small_maps_json`). Keep in lockstep with
/// `is_known_editor_payload_top_level` in `doc/store.rs`.
pub const KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS: &[&str] = &[
    "schemaVersion",
    "map",
    "environment",
    // T-505 / T-524 — hydrate loads top-level `title` into meta (T-375 wire emit). Must match
    // `is_known_editor_payload_top_level` in `doc/store.rs` (duplicated there so `doc` does not
    // depend on `mission`).
    "title",
    "loadouts",
    "objectives",
    "vehicles",
    "entities",
    "markers",
    "editor",
    "orbat",
    // T-432 — reserved compile side-channel name; not a wire key and not a passthrough candidate.
    "payloadExtras",
];

#[must_use]
pub fn is_known_editor_payload_top_level(key: &str) -> bool {
    KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS.contains(&key)
}

/// T-220 — emit by-id map values in `entityOrder[order_key]` sequence when present; otherwise
/// fall back to map iteration. Ids missing from the map are skipped; map entries not listed in
/// the order are appended in map-iteration order so nothing is silently dropped.
fn values_of_ordered(small: &Value, by_id_key: &str, order_key: &str) -> Vec<Value> {
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

/// `{ ...obj[key] }` — the by-id map itself as an object (React keeps `loadouts` object-shaped).
/// Missing / non-object → `{}`.
fn object_of(obj: &Value, key: &str) -> Value {
    obj.get(key)
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()))
}

/// Non-blank trimmed `meta.title` — matches the strip commit guard
/// (`eden_chrome`: `!v.trim().is_empty()` then `set_title(v.trim())`). Whitespace-only is not a
/// title (T-375). Absent / non-string → `None`.
fn meta_title_nonblank(meta: &Value) -> Option<&str> {
    meta.get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// Compile the doc's by-id JSON into the `MissionPayload` superset (`compileMission` /
/// `compileMissionWithProgress` + `assemblePayload`). `include_orbat` = the Export path (orbat
/// derived + injected); `false` = the Save path (orbat key entirely absent — the server re-derives).
///
/// `small_maps_json` is [`MissionDocCore::small_maps_json`] (has `meta` + the small by-id maps);
/// `slots_json` is [`MissionDocCore::slots_json`] (`slotsById`). `meta == null` → React defaults
/// (`terrain "everon"`, `environment {}`).
///
/// # Briefing prose (T-214) — the authoring half of a two-slice contract
///
/// `mission.schema.json` `$defs/briefings` keys the in-game briefing screen's
/// situation/mission/execution prose **by faction**, so the document stores it **on the faction
/// row**: `factionsById[<factionId>].briefing = { situation, mission, execution }`. That lands in
/// this payload at `editor.factions[i].briefing`, carried **verbatim** — [`values_of`] clones each
/// faction row whole, so the key needs no handling here and must not acquire any.
///
/// Attaching it to the faction row rather than to a sibling `briefings` map is the load-bearing
/// choice, for two reasons:
///
/// 1. **One key derivation.** The compiled document's `briefings` map must key on the same slug as
///    its `orbat` map and its `slots[].faction` — `flatten.rs` `slug_key(faction.key)`. Prose that
///    travels ON the faction whose `key` produces that slug cannot acquire a second key vocabulary,
///    and the emitter's existing per-faction loop already holds both the row and the slug.
/// 2. **No orphans.** `briefings` is `additionalProperties`-open, so a stale entry naming a deleted
///    faction VALIDATES — it would ship as prose for a side that no longer exists. Storing it on
///    the row makes that unrepresentable: delete the faction, delete its briefing.
///
/// Prose is **deliberately exempt** from `$defs/wireSafeString`'s control-character ban (the reason
/// is recorded at `mission.schema.json` `$defs/wireSafeString`: it never rides a delimited wire —
/// `TBD_BriefingService` ships it as parallel `array<string>` RPC parameters). So a multi-paragraph
/// value with embedded newlines is legitimate authoring and this compile must not sanitise it.
/// `briefing_prose_survives_compile_verbatim_per_faction` and
/// `briefing_prose_round_trips_through_the_document_core` pin exactly that.
#[must_use]
pub fn compile_payload(small_maps_json: &str, slots_json: &str, include_orbat: bool) -> Value {
    let small: Value = serde_json::from_str(small_maps_json).unwrap_or_else(|_| json!({}));
    let slots: Value = serde_json::from_str(slots_json).unwrap_or_else(|_| json!({}));
    let meta = small.get("meta").cloned().unwrap_or(Value::Null);

    // terrain = meta.terrain ?? 'everon'. Bounds default from terrain size when the author did not
    // store any (T-220: authored `map.bounds` and other `map.*` keys must survive round-trip).
    let terrain = meta
        .get("terrain")
        .and_then(Value::as_str)
        .unwrap_or("everon")
        .to_string();
    let b = terrain_bounds(&terrain);
    let default_bounds = json!([b[0] as i64, b[1] as i64, b[2] as i64, b[3] as i64]);

    // T-220 — start from the authored `meta.map` object (hydrate stores the whole `map` payload
    // there), then ensure `terrain` tracks the live meta key. Fill `bounds` only when absent so a
    // stored custom bounds is not silently recomputed away.
    let mut map_obj: Map<String, Value> = meta
        .get("map")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    map_obj.insert("terrain".to_string(), json!(terrain));
    if !map_obj.contains_key("bounds") {
        map_obj.insert("bounds".to_string(), default_bounds);
    }

    // T-220 — preserve authored editor-payload `schemaVersion` (integer; schema has no maximum).
    // Missing → literal 1 (fresh docs / pre-T-220 payloads).
    let schema_version = meta
        .get("schemaVersion")
        .cloned()
        .unwrap_or_else(|| json!(1));

    // environment = { ...(meta.environment ?? {}) }.
    let environment = meta
        .get("environment")
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));

    // editor.slots = Object.values(slotsById) (the full exact-f64 slot dicts), ordered by
    // hydrate's entityOrder.slots when present (T-220 — yrs maps are unordered).
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
            // Verbatim faction rows — this is also the wire for authored per-faction briefing
            // prose (`factions[i].briefing`); see this function's note. `mission-editor-payload
            // .schema.json` leaves `editor.factions` an unconstrained array, so the key validates
            // on Save, and the row stays LOSSLESS for reload (`MissionDocCore::hydrate`
            // `load_row`s every non-`id` field back verbatim).
            "factions": values_of_ordered(&small, "factionsById", "factions"),
            "squads": values_of_ordered(&small, "squadsById", "squads"),
            "slots": slots_vec,
            "editorLayers": values_of_ordered(&small, "editorLayersById", "editorLayers"),
        },
    });

    // T-375 — emit authored `meta.title` onto the wire payload. Save used to drop it (export
    // already carried it via [`compile_export`]), so reload's `apply_row_meta` could only see the
    // stale mission-row title. Blank / whitespace-only is omitted (non-blank guard spirit).
    // T-505 — hydrate loads this top-level key into `meta`; it is listed in
    // `KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS` (T-524 lockstep with `doc/store.rs`) so it is not
    // parked/re-emitted via `payloadExtras`.
    if let Some(title) = meta_title_nonblank(&meta)
        && let Some(obj) = payload.as_object_mut()
    {
        obj.insert("title".to_string(), json!(title));
    }

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

    // T-219 — re-emit unknown top-level keys that hydrate parked in `payloadExtras`. Never
    // overwrite a key this function already authored (schemaVersion / map / editor / …), and never
    // promote the side-channel name itself onto the wire payload (T-432: `payloadExtras` is in
    // `KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS` so the known-key skip below enforces that claim).
    if let Some(extras) = small.get("payloadExtras").and_then(Value::as_object)
        && let Some(obj) = payload.as_object_mut()
    {
        for (k, v) in extras {
            if is_known_editor_payload_top_level(k) || obj.contains_key(k) {
                continue;
            }
            obj.insert(k.clone(), v.clone());
        }
    }

    payload
}

/// Wrap a compiled payload in the camelCase `MissionExport` download envelope (`toMissionExport`).
/// `exported_at` is injected (the core never reads the wall clock — the editor passes
/// `js_sys::Date::new_0().to_iso_string()`; the smoke passes a fixed value for determinism).
/// `mission_id` is the route `:id`, used for `missionId` when `meta.id` is absent.
///
/// **`briefing` here is SINGULAR and it is not the per-faction briefing block.** The two were
/// conflated once and the conflation is worth naming, because both names live one keystroke apart:
///
/// - **`briefing`** (this envelope, a **string**) is the mission ROW's library blurb — the
///   `missions.briefing` DB column, authored by `POST`/`PATCH /missions/:id`, rendered on the
///   website by `mission_overview.rs` ("No briefing provided.") and `approvals.rs` ("The author
///   submitted no briefing."). The authority for this field is the backend's `build_mission_doc`
///   (`apps/website/api/src/handlers/missions.rs`), which sets it from `m.briefing`; this function
///   is the wasm editor's client-side mirror of that envelope.
/// - **`briefings`** (`mission.schema.json` `$defs/briefings`, an **object keyed by faction**) is
///   the in-game briefing screen's situation/mission/execution prose. It does not belong in this
///   envelope at all: it is authored per faction and rides inside `payload.editor.factions[]`
///   (see [`compile_payload`]), so an export download already carries it nested in `payload`.
///
/// Until T-214 this field was the literal `""`. It now reads `meta.briefing`, which is the
/// established channel for row fields the envelope needs — `title`, `terrain` and `environment`
/// all arrive the same way, threaded by `MissionDocCore::apply_row_meta` from the same
/// `GET /missions/:id` row that carries `briefing`. T-418 wired `apply_row_meta` + hydrate
/// `RowMeta` so a non-blank row briefing lands in `meta` and this field is no longer permanently
/// empty. Note the mirror is not exact — `build_mission_doc` omits the key when empty
/// (`skip_serializing_if`) while this always emits it. That divergence predates T-214 and is left
/// alone deliberately rather than changed under an unrelated ticket.
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
    // T-375 — same non-blank trim as [`compile_payload`]; whitespace-only falls through to the
    // envelope default (export always emits the key; Save omits when blank).
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
    // The mission row's library blurb, not the per-faction block — see this function's note.
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

/// The wire shape of `POST /missions/:id/versions`, written down **exactly once**.
///
/// Two doors reach that route — the editor's Save (`mission_commands::save_now`) and the
/// dossier's document upload (`missions.rs`) — and they must not drift into sending different
/// JSON. They call different functions ([`version_body`] and [`version_body_to_writer`]) because
/// they have different memory budgets, so "share one builder" is enforced *here* instead: both
/// are thin wrappers over this one struct, and neither states a key name of its own.
///
/// Borrowed on every field, so serialising it copies nothing — `payload` stays a `&Value` all the
/// way to the writer. Field order is the serialised key order (`serde_json` is built with
/// `preserve_order`, so objects keep insertion order rather than sorting); it matches the `json!`
/// literal this replaced, which is why `version_body_shape` still holds.
#[derive(serde::Serialize)]
struct VersionBody<'a> {
    semver: &'a str,
    editor_notes: &'a str,
    payload: &'a Value,
}

/// The Save Version POST body: `{ semver, editor_notes, payload }` (React `buildVersionBlob`;
/// the FE `notes` arg maps to the wire key `editor_notes`). Backend `CreateVersionInput`.
///
/// Materialises a `Value`, which means it **clones the whole payload tree** under `"payload"`.
/// That is fine for the editor's Save, which compiles its payload locally and owns it anyway;
/// it is not fine for the browser document upload, which already holds a parsed tree — that
/// door uses [`version_body_to_writer`] instead.
#[must_use]
pub fn version_body(semver: &str, editor_notes: &str, payload: &Value) -> Value {
    serde_json::to_value(VersionBody {
        semver,
        editor_notes,
        payload,
    })
    .unwrap_or_else(|_| Value::Null)
}

/// [`version_body`]'s JSON, serialised straight into `w` — **without building the `Value` first**.
///
/// This is the memory-critical door. `version_body` has to clone the payload tree to put it under
/// the `"payload"` key, and on `wasm32` that second tree is the difference between an upload that
/// lands and a dead tab: T-591 measured one parsed tree at **4.7x** the source document in a
/// 32-bit linear heap, and the browser upload is already holding one. Serialising through the
/// borrowed [`VersionBody`] never materialises a second: the document is visited once, in place,
/// and only the output bytes are allocated.
///
/// Pair it with a `Vec<u8>` pre-sized from the source document (so the growth reallocs do not
/// reintroduce a transient copy) and with `client::api_post_raw`, which takes the finished
/// `String` and therefore also drops the request helper's per-attempt body clone.
pub fn version_body_to_writer<W: std::io::Write>(
    w: W,
    semver: &str,
    editor_notes: &str,
    payload: &Value,
) -> serde_json::Result<()> {
    serde_json::to_writer(
        w,
        &VersionBody {
            semver,
            editor_notes,
            payload,
        },
    )
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
            "entitiesById": {},
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
        assert_eq!(p["entities"], json!([]));
        assert_eq!(p["markers"], json!([]));
    }

    /// T-254 — `entitiesById` values land on the payload's top-level `entities` array verbatim
    /// (editor shape with `id`/`alias`/`position`), so flatten can emit schema `entities[]`.
    #[test]
    fn compile_copies_entities_by_id_to_entities_array() {
        let small = json!({
            "meta": Value::Null,
            "factionsById": {},
            "squadsById": {},
            "loadoutsById": {},
            "itemsById": {},
            "objectivesById": {},
            "vehiclesById": {},
            "entitiesById": {
                "e1": {
                    "id": "e1",
                    "alias": "prop:ammo_crate",
                    "resourceName": "{FA}Prefabs/Props/AmmoBox.et",
                    "faction": "blufor",
                    "position": { "x": 10.0, "y": 20.0, "z": 0.0, "rotation": 45.0 }
                }
            },
            "markersById": {},
            "editorLayersById": {}
        })
        .to_string();
        let p = compile_payload(&small, "{}", false);
        let ents = p["entities"].as_array().expect("entities array");
        assert_eq!(ents.len(), 1);
        assert_eq!(ents[0]["alias"], "prop:ammo_crate");
        assert_eq!(ents[0]["id"], "e1");
        assert_eq!(ents[0]["position"]["x"], 10.0);
        assert_eq!(ents[0]["faction"], "blufor");
    }

    #[test]
    fn export_orbat_is_faction_then_squad_then_index_sorted() {
        let p = compile_payload(&small_maps(), &slots(), true);
        // faction array order (fa=BLUFOR, fb=OPFOR) → each squad → slots sorted by index asc.
        // Slots in this fixture have no loadout key → empty strings (I3).
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

    /// I6 — Export injects derived summary when editor slots carry loadout.summary.
    #[test]
    fn compile_export_orbat_loadout() {
        let slots = json!({
            "z1": {
                "id": "z1", "index": 0, "role": "Rifleman", "tag": "",
                "loadout": {
                    "primary": "{AAA}Rifle_M16A2.et",
                    "summary": "M16A2 \u{00b7} ACOG"
                }
            }
        })
        .to_string();
        let small = json!({
            "meta": Value::Null,
            "factionsById": {
                "fa": { "key": "BLUFOR", "squadIds": ["s1"] }
            },
            "squadsById": {
                "s1": { "id": "s1", "callsign": "Alpha", "name": "1st", "slotIds": ["z1"] }
            },
            "loadoutsById": {},
            "itemsById": {},
            "objectivesById": {},
            "vehiclesById": {},
            "entitiesById": {},
            "markersById": {},
            "editorLayersById": {}
        })
        .to_string();
        let p = compile_payload(&small, &slots, true);
        assert_eq!(
            p["orbat"][0]["slots"][0]["loadout"],
            json!("M16A2 \u{00b7} ACOG")
        );
        // Save path still omits top-level orbat.
        let save = compile_payload(&small, &slots, false);
        assert!(save.get("orbat").is_none());
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

    /// **The anti-drift pin for the two doors onto `create_version`.**
    ///
    /// The editor's Save (`mission_commands::save_now`) builds its body with [`version_body`];
    /// the dossier's document upload (`missions.rs`) builds it with [`version_body_to_writer`],
    /// because it cannot afford the payload clone. Two functions is a drift risk, and the risk is
    /// not hypothetical — a wrong key name here is a 400 the author reads as "my document is
    /// broken". So demand the two produce **byte-identical** JSON, not merely equivalent JSON:
    /// key order included, over inputs chosen to catch the ways a hand-written second
    /// implementation would differ.
    ///
    /// Byte equality (rather than `Value` equality) is deliberate: `Value` comparison is
    /// order-insensitive and would pass even if one door emitted its keys in a different order,
    /// which is exactly the class of difference a derived-vs-literal split introduces.
    ///
    /// RED under perturbation: rename a field on `VersionBody`, reorder its fields, or give
    /// either function a `json!`/`to_writer` body of its own that states a key name.
    #[test]
    fn both_doors_onto_create_version_serialise_identical_bytes() {
        let payloads = [
            // The everyday case.
            json!({ "schemaVersion": 1, "editor": { "slots": [] } }),
            // Empty, null and scalar payloads — a `json!` wrapper and a derived struct can
            // disagree about `Option`/unit handling at exactly these edges.
            json!({}),
            Value::Null,
            json!(0),
            json!([]),
            // Key order that is NOT alphabetical: with `preserve_order` these must come out in
            // insertion order, and a builder that rebuilt the map would re-sort them.
            json!({ "zulu": 1, "alpha": 2, "mike": 3 }),
            // Escaping and non-ASCII, in both the payload and its keys.
            json!({ "quote\"key": "line\nbreak\ttab", "unicode": "Ärland — Ω 🎖", "solidus": "a/b" }),
            // Nesting deep enough that a shallow copy would show up.
            json!({ "a": { "b": { "c": [1, 2, { "d": true, "e": Value::Null }] } } }),
            // Floats and big integers — number re-encoding is a classic silent difference.
            json!({ "f": 1.5, "neg": -0.000_25, "big": 9_007_199_254_740_993i64 }),
        ];
        // Notes/semver strings that themselves need escaping, so the wrapper keys are exercised
        // too and not just the payload.
        let metas = [
            ("0.1.0", "note"),
            ("", ""),
            (
                "1.2.3-rc.1+build",
                "Uploaded from \"my mission\".json\nwith a newline",
            ),
            ("9.9.9", "Ω — em dash and 🎖"),
        ];

        for payload in &payloads {
            for (semver, notes) in metas {
                let via_value = serde_json::to_string(&version_body(semver, notes, payload))
                    .expect("version_body's Value must serialise");
                let mut via_writer: Vec<u8> = Vec::new();
                version_body_to_writer(&mut via_writer, semver, notes, payload)
                    .expect("version_body_to_writer must not fail writing to a Vec");
                let via_writer =
                    String::from_utf8(via_writer).expect("serde_json emits valid UTF-8");

                assert_eq!(
                    via_value, via_writer,
                    "the two doors onto create_version must send the same bytes — they have \
                     drifted for semver={semver:?} notes={notes:?} payload={payload}"
                );
            }
        }

        // And pin the literal wire text once, so "identical to each other" cannot degrade into
        // "identically wrong". These are the three keys `CreateVersionInput` deserialises.
        let mut buf: Vec<u8> = Vec::new();
        version_body_to_writer(&mut buf, "0.1.0", "note", &json!({ "schemaVersion": 1 }))
            .expect("write");
        assert_eq!(
            String::from_utf8(buf).expect("utf8"),
            r#"{"semver":"0.1.0","editor_notes":"note","payload":{"schemaVersion":1}}"#
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
        // T-214 — absent `meta.briefing` still reads as the pre-T-214 literal.
        assert_eq!(doc["briefing"], json!(""));
    }

    // ── T-214 briefing prose ────────────────────────────────────────────────────────────────────
    //
    // Every briefing fixture below is MULTI-LINE on purpose. Prose is the one authored string
    // `mission.schema.json` excludes from `$defs/wireSafeString`'s control-character ban, and the
    // recorded reason is specifically the newline: briefing text does not ride a delimited wire —
    // `TBD_BriefingService` ships it as parallel `array<string>` RPC parameters. A value below
    // would therefore be a SCHEMA VIOLATION in a callsign and is legitimate authoring here, so
    // testing briefings with single-line strings would test the wrong string.

    /// Two paragraph breaks — the shape an author gets from pressing Enter twice.
    const SITUATION: &str =
        "Soviet airborne hold the Levie crossing.\n\nTwo BMPs were seen at the eastern abutment.";
    /// A hard line break inside one paragraph.
    const MISSION: &str = "Seize Levie Bridge.\nHold it to the time limit.";
    /// A paragraph plus a hand-written list — three newlines, mixed kinds.
    const EXECUTION: &str = "Alpha advances from the western treeline.\n\n\
                             - MG support from Hill 214\n- Sappers follow on foot";

    /// The authored block, as the faction row carries it.
    fn authored_briefing() -> Value {
        json!({ "situation": SITUATION, "mission": MISSION, "execution": EXECUTION })
    }

    /// T-214 — the authoring half of the briefing contract. Per-faction prose stored on the
    /// faction row reaches `editor.factions[i].briefing` byte-for-byte, newlines included.
    ///
    /// This is the exact shape `flatten.rs` (T-202) reads on the emitter side, so this assertion
    /// and that one are the two halves of one contract: if these keys move, that emitter emits
    /// empty briefings and nothing else fails.
    #[test]
    fn briefing_prose_survives_compile_verbatim_per_faction() {
        let small = json!({
            "meta": Value::Null,
            "factionsById": {
                "fa": {
                    "id": "fa", "key": "BLUFOR", "name": "US Army", "squadIds": ["s1"],
                    "briefing": authored_briefing()
                },
                "fb": { "id": "fb", "key": "OPFOR", "name": "Soviet VDV", "squadIds": [] }
            },
            "squadsById": {
                "s1": { "id": "s1", "callsign": "Alpha", "name": "1st", "slotIds": [] }
            },
            "loadoutsById": {},
            "itemsById": {},
            "objectivesById": {},
            "vehiclesById": {},
            "entitiesById": {},
            "markersById": {},
            "editorLayersById": {}
        })
        .to_string();

        let p = compile_payload(&small, "{}", false);
        let factions = p["editor"]["factions"].as_array().expect("factions array");
        assert_eq!(factions.len(), 2);

        // Key-sorted by faction id (fa, fb). The authored block arrives whole — no key added,
        // none dropped, nothing rewritten.
        assert_eq!(factions[0]["briefing"], authored_briefing());

        // The newline is the point, so assert on the characters and not only on equality: a
        // future "helpful" sanitiser that mapped `\n` to a space would still satisfy a loose
        // is-a-string check, and would fail HERE naming the reason.
        let situation = factions[0]["briefing"]["situation"]
            .as_str()
            .expect("situation is a string");
        assert_eq!(situation, SITUATION);
        assert!(
            situation.contains("\n\n"),
            "paragraph break must survive verbatim: {situation:?}"
        );
        let execution = factions[0]["briefing"]["execution"]
            .as_str()
            .expect("execution is a string");
        assert_eq!(execution.matches('\n').count(), 3, "{execution:?}");

        // Faction identity travels WITH the prose. `key` is what `flatten.rs` `slug_key`s into the
        // compiled document's `briefings` map key — the same slug it uses for `orbat` and
        // `slots[].faction` — which is the whole reason the prose hangs on this row.
        assert_eq!(factions[0]["key"], json!("BLUFOR"));

        // A faction that authored nothing carries no key at all: the emitter must be able to tell
        // "no briefing" from "an empty briefing" and not ship a blank block for the second side.
        assert!(factions[1].get("briefing").is_none());

        // Export path carries it too — orbat derivation must not disturb the editor graph.
        let ex = compile_payload(&small, "{}", true);
        assert_eq!(
            ex["editor"]["factions"][0]["briefing"]["mission"],
            json!(MISSION)
        );
    }

    /// T-214 — the same contract through the REAL document core. A shape that only works in a
    /// hand-written `small_maps_json` fixture is not a shape the editor can author, so this loads a
    /// compiled payload the way a reload does (`hydrate` → `load_row`), reads it back through the
    /// two getters `compile_payload` actually consumes, and recompiles. That closes the loop
    /// author → store → compile → author without a hand-built intermediate.
    ///
    /// It also proves the newline survives the **CRDT**, not just `serde_json`: the value goes into
    /// `yrs` as an opaque `Any::Map` on the faction row and comes back out through
    /// `MapRef::to_json`.
    ///
    /// `#[cfg(feature = "doc")]` — this test is why the suite must run `--features doc,mission`;
    /// `--features mission` alone compiles the whole `doc` module out and silently skips it.
    #[cfg(feature = "doc")]
    #[test]
    fn briefing_prose_round_trips_through_the_document_core() {
        use crate::doc::MissionDocCore;

        let payload = json!({
            "schemaVersion": 1,
            "map": { "terrain": "everon" },
            "editor": {
                "factions": [
                    {
                        "id": "fa", "key": "BLUFOR", "name": "US Army", "squadIds": ["s1"],
                        "briefing": authored_briefing()
                    },
                    { "id": "fb", "key": "OPFOR", "name": "Soviet VDV", "squadIds": [] }
                ],
                "squads": [
                    { "id": "s1", "factionId": "fa", "callsign": "Alpha", "name": "1st",
                      "slotIds": ["z1"] }
                ],
                "slots": [
                    { "id": "z1", "squadId": "s1", "index": 0, "role": "SL",
                      "position": { "x": 4839.2, "y": 6620.8, "z": 0.0, "rotation": 270.0 } }
                ],
                "editorLayers": []
            }
        })
        .to_string();

        let doc = MissionDocCore::new();
        doc.hydrate(&payload, "layer-1");

        // Out of yrs through the getter the compile reads.
        let small = doc.small_maps_json();
        let parsed: Value = serde_json::from_str(&small).expect("small_maps_json is JSON");
        assert_eq!(
            parsed["factionsById"]["fa"]["briefing"],
            authored_briefing()
        );

        let recompiled = compile_payload(&small, &doc.slots_json(), false);
        assert_eq!(
            recompiled["editor"]["factions"][0]["briefing"],
            authored_briefing()
        );

        // The newline came back out of the CRDT unchanged.
        let situation = recompiled["editor"]["factions"][0]["briefing"]["situation"]
            .as_str()
            .expect("situation is a string");
        assert_eq!(situation, SITUATION);
        assert!(situation.contains("\n\n"), "{situation:?}");

        // Sanity that this really was a full round trip and not an empty doc agreeing with itself.
        assert_eq!(recompiled["editor"]["slots"].as_array().unwrap().len(), 1);
        assert_eq!(recompiled["map"]["terrain"], json!("everon"));
    }

    /// T-219 — `compile_payload` merges `payloadExtras` onto the wire payload, but never lets an
    /// extra overwrite a key this function already authored (schemaVersion stays the doc/default,
    /// not the extras value — T-220 preserves authored versions via `meta.schemaVersion`).
    #[test]
    fn payload_extras_merge_does_not_overwrite_known_keys() {
        let small = json!({
            "meta": { "terrain": "everon" },
            "factionsById": {},
            "squadsById": {},
            "loadoutsById": {},
            "itemsById": {},
            "objectivesById": {},
            "vehiclesById": {},
            "entitiesById": {},
            "markersById": {},
            "editorLayersById": {},
            "payloadExtras": {
                "schemaVersion": 99,
                "serverMigrationToken": "keep-me",
                "map": { "terrain": "arland" }
            }
        })
        .to_string();

        let p = compile_payload(&small, "{}", false);
        assert_eq!(
            p["schemaVersion"],
            json!(1),
            "known key must win over extras"
        );
        assert_eq!(p["map"]["terrain"], json!("everon"));
        assert_eq!(p["serverMigrationToken"], json!("keep-me"));
        assert!(p.get("payloadExtras").is_none());
    }

    /// T-259 — `settings` is NOT a first-class hydrate map (that would need `doc/store.rs`, outside
    /// this slice's owns). Hydrate parks it in `payloadExtras` (T-219); compile must re-emit it
    /// onto the wire so flatten can pass it through to the mod document. Pinning the three schema
    /// fields by name so a rename here fails before a golden ever sees it.
    #[test]
    fn settings_in_payload_extras_reach_the_wire_payload() {
        let small = json!({
            "meta": { "terrain": "everon" },
            "factionsById": {},
            "squadsById": {},
            "loadoutsById": {},
            "itemsById": {},
            "objectivesById": {},
            "vehiclesById": {},
            "entitiesById": {},
            "markersById": {},
            "editorLayersById": {},
            "payloadExtras": {
                "settings": {
                    "respawn": "wave",
                    "spectatorPolicy": "free",
                    "nightVision": false
                }
            }
        })
        .to_string();

        let p = compile_payload(&small, "{}", false);
        let s = p
            .get("settings")
            .expect("settings must leave payloadExtras onto the wire");
        assert_eq!(s["respawn"], "wave");
        assert_eq!(s["spectatorPolicy"], "free");
        assert_eq!(s["nightVision"], false);
        assert!(p.get("payloadExtras").is_none());
    }

    /// T-432 — Class R: the side-channel name `payloadExtras` is reserved. If a stale / hostile
    /// `small_maps` parks a nested key literally named `payloadExtras`, compile must **not**
    /// promote that name onto the wire (T-219 already claimed this; make it true). Unrelated
    /// parked keys still re-emit. Nested reserved contents are dropped (reserved-key collision),
    /// not renamed.
    #[test]
    fn payload_extras_key_name_never_promoted_onto_wire() {
        let small = json!({
            "meta": { "terrain": "everon" },
            "factionsById": {},
            "squadsById": {},
            "loadoutsById": {},
            "itemsById": {},
            "objectivesById": {},
            "vehiclesById": {},
            "entitiesById": {},
            "markersById": {},
            "editorLayersById": {},
            "payloadExtras": {
                "payloadExtras": { "nested": true },
                "serverMigrationToken": "keep-me"
            }
        })
        .to_string();

        let p = compile_payload(&small, "{}", false);
        assert!(
            p.get("payloadExtras").is_none(),
            "side-channel name must never become a wire key; got {:?}",
            p.get("payloadExtras")
        );
        assert_eq!(
            p["serverMigrationToken"],
            json!("keep-me"),
            "unrelated parked keys must still re-emit"
        );
    }

    /// T-220 — authored `schemaVersion` on meta survives compile (not forced back to literal 1).
    #[test]
    fn authored_schema_version_on_meta_is_emitted() {
        let small = json!({
            "meta": { "terrain": "everon", "schemaVersion": 2 },
            "factionsById": {},
            "squadsById": {},
            "loadoutsById": {},
            "itemsById": {},
            "objectivesById": {},
            "vehiclesById": {},
            "entitiesById": {},
            "markersById": {},
            "editorLayersById": {}
        })
        .to_string();
        let p = compile_payload(&small, "{}", false);
        assert_eq!(p["schemaVersion"], json!(2));
    }

    /// T-220 — authored map keys beyond terrain/bounds survive; bounds are not recomputed away.
    #[test]
    fn authored_map_keys_and_bounds_survive_compile() {
        let small = json!({
            "meta": {
                "terrain": "everon",
                "map": {
                    "terrain": "everon",
                    "bounds": [10, 20, 30, 40],
                    "center": [6400.5, 6400.25],
                    "label": "ops-sector"
                }
            },
            "factionsById": {},
            "squadsById": {},
            "loadoutsById": {},
            "itemsById": {},
            "objectivesById": {},
            "vehiclesById": {},
            "entitiesById": {},
            "markersById": {},
            "editorLayersById": {}
        })
        .to_string();
        let p = compile_payload(&small, "{}", false);
        assert_eq!(p["map"]["bounds"], json!([10, 20, 30, 40]));
        assert_eq!(p["map"]["center"], json!([6400.5, 6400.25]));
        assert_eq!(p["map"]["label"], json!("ops-sector"));
        assert_eq!(p["map"]["terrain"], json!("everon"));
    }

    /// T-214 — the envelope's `briefing` is the mission ROW's library blurb (a **string**), and the
    /// per-faction block is a different thing that reaches an export nested inside `payload`.
    /// Asserted together on one document, because conflating the two is the misreading this ticket
    /// was filed on top of and an assertion is the only durable place to record which is which.
    #[test]
    fn export_envelope_briefing_is_the_row_blurb_not_the_faction_block() {
        let small = json!({
            "meta": {
                "title": "Bridgehead at Levie",
                "briefing": "A short library blurb for the mission card."
            },
            "factionsById": {
                "fa": {
                    "id": "fa", "key": "BLUFOR", "squadIds": [],
                    "briefing": authored_briefing()
                }
            },
            "squadsById": {},
            "loadoutsById": {},
            "itemsById": {},
            "objectivesById": {},
            "vehiclesById": {},
            "entitiesById": {},
            "markersById": {},
            "editorLayersById": {}
        })
        .to_string();

        let payload = compile_payload(&small, "{}", true);
        let doc = compile_export(
            &payload,
            &small,
            "smoke",
            "0.2.0",
            "1970-01-01T00:00:00.000Z",
        );

        // Singular, a string, read from `meta` — no longer the `""` literal. Mirrors the backend
        // authority `build_mission_doc`, which sets this from the `missions.briefing` column.
        assert!(doc["briefing"].is_string());
        assert_eq!(
            doc["briefing"],
            json!("A short library blurb for the mission card.")
        );

        // The per-faction prose is reachable only nested in the payload — and it survived the wrap.
        assert_eq!(
            doc["payload"]["editor"]["factions"][0]["briefing"]["situation"],
            json!(SITUATION)
        );
        // The envelope did NOT grow a per-faction key, and the singular field did not become an
        // object. Both would be a contract break against `build_mission_doc`.
        assert!(doc.get("briefings").is_none());
        assert!(!doc["briefing"].is_object());
    }

    // ── T-375 mission title on the compiled payload ─────────────────────────────────────────────

    /// T-375 Class R — `compile_payload` must include `meta.title` when the doc has a real one.
    /// Pre-fix: Save omitted the key entirely while Export carried it, so reload could only
    /// re-apply the stale mission-row title.
    #[test]
    fn compile_payload_includes_title_when_doc_has_one() {
        let small = json!({
            "meta": { "title": "Bridgehead at Levie", "terrain": "everon" },
            "factionsById": {},
            "squadsById": {},
            "loadoutsById": {},
            "itemsById": {},
            "objectivesById": {},
            "vehiclesById": {},
            "entitiesById": {},
            "markersById": {},
            "editorLayersById": {}
        })
        .to_string();

        let save = compile_payload(&small, "{}", false);
        assert_eq!(save["title"], json!("Bridgehead at Levie"));
        assert!(save.get("orbat").is_none());

        let export_payload = compile_payload(&small, "{}", true);
        assert_eq!(export_payload["title"], json!("Bridgehead at Levie"));

        // Export envelope stays consistent with the same non-blank trim helper.
        let doc = compile_export(
            &export_payload,
            &small,
            "smoke",
            "0.1.0",
            "1970-01-01T00:00:00.000Z",
        );
        assert_eq!(doc["title"], json!("Bridgehead at Levie"));
    }

    /// T-375 — blank / whitespace handling matches the strip's non-blank guard spirit
    /// (`!v.trim().is_empty()`). Payload omits the key; export falls back to `Untitled Mission`.
    #[test]
    fn compile_payload_omits_blank_or_whitespace_title() {
        for raw in ["", "   ", "\t\n"] {
            let small = json!({ "meta": { "title": raw, "terrain": "everon" } }).to_string();
            let p = compile_payload(&small, "{}", false);
            assert!(
                p.get("title").is_none(),
                "whitespace-only title {raw:?} must not appear on the Save payload; got {:?}",
                p.get("title")
            );
            let doc = compile_export(&p, &small, "smoke", "0.1.0", "1970-01-01T00:00:00.000Z");
            assert_eq!(doc["title"], json!("Untitled Mission"));
        }

        // Leading/trailing space is trimmed to the authored core (strip stores `v.trim()`).
        let padded = json!({ "meta": { "title": "  Op Red Dawn  " } }).to_string();
        let p = compile_payload(&padded, "{}", false);
        assert_eq!(p["title"], json!("Op Red Dawn"));
    }

    // ── T-524 known-keys / hydrate-title lockstep ───────────────────────────────────────────────

    /// T-524 Class R — after T-505, `title` is a first-class hydrate key in `doc/store.rs`.
    /// It MUST stay in `KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS` here or the two lists drift and
    /// compile would re-emit a parked extras title while store refuses to park it.
    #[test]
    fn title_is_known_editor_payload_top_level_key() {
        assert!(
            is_known_editor_payload_top_level("title"),
            "T-524 — `title` missing from KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS; lockstep with store.rs broken"
        );
        assert!(
            KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS.contains(&"title"),
            "T-524 — const list must include `title` (helper alone is not enough)"
        );
    }

    /// T-524 — `title` parked in `payloadExtras` must NOT be promoted onto the wire (known-key
    /// skip). Authored `meta.title` still emits via the T-375 path.
    #[test]
    fn title_in_payload_extras_is_not_re_emitted() {
        let small = json!({
            "meta": { "terrain": "everon" },
            "payloadExtras": {
                "title": "Should Not Leak From Extras",
                "serverMigrationToken": "keep-me"
            }
        })
        .to_string();
        let p = compile_payload(&small, "{}", false);
        assert!(
            p.get("title").is_none(),
            "known-key title in extras must not reach the wire; got {:?}",
            p.get("title")
        );
        assert_eq!(p["serverMigrationToken"], json!("keep-me"));

        let with_meta = json!({
            "meta": { "title": "Authored", "terrain": "everon" },
            "payloadExtras": { "title": "Extras Must Lose" }
        })
        .to_string();
        let p2 = compile_payload(&with_meta, "{}", false);
        assert_eq!(p2["title"], json!("Authored"));
    }
}
