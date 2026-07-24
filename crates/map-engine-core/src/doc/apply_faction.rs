//! T-180.8 / B2 — MUTATE-apply a Faction Library doc onto one mission side.
//!
//! B2 rewrote the original REPLACE semantics (delete + recreate = the "foundation
//! keeps shifting" bug): overlapping roles now mutate the existing slots in place,
//! so slot ids — and the compiled `uid` identity thread — survive re-applies.
//!
//! Pure helper over [`MissionDocCore`] so H1–H4 / H9 run as native `cargo test --features doc`.

use serde_json::Value;

use super::MissionDocCore;

/// Everon map center — matches Leptos `INITIAL_TARGET` (placement pin for Apply).
pub const APPLY_ANCHOR_X: f64 = 6400.0;
pub const APPLY_ANCHOR_Y: f64 = 6400.0;

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
}

impl std::fmt::Display for ApplyFactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSide(s) => write!(f, "invalid side {s:?}; expected BLUFOR|OPFOR|INDFOR"),
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
/// Placement for NEW slots: `(6400 + 15*i, 6400)`; vehicles `(6400 + 30 + 20*j, 6400 - 30)`.
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
    ensure_side_faction(doc, side, &faction_id, &lib.name);

    let squad_name = if lib.name.trim().is_empty() {
        "Squad 1".to_string()
    } else {
        lib.name.clone()
    };

    // Reuse the side's FIRST squad (rename to the library); extra squads are surplus
    // structure and are removed (their slots with them).
    let existing_squads = faction_squad_ids(doc, &faction_id);
    let squad_id = match existing_squads.first() {
        Some(first) => {
            doc.rename_squad(first, &squad_name);
            for sid in existing_squads.iter().skip(1) {
                doc.remove_squad(sid);
            }
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
        let x = APPLY_ANCHOR_X + 15.0 * i as f64;
        let y = APPLY_ANCHOR_Y;
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
        let x = APPLY_ANCHOR_X + 30.0 + 20.0 * j as f64;
        let y = APPLY_ANCHOR_Y - 30.0;
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

fn faction_squad_ids(doc: &MissionDocCore, faction_id: &str) -> Vec<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return Vec::new();
    };
    root.get("factionsById")
        .and_then(|v| v.get(faction_id))
        .and_then(|f| f.get("squadIds"))
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
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
}
