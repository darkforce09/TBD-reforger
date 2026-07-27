//! Backend adapter for the shared mod-document flatten (T-145 Phase 2b). The compile logic lives
//! in `map_engine_core::mission::flatten`; this builds the core `MissionMeta` from the backend
//! `Mission` model **plus the environment authored into the saved version payload** (T-192), and
//! re-exports the output types so `crate::services::…` callers are unchanged.
//!
//! Locked coordinate mapping (in core): editor `position.x → x`, `position.y → z`,
//! `position.z → y` (optional, 1.2), `position.rotation → headingDeg`.
//!
//! @contract mission.schema.json#/

use crate::models::Mission;
use map_engine_core::mission::flatten::{self, MissionMeta};
use map_engine_core::mission::wire_safety::{self, CargoPhysCatalog};

pub use map_engine_core::mission::flatten::{
    CompileError, ModMissionDocument, ModSlot, mission_terrain_key,
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
///
/// **T-243 — the precedence itself now lives in core** ([`flatten::apply_authored_environment`]).
/// It was private to this file, which put it out of reach of the browser; the editor's server-truth
/// Export preview calls `flatten::flatten_mod_document_json`, and a second hand-written copy of
/// this rule over there would have let the preview disagree with this route on the one field T-192
/// exists to fix. This function is now purely the **row → [`MissionMeta`] adapter**; everything
/// downstream of it is shared code, which is what makes the twin honest.
///
/// **T-500 — cargo capacity.** Routes through [`flatten_to_mod_document_with_catalog`] with an
/// **empty** catalog (cargo walk is a no-op — never invent limits). The live Save boundary
/// (`handlers::missions::validate_payload` → `load_cargo_phys_catalog` →
/// `validate_mission_editor_payload_with_catalog`) already refuses over-capacity before a version
/// is stored, so API-written payloads that reach compile are cargo-clean. Callers that hold the
/// same registry phys table Save loads must use [`flatten_to_mod_document_with_catalog`] so
/// pre-T-416 stored rows (and any write that bypassed Save) cannot compile either.
pub fn flatten_to_mod_document(
    m: &Mission,
    payload: &[u8],
) -> Result<ModMissionDocument, CompileError> {
    flatten_to_mod_document_with_catalog(m, payload, &CargoPhysCatalog::new())
}

/// T-500 — same compile as [`flatten_to_mod_document`], but the T-416 cargo-capacity walk uses
/// `catalog` (the same `resource_name →` phys table Save builds via `load_cargo_phys_catalog`).
///
/// Over-capacity findings become [`CompileError::Parse`] carrying the same `/editor/...` strings
/// Save puts in its 400 `details` — one helper ([`wire_safety::scan_cargo_capacity`]), two
/// boundaries. Empty catalog stays silent (never invent), matching Save.
pub fn flatten_to_mod_document_with_catalog(
    m: &Mission,
    payload: &[u8],
    catalog: &CargoPhysCatalog,
) -> Result<ModMissionDocument, CompileError> {
    if let Ok(instance) = serde_json::from_slice::<serde_json::Value>(payload) {
        let findings = wire_safety::scan_cargo_capacity(&instance, catalog);
        if !findings.is_empty() {
            return Err(CompileError::Parse(findings.join("; ")));
        }
    }

    let mut meta = MissionMeta {
        id: m.id.to_string(),
        title: m.title.clone(),
        author: m.author_id.clone(),
        terrain: m.terrain.as_str().to_string(),
        custom_terrain_name: m.custom_terrain_name.clone(),
        max_players: m.max_players,
        time_of_day: m.time_of_day.clone(),
        weather_preset: m.weather.as_str().to_string(),
    };
    flatten::apply_authored_environment(&mut meta, payload);
    flatten::flatten_to_mod_document(&meta, payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::validate::validate_mission_editor_payload_with_catalog;
    use crate::contract::validate_mission_document;
    use crate::models::{GameMode, MissionStatus, TerrainType, WeatherType};
    use chrono::Utc;
    use map_engine_core::mission::wire_safety::CargoPhys;
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
             "weapons": [{"slotIndex": 0, "slotType": "primary", "weapon": "res://m16", "optic": "res://acog", "magazine": "res://stanag", "attachments": []},
                         {"slotIndex": 1, "slotType": "primary", "weapon": "res://m72", "attachments": []},
                         {"slotIndex": 2, "slotType": "secondary", "weapon": "res://m9", "attachments": []},
                         {"slotIndex": 3, "slotType": "grenade", "weapon": "res://m67", "attachments": []}],
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

        // T-182 — all four authored weapon slots survive the compile, not just the rifle.
        assert_eq!(
            (
                g.primary.as_deref(),
                g.launcher.as_deref(),
                g.handgun.as_deref(),
                g.throwable.as_deref()
            ),
            (
                Some("res://m16"),
                Some("res://m72"),
                Some("res://m9"),
                Some("res://m67")
            )
        );

        // G6: the compiled document (incl. the T-068.11 loadout block and the T-182 weapon
        // slots) validates against mission.schema.json. This is the assertion that stands
        // between a widened compiler and a 500 on GET /missions/:id/compiled — `gear` is
        // `additionalProperties: false`, so emitting launcher/handgun/throwable without the
        // matching schema keys would fail here, and in production the mod's error path would
        // fall back to a STALE CACHED MISSION rather than surfacing the break.
        let bytes = serde_json::to_vec(&doc).unwrap();
        let details = validate_mission_document(&bytes).expect("schema compiles");
        assert!(details.is_empty(), "schema violations: {details:?}");
    }

    /// T-183 — the locked contract above only pins blufor/opfor, which is exactly how INDFOR
    /// shipped broken: `kit-aliases.json` had no `factionDefaults.indfor`, so
    /// `KitAliases::faction_default` fell through to `fallbackFaction` (blufor) and every INDFOR
    /// slot compiled to `kit:us_rifleman` / `preset:us_army_82nd`. Nothing failed — the document
    /// still validated — and the mod then spawned a `Character_US_Rifleman.et` body while
    /// `TBD_SpawnManager.EngineFactionKey` forced engine faction FIA. INDFOR is the third
    /// editor-mintable side (`apply_faction.rs` VALID_SIDES = BLUFOR|OPFOR|INDFOR), so this fired
    /// on every INDFOR mission. Pinning the compiled values is what stops a silent re-drift.
    #[test]
    fn indfor_compiles_to_fia_not_the_blufor_fallback() {
        let m = fixture_mission();
        let payload = r#"{
          "editor": {
            "factions": [{"id": "f1", "key": "INDFOR", "name": "FIA", "squadIds": ["sq1"]}],
            "squads": [{"id": "sq1", "factionId": "f1", "callsign": "Kilo", "name": "Kilo 1", "slotIds": ["s1", "s2"]}],
            "slots": [
              {"id": "s1", "squadId": "sq1", "index": 0, "role": "RFL", "position": {"x": 5000, "y": 5000, "z": 0, "rotation": 0}},
              {"id": "s2", "squadId": "sq1", "index": 1, "role": "SL", "assetId": "{677B515F119222C2}Prefabs/Characters/Factions/INDFOR/FIA/Character_FIA_SL.et", "position": {"x": 5010, "y": 5000, "z": 0, "rotation": 0}}
            ],
            "editorLayers": []
          }
        }"#;
        let doc = flatten_to_mod_document(&m, payload.as_bytes()).expect("compiles");

        // No assetId → faction default. This is the assertion the bug would have failed.
        assert_eq!(doc.slots[0].kit, "kit:fia_rifleman");
        assert_ne!(
            doc.slots[0].kit, "kit:us_rifleman",
            "INDFOR fell back to the blufor default — factionDefaults.indfor is missing"
        );
        // Mapped assetId → the FIA kit row, not a degrade to the default.
        assert_eq!(doc.slots[1].kit, "kit:fia_sl");

        // The faction's presetId is the other half of `faction_default`; a one-sided fix
        // would leave INDFOR wearing `preset:us_army_82nd`.
        let indfor = doc
            .factions
            .iter()
            .find(|f| f.key == "indfor")
            .expect("indfor faction emitted");
        assert_eq!(indfor.preset_id, "preset:fia");

        // The compiled document must still satisfy mission.schema.json — an empty kit would
        // fail `^kit:[a-z0-9_]+$` and the mod could not load the mission at all.
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

    /// **T-243 — the coupling that moving the allowlist to core would otherwise have dropped.**
    ///
    /// `flatten::WEATHER_PRESETS` used to be a `match` on this very enum, so the compiler
    /// guaranteed the compiled document could never carry a weather the row is unable to follow.
    /// `map-engine-core` cannot name a `sqlx` enum, so the guarantee is now a test — and it is a
    /// real one: adding a fifth `WeatherType` variant fails HERE, in the same commit, rather than
    /// silently making a legitimately-authored weather fall back to the row's value forever.
    ///
    /// Asserted by round-trip through the shared reader rather than against a copied literal list,
    /// so this cannot pass by two identical typos.
    #[test]
    fn weather_preset_list_matches_the_row_enum() {
        for w in [
            WeatherType::Clear,
            WeatherType::Overcast,
            WeatherType::HeavyRain,
            WeatherType::DenseFog,
        ] {
            let mut meta = MissionMeta {
                weather_preset: "row-sentinel".into(),
                ..MissionMeta::default()
            };
            let payload = saved_payload_with_env(json!({ "weather": w.as_str() }));
            flatten::apply_authored_environment(&mut meta, payload.as_bytes());
            assert_eq!(
                meta.weather_preset,
                w.as_str(),
                "{:?} is a row weather core will not accept — add it to flatten::WEATHER_PRESETS",
                w.as_str()
            );
        }
    }

    /// **T-243 — THE parity assertion. This is what makes the editor's server-truth Export
    /// trustworthy, and it is the only thing that does.**
    ///
    /// The editor cannot call `GET /missions/:id/compiled`: that route takes a [`ServiceAuth`]
    /// (`handlers::missions::get_compiled_mission`), so an author's browser session is refused by
    /// design. The preview is therefore a *twin* — `flatten::flatten_mod_document_json` run in
    /// wasm over the same payload — and a twin is worth less than nothing if it can drift, because
    /// a confident wrong preview is worse than no preview at all.
    ///
    /// So this runs BOTH paths over the same row and the same payload bytes and demands the output
    /// be **byte-identical**:
    ///
    ///   * server: `mission_compile::flatten_to_mod_document` → `serde_json::to_vec`, which is
    ///     exactly what `handlers::missions::validated_compiled_body` serves;
    ///   * client: `flatten::flatten_mod_document_json` over the camelCase [`MissionMeta`] the
    ///     frontend builds from `GET /missions/:id` (`mission_commands::compiled_meta_json`).
    ///
    /// The fixture deliberately carries an **authored environment that disagrees with the row**
    /// (row 05:30/clear, payload 21:45/dense_fog). That is not decoration — it is the T-192 case,
    /// and it is the only field where the two paths could plausibly diverge, because everything
    /// else is a straight copy out of the row. Before T-243 this test failed: the client twin had
    /// no payload-first precedence at all and reported the row's stale 05:30/clear.
    ///
    /// This is also the whole non-vacuity argument. A test that merely proved the binding was
    /// *called* would pass over a preview that is silently wrong; this one fails the instant the
    /// two implementations disagree about a single byte, in either direction, whichever side moved.
    #[test]
    fn client_twin_is_byte_identical_to_the_compiled_route() {
        let m = fixture_mission(); // row: 05:30 / clear
        assert_eq!(m.time_of_day, "05:30");
        assert_eq!(m.weather.as_str(), "clear");

        let payload = saved_payload_with_env(json!({
            "time": "21:45",
            "weather": "dense_fog",
        }));

        // The server's bytes — what a game server receives from `/compiled`.
        let served = serde_json::to_vec(
            &flatten_to_mod_document(&m, payload.as_bytes()).expect("server path compiles"),
        )
        .expect("server document serializes");

        // The client's bytes — what the author downloads from the editor. The meta JSON is the
        // camelCase shape `mission_commands::compiled_meta_json` builds from the mission row.
        let meta_json = json!({
            "id": m.id.to_string(),
            "title": m.title,
            "author": m.author_id,
            "terrain": m.terrain.as_str(),
            "customTerrainName": m.custom_terrain_name,
            "maxPlayers": m.max_players,
            "timeOfDay": m.time_of_day,
            "weatherPreset": m.weather.as_str(),
        })
        .to_string();
        let previewed =
            flatten::flatten_mod_document_json(meta_json.as_bytes(), payload.as_bytes())
                .expect("client path compiles");

        assert_eq!(
            String::from_utf8_lossy(&previewed),
            String::from_utf8_lossy(&served),
            "the editor's preview is not the document the game server would receive",
        );

        // And the shared bytes really are the T-192 answer, not two matching stale ones.
        let doc: serde_json::Value = serde_json::from_slice(&served).unwrap();
        assert_eq!(doc["environment"]["weatherPreset"], json!("dense_fog"));
        assert_eq!(
            doc["environment"]["dateTime"].as_str().unwrap_or_default(),
            format!(
                "{}T21:45:00Z",
                doc["environment"]["dateTime"]
                    .as_str()
                    .unwrap_or_default()
                    .split('T')
                    .next()
                    .unwrap_or_default()
            )
        );

        // A preview the mod would reject is not server truth either.
        let findings = validate_mission_document(&previewed).expect("schema compiles");
        assert!(findings.is_empty(), "schema violations: {findings:?}");
    }

    /* ───────────────────────── T-500 — cargo refuse at compile ───────────────────────── */

    /// Same phys table + over-capacity numbers as Save's T-416 Class-R
    /// (`handlers::missions::over_capacity_cargo_is_refused_at_save_with_catalog` /
    /// `contract::validate::over_capacity_cargo_is_a_save_time_finding_with_catalog`).
    fn cargo_phys_catalog_fixture() -> CargoPhysCatalog {
        let mut catalog = CargoPhysCatalog::new();
        catalog.insert(
            "mag".into(),
            CargoPhys {
                display_name: "Mag".into(),
                weight_kg: Some(0.5),
                volume_cm3: Some(60.0),
                ..CargoPhys::default()
            },
        );
        catalog.insert(
            "vest_rn".into(),
            CargoPhys {
                display_name: "Plate Carrier".into(),
                max_weight_kg: Some(5.0),
                max_volume_cm3: Some(200.0),
                ..CargoPhys::default()
            },
        );
        catalog
    }

    /// One placed slot (so flatten would otherwise succeed) carrying cargo qty against a vest.
    fn cargo_slot_payload(qty: u32) -> String {
        format!(
            r#"{{
              "schemaVersion": 1,
              "editor": {{
                "factions": [{{"id": "f1", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]}}],
                "squads": [{{"id": "sq1", "factionId": "f1", "callsign": "Alpha", "slotIds": ["s1"]}}],
                "slots": [{{
                  "id": "s1", "squadId": "sq1", "index": 0, "role": "RFL",
                  "position": {{"x": 100.0, "y": 200.0, "z": 0, "rotation": 0}},
                  "loadout": {{"version": 2,
                    "wear": {{"vest": "vest_rn"}}, "weapons": [],
                    "cargo": [{{"container": "vest", "item": "mag", "qty": {qty}}}]}}
                }}],
                "editorLayers": []
              }}
            }}"#
        )
    }

    /// T-500 — with the same catalog Save uses, compile refuses the same over-capacity finding.
    ///
    /// RED: delete the `scan_cargo_capacity` call from `flatten_to_mod_document_with_catalog`.
    /// RED: invent a second cargo arithmetic that disagrees with Save's finding string.
    #[test]
    fn compile_with_catalog_refuses_over_capacity_like_save() {
        let m = fixture_mission();
        let catalog = cargo_phys_catalog_fixture();
        let bad = cargo_slot_payload(4);
        let ok = cargo_slot_payload(3);

        // Save channel (exact helper CreateVersion uses after loading the catalog) + the shared
        // scan itself — compile must refuse with the same strings, not a restated message.
        let save_details =
            validate_mission_editor_payload_with_catalog(bad.as_bytes(), &catalog).expect("schema");
        let parsed: serde_json::Value = serde_json::from_str(&bad).unwrap();
        let scan = wire_safety::scan_cargo_capacity(&parsed, &catalog);
        assert!(
            !scan.is_empty()
                && scan
                    .iter()
                    .any(|d| d.contains("240 / 200 cm³") && d.contains("Plate Carrier")),
            "shared scan must fire on this fixture: {scan:?}"
        );
        for d in &scan {
            assert!(
                save_details.iter().any(|s| s == d),
                "Save details must include scan finding {d:?}; got {save_details:?}"
            );
        }

        // Compile channel — same helper, same strings, Parse so `/compiled` cannot ship it.
        let err = flatten_to_mod_document_with_catalog(&m, bad.as_bytes(), &catalog)
            .expect_err("over-capacity must refuse at compile");
        let CompileError::Parse(detail) = err else {
            panic!("expected CompileError::Parse, got {err:?}");
        };
        for d in &scan {
            assert!(
                detail.contains(d.as_str()),
                "compile refuse missing scan finding {d:?}; got {detail}"
            );
        }

        flatten_to_mod_document_with_catalog(&m, ok.as_bytes(), &catalog)
            .expect("under-capacity must compile");
        flatten_to_mod_document_with_catalog(&m, bad.as_bytes(), &CargoPhysCatalog::new())
            .expect("empty catalog must not invent a limit (matches Save silence)");
    }

    /// T-500 Class-R — the no-arg compile entry always routes through the catalogued gate
    /// (empty catalog today). RED: `flatten_to_mod_document` bypasses `with_catalog` again.
    #[test]
    fn flatten_routes_through_catalogued_compile_gate() {
        const SRC: &str = include_str!("mission_compile.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("mission_compile.rs must have a #[cfg(test)] module");
        assert!(
            production.contains("fn flatten_to_mod_document_with_catalog("),
            "catalogued compile gate must exist"
        );
        assert!(
            production.contains("wire_safety::scan_cargo_capacity"),
            "compile gate must call the same cargo helper Save uses"
        );
        let no_arg = production
            .split("pub fn flatten_to_mod_document(")
            .nth(1)
            .and_then(|s| {
                s.split("pub fn flatten_to_mod_document_with_catalog(")
                    .next()
            })
            .expect("flatten_to_mod_document must exist before with_catalog");
        assert!(
            no_arg.contains("flatten_to_mod_document_with_catalog("),
            "no-arg flatten must delegate to the catalogued gate"
        );
    }

    /// T-500 Class-R — compile-as-trust-saved for the empty-catalog default: Save owns the live
    /// refuse via `load_cargo_phys_catalog`. RED: Save drops the catalog load, or this adapter
    /// stops documenting that dependency.
    #[test]
    fn compile_documents_save_cargo_refuse() {
        const COMPILE: &str = include_str!("mission_compile.rs");
        let production = COMPILE
            .split("#[cfg(test)]")
            .next()
            .expect("mission_compile.rs must have a #[cfg(test)] module");
        assert!(
            production.contains("load_cargo_phys_catalog"),
            "compile adapter must name Save's catalog loader (trust-saved contract)"
        );
        assert!(
            production.contains("validate_mission_editor_payload_with_catalog"),
            "compile adapter must name Save's catalogued validator"
        );

        const HANDLER: &str = include_str!("../handlers/missions.rs");
        let handler_prod = HANDLER
            .split("#[cfg(test)]")
            .next()
            .expect("missions.rs must have a #[cfg(test)] module");
        assert!(
            handler_prod.contains("load_cargo_phys_catalog"),
            "Save must still load registry phys into the catalog"
        );
        let helper = handler_prod
            .split("fn validate_payload_with_catalog(")
            .nth(1)
            .expect("validate_payload_with_catalog must exist");
        assert!(
            helper.contains("validate_mission_editor_payload_with_catalog"),
            "Save helper must call the catalogued validator"
        );
    }
}
