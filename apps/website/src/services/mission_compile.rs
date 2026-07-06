//! Backend adapter for the shared mod-document flatten (T-145 Phase 2b). The compile logic lives
//! in `map_engine_core::mission::flatten`; this builds the core `MissionMeta` from the backend
//! `Mission` model and re-exports the output types so `crate::services::…` callers are unchanged.
//!
//! Locked coordinate mapping (in core): editor `position.x → x`, `position.y → z`,
//! `position.z → y` (optional, 1.2), `position.rotation → headingDeg`.
//!
//! @contract mission.schema.json#/

use crate::models::Mission;
use map_engine_core::mission::flatten::{self, MissionMeta};

pub use map_engine_core::mission::flatten::{CompileError, ModMissionDocument};

/// Build the compiled mod mission document from a mission row + its version payload. Thin wrapper
/// over the shared [`map_engine_core::mission::flatten::flatten_to_mod_document`].
pub fn flatten_to_mod_document(
    m: &Mission,
    payload: &[u8],
) -> Result<ModMissionDocument, CompileError> {
    let meta = MissionMeta {
        id: m.id.to_string(),
        title: m.title.clone(),
        author: m.author_id.clone(),
        terrain: m.terrain.as_str().to_string(),
        custom_terrain_name: m.custom_terrain_name.clone(),
        max_players: m.max_players,
        time_of_day: m.time_of_day.clone(),
        weather_preset: m.weather.as_str().to_string(),
    };
    flatten::flatten_to_mod_document(&meta, payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::validate_mission_document;
    use crate::models::{GameMode, MissionStatus, TerrainType, WeatherType};
    use chrono::Utc;
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
          {"id": "s1", "squadId": "sq1", "index": 0, "role": "SL", "assetId": "{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et", "position": {"x": 4839.2, "y": 6620.8, "z": 0, "rotation": 270}},
          {"id": "s2", "squadId": "sq1", "index": 1, "role": "TL", "position": {"x": 4836.9, "y": 6626.5, "z": 142.5, "rotation": 450}},
          {"id": "s3", "squadId": "sq1", "index": 2, "role": "TL", "position": {"x": 4831.2, "y": 6628.8, "z": 0, "rotation": 0}},
          {"id": "s4", "squadId": "sq2", "index": 0, "role": "RFL", "assetId": "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et", "position": {"x": 6010, "y": 7211.5, "z": 0, "rotation": 90}}
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

        // G6: the compiled document validates against mission.schema.json.
        let bytes = serde_json::to_vec(&doc).unwrap();
        let details = validate_mission_document(&bytes).expect("schema compiles");
        assert!(details.is_empty(), "schema violations: {details:?}");
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
}
