//! Role: the authored triggers — the stored-not-evaluated activation conditions a mission declares,
//! and the owner edge that ties one to a placed entity.
//! Position: `editing/hosted_commands` in the map engine.
//! Signals & state: none of its own; the document comes from the host.
//! Invariants: only a write that actually changed the document takes the post-change tail, so a
//! refused value never costs an author an undo step. A value outside the closed `activation` list
//! is refused rather than stored, for the same reason a zone type is: it saves once and then fails
//! every compile after. The owner edge carries no referential check — a later deletion of the owner
//! is tolerated as a dangling edge rather than cascading.

use crate::data::store::MissionDocCore;
use crate::data::store::operations::entity as entity_ops;
use crate::editing::history::after_local_edit;
use crate::editing::host::with_doc;

/// One authored trigger, as the palette's row needs it.
pub use crate::data::store::operations::entity::TriggerRow;

/// The closed set of activation kinds a trigger may carry.
pub use crate::data::store::operations::entity::TRIGGER_ACTIVATIONS;

/// Every authored trigger, sorted by id — the palette's list.
#[must_use]
pub fn trigger_rows() -> Vec<TriggerRow> {
    with_doc(entity_ops::trigger_rows)
        .flatten()
        .unwrap_or_default()
}

/// How many triggers the document declares — the palette header's count.
#[must_use]
pub fn trigger_count() -> usize {
    with_doc(entity_ops::trigger_count).unwrap_or(0)
}

/// Run one trigger edit and take the tail only if it wrote.
fn edit_trigger(edit: impl FnOnce(&MissionDocCore) -> bool) -> bool {
    let did = with_doc(edit).unwrap_or(false);
    if did {
        after_local_edit();
    }
    did
}

/// Set or remove a trigger's `name`. `None` removes the key, which is what an emptied box sends.
pub fn set_trigger_name(id: &str, name: Option<String>) -> bool {
    edit_trigger(|core| {
        entity_ops::set_trigger_name(core, id, name.as_deref());
        true
    })
}

/// Set the stored `activation` kind. A value outside [`TRIGGER_ACTIVATIONS`] is refused.
pub fn set_trigger_activation(id: &str, activation: &str) -> bool {
    edit_trigger(|core| entity_ops::set_trigger_activation(core, id, activation))
}

/// Assign or clear the owner edge. `Some(id)` is a placed entity; `None` clears the link.
pub fn set_trigger_owner(id: &str, owner_id: Option<String>) -> bool {
    edit_trigger(|core| {
        entity_ops::set_trigger_owner(core, id, owner_id.as_deref());
        true
    })
}

/// Set or clear ONE `rules` key, read-modify-write over the opaque rules object. `value: None`
/// removes the key, and clearing the last key drops the whole object — "cleared all" and "never
/// authored" are the same authored state.
pub fn set_trigger_rule(id: &str, key: &str, value: Option<serde_json::Value>) -> bool {
    edit_trigger(|core| entity_ops::apply_trigger_rule(core, id, key, value))
}

/// Delete a trigger.
pub fn delete_trigger(id: &str) -> bool {
    edit_trigger(|core| {
        entity_ops::delete_trigger(core, id);
        true
    })
}

/// The world-space segment from a trigger to its owner, for the overlay that draws the edge.
/// `selected_trigger` is the palette's current row — session state the surface owns, passed in so
/// this stays a resolve over the document plus one id.
#[must_use]
pub fn owner_line_world(selected_trigger: Option<&str>) -> Option<((f64, f64), (f64, f64))> {
    with_doc(|core| entity_ops::owner_line_world(core, selected_trigger)).flatten()
}
