//! Role: authored trigger edits and the owner link a trigger draws to its entity.
//! Position: the `entity::triggers` module of `mission_operations`; hosted commands drive it.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations. A value
//! outside a closed set never reaches the document — an invalid edit is refused, not coerced.

use mission_document::ids::EntityId;
use mission_model::ids::TriggerId;

use super::MissionDocCore;
use super::TRIGGER_ACTIVATIONS;
use super::placed_entity_pos;
use super::set_trigger_rule;
use super::trigger_rows;
use super::vehicle_rows;

/// Set or clear the trigger's `name`. `None` REMOVES the key, which is what an emptied field means.
pub fn set_trigger_name(core: &MissionDocCore, id: impl Into<TriggerId>, name: Option<&str>) {
    let id: TriggerId = id.into();
    let id = id.as_str();
    core.set_trigger_name(id, name);
}

/// Set the stored-not-evaluated `activation` kind. A value outside [`TRIGGER_ACTIVATIONS`] is
/// REFUSED and the document is left untouched — the same discipline `zone.type` is held to.
pub fn set_trigger_activation(
    core: &MissionDocCore,
    id: impl Into<TriggerId>,
    activation: &str,
) -> bool {
    let id: TriggerId = id.into();
    let id = id.as_str();
    if !TRIGGER_ACTIVATIONS.contains(&activation) {
        return false;
    }
    core.set_trigger_activation(id, activation);
    true
}

/// Assign or clear the owner edge. `Some(id)` names a placed entity; `None` clears the link. There
/// is no referential check: deleting the owner later leaves a DANGLING edge, which readers resolve
/// to nothing rather than repair.
pub fn set_trigger_owner(
    core: &MissionDocCore,
    id: impl Into<TriggerId>,
    owner_id: Option<EntityId>,
) {
    let id: TriggerId = id.into();
    let id = id.as_str();
    core.set_trigger_owner(id, owner_id);
}

/// Set or clear ONE `rules` key, read-modify-write over the opaque rules object: compute the next
/// object with [`set_trigger_rule`], then commit it. `value: None` removes the key, and clearing
/// the last key drops the whole object — "cleared every rule" and "never authored one" are the
/// same document. `false` when the trigger does not exist.
pub fn apply_trigger_rule(
    core: &MissionDocCore,
    id: impl Into<TriggerId>,
    key: &str,
    value: Option<serde_json::Value>,
) -> bool {
    let id: TriggerId = id.into();
    let id = id.as_str();
    let Some(next) = set_trigger_rule(core, id, key, value) else {
        return false;
    };
    core.set_trigger_rules(id, Some(&next));
    true
}

/// Delete the trigger.
pub fn delete_trigger(core: &MissionDocCore, id: impl Into<TriggerId>) {
    let id: TriggerId = id.into();
    let id = id.as_str();
    core.remove_trigger(id);
}

/// How many triggers the document declares.
pub fn trigger_count(core: &MissionDocCore) -> usize {
    core.trigger_count()
}

/// Where a placed entity sits in world metres, across BOTH collections an owner link can name:
/// the slot table first, then the placed vehicles. [`placed_entity_pos`] answers only for slots,
/// and an owner that is a vehicle would otherwise resolve to nothing.
pub fn placed_or_vehicle_position(
    core: &MissionDocCore,
    id: impl Into<EntityId>,
) -> Option<(f64, f64)> {
    let id: EntityId = id.into();
    let id = id.as_str();
    if let Some(slot) = placed_entity_pos(core, id) {
        return Some(slot);
    }
    vehicle_rows(core)
        .into_iter()
        .find(|vehicle| vehicle.id == id)
        .and_then(|vehicle| vehicle.xy)
}

/// The two ends of the owner-link line for `selected_trigger`: the trigger's geometric centre and
/// its owner's position. `None` when nothing is selected, the trigger is unowned or shapeless, or
/// the owner it names is no longer placed.
pub fn owner_line_world(
    core: &MissionDocCore,
    selected_trigger: Option<&str>,
) -> Option<((f64, f64), (f64, f64))> {
    let id = selected_trigger?;
    let row = trigger_rows(core)?.into_iter().find(|r| r.id == id)?;
    let owner_id = row.owner_id.as_ref().map(EntityId::as_str)?;
    let centre = row.centre()?;
    let owner = placed_or_vehicle_position(core, owner_id)?;
    Some((centre, owner))
}

#[cfg(test)]
#[path = "tests/triggers.rs"]
mod tests;
