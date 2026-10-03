//! Role: placement.
//! Position: the `entity::placement` module of `mission_operations`; hosted commands drive it.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use mission_document::ids::{EntityId, SquadId};

use super::ANCHOR;
use super::MissionDocCore;
use super::PlacePayload;
use super::asset_id_for_role;
use super::ensure_side_faction;
use super::mint_id;
use super::next_slot_xy;
use super::squad_anchor_xy;
use super::squad_rows;

/// `alias` is derived from the ResourceName + display label (`derive_object_alias`); `faction` is the schema factionKey slug (`blufor`/`opfor`/`indfor`) from the active Eden side.
pub fn place_object_in_core(
    core: &MissionDocCore,
    side: &str,
    entity_id: impl Into<EntityId>,
    payload: &PlacePayload,
    x: f64,
    y: f64,
) -> bool {
    let entity_id: EntityId = entity_id.into();
    let entity_id = entity_id.as_str();
    if !matches!(side, "BLUFOR" | "OPFOR" | "INDFOR") || payload.asset_id.as_str().trim().is_empty()
    {
        return false;
    }
    let alias = crate::assets::derive_object_alias(payload.asset_id.as_str(), &payload.role);
    if alias.is_empty() {
        return false;
    }

    let _ = ensure_side_faction(core, side);
    let faction = side.to_lowercase();
    core.add_entity(entity_id, &alias, payload.asset_id.as_str(), x, y, 0.0, 0.0);
    core.set_entity_faction(entity_id, &faction);
    true
}

/// Apply orbat_add_slot to explicit document state.
pub fn orbat_add_slot(
    core: &MissionDocCore,
    squad_id: impl Into<SquadId>,
    role: String,
    next_id: &std::cell::Cell<u32>,
    ensure_layer: impl FnOnce(&MissionDocCore) -> String,
    seed_cargo_in_core: impl Fn(&MissionDocCore, &str, &str, Option<&str>) -> bool,
) -> Option<String> {
    let squad_id: SquadId = squad_id.into();
    let squad_id = squad_id.into_inner();
    let sq = squad_rows(core).into_iter().find(|s| s.id == *squad_id)?;
    let index = sq.slot_ids.len() as u32;
    let layer_id = ensure_layer(core);
    let slot_id = mint_id(core, next_id);
    let role = if role.trim().is_empty() {
        "Rifleman".to_string()
    } else {
        role
    };
    let (x, y) = next_slot_xy(core, &sq);
    let asset_id = asset_id_for_role(core, &sq, &role);
    core.add_slot(
        slot_id.as_str(),
        squad_id.as_str(),
        layer_id.as_str(),
        index,
        &role,
        None,
        asset_id.clone().map(Into::into),
        x,
        y,
        0.0,
        0.0,
    );

    if let Some(a) = &asset_id {
        seed_cargo_in_core(core, &slot_id, a, None);
    }
    if sq.leader_slot_id.as_str().is_empty() {
        core.set_leader(squad_id.as_str(), slot_id.as_str());
    }
    Some(slot_id)
}

/// Apply orbat_add_vehicle to explicit document state.
pub fn orbat_add_vehicle(
    core: &MissionDocCore,
    squad_id: impl Into<SquadId>,
    resource_name: &str,
    next_id: &std::cell::Cell<u32>,
) -> Option<String> {
    let squad_id: SquadId = squad_id.into();
    let squad_id = squad_id.into_inner();
    let sq = squad_rows(core).into_iter().find(|s| s.id == *squad_id)?;
    let n = sq.vehicle_ids.len();
    let vehicle_id = mint_id(core, next_id);

    let (x, y) = squad_anchor_xy(core, &sq).unwrap_or((ANCHOR[0], ANCHOR[1]));
    let x = x + 30.0 + 20.0 * n as f64;
    let y = y - 30.0;
    core.add_vehicle(
        vehicle_id.as_str(),
        resource_name,
        Some(x),
        Some(y),
        Some(0.0),
        Some(0.0),
    );
    core.attach_vehicle(squad_id.as_str(), vehicle_id.as_str());
    Some(vehicle_id)
}
