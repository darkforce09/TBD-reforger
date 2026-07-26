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
    /// Stable slot identity (B1): the editor doc's slot id, carried verbatim so the
    /// identity survives recompiles — `id` above is DERIVED (faction:callsign:role:
    /// occurrence) and shifts under role renames/reorders/deletes. Spawn points,
    /// rosters and logs should key on `uid`; `id` stays the human-readable label.
    /// (Named `uid`, not `ref` — `ref` is an EnforceScript keyword and the mod
    /// struct field names must equal the JSON keys.)
    pub uid: String,
    pub faction: String,
    pub group_callsign: String,
    pub role: String,
    pub kit: String,
    pub x: f64,
    pub z: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    pub heading_deg: f64,
    /// Optional Arsenal loadout (T-068.11) — omitted when the editor slot carries none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loadout: Option<ModSlotLoadout>,
}

/// Per-slot loadout block (mission.schema.json `slot.loadout`): fixed gear + container
/// cargo, derived from the editor `SlotLoadoutV2`. Kit alias stays the base character;
/// this layers on top (T-068.12 equips it onto the spawned player).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSlotLoadout {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gear: Option<ModSlotGear>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cargo: Vec<ModSlotCargo>,
}

/// Fixed gear ResourceNames — the v1 mod-reader shape, same derivation the
/// loadout-export schema documents: jacket→uniform, **armoredVest else vest→vest
/// (known collapse: a chest rig layered under a plate carrier loses the rig —
/// single-vest rule, documented)**, headCover→helmet; A3 widens with
/// pants/boots/handwear/backpack so an Arsenal-authored slot arrives complete.
/// **T-182** adds the three weapon slots the compiler used to discard, so all
/// four authored weapons now reach the wire — see `mod_slot_loadout` for the
/// `(slotIndex, slotType)` selectors. Empty slots are omitted, never empty
/// strings.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSlotGear {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magazine: Option<String>,
    /// T-182 — the other three authored weapon slots. Named with the EDITOR's own vocabulary
    /// (`arsenal_rules.rs` `WEAPON_SLOTS`) so the compiled document reads the same words the
    /// Arsenal UI shows. None of the three carry optic/magazine sub-slots — those ride the
    /// slotIndex-0 primary alone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launcher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handgun: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throwable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub helmet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pants: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boots: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handwear: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backpack: Option<String>,
}

impl ModSlotGear {
    fn is_empty(&self) -> bool {
        self.primary.is_none()
            && self.optic.is_none()
            && self.magazine.is_none()
            // T-182 — a launcher-only (or throwable-only) gear block is authored content. Omit
            // these three and `mod_slot_loadout` would drop the whole `loadout` key for such a
            // slot, so the fields would never reach the wire in the one case they are the only
            // thing on it.
            && self.launcher.is_none()
            && self.handgun.is_none()
            && self.throwable.is_none()
            && self.uniform.is_none()
            && self.vest.is_none()
            && self.helmet.is_none()
            && self.pants.is_none()
            && self.boots.is_none()
            && self.handwear.is_none()
            && self.backpack.is_none()
    }
}

/// One container cargo row (`{container, item, qty}` — loadout-export v2), copied
/// verbatim from the editor cargo.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSlotCargo {
    pub container: String,
    pub item: String,
    pub qty: i64,
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
    /// The editor `SlotLoadoutV2` dict (T-068.10/.15.2) — mapped by [`mod_slot_loadout`].
    loadout: Option<serde_json::Value>,
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

/// `mission.schema.json#/$defs/meta/name` — `maxLength: 120`.
const META_NAME_MAX_CHARS: usize = 120;

/// Stand-in for a slot the author never gave a role. The schema demands
/// `minLength: 1` on both `slots[].role` and `orbat.*.groups[].roles[].slot`;
/// the editor does not require the field, so the compile must supply something
/// rather than emit a document we would then reject (T-181.31).
const ROLE_FALLBACK: &str = "unassigned";

/// Stand-in for a squad with neither `callsign` nor `name`. Only reached when the
/// squad also has no id, because the id is preferred — two unnamed squads must not
/// collapse onto one callsign, or their derived slot ids collide and the mod's
/// duplicate-id check (a hard error there) rejects the whole document.
const CALLSIGN_FALLBACK: &str = "squad";

