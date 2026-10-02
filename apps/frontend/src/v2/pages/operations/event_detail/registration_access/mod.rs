//! What the viewer may register for in an operation, and why.
//!
//! **Role:** groups the viewer-facing access rules of the operation dossier — the place outlook
//! the pools and the operation-wide limit give, the standing on each mission (waiting position,
//! released signup, eligibility), the seat restrictions of the order of battle, the wording of a
//! refused registration, the Places panel, and the leader's waiting-list promotion.
//! **Position:** inside the operation dossier page; read by the hub body, the mission cards, the
//! slotting selector and the standalone slotting page.
//! **Signals & state:** none at this level; only the promotion control owns a busy flag.
//! **Invariants:** everything is derived from what the backend returned to this viewer. A partial
//! viewer's dossier carries only the missions and seats open to them, and nothing here reaches
//! past it. The decisions are pure and native-tested; the views only render them.

pub(crate) mod mission_standing;
pub(crate) mod place_outlook;
pub(crate) mod places_panel;
pub(crate) mod refusal_notices;
pub(crate) mod seat_eligibility;
pub(crate) mod waitlist_promotion;

#[cfg(test)]
#[path = "tests/registration_access.rs"]
mod tests;
