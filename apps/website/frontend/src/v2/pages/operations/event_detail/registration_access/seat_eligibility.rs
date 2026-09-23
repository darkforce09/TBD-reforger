//! Whether one seat of the order of battle is open to the viewer, and why it is not.
//!
//! **Role:** reads a seat's viewer standing — eligible or restricted — and names the policy that
//! restricts it.
//! **Position:** read by each seat row of the squad pane.
//! **Signals & state:** none; pure over the seat.
//! **Invariants:** only the exact value `eligible` opens a seat; anything else, including a value
//! this build does not know, reads as restricted, so a seat the backend would refuse is never
//! offered. The reason names the nearest policy that is set — the seat's own, its squad's, or the
//! operation's — and never another participant.

use crate::v2::core::api::dto::OrbatSlot;

/// Whether the seat's effective policy admits the viewer now.
pub(crate) fn seat_admits_viewer(slot: &OrbatSlot) -> bool {
    slot.viewer_access == "eligible"
}

/// Why a restricted seat is closed to the viewer, by the policy that decides it.
pub(crate) fn restriction_reason(policy_source: &str) -> &'static str {
    match policy_source {
        "slot" => "Restricted by this seat's access policy",
        "squad" => "Restricted by this squad's access policy",
        "event" => "Restricted by the operation's access policy",
        _ => "Restricted by an access policy",
    }
}