/// The schema's `minLength: 1` string fields cannot take the empty string, and the
/// editor does not guarantee these are set. Substitute rather than emit a document
/// that fails our own contract.
fn or_fallback<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() { fallback } else { value }
}

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

/// The compiled document's `meta.terrain` — the ONE definition, because the mod
/// routes worlds on it. `TBD_FrameworkManager.SelectMissionByNumber` compares the
/// mission-list entry's `terrain` against the loaded document's `meta.terrain` and
/// feeds it to `TBD_ScenarioRouter.GetScenarioForTerrain`; a list that said
/// `"Everon"` where the document says `"everon"` would restart the scenario it was
/// already on, or fail to find one at all. `GET /api/v1/ingest/missions` therefore
/// calls THIS rather than re-deriving the slug (T-181.51).
pub fn mission_terrain_key(terrain: &str, custom_terrain_name: &str) -> String {
    let raw = if terrain == "custom" && !custom_terrain_name.is_empty() {
        custom_terrain_name
    } else {
        terrain
    };
    slug_key(raw, "everon")
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

/// Map an editor `SlotLoadoutV2` dict onto the compiled loadout block. Empty
/// strings and malformed cargo rows drop (the editor tolerance); an all-empty
/// result returns `None` so the whole `loadout` key is omitted. Gear derivation
/// is the locked loadout-export rule: jacket→uniform, armoredVest else
/// vest→vest, headCover→helmet; and, since T-182, ALL FOUR authored weapon slots
/// by `(slotIndex, slotType)` — `(0,primary)`→primary (+optic/magazine),
/// `(1,primary)`→launcher, `(2,secondary)`→handgun, `(3,grenade)`→throwable.
/// Before T-182 only `(0,primary)` was selected, so a player authored with a
/// launcher, a sidearm or a grenade spawned without it.
fn mod_slot_loadout(lo: &serde_json::Value) -> Option<ModSlotLoadout> {
    let non_empty = |v: Option<&serde_json::Value>| {
        v.and_then(serde_json::Value::as_str)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    let wear = lo.get("wear");
    let wear_key = |k: &str| non_empty(wear.and_then(|w| w.get(k)));

    let mut gear = ModSlotGear {
        uniform: wear_key("jacket"),
        vest: wear_key("armoredVest").or_else(|| wear_key("vest")),
        helmet: wear_key("headCover"),
        pants: wear_key("pants"),
        boots: wear_key("boots"),
        handwear: wear_key("handwear"),
        backpack: wear_key("backpack"),
        ..ModSlotGear::default()
    };
    // T-182 — select ALL FOUR authored weapon slots, each by its exact (slotIndex, slotType) pair.
    // This used to match only (0, "primary") and silently drop the rest, so a slot authored with a
    // launcher, a sidearm and a grenade spawned carrying none of them. The pairs are the editor's
    // own table — keep byte-identical to `arsenal_rules.rs` `WEAPON_SLOTS`. Matching on the PAIR
    // rather than the index alone matters: slots 0 and 1 are both slotType "primary" (two untyped
    // long slots), so the index is what separates rifle from launcher, while slotType is what
    // stops a mis-authored row landing in the wrong key.
    let weapons = lo.get("weapons").and_then(serde_json::Value::as_array);
    let weapon_at = |slot_index: i64, slot_type: &'static str| {
        weapons.and_then(|ws| {
            ws.iter().find(|w| {
                w.get("slotIndex").and_then(serde_json::Value::as_i64) == Some(slot_index)
                    && w.get("slotType").and_then(serde_json::Value::as_str) == Some(slot_type)
            })
        })
    };

    if let Some(primary) = weapon_at(0, "primary") {
        gear.primary = non_empty(primary.get("weapon"));
        // optic/magazine exist on the primary rifle alone — the other three slots have no
        // sub-slots in the editor, so nothing is being dropped by not reading them there.
        gear.optic = non_empty(primary.get("optic"));
        gear.magazine = non_empty(primary.get("magazine"));
    }
    gear.launcher = weapon_at(1, "primary").and_then(|w| non_empty(w.get("weapon")));
    gear.handgun = weapon_at(2, "secondary").and_then(|w| non_empty(w.get("weapon")));
    gear.throwable = weapon_at(3, "grenade").and_then(|w| non_empty(w.get("weapon")));

    let cargo: Vec<ModSlotCargo> = lo
        .get("cargo")
        .and_then(serde_json::Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|r| {
                    Some(ModSlotCargo {
                        container: non_empty(r.get("container"))?,
                        item: non_empty(r.get("item"))?,
                        qty: r
                            .get("qty")
                            .and_then(serde_json::Value::as_i64)
                            .filter(|q| *q >= 1)?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let gear = (!gear.is_empty()).then_some(gear);
    if gear.is_none() && cargo.is_empty() {
        return None;
    }
    Some(ModSlotLoadout { gear, cargo })
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

            // callsign → name → squad id → literal. The id rung keeps two unnamed
            // squads distinct so their derived slot ids stay unique.
            let callsign = if sq.callsign.is_empty() {
                or_fallback(or_fallback(&sq.name, &sq.id), CALLSIGN_FALLBACK).to_string()
            } else {
                sq.callsign.clone()
            };

            let mut role_counters: HashMap<&str, i64> = HashMap::new();
            let mut role_index: HashMap<&str, usize> = HashMap::new();
            let mut roles: Vec<ModOrbatRole> = Vec::new();

            for sl in &rows {
                let role = or_fallback(&sl.role, ROLE_FALLBACK);
                let occurrence = *role_counters.get(role).unwrap_or(&0);
                role_counters.insert(role, occurrence + 1);

                let kit = aliases
                    .kit_for_resource(&sl.asset_id)
                    .map_or_else(|| default_kit.to_string(), String::from);

                if let Some(&idx) = role_index.get(role) {
                    roles[idx].count += 1;
                } else {
                    role_index.insert(role, roles.len());
                    roles.push(ModOrbatRole {
                        slot: role.to_string(),
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
                    id: format!("{faction_key}:{callsign}:{role}:{occurrence}"),
                    uid: sl.id.clone(),
                    faction: faction_key.clone(),
                    group_callsign: callsign.clone(),
                    role: role.to_string(),
                    kit,
                    x,
                    z,
                    y,
                    heading_deg: normalize_heading(sl.position.rotation),
                    loadout: sl.loadout.as_ref().and_then(mod_slot_loadout),
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

    // `faction_eliminated` is only declared when at least two factions actually HOLD SLOTS. The
    // mod's validator rejects the document outright otherwise ("declares faction_eliminated but
    // only 1 faction(s) actually have slots — no second side can ever be eliminated"), and since
    // the editor never authors winConditions, an unconditional default made EVERY single-faction
    // mission unloadable with no way for the author to fix it. Counted over the FLATTENED SLOTS
    // rather than `factions`, because a faction can be declared with no seats — which is exactly
    // the case that triggered this (an operator's live mission declared opfor with zero slots).
    // Computed here rather than inline below because the struct literal moves `doc_slots`.
    let end_on = {
        let mut sides: Vec<&str> = doc_slots.iter().map(|s| s.faction.as_str()).collect();
        sides.sort_unstable();
        sides.dedup();
        let mut triggers = vec!["time_limit".to_string()];
        if sides.len() >= 2 {
            triggers.push("faction_eliminated".to_string());
        }
        triggers
    };

    let max_players = if mission.max_players < 1 {
        (doc_slots.len() as i64).max(1)
    } else {
        mission.max_players
    };

    let terrain = mission_terrain_key(&mission.terrain, &mission.custom_terrain_name);

    let meta = ModMeta {
        id: mission_doc_id(&mission.id),
        // maxLength is counted in characters, not bytes — truncate on a char boundary.
        name: if mission.title.is_empty() {
            "Untitled Mission".to_string()
        } else {
            mission.title.chars().take(META_NAME_MAX_CHARS).collect()
        },
        author: mission.author.clone(),
        terrain,
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
            // `faction_eliminated` is only declared when at least two factions actually HOLD
            // SLOTS. The mod's validator rejects the document outright otherwise ("declares
            // faction_eliminated but only 1 faction(s) actually have slots — no second side can
            // ever be eliminated"), and since the editor never authors winConditions, an
            // unconditional default made EVERY single-faction mission unloadable with no way for
            // the author to fix it. Counted over the flattened slots rather than `factions`,
            // because a faction can be declared with no seats — which is exactly the case that
            // triggered this.
            end_on,
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
          {"id": "s1", "squadId": "sq1", "index": 0, "role": "SL", "assetId": "{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et", "position": {"x": 4839.2, "y": 6620.8, "z": 0, "rotation": 270},
           "loadout": {"version": 2,
             "wear": {"headCover": "res://helmet", "jacket": "res://bdu_blouse", "vest": "res://chest_rig", "armoredVest": "res://pasgt", "pants": "res://bdu_pants", "boots": null},
             "weapons": [{"slotIndex": 0, "slotType": "primary", "weapon": "res://m16", "optic": "res://acog", "magazine": "res://stanag", "attachments": []},
                         {"slotIndex": 1, "slotType": "primary", "weapon": "res://m72", "attachments": []},
                         {"slotIndex": 2, "slotType": "secondary", "weapon": "res://m9", "attachments": []},
                         {"slotIndex": 3, "slotType": "grenade", "weapon": "res://m67", "attachments": []}],
             "cargo": [{"container": "vest", "item": "res://stanag", "qty": 4},
                       {"container": "pants", "item": "res://bandage", "qty": 2},
                       {"container": "", "item": "res://dropped", "qty": 1}]}},
          {"id": "s2", "squadId": "sq1", "index": 1, "role": "TL", "position": {"x": 4836.9, "y": 6626.5, "z": 142.5, "rotation": 450}},
          {"id": "s3", "squadId": "sq1", "index": 2, "role": "TL", "position": {"x": 4831.2, "y": 6628.8, "z": 0, "rotation": 0},
           "loadout": {"version": 2, "wear": {"jacket": ""}, "weapons": [], "cargo": []}},
          {"id": "s4", "squadId": "sq2", "index": 0, "role": "RFL", "assetId": "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et", "position": {"x": 6010, "y": 7211.5, "z": 0, "rotation": 90},
           "loadout": {"version": 2, "wear": {}, "weapons": [],
             "cargo": [{"container": "backpack", "item": "res://ak_mag", "qty": 40}]}}
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

        // B1 — uid carries the editor slot id verbatim (identity thread).
        let uids: Vec<&str> = doc.slots.iter().map(|s| s.uid.as_str()).collect();
        assert_eq!(uids, ["s1", "s2", "s3", "s4"]);

        // T-068.11/A3 — s1: full gear + cargo. armoredVest wins over vest; jacket→uniform;
        // headCover→helmet; pants copied (A3), null boots omitted; weapons[0] triple;
        // malformed cargo row (empty container) drops.
        let lo = doc.slots[0].loadout.as_ref().expect("s1 loadout");
        let g = lo.gear.as_ref().expect("s1 gear");
        assert_eq!(
            (
                g.primary.as_deref(),
                g.optic.as_deref(),
                g.magazine.as_deref(),
                g.uniform.as_deref(),
                g.vest.as_deref(),
                g.helmet.as_deref(),
                g.pants.as_deref(),
                g.boots.as_deref()
            ),
            (
                Some("res://m16"),
                Some("res://acog"),
                Some("res://stanag"),
                Some("res://bdu_blouse"),
                Some("res://pasgt"),
                Some("res://helmet"),
                Some("res://bdu_pants"),
                None
            )
        );
        // T-182 — the other three authored weapon slots reach the wire under the editor's own
        // key names. Asserted on the SERIALIZED document, not just the struct, because the whole
        // point of the ticket is what the game server is handed.
        assert_eq!(
            (
                g.launcher.as_deref(),
                g.handgun.as_deref(),
                g.throwable.as_deref()
            ),
            (Some("res://m72"), Some("res://m9"), Some("res://m67"))
        );
        assert_eq!(lo.cargo.len(), 2);
        assert_eq!(
            (lo.cargo[0].container.as_str(), lo.cargo[0].qty),
            ("vest", 4)
        );
        // s2 (no loadout) + s3 (all-empty loadout) omit the key entirely on the wire.
        assert!(doc.slots[1].loadout.is_none() && doc.slots[2].loadout.is_none());
        let wire = serde_json::to_value(&doc).unwrap();
        assert!(wire["slots"][1].get("loadout").is_none());
        assert!(wire["slots"][2].get("loadout").is_none());
        // s4: cargo-only loadout → gear key omitted, cargo verbatim (qty 40 preserved).
        let lo4 = doc.slots[3].loadout.as_ref().expect("s4 loadout");
        assert!(lo4.gear.is_none());
        assert_eq!(
            (lo4.cargo[0].item.as_str(), lo4.cargo[0].qty),
            ("res://ak_mag", 40)
        );
        assert!(wire["slots"][3]["loadout"].get("gear").is_none());
        assert_eq!(wire["slots"][3]["loadout"]["cargo"][0]["qty"], 40);

        // T-182 — the three new keys on the actual wire, spelled exactly as the Arsenal UI and
        // mission.schema.json spell them. A rename here is a silent contract break: the mod reads
        // this block by field NAME via JsonLoadContext, which ignores keys it does not recognise.
        let s1_gear = &wire["slots"][0]["loadout"]["gear"];
        assert_eq!(s1_gear["launcher"], "res://m72");
        assert_eq!(s1_gear["handgun"], "res://m9");
        assert_eq!(s1_gear["throwable"], "res://m67");
    }

    #[test]
    fn slot_loadout_mapper_edge_cases() {
        // vest falls back when armoredVest is absent/empty.
        let lo = serde_json::json!({"wear": {"vest": "res://rig", "armoredVest": ""}});
        let m = mod_slot_loadout(&lo).expect("gear");
        assert_eq!(m.gear.unwrap().vest.as_deref(), Some("res://rig"));
        // T-182 — INVERTED. This assertion used to read `is_none()`, pinning the bug: an RPG
        // authored at slotIndex 1 produced no loadout at all, which is precisely how the silent
        // discard survived a green test suite. A launcher is authored content and now stands on
        // its own — the whole loadout survives on the strength of it, even though the jacket is
        // an empty string and the cargo row is dropped for qty<1.
        let lo = serde_json::json!({
            "wear": {"jacket": ""},
            "weapons": [{"slotIndex": 1, "slotType": "primary", "weapon": "res://rpg"}],
            "cargo": [{"container": "vest", "item": "res://mag", "qty": 0}]
        });
        let m = mod_slot_loadout(&lo).expect("launcher-only loadout must survive");
        let g = m.gear.expect("launcher-only gear");
        assert_eq!(g.launcher.as_deref(), Some("res://rpg"));
        // It must NOT be mistaken for the rifle — that would be the same loss wearing a new name.
        assert!(g.primary.is_none() && g.handgun.is_none() && g.throwable.is_none());
        assert!(m.cargo.is_empty());

        // All four slots at once, each landing in its own key and none stealing another's.
        let lo = serde_json::json!({
            "weapons": [
                {"slotIndex": 0, "slotType": "primary",   "weapon": "res://m4", "optic": "res://acog", "magazine": "res://stanag"},
                {"slotIndex": 1, "slotType": "primary",   "weapon": "res://rpg"},
                {"slotIndex": 2, "slotType": "secondary", "weapon": "res://m9"},
                {"slotIndex": 3, "slotType": "grenade",   "weapon": "res://m67"}
            ]
        });
        let g = mod_slot_loadout(&lo)
            .expect("four weapons")
            .gear
            .expect("gear");
        assert_eq!(
            (
                g.primary.as_deref(),
                g.launcher.as_deref(),
                g.handgun.as_deref(),
                g.throwable.as_deref(),
                g.optic.as_deref(),
                g.magazine.as_deref()
            ),
            (
                Some("res://m4"),
                Some("res://rpg"),
                Some("res://m9"),
                Some("res://m67"),
                Some("res://acog"),
                Some("res://stanag")
            )
        );

        // The PAIR is the selector, not the index: a row at the right index with the wrong
        // slotType is not silently promoted into the key it half-matches.
        let lo = serde_json::json!({
            "weapons": [{"slotIndex": 2, "slotType": "primary", "weapon": "res://bogus"}]
        });
        assert!(mod_slot_loadout(&lo).is_none());

        // A weapon row with an empty ResourceName drops rather than emitting an empty string
        // (the schema's minLength: 1 would reject it at /compiled).
        let lo = serde_json::json!({
            "weapons": [{"slotIndex": 3, "slotType": "grenade", "weapon": ""}]
        });
        assert!(mod_slot_loadout(&lo).is_none());
        // Cargo-only survives without gear.
        let lo =
            serde_json::json!({"cargo": [{"container": "pants", "item": "res://b", "qty": 1}]});
        let m = mod_slot_loadout(&lo).expect("cargo-only");
        assert!(m.gear.is_none());
        assert_eq!(m.cargo.len(), 1);
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
