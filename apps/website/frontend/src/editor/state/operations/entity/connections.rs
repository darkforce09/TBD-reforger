//! Role: connections.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

/// Connection exists using the supplied domain data.
#[must_use]
pub fn connection_exists(id: &str) -> bool {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        d.as_ref()
            .is_some_and(|core| connection_id_in_doc(core, id))
    })
}

/// Re-arming REPLACES any previous arm rather than stacking: the operator changed their mind about the source or the kind, and a queue of pending connects is not a thing anyone asked for.
pub fn arm_connect(kind: &str, from_id: &str) -> bool {
    if from_id.is_empty() || crate::editor::panels::context_menu::ConnKind::parse(kind).is_none() {
        return false;
    }
    PENDING_CONNECT.with(|p| {
        *p.borrow_mut() = Some((kind.to_string(), from_id.to_string()));
    });
    true
}

/// Pending connect using the supplied domain data.
#[must_use]
pub fn pending_connect() -> Option<(String, String)> {
    PENDING_CONNECT.with(|p| p.borrow().clone())
}

/// Cancel connect using the supplied domain data.
pub fn cancel_connect() {
    PENDING_CONNECT.with(|p| *p.borrow_mut() = None);
}

/// Complete connect using the supplied domain data.
pub fn complete_connect(to_id: &str) -> bool {
    let Some((kind, from_id)) = PENDING_CONNECT.with(|p| p.borrow_mut().take()) else {
        return false;
    };
    if to_id.is_empty() {
        return false;
    }
    let drawn = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        website_mission_core::doc::operations::entity::complete_connect(core, to_id, kind, from_id)
    });
    if drawn {
        mission_history::after_local_edit();
    }
    drawn
}

/// `MissionDocCore::remove_connection` returns unit and cannot report what it removed, so the answer is taken from the document BEFORE the write ([`connection_id_in_doc`]) instead of guessed after it. If the core mutator ever grows a `bool` like `add_connection` has, this gate becomes its return value and the pre-read goes away.
pub fn delete_connection(id: &str) -> bool {
    let removed = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        website_mission_core::doc::operations::entity::delete_connection(core, id)
    });
    if removed {
        mission_history::after_local_edit();
    }
    removed
}

/// Connection list using the supplied domain data.
#[must_use]
pub fn connection_list() -> Vec<ConnectionListRow> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        website_mission_core::doc::operations::entity::connection_list(core)
    })
}

/// Connection findings using the supplied domain data.
#[must_use]
pub fn connection_findings() -> Vec<ConnectionFindingRow> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        website_mission_core::doc::operations::entity::connection_findings(core)
    })
}

/// The geometry and the single-transaction guarantee live in `MissionDocCore::force_to_formation`; this is the thin editor-side call plus the history tick, so the action is one Ctrl+Z.
pub fn force_to_formation(leader_slot_id: &str, formation: &str) -> usize {
    let moved = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return 0;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return 0;
        };
        core.force_to_formation(leader_slot_id, formation)
    });
    if moved > 0 {
        mission_history::after_local_edit();
    }
    moved
}
