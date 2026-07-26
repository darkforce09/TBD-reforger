//! Backend adapter for the shared mod-document flatten (T-145 Phase 2b). The compile logic lives
//! in `map_engine_core::mission::flatten`; this builds the core `MissionMeta` from the backend
//! `Mission` model **plus the environment authored into the saved version payload** (T-192), and
//! re-exports the output types so `crate::services::…` callers are unchanged.
//!
//! Locked coordinate mapping (in core): editor `position.x → x`, `position.y → z`,
//! `position.z → y` (optional, 1.2), `position.rotation → headingDeg`.
//!
//! @contract mission.schema.json#/

use serde::Deserialize;

use crate::models::{Mission, WeatherType};
use map_engine_core::mission::flatten::{self, MissionMeta};

pub use map_engine_core::mission::flatten::{
    mission_terrain_key, CompileError, ModMissionDocument, ModSlot,
};

/// Build the compiled mod mission document from a mission row + its version payload. Thin wrapper
/// over the shared [`map_engine_core::mission::flatten::flatten_to_mod_document`].
///
/// **T-192 — time/weather come from the payload first, the row second.** The Mission Settings
/// dialog and the top-strip scrubber author `meta.environment.{time,weather}` into the editor
/// document, and the save compiler carries them out to the payload's top-level `environment`.
/// Building `MissionMeta` from `missions.time_of_day` / `missions.weather` alone meant the game
/// server was handed the values the mission was *created* with while the author was looking at the
/// ones they had set — an edit that never reached the wire. The row stays as the fallback, for
/// versions saved before the editor authored an environment and for any field the payload leaves
/// blank or malformed.
///
/// This is one half of the fix. The other half is the editor's `PATCH /missions/{id}` mirror
/// (`eden_chrome::RowMirror`), because `mission_hydrate::apply_row_meta` re-applies the row over the
/// document on every load — without it the authored value would still be reverted locally, and this
/// preference would only paper over a row the editor had gone out of sync with. Neither half ships
/// alone.
pub fn flatten_to_mod_document(
    m: &Mission,
    payload: &[u8],
) -> Result<ModMissionDocument, CompileError> {
    let authored = authored_environment(payload);
    let meta = MissionMeta {
        id: m.id.to_string(),
        title: m.title.clone(),
        author: m.author_id.clone(),
        terrain: m.terrain.as_str().to_string(),
        custom_terrain_name: m.custom_terrain_name.clone(),
        max_players: m.max_players,
        time_of_day: authored
            .time_of_day
            .unwrap_or_else(|| m.time_of_day.clone()),
        weather_preset: authored
            .weather_preset
            .unwrap_or_else(|| m.weather.as_str().to_string()),
    };
    flatten::flatten_to_mod_document(&meta, payload)
}

/// What the saved payload authored, as far as it is usable (T-192). `None` on a field means "the
/// payload does not carry a usable one" → the caller falls back to the mission row.
#[derive(Debug, Default, PartialEq, Eq)]
struct AuthoredEnvironment {
    time_of_day: Option<String>,
    weather_preset: Option<String>,
}

/// The one key of the save payload this adapter reads. Everything else deserializes through serde's
/// ignored-any path, so a 140 MB slot payload (T-060 measured one at 141,574,630 bytes) costs one
/// extra scan and **no** extra allocation — `flatten` still does the one real parse.
#[derive(Deserialize)]
struct PayloadEnvelope {
    #[serde(default)]
    environment: serde_json::Value,
}

/// Read `payload.environment.{time,weather}`. Anything unparseable — a legacy payload with no
/// `environment`, a non-object one, a field of the wrong type — yields `None` and defers to the row.
fn authored_environment(payload: &[u8]) -> AuthoredEnvironment {
    let Ok(envelope) = serde_json::from_slice::<PayloadEnvelope>(payload) else {
        return AuthoredEnvironment::default();
    };
    let env = envelope.environment;
    AuthoredEnvironment {
        time_of_day: env
            .get("time")
            .and_then(serde_json::Value::as_str)
            .and_then(clock_hhmm),
        weather_preset: env
            .get("weather")
            .and_then(serde_json::Value::as_str)
            .and_then(weather_preset),
    }
}

