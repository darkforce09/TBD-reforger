//! Role: factions.
//! Position: `doc/operations/entity` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::FactionDoc;
use super::FactionLibraryInput;
use super::FactionLibraryRole;
use super::FactionLibraryVehicle;
use super::FactionRole;
use super::FactionVehicle;
use super::MissionDocCore;
use super::apply_faction_library;
use super::faction_rows;
use super::squad_rows;

/// `None` when `slots_json` / `small_maps_json` will not parse.
pub fn faction_doc_from_side_core(core: &MissionDocCore, side: &str) -> Option<FactionDoc> {
    let factions = faction_rows(core);
    let squads = squad_rows(core);
    let faction = factions.iter().find(|f| f.key == side);
    let name = faction
        .map(|f| f.name.clone())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| side.to_string());
    let squad_ids: Vec<String> = faction.map(|f| f.squad_ids.clone()).unwrap_or_default();
    let Ok(slots_root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return None;
    };
    let Ok(small) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) else {
        return None;
    };
    let mut roles = Vec::new();
    let mut vehicles = Vec::new();
    for sid in &squad_ids {
        let Some(sq) = squads.iter().find(|s| s.id == *sid) else {
            continue;
        };
        for slot_id in &sq.slot_ids {
            let Some(slot) = slots_root.get(slot_id) else {
                continue;
            };
            roles.push(FactionRole {
                role: slot
                    .get("role")
                    .and_then(|r| r.as_str())
                    .unwrap_or("Rifleman")
                    .to_string(),
                tag: slot
                    .get("tag")
                    .and_then(|t| t.as_str())
                    .map(str::to_string)
                    .filter(|s| !s.is_empty()),
                character: slot
                    .get("assetId")
                    .and_then(|a| a.as_str())
                    .unwrap_or_default()
                    .to_string(),
                loadout: slot.get("loadout").cloned(),
            });
        }
        for vid in &sq.vehicle_ids {
            let Some(v) = small.get("vehiclesById").and_then(|m| m.get(vid)) else {
                continue;
            };
            vehicles.push(FactionVehicle {
                vehicle: v
                    .get("resourceName")
                    .and_then(|r| r.as_str())
                    .unwrap_or_default()
                    .to_string(),

                label: None,
            });
        }
    }
    Some(FactionDoc {
        side: side.into(),
        name,

        emblem: None,
        roles,
        vehicles,
    })
}

/// Apply orbat_apply_faction to explicit document state.
pub fn orbat_apply_faction(
    core: &MissionDocCore,
    side: String,
    doc: FactionDoc,
    ensure_layer: impl FnOnce(&MissionDocCore) -> String,
    seed_cargo_in_core: impl Fn(&MissionDocCore, &str, &str, Option<&str>) -> bool,
) -> Result<(), String> {
    let layer_id = ensure_layer(core);
    let input = FactionLibraryInput {
        name: doc.name,
        roles: doc
            .roles
            .into_iter()
            .map(|r| FactionLibraryRole {
                role: r.role,
                tag: r.tag,
                character: r.character,
                loadout: r.loadout,
            })
            .collect(),
        vehicles: doc
            .vehicles
            .into_iter()
            .map(|v| FactionLibraryVehicle {
                vehicle: v.vehicle,
                label: v.label,
            })
            .collect(),
    };
    apply_faction_library(core, &side, &layer_id, &input).map_err(|e| e.to_string())?;

    if let Ok(map) = serde_json::from_str::<serde_json::Value>(&core.slots_json())
        && let Some(obj) = map.as_object()
    {
        for (sid, slot) in obj {
            let Some(rn) = slot
                .get("assetId")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            let lo = slot
                .get("loadout")
                .filter(|l| !l.is_null())
                .map(|l| l.to_string());
            seed_cargo_in_core(core, sid, rn, lo.as_deref());
        }
    }
    Ok(())
}
