//! Taking a place in a mission, and moving waiting participants into free places.
//!
//! **Role:** the registration route — for a named seat, or for a seatless place that joins the
//! waiting list when none is free — and the waiting-list promotion a leader or administrator asks
//! for.
//! **Position:** called by the operation dossier's slotting controls and by the event manager's
//! access panel.
//! **Signals & state:** none.
//! **Invariants:** both answer an [`ApiRefusal`](crate::v2::core::api::client::ApiRefusal) on
//! failure, because the reason decides what the viewer is told: a registration refusal names one of
//! `ACCESS_POLICY`, `MEMBERSHIP_VERIFICATION_REQUIRED`, `QUOTA_NOT_OPEN`, `EVENT_FULL`,
//! `MISSION_FULL`, `SEAT_NEEDED_BY_HOLDER`, `SEAT_TAKEN`, `SQUAD_HELD`, `NO_SEATS`,
//! `REGISTRATION_CLOSED`, `ACCOUNT_UNAVAILABLE` or `DEPLOYMENT_REQUIREMENTS`, and a promotion with
//! no place to give answers `EVENT_FULL`. A seatless request while no place is free succeeds, with
//! the reservation waitlisted.

use super::encode_path_segment as segment;

/// `POST` (register) and `DELETE` (withdraw) `/event-missions/:emid/register`.
pub fn mission_registration_path(event_mission_id: &str) -> String {
    format!("/event-missions/{}/register", segment(event_mission_id))
}

/// `POST /event-missions/:emid/waitlist/promote`.
pub fn waitlist_promotion_path(event_mission_id: &str) -> String {
    format!(
        "/event-missions/{}/waitlist/promote",
        segment(event_mission_id)
    )
}

/// The registration body: the seat asked for, or the empty id that asks for a seatless place.
pub fn registration_body(seat: Option<&str>) -> serde_json::Value {
    serde_json::json!({ "slot_id": seat.unwrap_or("") })
}

#[cfg(target_arch = "wasm32")]
pub use calls::*;

/// The browser-only calls, one per route.
#[cfg(target_arch = "wasm32")]
mod calls {
    use super::*;
    use crate::v2::core::api::client::{api_post_keeping_refusal, ApiRefusal};
    use crate::v2::core::api::dto::{ReservationResponse, WaitlistPromotion};
    use crate::v2::core::auth::AuthStore;

    /// Register for `seat`, or — with no seat — for a seatless place, which waitlists the viewer
    /// when no place is free.
    pub async fn register_for_mission(
        store: AuthStore,
        event_mission_id: &str,
        seat: Option<&str>,
    ) -> Result<ReservationResponse, ApiRefusal> {
        let path = mission_registration_path(event_mission_id);
        api_post_keeping_refusal(store, &path, registration_body(seat)).await
    }

    /// Promote the earliest eligible waiting participants into free seats and places.
    pub async fn promote_waitlisted_participants(
        store: AuthStore,
        event_mission_id: &str,
    ) -> Result<WaitlistPromotion, ApiRefusal> {
        let path = waitlist_promotion_path(event_mission_id);
        api_post_keeping_refusal(store, &path, serde_json::json!({})).await
    }
}
