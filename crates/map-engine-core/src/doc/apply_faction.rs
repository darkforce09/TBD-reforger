//! T-180.8 / B2 — MUTATE-apply a Faction Library doc onto one mission side.
//!
//! B2 rewrote the original REPLACE semantics (delete + recreate = the "foundation
//! keeps shifting" bug): overlapping roles now mutate the existing slots in place,
//! so slot ids — and the compiled `uid` identity thread — survive re-applies.
//!
//! T-217 closed two silent-data-loss holes in that design: Apply used to delete every squad on the
//! side past the first (a `FactionLibraryInput` is flat, so it cannot put them back — see
//! [`ApplyFactionError::WouldCollapseSquads`]), and it pinned every placement to the Everon centre
//! regardless of the doc's actual terrain.
//!
//! Pure helper over [`MissionDocCore`] so H1–H4 / H9 run as native `cargo test --features doc`.

use serde_json::Value;

use super::MissionDocCore;

/// Everon map centre (`12800² / 2`) — matches Leptos `INITIAL_TARGET`, and the Apply anchor for
/// every terrain whose bounds are Everon's (`everon`, `custom`, anything unknown).
///
/// **T-217: this is the default, not "the" anchor.** It read as correct only because Everon is the
/// default terrain. Arland is `4096²`, so on an Arland mission `(6400, 6400)` is off the map
/// entirely and an Apply dumped the whole faction outside the world. Resolve the real pin with
/// [`apply_anchor_xy`], which falls back to this constant. Still `pub` because `editor_ops` uses it
/// as the fallback for a squad with no live slot to anchor against.
pub const APPLY_ANCHOR_X: f64 = 6400.0;
pub const APPLY_ANCHOR_Y: f64 = 6400.0;

/// Arland map centre (`4096² / 2`).
const ARLAND_ANCHOR_X: f64 = 2048.0;
const ARLAND_ANCHOR_Y: f64 = 2048.0;

/// Metres between adjacent slots in an applied squad's row. Same constant as `editor_ops`'
/// `ORBAT_SLOT_SPACING_X` (T-188 `next_slot_xy`), which is why a hand-built squad and an applied
/// one line up; only the origin the row starts from is terrain-dependent.
const SLOT_SPACING_X: f64 = 15.0;

/// Where Apply pins a faction on `terrain`: the centre of that terrain's world bounds.
///
/// Mirrors the centre of `mission::compile::terrain_bounds` — deliberately re-stated rather than
/// called, because `mission` and `doc` are independent cargo features and `--features doc` alone
/// must still compile. `apply_anchor_matches_terrain_bounds` (built only when both features are on)
/// pins the two together so this copy cannot drift.
fn apply_anchor_for_terrain(terrain: &str) -> (f64, f64) {
    match terrain {
        "arland" => (ARLAND_ANCHOR_X, ARLAND_ANCHOR_Y),
        // everon + custom + anything unknown → 12800², the same fallback `terrain_bounds` takes.
        _ => (APPLY_ANCHOR_X, APPLY_ANCHOR_Y),
    }
}