/// `HH:MM` / `HH:MM:SS` → canonical `HH:MM`; anything else → `None`.
///
/// The `:SS` rung is not hypothetical: `missions.time_of_day` is a Postgres `time`, selected as
/// `time_of_day::text`, so the hydrate puts `06:00:00` into the document and it comes straight back
/// out here on the next save.
///
/// Validating at all is the load-bearing part. `flatten` splices this value into
/// `environment.dateTime` (`<anchor>T<hh:mm>:00Z`), which `mission.schema.json` types
/// `format: date-time` — so an unchecked string out of the document would turn one bad edit into a
/// 500 at `GET /missions/:id/compiled`, in front of a game server rather than the author.
fn clock_hhmm(s: &str) -> Option<String> {
    let mut parts = s.split(':');
    let h: u32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    if let Some(sec) = parts.next() {
        let sec: u32 = sec.parse().ok()?;
        if sec > 59 {
            return None;
        }
    }
    if parts.next().is_some() || h > 23 || m > 59 {
        return None;
    }
    Some(format!("{h:02}:{m:02}"))
}

/// The document's `weather` string, accepted only when it names a real `weather_type`.
///
/// Same domain `PATCH /missions/{id}` accepts for the row (`handlers::missions::valid_weather`), so
/// the document can never push the compiled mission somewhere the row is unable to follow — which
/// matters precisely because the editor mirror keeps the two in step. `""` is *not* accepted here
/// (the PATCH reads it as "clear"): an absent value must fall through to the row, not overwrite it.
fn weather_preset(s: &str) -> Option<String> {
    let w = match s {
        "clear" => WeatherType::Clear,
        "overcast" => WeatherType::Overcast,
        "heavy_rain" => WeatherType::HeavyRain,
        "dense_fog" => WeatherType::DenseFog,
        _ => return None,
    };
    Some(w.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::validate_mission_document;
    use crate::models::{GameMode, MissionStatus, TerrainType, WeatherType};
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    // The exact fixture from missions_compiled_integration_test.go: two factions,
    // callsigned squads, a duplicate role (TL x2), one slot carrying real elevation.
    const FIXTURE: &str = r#"{
      "schemaVersion": 1,
      "map": {"terrain": "everon", "bounds": [0, 0, 12800, 12800]},
      "editor": {
        "factions": [
          {"id": "f1", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]},
          {"id": "f2", "key": "OPFOR", "name": "Soviet VDV", "squadIds": ["sq2"]}
        ],
        "squads": [
          {"id": "sq1", "factionId": "f1", "callsign": "Alpha", "name": "Alpha 1-1", "slotIds": ["s1", "s2", "s3"]},
          {"id": "sq2", "factionId": "f2", "name": "Grom", "slotIds": ["s4"]}
        ],
        "slots": [
          {"id": "s1", "squadId": "sq1", "index": 0, "role": "SL", "assetId": "{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et", "position": {"x": 4839.2, "y": 6620.8, "z": 0, "rotation": 270},
           "loadout": {"version": 2,
             "wear": {"headCover": "res://helmet", "jacket": "res://bdu_blouse", "vest": "res://chest_rig", "armoredVest": "res://pasgt"},
             "weapons": [{"slotIndex": 0, "slotType": "primary", "weapon": "res://m16", "optic": "res://acog", "magazine": "res://stanag", "attachments": []}],
             "cargo": [{"container": "vest", "item": "res://stanag", "qty": 4}]}},
          {"id": "s2", "squadId": "sq1", "index": 1, "role": "TL", "position": {"x": 4836.9, "y": 6626.5, "z": 142.5, "rotation": 450}},
          {"id": "s3", "squadId": "sq1", "index": 2, "role": "TL", "position": {"x": 4831.2, "y": 6628.8, "z": 0, "rotation": 0}},
          {"id": "s4", "squadId": "sq2", "index": 0, "role": "RFL", "assetId": "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et", "position": {"x": 6010, "y": 7211.5, "z": 0, "rotation": 90},
           "loadout": {"version": 2, "cargo": [{"container": "backpack", "item": "res://ak_mag", "qty": 40}]}}
        ],
        "editorLayers": []
      }
    }"#;

    fn fixture_mission() -> Mission {
        Mission {
            id: Uuid::new_v4(),
            title: "Compiled Fixture".into(),
            author_id: "maker".into(),
            terrain: TerrainType::Everon,
            custom_terrain_name: String::new(),
            game_mode: GameMode::PveCoop,
            weather: WeatherType::Clear,
            time_of_day: "05:30".into(),
            max_players: 64,
            status: MissionStatus::Draft,
            thumbnail_url: String::new(),
            briefing: String::new(),
            current_version_id: None,
            rejection_reason: String::new(),
            reviewed_by: None,
            reviewed_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn flatten_matches_locked_contract() {
        let m = fixture_mission();
        let doc = flatten_to_mod_document(&m, FIXTURE.as_bytes()).expect("compiles");

        // One slot carries y → schemaVersion bumps to 1.2.
        assert_eq!(doc.schema_version, "1.2");

        // Deterministic slot ids (faction:callsign:role:occurrence).
        let ids: Vec<&str> = doc.slots.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "blufor:Alpha:SL:0",
                "blufor:Alpha:TL:0",
                "blufor:Alpha:TL:1",
                "opfor:Grom:RFL:0"
            ]
        );

        // Locked mapping: x→x, y→z, z→y (optional), rotation→headingDeg (mod 360).
        let s0 = &doc.slots[0];
        assert!((s0.x - 4839.2).abs() < 1e-9 && (s0.z - 6620.8).abs() < 1e-9);
        assert!(s0.y.is_none() && (s0.heading_deg - 270.0).abs() < 1e-9);
        assert_eq!(doc.slots[1].y, Some(142.5));
        assert!((doc.slots[1].heading_deg - 90.0).abs() < 1e-9); // 450 % 360

        // Kit aliases: mapped assetId → kit; unmapped → faction default.
        assert_eq!(s0.kit, "kit:us_sl");
        assert_eq!(doc.slots[1].kit, "kit:us_rifleman"); // no assetId → default
        assert_eq!(doc.slots[3].kit, "kit:sov_rifleman");

        // Orbat instance count must equal slots length (loader parity gate).
        let orbat_count: i64 = doc
            .orbat
            .values()
            .flat_map(|f| &f.groups)
            .flat_map(|g| &g.roles)
            .map(|r| r.count)
            .sum();
        assert_eq!(orbat_count, doc.slots.len() as i64);

        assert_eq!(doc.meta.player_range, [1, 64]);

        // B1 — uid carries the editor slot id (identity thread through the API route).
        assert_eq!(doc.slots[0].uid, "s1");

        // T-068.11 — compiled slots carry the loadout block (gear derivation +
        // verbatim cargo); loadout-less slots omit the key.
        let lo = doc.slots[0].loadout.as_ref().expect("s1 loadout");
        let g = lo.gear.as_ref().expect("s1 gear");
        assert_eq!(g.uniform.as_deref(), Some("res://bdu_blouse"));
        assert_eq!(g.vest.as_deref(), Some("res://pasgt")); // armoredVest wins
        assert_eq!(lo.cargo[0].qty, 4);
        assert!(doc.slots[1].loadout.is_none());
        assert_eq!(
            doc.slots[3].loadout.as_ref().unwrap().cargo[0].qty,
            40,
            "cargo qty verbatim"
        );

        // G6: the compiled document (incl. the T-068.11 loadout block) validates
        // against mission.schema.json.
        let bytes = serde_json::to_vec(&doc).unwrap();
        let details = validate_mission_document(&bytes).expect("schema compiles");
        assert!(details.is_empty(), "schema violations: {details:?}");
    }

    /// One squad, one slot, with the editor fields under test left to the caller.
    fn payload_with(role: &str, callsign: &str, squad_name: &str, slot_id: &str) -> String {
        format!(
            r#"{{"editor":{{
                "factions":[{{"id":"f1","key":"BLUFOR","name":"US Army","squadIds":["sq1"]}}],
                "squads":[{{"id":"sq1","factionId":"f1","callsign":"{callsign}","name":"{squad_name}","slotIds":["{slot_id}"]}}],
                "slots":[{{"id":"{slot_id}","squadId":"sq1","index":0,"role":"{role}",
                    "position":{{"x":100,"y":200,"z":0,"rotation":0}}}}],
                "editorLayers":[]}}}}"#
        )
    }

    fn findings_for(m: &Mission, payload: &str) -> Vec<String> {
        let doc = flatten_to_mod_document(m, payload.as_bytes()).expect("compiles");
        let bytes = serde_json::to_vec(&doc).unwrap();
        validate_mission_document(&bytes).expect("schema compiles")
    }

    // T-181.31 — the editor payload schema leaves `editor.slots[]` unconstrained on
    // purpose (O(1) validation on 100k-slot missions), so these blanks reach the
    // compile. `role` and `groupCallsign` are display labels the mod only warns about,
    // so the compile substitutes rather than emitting a document we would reject —
    // otherwise turning the /compiled gate on would hard-fail missions that load today.
    #[test]
    fn blank_role_and_callsign_still_compile_to_a_valid_document() {
        let m = fixture_mission();
        let details = findings_for(&m, &payload_with("", "", "", "s1"));
        assert!(details.is_empty(), "schema violations: {details:?}");

        let doc = flatten_to_mod_document(&m, payload_with("", "", "", "s1").as_bytes()).unwrap();
        assert_eq!(doc.slots[0].role, "unassigned");
        // No callsign and no name → the squad id, so two unnamed squads keep distinct
        // slot ids (a duplicate id is a hard error in TBD_MissionValidator).
        assert_eq!(doc.slots[0].group_callsign, "sq1");
        assert_eq!(doc.slots[0].id, "blufor:sq1:unassigned:0");
    }

    /// Two factions, each holding one slot — the case where elimination CAN resolve.
    fn payload_two_sides() -> String {
        r#"{"editor":{
            "factions":[{"id":"f1","key":"BLUFOR","name":"US Army","squadIds":["sq1"]},
                        {"id":"f2","key":"OPFOR","name":"USSR","squadIds":["sq2"]}],
            "squads":[{"id":"sq1","factionId":"f1","callsign":"Alpha","name":"A","slotIds":["s1"]},
                      {"id":"sq2","factionId":"f2","callsign":"Grom","name":"G","slotIds":["s2"]}],
            "slots":[{"id":"s1","squadId":"sq1","index":0,"role":"RFL",
                        "position":{"x":100,"y":200,"z":0,"rotation":0}},
                     {"id":"s2","squadId":"sq2","index":0,"role":"RFL",
                        "position":{"x":300,"y":400,"z":0,"rotation":0}}],
            "editorLayers":[]}}"#
            .to_string()
    }

    // T-181.46 — the editor never authors winConditions, so this is synthesized here. It used
    // to declare `faction_eliminated` unconditionally, which made EVERY single-faction mission
    // unloadable: TBD_MissionValidator rejects the document outright ("declares
    // faction_eliminated but only 1 faction(s) actually have slots — no second side can ever be
    // eliminated"), the server parks in LOADING, and the author has no way to fix it because the
    // field is not theirs to edit. Counted over the flattened SLOTS, not `factions`, because a
    // faction can be declared with no seats — which is exactly the shape that surfaced it.
    #[test]
    fn faction_eliminated_is_only_declared_when_two_sides_hold_slots() {
        let m = fixture_mission();

        let one = flatten_to_mod_document(&m, payload_with("RFL", "Alpha", "A", "s1").as_bytes())
            .expect("compiles");
        assert!(
            !one.win_conditions
                .end_on
                .iter()
                .any(|t| t == "faction_eliminated"),
            "one-sided mission must not declare faction_eliminated: {:?}",
            one.win_conditions.end_on
        );
        assert!(one.win_conditions.end_on.iter().any(|t| t == "time_limit"));

        let two = flatten_to_mod_document(&m, payload_two_sides().as_bytes()).expect("compiles");
        assert!(
            two.win_conditions
                .end_on
                .iter()
                .any(|t| t == "faction_eliminated"),
            "two-sided mission must still declare faction_eliminated: {:?}",
            two.win_conditions.end_on
        );
    }

    #[test]
    fn long_title_truncates_to_the_schema_maximum() {
        let mut m = fixture_mission();
        m.title = "T".repeat(200);
        let details = findings_for(&m, &payload_with("RFL", "Alpha", "A", "s1"));
        assert!(details.is_empty(), "schema violations: {details:?}");

        let doc = flatten_to_mod_document(&m, payload_with("RFL", "Alpha", "A", "s1").as_bytes())
            .unwrap();
        assert_eq!(doc.meta.name.chars().count(), 120);
    }

    /// The gate has to have something real to catch. A slot that lost its `id`
    /// compiles to `uid: ""`, which `mission.schema.json` rejects — and unlike the
    /// display labels above this one is NOT substituted, because `uid` is the durable
    /// slot identity the mod keys spawn points, rosters and logs on. Inventing one
    /// would be worse than refusing to serve the document.
    #[test]
    fn blank_slot_uid_is_a_schema_violation() {
        let m = fixture_mission();
        let details = findings_for(&m, &payload_with("RFL", "Alpha", "A", ""));
        assert!(
            details.iter().any(|d| d.contains("/slots/0/uid")),
            "expected a uid finding, got {details:?}"
        );
    }

    /// The load-bearing invariant of the T-181.44 save-time scan, pinned against the REAL compiler
    /// and the REAL schema rather than a restatement of either: for every authored string, the
    /// save-time scan fires **exactly** when compiling that payload would produce a
    /// `wireSafeString` violation.
    ///
    /// `⟸` (no false negatives) is the one that makes the `/compiled` 500 unreachable for this
    /// cause. `⟹` (no false positives) is what makes the save-time 400 trustworthy — it is why the
    /// scan mirrors flatten's fallback chains instead of checking every string it can find: a bad
    /// squad `name` that a non-empty `callsign` shadows never reaches the wire, so rejecting the
    /// save on it would be a gate crying wolf.
    ///
    /// Both directions break if flatten changes which authored field it reads. That is the point.
    #[test]
    fn save_scan_agrees_with_the_compiled_schema() {
        let m = fixture_mission();

        // Each case is a payload + one authored defect (or none). `\\t` here is the two-character
        // JSON escape, so the parsed value carries a real TAB — the T-181.42 callsign exactly.
        let cases: Vec<(&str, String)> = vec![
            ("clean", payload_with("RFL", "Alpha", "Alpha 1-1", "s1")),
            ("tab in role", payload_with("S\\tL", "Alpha", "A", "s1")),
            (
                "tab in callsign",
                payload_with("RFL", "AL\\tPHA", "A", "s1"),
            ),
            // callsign wins, so the bad `name` is never read: must be clean on BOTH sides.
            (
                "shadowed squad name",
                payload_with("RFL", "Alpha", "A\\tB", "s1"),
            ),
            // callsign blank → flatten reads `name`, so now it does reach the wire.
            ("read squad name", payload_with("RFL", "", "A\\tB", "s1")),
            (
                "newline in slot id",
                payload_with("RFL", "Alpha", "A", "s\\n1"),
            ),
            (
                "DEL in faction name",
                payload_with("RFL", "Alpha", "A", "s1").replace("US Army", "US\\u007fArmy"),
            ),
        ];

        let (mut fired, mut clean) = (0, 0);
        for (name, payload) in cases {
            let parsed: serde_json::Value =
                serde_json::from_str(&payload).unwrap_or_else(|e| panic!("{name}: {e}"));
            let scan = map_engine_core::mission::wire_safety::scan_editor_payload(&parsed);

            // Only the wireSafeString findings — the schema rejects other things (a blank uid, a
            // bad kit alias) for reasons this scan is not responsible for.
            let compiled: Vec<String> = findings_for(&m, &payload)
                .into_iter()
                .filter(|d| d.contains(r"^[^\x00-\x1F\x7F]*$"))
                .collect();

            assert_eq!(
                scan.is_empty(),
                compiled.is_empty(),
                "{name}: save-time scan and compiled-document schema disagree.\n  \
                 scan: {scan:?}\n  compiled: {compiled:?}"
            );
            if compiled.is_empty() {
                clean += 1
            } else {
                fired += 1
            }
        }

        // An `A == B` assertion over cases that never fire is a green that proves nothing — e.g. if
        // the filter substring above stopped matching the schema's pattern. Both sides must occur.
        assert!(
            fired >= 4,
            "only {fired} case(s) reached the schema pattern"
        );
        assert!(clean >= 2, "only {clean} case(s) compiled clean");
    }

    #[test]
    fn empty_editor_is_no_slots() {
        let m = fixture_mission();
        let payload = br#"{"editor":{"factions":[],"squads":[],"slots":[],"editorLayers":[]}}"#;
        assert!(matches!(
            flatten_to_mod_document(&m, payload),
            Err(CompileError::NoSlots)
        ));
    }

    /* ───────────────────────── T-192 — authored environment ───────────────────────── */

    /// Rebuild the two document JSON strings the save compiler reads
    /// (`MissionDocCore::small_maps_json` / `slots_json`) from the FIXTURE editor graph, with
    /// `meta.environment` set to whatever the Mission Settings dialog would have authored.
    ///
    /// The API crate does not enable map-engine-core's `doc` feature, so the CRDT itself cannot be
    /// driven from here; this reproduces its output shape. From `compile_payload` on it is the
    /// **real** save path — the same function that produces the bytes
    /// `POST /missions/:id/versions` stores, so the payload under test is not a hand-written
    /// restatement of what the editor emits.
    fn saved_payload_with_env(environment: serde_json::Value) -> String {
        use map_engine_core::mission::compile::compile_payload;

        let fixture: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
        let editor = &fixture["editor"];
        let by_id = |key: &str| -> serde_json::Value {
            editor[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| (v["id"].as_str().unwrap().to_string(), v.clone()))
                .collect::<serde_json::Map<String, serde_json::Value>>()
                .into()
        };
        let small = serde_json::json!({
            "meta": {
                "id": "m1",
                "title": "Compiled Fixture",
                "terrain": "everon",
                "environment": environment,
            },
            "factionsById": by_id("factions"),
            "squadsById": by_id("squads"),
            "loadoutsById": {},
            "itemsById": {},
            "objectivesById": {},
            "vehiclesById": {},
            "markersById": {},
            "editorLayersById": by_id("editorLayers"),
        });
        // `include_orbat: false` — the Save path exactly (Export is the one that injects orbat).
        compile_payload(&small.to_string(), &by_id("slots").to_string(), false).to_string()
    }

    /// **The T-192 regression.** An authored time/weather survives edit → save → compile.
    ///
    /// The row deliberately still carries the creation-time `05:30` / `clear`, because nothing in
    /// the editor ever PATCHed it and that is exactly the state this bug was found in. Before
    /// T-192 the compiled document reported the row, so the mission the game server loaded was
    /// never the mission the author had set up.
    ///
    /// `viewDistance` / `thermals` are in the payload because the dialog really writes them; they
    /// are **T-193**'s problem, and this test only pins that their presence keeps the compiled
    /// document schema-clean (the compiled `environment` is a fixed two-field struct).
    #[test]
    fn authored_environment_beats_a_stale_mission_row() {
        let m = fixture_mission();
        assert_eq!(m.time_of_day, "05:30");
        assert_eq!(m.weather.as_str(), "clear");

        let payload = saved_payload_with_env(serde_json::json!({
            "time": "21:45",
            "weather": "dense_fog",
            "viewDistance": 2500,
            "thermals": true,
        }));
        let doc = flatten_to_mod_document(&m, payload.as_bytes()).expect("compiles");
        let env = doc.environment.as_ref().expect("environment");

        assert_eq!(env.weather_preset, "dense_fog");
        assert!(
            env.date_time.ends_with("T21:45:00Z"),
            "authored time missing from dateTime: {}",
            env.date_time
        );

        // Still a document the game server will accept.
        let bytes = serde_json::to_vec(&doc).unwrap();
        let details = validate_mission_document(&bytes).expect("schema compiles");
        assert!(details.is_empty(), "schema violations: {details:?}");
    }

    /// The row is the fallback, not dead weight: a version saved before the editor authored an
    /// environment (or one whose environment is unusable) must still compile to the row's values
    /// rather than to nothing.
    #[test]
    fn unusable_or_absent_environment_falls_back_to_the_row() {
        let m = fixture_mission(); // 05:30 / clear

        let cases: Vec<(&str, String)> = vec![
            ("no environment at all", FIXTURE.to_string()),
            ("empty environment", saved_payload_with_env(json!({}))),
            (
                "wrong types",
                saved_payload_with_env(json!({ "time": 2145, "weather": false })),
            ),
            (
                "blank strings",
                saved_payload_with_env(json!({ "time": "", "weather": "" })),
            ),
            (
                "off-enum weather + junk time",
                saved_payload_with_env(json!({ "time": "half past four", "weather": "blizzard" })),
            ),
            (
                "out-of-range clock",
                saved_payload_with_env(json!({ "time": "24:00", "weather": "clear" })),
            ),
        ];

        for (name, payload) in cases {
            let doc = flatten_to_mod_document(&m, payload.as_bytes())
                .unwrap_or_else(|e| panic!("{name}: {e:?}"));
            let env = doc.environment.as_ref().expect("environment");
            assert!(
                env.date_time.ends_with("T05:30:00Z"),
                "{name}: expected the row's time, got {}",
                env.date_time
            );
            assert_eq!(env.weather_preset, "clear", "{name}: expected the row");
            let bytes = serde_json::to_vec(&doc).unwrap();
            let details = validate_mission_document(&bytes).expect("schema compiles");
            assert!(details.is_empty(), "{name}: schema violations: {details:?}");
        }
    }

    /// One field can be authored without the other — `weather` alone must not drag the row's time
    /// along, and vice versa.
    #[test]
    fn each_environment_field_falls_back_independently() {
        let m = fixture_mission();

        let weather_only = saved_payload_with_env(json!({ "weather": "overcast" }));
        let doc = flatten_to_mod_document(&m, weather_only.as_bytes()).unwrap();
        let env = doc.environment.as_ref().unwrap();
        assert_eq!(env.weather_preset, "overcast");
        assert!(env.date_time.ends_with("T05:30:00Z"), "{}", env.date_time);

        let time_only = saved_payload_with_env(json!({ "time": "19:05" }));
        let doc = flatten_to_mod_document(&m, time_only.as_bytes()).unwrap();
        let env = doc.environment.as_ref().unwrap();
        assert_eq!(env.weather_preset, "clear");
        assert!(env.date_time.ends_with("T19:05:00Z"), "{}", env.date_time);
    }

    /// `time_of_day` round-trips through Postgres as `HH:MM:SS`, so that is what
    /// `apply_row_meta` puts in the document and what the next save hands back here.
    #[test]
    fn clock_accepts_the_shapes_the_document_can_hold() {
        assert_eq!(clock_hhmm("21:45").as_deref(), Some("21:45"));
        assert_eq!(clock_hhmm("21:45:00").as_deref(), Some("21:45"));
        assert_eq!(clock_hhmm("06:00:59").as_deref(), Some("06:00"));
        assert_eq!(clock_hhmm("6:5").as_deref(), Some("06:05"));
        assert_eq!(clock_hhmm("00:00").as_deref(), Some("00:00"));
        assert_eq!(clock_hhmm("23:59").as_deref(), Some("23:59"));

        for bad in [
            "",
            "24:00",
            "12:60",
            "12:00:60",
            "12",
            "12:00:00:00",
            "-1:00",
            " 12:00",
            "noon",
            "12:0a",
        ] {
            assert_eq!(clock_hhmm(bad), None, "{bad:?} must not reach dateTime");
        }
    }

    #[test]
    fn weather_preset_is_the_row_enum_and_nothing_else() {
        for good in ["clear", "overcast", "heavy_rain", "dense_fog"] {
            assert_eq!(weather_preset(good).as_deref(), Some(good));
        }
        // "" is the PATCH's alias for "clear"; here it must mean "not authored" so the row wins.
        for bad in ["", "Clear", "blizzard", "heavy rain", "sunny"] {
            assert_eq!(weather_preset(bad), None, "{bad:?} must not reach the wire");
        }
    }
}
