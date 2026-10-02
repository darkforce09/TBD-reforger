//! Role: apply.
//! Position: `doc/operations/apply_faction` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::ApplyFactionError;
use super::ApplyFactionResult;
use super::AuthoredSquad;
use super::FactionLibraryInput;
use super::MissionDocCore;
use super::SLOT_SPACING_X;
use super::VALID_SIDES;
use super::Value;
use super::apply_anchor_xy;
use super::faction_squad_ids;
use super::mint_slot_id;
use super::mint_squad_id;
use super::mint_vehicle_id;
use super::squad_authorship;
use super::squad_slot_ids;
use super::squad_vehicle_ids;

/// MUTATE-apply a Faction Library doc onto one mission side (B2).
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

    let existing_squads = faction_squad_ids(doc, &faction_id);
    let blocking: Vec<AuthoredSquad> = existing_squads
        .iter()
        .skip(1)
        .filter_map(|sid| squad_authorship(doc, sid))
        .collect();
    if !blocking.is_empty() {
        let slots_at_risk: usize = blocking.iter().map(|b| b.slots).sum();
        return Err(ApplyFactionError::WouldCollapseSquads {
            side: side.to_string(),
            squad_ids: existing_squads,
            blocking,
            slots_at_risk,
        });
    }

    ensure_side_faction(doc, side, &faction_id, &lib.name);
    let (anchor_x, anchor_y) = apply_anchor_xy(doc);

    let squad_name = if lib.name.trim().is_empty() {
        "Squad 1".to_string()
    } else {
        lib.name.clone()
    };

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

    for folded in existing_squads.iter().skip(1) {
        let carried = squad_slot_ids(doc, folded);
        if carried.is_empty() {
            doc.remove_squad(folded);
            continue;
        }
        for slot_id in carried {
            doc.move_slot_to_squad(&slot_id, &squad_id);
        }
    }

    let existing_slots = squad_slot_ids(doc, &squad_id);

    let mut slot_ids: Vec<String> = Vec::with_capacity(lib.roles.len());
    for (i, role) in lib.roles.iter().enumerate() {
        if let Some(slot_id) = existing_slots.get(i) {
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
        let _ = v.label;
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

/// Is squad leader role using the supplied domain data.
pub(super) fn is_squad_leader_role(role: &str) -> bool {
    role.to_ascii_lowercase().contains("squad leader")
}

/// Ensure side faction using the supplied domain data.
pub(super) fn ensure_side_faction(doc: &MissionDocCore, side: &str, faction_id: &str, name: &str) {
    if faction_exists(doc, faction_id) {
        doc.set_faction_name(faction_id, name);
        return;
    }
    let display = if name.trim().is_empty() { side } else { name };
    doc.add_faction(faction_id, side, display);
}

/// Faction exists using the supplied domain data.
pub(super) fn faction_exists(doc: &MissionDocCore, faction_id: &str) -> bool {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return false;
    };
    root.get("factionsById")
        .and_then(|v| v.as_object())
        .is_some_and(|m| m.contains_key(faction_id))
}