/// This doc's Apply anchor, read from `meta.terrain`. Absent / unreadable → `everon`, which is what
/// `seed_meta` writes and what `compile_payload` assumes, so a doc that predates any terrain choice
/// keeps the historical `(6400, 6400)` exactly.
fn apply_anchor_xy(doc: &MissionDocCore) -> (f64, f64) {
    let terrain = serde_json::from_str::<Value>(&doc.small_maps_json())
        .ok()
        .and_then(|root| {
            root.get("meta")
                .and_then(|m| m.get("terrain"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "everon".to_string());
    apply_anchor_for_terrain(&terrain)
}

const VALID_SIDES: &[&str] = &["BLUFOR", "OPFOR", "INDFOR"];

/// One role row from `faction-library.schema.json` (serde-owned; no FE types).
#[derive(Debug, Clone)]
pub struct FactionLibraryRole {
    pub role: String,
    pub tag: Option<String>,
    pub character: String,
    pub loadout: Option<Value>,
}

/// One vehicle row from the library pool.
#[derive(Debug, Clone)]
pub struct FactionLibraryVehicle {
    pub vehicle: String,
    pub label: Option<String>,
}

/// Library payload for [`apply_faction_library`] (name + roles + vehicles).
#[derive(Debug, Clone)]
pub struct FactionLibraryInput {
    pub name: String,
    pub roles: Vec<FactionLibraryRole>,
    pub vehicles: Vec<FactionLibraryVehicle>,
}

/// Result of a successful Apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyFactionResult {
    pub faction_id: String,
    pub squad_id: String,
    pub leader_slot_id: String,
    pub roles_applied: usize,
    pub vehicles_applied: usize,
}

/// Error from [`apply_faction_library`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyFactionError {
    /// `side` was not BLUFOR / OPFOR / INDFOR.
    InvalidSide(String),
    /// T-217 — the side holds more than one squad, and [`FactionLibraryInput`] has no squad level
    /// to hold them. Apply is refused **before it writes anything**.
    ///
    /// A faction library doc is `roles[] + vehicles[]`. Squad boundaries, `leaderSlotId`, callsign,
    /// rank and map positions are not in it, so Save-as-template never captured them and Apply
    /// could not restore them: the old code kept `squadIds[0]`, `remove_squad`'d the rest — which
    /// takes each squad's slots with it — and reported success. That is unrecoverable loss of
    /// operator work, so a multi-squad side is now refused instead. Expressing it properly needs a
    /// squad level in `faction-library.schema.json` + `FactionDoc`, which is a schema change and
    /// out of this fix's reach.
    WouldCollapseSquads {
        /// The side that was targeted.
        side: String,
        /// Every live squad on the side, in `faction.squadIds` order. `[0]` is the one Apply would
        /// have kept; `[1..]` are the ones it would have deleted.
        squad_ids: Vec<String>,
        /// Slots inside `squad_ids[1..]` — the bodies that would have gone with them.
        slots_destroyed: usize,
    },
}

impl std::fmt::Display for ApplyFactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSide(s) => write!(f, "invalid side {s:?}; expected BLUFOR|OPFOR|INDFOR"),
            Self::WouldCollapseSquads {
                side,
                squad_ids,
                slots_destroyed,
            } => write!(
                f,
                "refusing to apply a template onto {side}: it has {} squads, and a faction \
                 template is a flat role list with no squad level — applying would keep {} and \
                 permanently delete the other {}, along with {slots_destroyed} slot(s) and their \
                 leaders, callsigns, ranks and map positions. Nothing was changed. Merge the side \
                 down to one squad first, or apply onto a side that has none.",
                squad_ids.len(),
                squad_ids.first().map_or("no squad", String::as_str),
                squad_ids.len().saturating_sub(1),
            ),
        }
    }
}

impl std::error::Error for ApplyFactionError {}

