//! Mission compile flatten (gate G6) — Rust port of `services/mission_compile.go`,
//! the twin of the frontend `flattenModDocument.ts`. Derives the CANONICAL mod
//! mission document (mission.schema.json, string schemaVersion "1.1"/"1.2") from a
//! mission row + its version payload, mirroring the TS traversal EXACTLY so
//! `/missions/:id/compiled` and the client-side flatten agree.
//!
//! Locked coordinate mapping: editor `position.x → x`, `position.y → z`,
//! `position.z → y` (optional, 1.2), `position.rotation → headingDeg`.
//!
//! @contract mission.schema.json#/

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use crate::mission::kit::load_kit_aliases;

// ---- output document types (camelCase — the game-server contract) ----

/// One flattened `slots[]` entry.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSlot {
    pub id: String,
    pub faction: String,
    pub group_callsign: String,
    pub role: String,
    pub kit: String,
    pub x: f64,
    pub z: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    pub heading_deg: f64,
}

#[derive(Debug, Serialize)]
pub struct ModOrbatRole {
    pub slot: String,
    pub kit: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct ModOrbatGroup {
    pub callsign: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub roles: Vec<ModOrbatRole>,
}

#[derive(Debug, Serialize)]
pub struct ModOrbatFaction {
    pub groups: Vec<ModOrbatGroup>,
}

#[derive(Debug, Serialize)]
pub struct ModCircle {
    pub x: f64,
    pub z: f64,
    pub r: f64,
}

#[derive(Debug, Serialize)]
pub struct ModZoneShape {
    pub circle: ModCircle,
}

#[derive(Debug, Serialize)]
pub struct ModZone {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub faction: String,
    pub shape: ModZoneShape,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFaction {
    pub key: String,
    pub display_name: String,
    pub preset_id: String,
    pub tickets: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModMeta {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub author: String,
    pub terrain: String,
    pub template_id: String,
    pub player_range: [i64; 2],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModEnvironment {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub date_time: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub weather_preset: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFlow {
    pub briefing_seconds: i64,
    pub safe_start_seconds: i64,
    pub time_limit_seconds: i64,
    pub jip: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModWinConditions {
    pub mode: String,
    pub end_on: Vec<String>,
}

/// The full compiled document served to the game server.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModMissionDocument {
    pub schema_version: String,
    pub meta: ModMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<ModEnvironment>,
    pub factions: Vec<ModFaction>,
    /// `BTreeMap` → sorted keys, matching Go's map marshalling.
    pub orbat: BTreeMap<String, ModOrbatFaction>,
    pub slots: Vec<ModSlot>,
    pub zones: Vec<ModZone>,
    pub flow: ModFlow,
    pub win_conditions: ModWinConditions,
}

/// Compile failure — mirrors `ErrNoSlots` + a payload-parse error.
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("mission version has no placed slots")]
    NoSlots,
    #[error("parse mission version payload: {0}")]
    Parse(String),
}

// ---- input payload (the editor graph the TS flatten walks) ----

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct EditorPayload {
    editor: EditorGraph,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct EditorGraph {
    factions: Vec<FactionIn>,
    squads: Vec<SquadIn>,
    slots: Vec<SlotIn>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct FactionIn {
    key: String,
    name: String,
    squad_ids: Vec<String>,
}

#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SquadIn {
    id: String,
    callsign: String,
    name: String,
    slot_ids: Vec<String>,
}

#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SlotIn {
    id: String,
    index: i64,
    role: String,
    asset_id: String,
    position: PositionIn,
}

#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
struct PositionIn {
    x: f64,
    y: f64,
    z: f64,
    rotation: f64,
}

/// Mission-level metadata the flatten needs. The backend builds this from its `Mission` sqlx
/// model; the wasm client passes it as JSON (camelCase). Decouples the core compiler from any
/// backend type (T-145 Phase 2b). `terrain`/`weather_preset` are already the `as_str()` values.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MissionMeta {
    pub id: String,
    pub title: String,
    pub author: String,
    pub terrain: String,
    pub custom_terrain_name: String,
    pub max_players: i64,
    pub time_of_day: String,
    pub weather_preset: String,
}

const COMPILE_DATE_ANCHOR: &str = "1989-06-14";
const SPAWN_ZONE_RADIUS_M: f64 = 150.0;

/// Lowercase into the schema's `^[a-z][a-z0-9_]*$` pattern.
fn slug_key(raw: &str, fallback: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_repl = false;
    for c in raw.to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' {
            out.push(c);
            prev_repl = false;
        } else if !prev_repl {
            out.push('_');
            prev_repl = true;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        return fallback.to_string();
    }
    match trimmed.chars().next() {
        Some(c) if c.is_ascii_lowercase() => trimmed.to_string(),
        _ => format!("f_{trimmed}"),
    }
}

/// Reduce the mission UUID to the schema's `^msn_[a-z0-9]+$` id space.
fn mission_doc_id(id: &str) -> String {
    let hex: String = id
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .collect();
    format!("msn_{}", if hex.is_empty() { "editor" } else { &hex })
}

fn normalize_heading(rotation: f64) -> f64 {
    if rotation.is_nan() || rotation.is_infinite() {
        return 0.0;
    }
    (rotation % 360.0 + 360.0) % 360.0
}

/// Build the compiled mod mission document. Fields the editor never authors (zones,
/// flow, winConditions, templateId, playerRange, presetId) are synthesized with the
/// same defaults as `flattenModDocument.ts`. Returns [`CompileError::NoSlots`] when
/// the editor graph holds no placed slots.
pub fn flatten_to_mod_document(
    mission: &MissionMeta,
    payload: &[u8],
) -> Result<ModMissionDocument, CompileError> {
    let aliases = load_kit_aliases();
    let parsed: EditorPayload =
        serde_json::from_slice(payload).map_err(|e| CompileError::Parse(e.to_string()))?;
    let ed = parsed.editor;

    let squads_by_id: HashMap<&str, &SquadIn> =
        ed.squads.iter().map(|s| (s.id.as_str(), s)).collect();
    let slots_by_id: HashMap<&str, &SlotIn> = ed.slots.iter().map(|s| (s.id.as_str(), s)).collect();

    let mut factions: Vec<ModFaction> = Vec::new();
    let mut orbat: BTreeMap<String, ModOrbatFaction> = BTreeMap::new();
    let mut doc_slots: Vec<ModSlot> = Vec::new();
    let mut centroids: HashMap<String, (f64, f64, i64)> = HashMap::new();
    let mut centroid_order: Vec<String> = Vec::new();
    let mut any_y = false;

    for f in &ed.factions {
        let faction_key = slug_key(&f.key, "faction");
        let (default_kit, preset) = aliases.faction_default(&faction_key);
        let mut groups: Vec<ModOrbatGroup> = Vec::new();

        for squad_id in &f.squad_ids {
            let Some(sq) = squads_by_id.get(squad_id.as_str()) else {
                continue;
            };
            let mut rows: Vec<&SlotIn> = sq
                .slot_ids
                .iter()
                .filter_map(|id| slots_by_id.get(id.as_str()).copied())
                .collect();
            if rows.is_empty() {
                continue;
            }
            rows.sort_by_key(|s| s.index); // stable

            let callsign = if sq.callsign.is_empty() {
                sq.name.clone()
            } else {
                sq.callsign.clone()
            };

            let mut role_counters: HashMap<&str, i64> = HashMap::new();
            let mut role_index: HashMap<&str, usize> = HashMap::new();
            let mut roles: Vec<ModOrbatRole> = Vec::new();

            for sl in &rows {
                let occurrence = *role_counters.get(sl.role.as_str()).unwrap_or(&0);
                role_counters.insert(sl.role.as_str(), occurrence + 1);

                let kit = aliases
                    .kit_for_resource(&sl.asset_id)
                    .map_or_else(|| default_kit.to_string(), String::from);

                if let Some(&idx) = role_index.get(sl.role.as_str()) {
                    roles[idx].count += 1;
                } else {
                    role_index.insert(sl.role.as_str(), roles.len());
                    roles.push(ModOrbatRole {
                        slot: sl.role.clone(),
                        kit: kit.clone(),
                        count: 1,
                    });
                }

                let x = sl.position.x;
                let z = sl.position.y; // editor y (map north) → mod z
                let elev = sl.position.z; // editor z (elevation) → mod y (optional)
                let y = if elev != 0.0 && !elev.is_nan() && !elev.is_infinite() {
                    any_y = true;
                    Some(elev)
                } else {
                    None
                };

                doc_slots.push(ModSlot {
                    id: format!("{faction_key}:{callsign}:{}:{occurrence}", sl.role),
                    faction: faction_key.clone(),
                    group_callsign: callsign.clone(),
                    role: sl.role.clone(),
                    kit,
                    x,
                    z,
                    y,
                    heading_deg: normalize_heading(sl.position.rotation),
                });

                if !centroids.contains_key(&faction_key) {
                    centroids.insert(faction_key.clone(), (0.0, 0.0, 0));
                    centroid_order.push(faction_key.clone());
                }
                let c = centroids.get_mut(&faction_key).expect("inserted");
                c.0 += x;
                c.1 += z;
                c.2 += 1;
            }

            groups.push(ModOrbatGroup {
                callsign,
                kind: "rifle_squad".to_string(),
                roles,
            });
        }

        if !groups.is_empty() {
            orbat.insert(faction_key.clone(), ModOrbatFaction { groups });
        }
        let display_name = if f.name.is_empty() {
            faction_key.clone()
        } else {
            f.name.clone()
        };
        factions.push(ModFaction {
            key: faction_key,
            display_name,
            preset_id: preset.to_string(),
            tickets: 0,
        });
    }

    if doc_slots.is_empty() {
        return Err(CompileError::NoSlots);
    }

    let schema_version = if any_y { "1.2" } else { "1.1" }.to_string();

    // Schema requires ≥ 2 factions; pad a stub opposing faction for single-faction drafts.
    if factions.len() < 2 {
        let mut stub = "opfor";
        for f in &factions {
            if f.key == "opfor" {
                stub = "blufor";
            }
        }
        let (_, preset) = aliases.faction_default(stub);
        factions.push(ModFaction {
            key: stub.to_string(),
            display_name: stub.to_uppercase(),
            preset_id: preset.to_string(),
            tickets: 0,
        });
    }

    let mut zones: Vec<ModZone> = Vec::new();
    for faction_key in &centroid_order {
        let (sx, sz, n) = centroids[faction_key];
        let nf = n as f64;
        zones.push(ModZone {
            id: format!("z_spawn_{faction_key}"),
            kind: "spawn".to_string(),
            faction: faction_key.clone(),
            shape: ModZoneShape {
                circle: ModCircle {
                    x: (sx / nf * 10.0).round() / 10.0,
                    z: (sz / nf * 10.0).round() / 10.0,
                    r: SPAWN_ZONE_RADIUS_M,
                },
            },
        });
    }

    let max_players = if mission.max_players < 1 {
        (doc_slots.len() as i64).max(1)
    } else {
        mission.max_players
    };

    let mut terrain = mission.terrain.clone();
    if terrain == "custom" && !mission.custom_terrain_name.is_empty() {
        terrain = mission.custom_terrain_name.clone();
    }

    let meta = ModMeta {
        id: mission_doc_id(&mission.id),
        name: if mission.title.is_empty() {
            "Untitled Mission".to_string()
        } else {
            mission.title.clone()
        },
        author: mission.author.clone(),
        terrain: slug_key(&terrain, "everon"),
        template_id: "editor_v1".to_string(),
        player_range: [1, max_players],
    };

    let mut environment = ModEnvironment {
        date_time: String::new(),
        weather_preset: mission.weather_preset.clone(),
    };
    if !mission.time_of_day.is_empty() {
        // time_of_day may be HH:MM or HH:MM:SS — keep exactly HH:MM.
        let t = if mission.time_of_day.len() > 5 {
            &mission.time_of_day[..5]
        } else {
            &mission.time_of_day
        };
        environment.date_time = format!("{COMPILE_DATE_ANCHOR}T{t}:00Z");
    }

    Ok(ModMissionDocument {
        schema_version,
        meta,
        environment: Some(environment),
        factions,
        orbat,
        slots: doc_slots,
        zones,
        flow: ModFlow {
            briefing_seconds: 600,
            safe_start_seconds: 300,
            time_limit_seconds: 5400,
            jip: "until_safestart_end".to_string(),
        },
        win_conditions: ModWinConditions {
            mode: "attrition".to_string(),
            end_on: vec!["time_limit".to_string(), "faction_eliminated".to_string()],
        },
    })
}

/// JSON-in / JSON-out flatten for the wasm client: `meta_json` (camelCase [`MissionMeta`]) + the
/// stored version `payload` → the compiled mod-document JSON bytes. Keeps serde_json on the core
/// side so the wasm shim stays dependency-thin.
///
/// # Errors
/// Returns a message on meta/payload parse failure or a compile error (e.g. no slots).
pub fn flatten_mod_document_json(meta_json: &[u8], payload: &[u8]) -> Result<Vec<u8>, String> {
    let meta: MissionMeta = serde_json::from_slice(meta_json).map_err(|e| e.to_string())?;
    let doc = flatten_to_mod_document(&meta, payload).map_err(|e| e.to_string())?;
    serde_json::to_vec(&doc).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Two factions, callsigned squads, a duplicate role (TL x2), one slot with real elevation.
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

    fn meta() -> MissionMeta {
        MissionMeta {
            id: "11112222333344445555666677778888".into(),
            title: "Compiled Fixture".into(),
            author: "maker".into(),
            terrain: "everon".into(),
            custom_terrain_name: String::new(),
            max_players: 64,
            time_of_day: "05:30".into(),
            weather_preset: "clear".into(),
        }
    }

    #[test]
    fn flatten_matches_locked_contract() {
        let doc = flatten_to_mod_document(&meta(), FIXTURE.as_bytes()).expect("compiles");
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
        assert_eq!(doc.slots[1].kit, "kit:us_rifleman");
        assert_eq!(doc.slots[3].kit, "kit:sov_rifleman");
        // Orbat instance count == slots length (loader parity gate).
        let orbat_count: i64 = doc
            .orbat
            .values()
            .flat_map(|f| &f.groups)
            .flat_map(|g| &g.roles)
            .map(|r| r.count)
            .sum();
        assert_eq!(orbat_count, doc.slots.len() as i64);
        assert_eq!(doc.meta.player_range, [1, 64]);
    }

    #[test]
    fn empty_editor_is_no_slots() {
        let payload = br#"{"editor":{"factions":[],"squads":[],"slots":[],"editorLayers":[]}}"#;
        assert!(matches!(
            flatten_to_mod_document(&meta(), payload),
            Err(CompileError::NoSlots)
        ));
    }
}
