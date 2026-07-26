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

/// Top-level keys [`compile_payload`] itself authors (and `orbat` on the Export path).
/// Everything else is a T-219 passthrough candidate, carried through the doc as
/// `payloadExtras` (see `MissionDocCore::hydrate` / `small_maps_json`). Keep in lockstep with
/// `is_known_editor_payload_top_level` in `doc/store.rs`.
pub const KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS: &[&str] = &[
    "schemaVersion",
    "map",
    "environment",
    "loadouts",
    "objectives",
    "vehicles",
    "entities",
    "markers",
    "editor",
    "orbat",
];

#[must_use]
pub fn is_known_editor_payload_top_level(key: &str) -> bool {
    KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS.contains(&key)
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
        "entities": values_of(&small, "entitiesById"),
        "markers": values_of(&small, "markersById"),
        "editor": {
            // Verbatim faction rows — this is also the wire for authored per-faction briefing
            // prose (`factions[i].briefing`); see this function's note. `mission-editor-payload
            // .schema.json` leaves `editor.factions` an unconstrained array, so the key validates
            // on Save, and the row stays LOSSLESS for reload (`MissionDocCore::hydrate`
            // `load_row`s every non-`id` field back verbatim).
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

    // T-219 — re-emit unknown top-level keys that hydrate parked in `payloadExtras`. Never
    // overwrite a key this function already authored (schemaVersion / map / editor / …), and never
    // promote the side-channel name itself onto the wire payload.
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
/// `GET /missions/:id` row that carries `briefing`. `apply_row_meta` does **not** thread it yet, so
/// the key is absent and this still resolves to `""`: today's output is byte-identical to the
/// hardcode, and the field becomes correct the moment the doc side lands. Note the mirror is not
/// exact — `build_mission_doc` omits the key when empty (`skip_serializing_if`) while this always
/// emits it. That divergence predates T-214 and is left alone deliberately rather than changed
/// under an unrelated ticket.
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
    /// extra overwrite a key this function already authored (schemaVersion stays the literal 1).
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
}
