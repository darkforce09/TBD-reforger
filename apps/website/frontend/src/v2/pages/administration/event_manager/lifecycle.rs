//! The operation lifecycle: its six states, the moves between them, and how they are shown.
//!
//! **Role:** the status table the edit form's picker is built from, the client-side mirror of the
//! server's transition rules, the badge variant each state renders as, and the copy the delete
//! confirmation puts in front of the operator.
//! **Position:** pure helpers under the operations calendar; no view, no request.
//! **Signals & state:** none.
//! **Invariants:** the server owns the transition rules. The mirror here exists so the picker
//! cannot offer a move that will be refused, never to decide one: every rule it cannot evaluate —
//! notably that reopening a started operation also requires rescheduling it into the future — stays
//! server-side, and the refusal is shown as the server worded it. The delete copy describes a soft
//! delete, which is what the endpoint performs, and the guard tests hold it to that.
#![allow(dead_code)]

use crate::v2::core::ui::badge_class;

/// A terrain's wire name with its first letter capitalised, or a dash when there is none.
pub(super) fn terrain_label(t: &str) -> String {
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => "—".into(),
    }
}

/// Every lifecycle state, as the wire value the API takes paired with the label the picker shows.
pub(super) const EVENT_STATUSES: [(&str, &str); 6] = [
    ("scheduled", "Scheduled"),
    ("open", "Open"),
    ("locked", "Locked"),
    ("live", "Live"),
    ("completed", "Completed"),
    ("cancelled", "Cancelled"),
];

/// Whether the server will accept moving an operation from `from` to `to`.
///
/// Staying put counts as legal, so the picker always offers the state the operation is already in.
/// `completed` and `cancelled` are terminal, and so is any state this table has not heard of —
/// offering nothing is the safe answer when the server has moved ahead of the client.
pub(super) fn can_transition(from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    match from {
        "scheduled" => matches!(to, "open" | "locked" | "live" | "cancelled"),
        "open" => matches!(to, "locked" | "live" | "cancelled"),
        "locked" => matches!(to, "open" | "live" | "cancelled"),
        "live" => matches!(to, "open" | "locked" | "completed" | "cancelled"),
        // `completed` and `cancelled` are terminal — an operation that was fought or called off
        // is a matter of record. An unknown status (a state added server-side before this table
        // learns about it) is treated the same way: offer nothing rather than guess.
        _ => false,
    }
}

/// Title of the confirmation shown before an operation is deleted.
pub(super) const DELETE_EVENT_CONFIRM_TITLE: &str = "Delete this operation?";

/// Body of that confirmation.
///
/// The endpoint marks the operation deleted and touches nothing else: the attached missions, their
/// ORBATs and every registration survive, and an administrator can put the row back. The copy says
/// exactly that, and a guard test refuses any wording that promises destruction or irreversibility.
pub(super) const DELETE_EVENT_CONFIRM_DESC: &str = "It leaves the schedule, the dashboard and everyone's deployments, and no one can register on it. Nothing is erased — the attached missions, their ORBATs and every registration are kept, so an administrator can still restore it from the database.";

/// Badge variant for a lifecycle status, so the day list shows where an operation *is* and not
/// just whether registration happens to be locked.
pub(super) fn status_badge(status: &str) -> String {
    badge_class(match status {
        "open" => "success",
        "locked" => "warning",
        "live" => "primary",
        "completed" => "tertiary",
        "cancelled" => "error",
        _ => "neutral",
    })
}
