//! **Role:** the relations between placed entities — arming a connect, completing it, listing what
//! is connected, and forcing a group into formation.
//! **Position:** `hosted_commands::entity_connections` in `mission_editing_commands`.
//! **Signals & state:** the armed connect, one per thread. The document comes from the host.
//! **Invariants:** the armed connect is interaction state and never reaches the document, so
//! arming, re-arming and cancelling are not undo steps. Re-arming REPLACES the previous arm rather
//! than stacking — an operator who picks a new source or a new kind has changed their mind, and a
//! queue of pending connects is not a thing any surface offers. An arm is refused on a token the
//! document would refuse anyway, so a bad kind cannot sit live waiting to fail at completion.

use std::cell::RefCell;

use mission_document::ConnectionKind;
use mission_document::ids::{ConnectionId, EntityId};
use mission_editing_session::history::after_local_edit;
use mission_editing_session::host::with_doc;
use mission_operations::entity as entity_ops;

use mission_operations::entity::{ConnectionFindingRow, ConnectionListRow, OwnerOption};
use orbat_slot_ids::SlotUid;

thread_local! {
    static ARMED_CONNECT: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
}

/// Whether the document still carries a connection with this id.
#[must_use]
pub fn connection_exists(id: impl Into<ConnectionId>) -> bool {
    let id: ConnectionId = id.into();
    with_doc(|core| entity_ops::connection_id_in_doc(core, id)).unwrap_or(false)
}

/// Arm a connect of `kind` from `from_id`, to be completed by the next pick. `false` for a blank
/// source or a kind the document's own vocabulary does not name, and in both cases nothing is
/// armed.
pub fn arm_connect(kind: &str, from_id: impl Into<EntityId>) -> bool {
    let from_id: EntityId = from_id.into();
    if from_id.as_str().is_empty() || ConnectionKind::parse(kind).is_none() {
        return false;
    }
    ARMED_CONNECT.with(|p| {
        *p.borrow_mut() = Some((kind.to_string(), from_id.into_inner()));
    });
    true
}

/// The armed connect as `(kind, from_id)`, or `None`.
#[must_use]
pub fn pending_connect() -> Option<(String, String)> {
    ARMED_CONNECT.with(|p| p.borrow().clone())
}

/// Drop the armed connect.
pub fn cancel_connect() {
    ARMED_CONNECT.with(|p| *p.borrow_mut() = None);
}

/// Complete the armed connect onto `to_id`. The arm is CONSUMED whatever follows, so a pick that
/// landed on nothing ends the gesture instead of leaving a stale arm live under the cursor.
pub fn complete_connect(to_id: impl Into<EntityId>) -> bool {
    let to_id: EntityId = to_id.into();
    let Some((kind, from_id)) = ARMED_CONNECT.with(|p| p.borrow_mut().take()) else {
        return false;
    };
    if to_id.as_str().is_empty() {
        return false;
    }
    let drawn =
        with_doc(|core| entity_ops::complete_connect(core, to_id, kind, from_id)).unwrap_or(false);
    if drawn {
        after_local_edit();
    }
    drawn
}

/// Delete one connection. Whether anything was removed is read from the document BEFORE the write,
/// because `remove_connection` returns unit and cannot report it; guessing after the fact would
/// tick history for a delete that removed nothing.
pub fn delete_connection(id: impl Into<ConnectionId>) -> bool {
    let id: ConnectionId = id.into();
    let removed = with_doc(|core| entity_ops::delete_connection(core, id)).unwrap_or(false);
    if removed {
        after_local_edit();
    }
    removed
}

/// Every authored connection, as the panel lists them.
#[must_use]
pub fn connection_list() -> Vec<ConnectionListRow> {
    with_doc(entity_ops::connection_list).unwrap_or_default()
}

/// The rule findings over the connection graph — cycles, dangling ends, and the rest.
#[must_use]
pub fn connection_findings() -> Vec<ConnectionFindingRow> {
    with_doc(entity_ops::connection_findings).unwrap_or_default()
}

/// The placed entities a trigger's owner picker can offer.
#[must_use]
pub fn placed_owner_options() -> Vec<OwnerOption> {
    with_doc(entity_ops::placed_owner_options).unwrap_or_default()
}

/// Lay a leader's group out in `formation` and report how many moved. The geometry and the
/// single-transaction guarantee are the document's, so the whole re-form is one Ctrl+Z.
pub fn force_to_formation(leader_slot_id: impl Into<SlotUid>, formation: &str) -> usize {
    let leader_slot_id: SlotUid = leader_slot_id.into();
    let moved = with_doc(|core| core.force_to_formation(leader_slot_id, formation)).unwrap_or(0);
    if moved > 0 {
        after_local_edit();
    }
    moved
}