/// MUTATE-apply a Faction Library doc onto one mission side (B2).
///
/// Slot identity is the contract: overlapping roles are written **onto the side's
/// existing slots in place** (role/tag/character/loadout; operator-moved positions
/// survive), only surplus roles mint new slots, only surplus slots are removed.
/// A re-apply therefore keeps every overlapping slot id — and with it the compiled
/// `uid` every spawn point / roster reference keys on. Vehicles stay
/// replace-semantics (no downstream identity hangs off them).
///
/// **Squads are never destroyed (T-217).** A side holding more than one squad is refused with
/// [`ApplyFactionError::WouldCollapseSquads`] before the first write, because `lib` is flat and
/// cannot carry them back. Apply targets a side with zero squads (mint one) or exactly one
/// (mutate it) — one squad in, one squad out.
///
/// Placement for NEW slots: `(anchor.x + 15*i, anchor.y)`; vehicles
/// `(anchor.x + 30 + 20*j, anchor.y - 30)`, where `anchor` is [`apply_anchor_xy`] — the centre of
/// the doc's own terrain, not a hard-coded Everon centre.
pub fn apply_faction_library(
    doc: &MissionDocCore,
    side: &str,
    layer_id: &str,
    lib: &FactionLibraryInput,
) -> Result<ApplyFactionResult, ApplyFactionError> {
    if !VALID_SIDES.contains(&side) {
        return Err(ApplyFactionError::InvalidSide(side.to_string()));
    }

    let faction_id = format!("faction-{side}");

    // T-217 — refuse BEFORE the first write. Everything below this point mutates the doc,
    // `ensure_side_faction` included, so the guard sits above all of it: a refused Apply leaves
    // the document exactly as it found it, with nothing half-applied to undo.
    let existing_squads = faction_squad_ids(doc, &faction_id);
    if existing_squads.len() > 1 {
        let slots_destroyed: usize = existing_squads
            .iter()
            .skip(1)
            .map(|sid| squad_slot_ids(doc, sid).len())
            .sum();
        return Err(ApplyFactionError::WouldCollapseSquads {
            side: side.to_string(),
            squad_ids: existing_squads,
            slots_destroyed,
        });
    }

    ensure_side_faction(doc, side, &faction_id, &lib.name);
    let (anchor_x, anchor_y) = apply_anchor_xy(doc);

    let squad_name = if lib.name.trim().is_empty() {
        "Squad 1".to_string()
    } else {
        lib.name.clone()
    };

    // Reuse the side's one squad (renamed to the library), or mint one if it has none. The guard
    // above already proved there is no second squad to lose.
    let squad_id = match existing_squads.first() {
        Some(first) => {
            doc.rename_squad(first, &squad_name);
            first.clone()
        }
        None => {
            let id = mint_squad_id(doc, side);
            doc.add_squad(&id, &faction_id, &squad_name, None);
            id
        }
    };

    // Existing slots in squad order = the mutate targets.
    let existing_slots = squad_slot_ids(doc, &squad_id);

    let mut slot_ids: Vec<String> = Vec::with_capacity(lib.roles.len());
    for (i, role) in lib.roles.iter().enumerate() {
        if let Some(slot_id) = existing_slots.get(i) {
            // Overlap: mutate in place — id survives, position survives.
            doc.update_slot_role_character(
                slot_id,
                &role.role,
                role.tag.clone(),
                Some(role.character.clone()).filter(|s| !s.is_empty()),
            );
            doc.update_slot_loadout(slot_id, role.loadout.as_ref().map(ToString::to_string));
            slot_ids.push(slot_id.clone());
            continue;
        }

        // Surplus role: mint a fresh slot (deterministic name when free).
        let slot_id = mint_slot_id(doc, side, i);
        let x = anchor_x + SLOT_SPACING_X * i as f64;
        let y = anchor_y;
        doc.add_slot(
            &slot_id,
            &squad_id,
            layer_id,
            i as u32,
            &role.role,
            role.tag.clone(),
            Some(role.character.clone()).filter(|s| !s.is_empty()),
            x,
            y,
            0.0,
            0.0,
        );
        if let Some(lo) = &role.loadout {
            doc.update_slot_loadout(&slot_id, Some(lo.to_string()));
        }
        slot_ids.push(slot_id);
    }

    // Surplus existing slots (beyond the library's role count) are removed.
    if existing_slots.len() > lib.roles.len() {
        doc.remove_slots(existing_slots[lib.roles.len()..].to_vec());
    }

    let leader_idx = lib
        .roles
        .iter()
        .position(|r| is_squad_leader_role(&r.role))
        .unwrap_or(0);
    let leader_slot_id = slot_ids.get(leader_idx).cloned().unwrap_or_default();
    if !leader_slot_id.is_empty() {
        doc.set_leader(&squad_id, &leader_slot_id);
    }

    // Vehicles: replace semantics (delete the squad's old rows, add the library's).
    for vid in squad_vehicle_ids(doc, &squad_id) {
        doc.remove_vehicle(&vid);
    }
    let mut vehicles_applied = 0usize;
    for (j, v) in lib.vehicles.iter().enumerate() {
        if v.vehicle.trim().is_empty() {
            continue;
        }
        let vid = mint_vehicle_id(doc, side, j);
        let x = anchor_x + 30.0 + 20.0 * j as f64;
        let y = anchor_y - 30.0;
        let _ = v.label; // label is UI-only; resourceName is the graph pin
        doc.add_vehicle(&vid, &v.vehicle, Some(x), Some(y), Some(0.0), Some(0.0));
        doc.attach_vehicle(&squad_id, &vid);
        vehicles_applied += 1;
    }

    Ok(ApplyFactionResult {
        faction_id,
        squad_id,
        leader_slot_id,
        roles_applied: slot_ids.len(),
        vehicles_applied,
    })
}

fn is_squad_leader_role(role: &str) -> bool {
    role.to_ascii_lowercase().contains("squad leader")
}

fn ensure_side_faction(doc: &MissionDocCore, side: &str, faction_id: &str, name: &str) {
    if faction_exists(doc, faction_id) {
        doc.set_faction_name(faction_id, name);
        return;
    }
    let display = if name.trim().is_empty() { side } else { name };
    doc.add_faction(faction_id, side, display);
}

