//! Role: triggers.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

/// Every authored trigger, sorted by id, for the palette list. Off `triggers_json`, the [`zone_rows`] twin.
#[must_use]
pub fn trigger_rows() -> Vec<TriggerRow> {
    OPS_CTX
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            website_map_engine::data::store::operations::entity::trigger_rows(core)
        })
        .unwrap_or_default()
}

/// Edit trigger using the supplied domain data.
pub(in crate::editor::state::operations) fn edit_trigger(f: impl FnOnce(&MissionDocCore)) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        f(core);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Attributes — the trigger `name`. `None` REMOVES the key; the panel sends `None` on an emptied box.
pub fn set_trigger_name(id: &str, name: Option<String>) -> bool {
    edit_trigger(|core| core.set_trigger_name(id, name.as_deref()))
}

/// Attributes — the stored-not-evaluated `activation` kind. Refuses a value outside [`TRIGGER_ACTIVATIONS`], the same "no invented value reaches the doc" discipline `set_zone_kind` applies to `zone.type`.
pub fn set_trigger_activation(id: &str, activation: &str) -> bool {
    if !TRIGGER_ACTIVATIONS.contains(&activation) {
        return false;
    }
    edit_trigger(|core| core.set_trigger_activation(id, activation))
}

/// CONN-TRG-OWNER-001 (the picker's write) — assign / clear the owner edge. `Some(id)` is a placed entity id from [`placed_owner_options`]; `None` clears the link. No referential check — a later deletion of the owner is TOLERATED as a dangling edge (see [`MissionDocCore::set_trigger_owner`]).
pub fn set_trigger_owner(id: &str, owner_id: Option<String>) -> bool {
    edit_trigger(|core| core.set_trigger_owner(id, owner_id.as_deref()))
}

/// Attributes — set or clear ONE `rules` key, read-modify-write over the OPAQUE object. Reuses the exact `set_zone_rule` shape: the key is a `$defs/zoneRules` property supplied by the schema-driven panel; this never names one. `value: None` removes the key, and clearing the last key drops the whole object (the "cleared all" == "never authored" identity).
pub fn set_trigger_rule(id: &str, key: &str, value: Option<serde_json::Value>) -> bool {
    let next = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        website_map_engine::data::store::operations::entity::set_trigger_rule(core, id, key, value)
    });
    let Some(next) = next else {
        return false;
    };
    edit_trigger(|core| core.set_trigger_rules(id, Some(&next)))
}

/// Attributes — delete the trigger.
pub fn delete_trigger(id: &str) -> bool {
    edit_trigger(|core| core.remove_trigger(id))
}

/// How many triggers the document declares — backs the palette header count.
#[must_use]
pub fn trigger_count() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| ctx.doc.borrow().as_ref().map(MissionDocCore::trigger_count))
            .unwrap_or(0)
    })
}

/// Placed entity pos using the supplied domain data.
#[must_use]
pub(in crate::editor::state::operations) fn placed_entity_pos(id: &str) -> Option<(f64, f64)> {
    let slot = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        website_map_engine::data::store::operations::entity::placed_entity_pos(core, id)
    });
    if slot.is_some() {
        return slot;
    }
    vehicle_rows()
        .into_iter()
        .find(|v| v.id == id)
        .and_then(|v| v.xy)
}

/// `selected_trigger` is the trigger id the palette currently has selected (session UI state the panel owns), passed in so this stays a pure resolve over the doc + one id.
#[must_use]
pub fn owner_line_world(selected_trigger: Option<&str>) -> Option<((f64, f64), (f64, f64))> {
    let id = selected_trigger?;
    let row = trigger_rows().into_iter().find(|r| r.id == id)?;
    let owner_id = row.owner_id.as_deref()?;
    let centre = row.centre()?;

    let owner = placed_entity_pos(owner_id)?;
    Some((centre, owner))
}
