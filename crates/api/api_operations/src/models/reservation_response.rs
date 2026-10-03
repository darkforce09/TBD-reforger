//! Reservation actions expose allocation independently from factual attendance.
use super::event::RegistrationState;
use api_identifiers::OrbatSlotId;
use serde::{Deserialize, Serialize};

/// The registration after a reservation action, with allocation and attendance kept apart.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
pub struct ReservationResponse {
    /// Compatibility state: the attendance state once recorded, otherwise the reservation state.
    pub state: RegistrationState,
    /// Reservation outcome: registered, waitlisted, withdrawn or legacy unknown.
    pub reservation_state: RegistrationState,
    /// Recorded attendance (attended or no show); absent until attendance is recorded.
    pub attendance_state: Option<RegistrationState>,
    /// The ORBAT seat the registration holds; absent when it names none.
    pub slot_id: Option<OrbatSlotId>,
}