fn faction_exists(doc: &MissionDocCore, faction_id: &str) -> bool {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return false;
    };
    root.get("factionsById")
        .and_then(|v| v.as_object())
        .is_some_and(|m| m.contains_key(faction_id))
}

/// Squad ids of a faction in `faction.squadIds` order, **filtered to squads that still exist in
/// `squadsById`**. A stale id (squad removed, faction row not yet cleaned) is not a squad: it must
/// not count toward the T-217 multi-squad refusal, and it must not be picked as the mutate target
/// either — `rename_squad`/`add_slot` against a ghost id silently do nothing.
fn faction_squad_ids(doc: &MissionDocCore, faction_id: &str) -> Vec<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return Vec::new();
    };
    let live = root.get("squadsById").and_then(Value::as_object);
    root.get("factionsById")
        .and_then(|v| v.get(faction_id))
        .and_then(|f| f.get("squadIds"))
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .filter(|id| live.is_some_and(|m| m.contains_key(*id)))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Slot ids of a squad, in `slotIds` order (the mutate targets).
fn squad_slot_ids(doc: &MissionDocCore, squad_id: &str) -> Vec<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return Vec::new();
    };
    root.get("squadsById")
        .and_then(|m| m.get(squad_id))
        .and_then(|sq| sq.get("slotIds"))
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Vehicle ids attached to a squad (replace-semantics targets).
fn squad_vehicle_ids(doc: &MissionDocCore, squad_id: &str) -> Vec<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return Vec::new();
    };
    root.get("squadsById")
        .and_then(|m| m.get(squad_id))
        .and_then(|sq| sq.get("vehicleIds"))
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Deterministic `slot-{side}-apply-{i}` when free, suffixed until doc-unique
/// (an operator-placed slot could occupy the deterministic name).
fn mint_slot_id(doc: &MissionDocCore, side: &str, i: usize) -> String {
    let existing = existing_slot_ids(doc);
    let base = format!("slot-{side}-apply-{i}");
    if !existing.contains(&base) {
        return base;
    }
    let mut n: u32 = 2;
    loop {
        let id = format!("{base}-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n = n.saturating_add(1);
    }
}

fn mint_vehicle_id(doc: &MissionDocCore, side: &str, j: usize) -> String {
    let existing = existing_vehicle_ids(doc);
    let base = format!("veh-{side}-apply-{j}");
    if !existing.contains(&base) {
        return base;
    }
    let mut n: u32 = 2;
    loop {
        let id = format!("{base}-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n = n.saturating_add(1);
    }
}

fn existing_slot_ids(doc: &MissionDocCore) -> std::collections::HashSet<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.slots_json()) else {
        return std::collections::HashSet::new();
    };
    root.as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

fn existing_vehicle_ids(doc: &MissionDocCore) -> std::collections::HashSet<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return std::collections::HashSet::new();
    };
    root.get("vehiclesById")
        .and_then(|v| v.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

fn mint_squad_id(doc: &MissionDocCore, side: &str) -> String {
    let existing = existing_squad_ids(doc);
    let mut n: u32 = 1;
    loop {
        let id = format!("squad-{side}-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n = n.saturating_add(1);
    }
}

fn existing_squad_ids(doc: &MissionDocCore) -> std::collections::HashSet<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return std::collections::HashSet::new();
    };
    root.get("squadsById")
        .and_then(|v| v.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn side_slot_count(doc: &MissionDocCore, side: &str) -> usize {
        let faction_id = format!("faction-{side}");
        let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
            return 0;
        };
        let squad_ids = root
            .get("factionsById")
            .and_then(|v| v.get(&faction_id))
            .and_then(|f| f.get("squadIds"))
            .and_then(|a| a.as_array())
            .cloned()
            .unwrap_or_default();
        let mut n = 0usize;
        for sid in squad_ids {
            let Some(sid) = sid.as_str() else {
                continue;
            };
            if let Some(arr) = root
                .get("squadsById")
                .and_then(|m| m.get(sid))
                .and_then(|sq| sq.get("slotIds"))
                .and_then(|a| a.as_array())
            {
                n += arr.len();
            }
        }
        n
    }

    fn layer(doc: &MissionDocCore) {
        doc.add_editor_layer("lyr", "Layer 1", None);
    }

    fn small(doc: &MissionDocCore) -> Value {
        serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json")
    }

    fn slots(doc: &MissionDocCore) -> Value {
        serde_json::from_str(&doc.slots_json()).expect("slots_json")
    }

    fn two_role_lib() -> FactionLibraryInput {
        FactionLibraryInput {
            name: "Soviet Army 1980s".into(),
            roles: vec![
                FactionLibraryRole {
                    role: "Squad Leader".into(),
                    tag: None,
                    character: "{AAAA}Char.et".into(),
                    loadout: Some(json!({
                        "version": 2,
                        "wear": {},
                        "weapons": [],
                        "summary": "AK-74"
                    })),
                },
                FactionLibraryRole {
                    role: "Rifleman".into(),
                    tag: Some("AT".into()),
                    character: "{BBBB}Rifleman.et".into(),
                    loadout: None,
                },
            ],
            vehicles: vec![FactionLibraryVehicle {
                vehicle: "{CCCC}UAZ.et".into(),
                label: Some("UAZ-469".into()),
            }],
        }
    }

    /// H1 — Apply with R roles ⇒ exactly R slots under side; squad count ≥ 1.
    #[test]
    fn apply_faction_library_counts() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let lib = two_role_lib();
        let r = apply_faction_library(&doc, "OPFOR", "lyr", &lib).expect("apply");
        assert_eq!(r.roles_applied, 2);
        assert_eq!(side_slot_count(&doc, "OPFOR"), 2);
        let root = small(&doc);
        let squad_ids = root["factionsById"]["faction-OPFOR"]["squadIds"]
            .as_array()
            .expect("squadIds");
        assert!(!squad_ids.is_empty());
        assert_eq!(squad_ids.len(), 1);
        assert_eq!(root["squadsById"][&r.squad_id]["name"], "Soviet Army 1980s");
    }

    /// H2 — leaderSlotId = first /Squad Leader/i role, else index 0.
    #[test]
    fn apply_faction_sets_leader() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let mut lib = two_role_lib();
        // Put SL second — must still become leader.
        lib.roles.swap(0, 1);
        let r = apply_faction_library(&doc, "BLUFOR", "lyr", &lib).expect("apply");
        let root = small(&doc);
        assert_eq!(
            root["squadsById"][&r.squad_id]["leaderSlotId"],
            "slot-BLUFOR-apply-1"
        );
        assert_eq!(r.leader_slot_id, "slot-BLUFOR-apply-1");

        // No SL name → first slot.
        let lib2 = FactionLibraryInput {
            name: "Alpha".into(),
            roles: vec![
                FactionLibraryRole {
                    role: "Rifleman".into(),
                    tag: None,
                    character: "c1".into(),
                    loadout: None,
                },
                FactionLibraryRole {
                    role: "Medic".into(),
                    tag: None,
                    character: "c2".into(),
                    loadout: None,
                },
            ],
            vehicles: vec![],
        };
        let r2 = apply_faction_library(&doc, "INDFOR", "lyr", &lib2).expect("apply2");
        assert_eq!(r2.leader_slot_id, "slot-INDFOR-apply-0");
    }

    /// H3 — loadout JSON copied onto slot when present.
    #[test]
    fn apply_faction_copies_loadout() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let lib = two_role_lib();
        let r = apply_faction_library(&doc, "OPFOR", "lyr", &lib).expect("apply");
        let s = slots(&doc);
        let lo = &s["slot-OPFOR-apply-0"]["loadout"];
        assert_eq!(lo["summary"], "AK-74");
        assert_eq!(lo["version"], 2);
        // Rifleman had no loadout.
        assert!(s["slot-OPFOR-apply-1"].get("loadout").is_none());
        assert_eq!(r.roles_applied, 2);
    }

    /// H4 — Apply with V vehicles ⇒ vehicleIds.len()==V.
    #[test]
    fn apply_faction_vehicles() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let lib = two_role_lib();
        let r = apply_faction_library(&doc, "OPFOR", "lyr", &lib).expect("apply");
        assert_eq!(r.vehicles_applied, 1);
        let root = small(&doc);
        let vids = root["squadsById"][&r.squad_id]["vehicleIds"]
            .as_array()
            .expect("vehicleIds");
        assert_eq!(vids.len(), 1);
        assert!(root["vehiclesById"].get("veh-OPFOR-apply-0").is_some());
        assert_eq!(
            root["vehiclesById"]["veh-OPFOR-apply-0"]["resourceName"],
            "{CCCC}UAZ.et"
        );
        assert!(
            root["vehiclesById"]["veh-OPFOR-apply-0"]
                .get("position")
                .is_some()
        );
        let xy = doc.vehicle_xy_flat();
        assert_eq!(xy.len(), 2);
    }

    /// B2 headline — a re-apply MUTATES: overlapping slot ids AND operator-moved
    /// positions survive; only surplus roles mint, only surplus slots vanish.
    #[test]
    fn reapply_keeps_overlapping_slot_ids_and_positions() {
        let doc = MissionDocCore::new();
        layer(&doc);
        apply_faction_library(&doc, "BLUFOR", "lyr", &two_role_lib()).expect("first");

        // Operator moves the first slot — the mutate contract must preserve it.
        doc.set_slot_position("slot-BLUFOR-apply-0", 111.0, 222.0, 0.0, 0.0);

        // Edited library: renamed roles + a third role.
        let lib2 = FactionLibraryInput {
            name: "Soviet Army 1980s v2".into(),
            roles: vec![
                FactionLibraryRole {
                    role: "Platoon Leader".into(),
                    tag: None,
                    character: "{DDDD}PL.et".into(),
                    loadout: None,
                },
                FactionLibraryRole {
                    role: "Squad Leader".into(),
                    tag: None,
                    character: "{AAAA}Char.et".into(),
                    loadout: None,
                },
                FactionLibraryRole {
                    role: "Machinegunner".into(),
                    tag: Some("MG".into()),
                    character: "{EEEE}MG.et".into(),
                    loadout: None,
                },
            ],
            vehicles: vec![],
        };
        let r2 = apply_faction_library(&doc, "BLUFOR", "lyr", &lib2).expect("second");
        assert_eq!(r2.roles_applied, 3);
        assert_eq!(side_slot_count(&doc, "BLUFOR"), 3);

        let s = slots(&doc);
        // Overlaps kept their ids; the surplus role minted the next deterministic id.
        assert_eq!(s["slot-BLUFOR-apply-0"]["role"], "Platoon Leader");
        assert_eq!(s["slot-BLUFOR-apply-1"]["role"], "Squad Leader");
        assert_eq!(s["slot-BLUFOR-apply-2"]["role"], "Machinegunner");
        // Operator-moved position survived the re-apply.
        assert_eq!(s["slot-BLUFOR-apply-0"]["position"]["x"], 111.0);
        assert_eq!(s["slot-BLUFOR-apply-0"]["position"]["y"], 222.0);
        // The first apply's SL loadout was cleared by the loadout-less v2 row
        // (library rows are authoritative on Apply).
        assert!(s["slot-BLUFOR-apply-0"].get("loadout").is_none());
        // Leader recomputed onto the SL role's surviving slot.
        assert_eq!(r2.leader_slot_id, "slot-BLUFOR-apply-1");
    }

    /// H9 — second Apply converges the side to the new library (slot count becomes
    /// new R, not R_old+R_new; surplus slots + old vehicles removed).
    #[test]
    fn apply_faction_replace_not_merge() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let lib2 = two_role_lib();
        apply_faction_library(&doc, "BLUFOR", "lyr", &lib2).expect("first");
        assert_eq!(side_slot_count(&doc, "BLUFOR"), 2);

        let lib1 = FactionLibraryInput {
            name: "Solo".into(),
            roles: vec![FactionLibraryRole {
                role: "Rifleman".into(),
                tag: None,
                character: "c".into(),
                loadout: None,
            }],
            vehicles: vec![],
        };
        apply_faction_library(&doc, "BLUFOR", "lyr", &lib1).expect("second");
        assert_eq!(side_slot_count(&doc, "BLUFOR"), 1);
        let root = small(&doc);
        assert_eq!(
            root["factionsById"]["faction-BLUFOR"]["squadIds"]
                .as_array()
                .map(|a| a.len())
                .unwrap_or(0),
            1
        );
        // Old vehicles gone.
        assert!(
            root["vehiclesById"]
                .as_object()
                .map(|m| m.is_empty())
                .unwrap_or(true)
        );
    }

    #[test]
    fn apply_rejects_civ() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let err = apply_faction_library(&doc, "CIV", "lyr", &two_role_lib()).expect_err("civ");
        assert!(matches!(err, ApplyFactionError::InvalidSide(_)));
    }

    // ── T-217 — squad collapse + terrain anchor ─────────────────────────────────────────────────

    /// Hand-build `n` squads under `side`, each with `slots_per` slots, a leader, and per-slot
    /// callsign/rank — i.e. exactly the structure a `FactionLibraryInput` cannot express.
    fn seed_squads(doc: &MissionDocCore, side: &str, n: usize, slots_per: usize) -> Vec<String> {
        let faction_id = format!("faction-{side}");
        doc.add_faction(&faction_id, side, side);
        let mut ids = Vec::with_capacity(n);
        for s in 0..n {
            let sq = format!("squad-{side}-{s}");
            doc.add_squad(&sq, &faction_id, &format!("Squad {s}"), None);
            for k in 0..slots_per {
                let slot = format!("slot-{side}-{s}-{k}");
                doc.add_slot(
                    &slot,
                    &sq,
                    "lyr",
                    k as u32,
                    "Rifleman",
                    None,
                    Some("{FFFF}Body.et".to_string()),
                    1000.0 + 100.0 * s as f64,
                    2000.0 + 15.0 * k as f64,
                    0.0,
                    0.0,
                );
                doc.update_slot_identity(
                    &slot,
                    Some(format!("A{s}-{k}")),
                    Some("Corporal".to_string()),
                );
                if k == 0 {
                    doc.set_leader(&sq, &slot);
                }
            }
            ids.push(sq);
        }
        ids
    }

    /// T-217 headline — **N squads in, N squads out.** Apply onto a multi-squad side is refused,
    /// and the refusal is total: not one byte of the doc moves, so every squad, slot, leader,
    /// callsign, rank and position is exactly where the operator left it.
    ///
    /// The old code kept `squadIds[0]` and `remove_squad`'d the rest — which deletes their slots
    /// too — then returned `Ok`, so the UI said "Template applied." while two squads' worth of
    /// authoring vanished with no way back (the flat template never held them either).
    #[test]
    fn apply_refuses_to_collapse_squads_and_writes_nothing() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let seeded = seed_squads(&doc, "OPFOR", 3, 2);
        let small_before = small(&doc);
        let slots_before = slots(&doc);
        assert_eq!(side_slot_count(&doc, "OPFOR"), 6);

        let err = apply_faction_library(&doc, "OPFOR", "lyr", &two_role_lib()).expect_err("refuse");
        match &err {
            ApplyFactionError::WouldCollapseSquads {
                side,
                squad_ids,
                slots_destroyed,
            } => {
                assert_eq!(side, "OPFOR");
                assert_eq!(squad_ids, &seeded);
                // squads 1 and 2, two slots each — the bodies the old path took with them.
                assert_eq!(*slots_destroyed, 4);
            }
            other => panic!("expected WouldCollapseSquads, got {other:?}"),
        }

        // "…and say so": the message names the side, the count and the fact nothing changed.
        let msg = err.to_string();
        assert!(msg.contains("OPFOR"), "{msg}");
        assert!(msg.contains("3 squads"), "{msg}");
        assert!(msg.contains("Nothing was changed"), "{msg}");

        // N in, N out — the whole document is untouched.
        assert_eq!(small(&doc), small_before);
        assert_eq!(slots(&doc), slots_before);
        assert_eq!(faction_squad_ids(&doc, "faction-OPFOR").len(), 3);
        assert_eq!(side_slot_count(&doc, "OPFOR"), 6);

        // Spelled out for the things the template could never have restored.
        let s = slots(&doc);
        let root = small(&doc);
        assert_eq!(root["squadsById"]["squad-OPFOR-2"]["name"], "Squad 2");
        assert_eq!(
            root["squadsById"]["squad-OPFOR-2"]["leaderSlotId"],
            "slot-OPFOR-2-0"
        );
        assert_eq!(s["slot-OPFOR-2-1"]["callsign"], "A2-1");
        assert_eq!(s["slot-OPFOR-2-1"]["rank"], "Corporal");
        assert_eq!(s["slot-OPFOR-2-1"]["position"]["x"], 1200.0);
    }

    /// One squad is still the happy path — the refusal must not turn a normal Apply into an error.
    #[test]
    fn apply_onto_a_single_squad_side_still_applies() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let seeded = seed_squads(&doc, "BLUFOR", 1, 3);
        let r = apply_faction_library(&doc, "BLUFOR", "lyr", &two_role_lib()).expect("apply");
        assert_eq!(r.squad_id, seeded[0]);
        assert_eq!(r.roles_applied, 2);
        assert_eq!(side_slot_count(&doc, "BLUFOR"), 2);
        assert_eq!(faction_squad_ids(&doc, "faction-BLUFOR").len(), 1);
    }

    /// A stale id in `faction.squadIds` is not a squad. It is reachable — `hydrate` loads faction
    /// and squad rows verbatim with no cross-reference check — and it must neither trip the T-217
    /// refusal nor be chosen as the mutate target: writes against a ghost id are silent no-ops, so
    /// an Apply that "targeted" one would apply nothing and still report success.
    #[test]
    fn stale_squad_id_neither_refuses_nor_captures_the_apply() {
        let doc = MissionDocCore::new();
        doc.hydrate(
            &json!({
                "editor": {
                    "factions": [{
                        "id": "faction-INDFOR",
                        "key": "INDFOR",
                        "name": "INDFOR",
                        // `squad-ghost` has no row in `squads` below.
                        "squadIds": ["squad-ghost", "squad-real"],
                    }],
                    "squads": [{
                        "id": "squad-real",
                        "factionId": "faction-INDFOR",
                        "name": "Real",
                        "slotIds": [],
                        "vehicleIds": [],
                    }],
                    "editorLayers": [{
                        "id": "lyr",
                        "name": "Layer 1",
                        "parentId": null,
                        "entityIds": [],
                    }],
                }
            })
            .to_string(),
            "lyr",
        );
        assert_eq!(
            faction_squad_ids(&doc, "faction-INDFOR"),
            vec!["squad-real"]
        );

        let r = apply_faction_library(&doc, "INDFOR", "lyr", &two_role_lib()).expect("apply");
        assert_eq!(r.squad_id, "squad-real");
        assert_eq!(side_slot_count(&doc, "INDFOR"), 2);
    }

    /// T-217 — the Apply anchor follows `meta.terrain`. Arland is `4096²`, so the Everon constant
    /// put every applied slot and vehicle thousands of metres off the map.
    #[test]
    fn apply_anchors_on_the_docs_own_terrain() {
        let doc = MissionDocCore::new();
        layer(&doc);
        doc.apply_row_meta("", "arland", None, None);
        apply_faction_library(&doc, "OPFOR", "lyr", &two_role_lib()).expect("apply");

        let s = slots(&doc);
        assert_eq!(s["slot-OPFOR-apply-0"]["position"]["x"], 2048.0);
        assert_eq!(s["slot-OPFOR-apply-0"]["position"]["y"], 2048.0);
        // Same 15 m lane as `next_slot_xy` / a hand-built squad — only the origin changed.
        assert_eq!(s["slot-OPFOR-apply-1"]["position"]["x"], 2063.0);

        let root = small(&doc);
        let v = &root["vehiclesById"]["veh-OPFOR-apply-0"]["position"];
        assert_eq!(v["x"], 2078.0);
        assert_eq!(v["y"], 2018.0);
    }

    /// Regression pin for the terrain that was already right: everon / custom / no-meta must still
    /// land on the historical `(6400, 6400)`, byte-for-byte as before T-217.
    #[test]
    fn everon_and_unknown_terrain_keep_the_historical_anchor() {
        for terrain in ["", "everon", "custom"] {
            let doc = MissionDocCore::new();
            layer(&doc);
            if !terrain.is_empty() {
                doc.apply_row_meta("", terrain, None, None);
            }
            apply_faction_library(&doc, "BLUFOR", "lyr", &two_role_lib()).expect("apply");
            let s = slots(&doc);
            assert_eq!(
                s["slot-BLUFOR-apply-0"]["position"]["x"], 6400.0,
                "terrain {terrain:?}"
            );
            assert_eq!(
                s["slot-BLUFOR-apply-1"]["position"]["x"], 6415.0,
                "terrain {terrain:?}"
            );
            assert_eq!(
                s["slot-BLUFOR-apply-0"]["position"]["y"], 6400.0,
                "terrain {terrain:?}"
            );
        }
    }

    /// The local terrain table must stay the centre of `mission::compile::terrain_bounds`. Only
    /// built with `--features doc,mission`; `doc` alone has no `mission` module to compare against.
    #[cfg(feature = "mission")]
    #[test]
    fn apply_anchor_matches_terrain_bounds() {
        for t in ["everon", "arland", "custom", "not-a-terrain"] {
            let [min_x, min_y, max_x, max_y] = crate::mission::compile::terrain_bounds(t);
            let want = ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
            assert_eq!(apply_anchor_for_terrain(t), want, "terrain {t:?}");
        }
    }
}
