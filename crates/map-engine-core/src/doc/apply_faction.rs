//! T-180.8 — REPLACE-materialize a Faction Library doc onto one mission side.
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

/// REPLACE all squads/slots/vehicles under `side` with one squad materialised from `lib`.
///
/// Placement: slots at `(6400 + 15*i, 6400)`; vehicles at `(6400 + 30 + 20*j, 6400 - 30)`.
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

    // Collect squad ids first — `remove_squad` mutates `squadIds` while we iterate.
    let squad_ids = faction_squad_ids(doc, &faction_id);
    for sid in &squad_ids {
        doc.remove_squad(sid);
    }

    let squad_id = mint_squad_id(doc, side);
    let squad_name = if lib.name.trim().is_empty() {
        "Squad 1".to_string()
    } else {
        lib.name.clone()
    };
    doc.add_squad(&squad_id, &faction_id, &squad_name, None);

    let mut slot_ids: Vec<String> = Vec::with_capacity(lib.roles.len());
    for (i, role) in lib.roles.iter().enumerate() {
        let slot_id = format!("slot-{side}-apply-{i}");
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

    let leader_idx = lib
        .roles
        .iter()
        .position(|r| is_squad_leader_role(&r.role))
        .unwrap_or(0);
    let leader_slot_id = slot_ids.get(leader_idx).cloned().unwrap_or_default();
    if !leader_slot_id.is_empty() {
        doc.set_leader(&squad_id, &leader_slot_id);
    }

    let mut vehicles_applied = 0usize;
    for (j, v) in lib.vehicles.iter().enumerate() {
        if v.vehicle.trim().is_empty() {
            continue;
        }
        let vid = format!("veh-{side}-apply-{j}");
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

    /// H9 — second Apply replaces (slot count becomes new R, not R_old+R_new).
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
