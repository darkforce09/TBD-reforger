//! Reservation actions expose allocation independently from factual attendance.
use super::event::RegistrationState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
pub struct ReservationResponse {
    pub state: RegistrationState,
    pub reservation_state: RegistrationState,
    pub attendance_state: Option<RegistrationState>,
    pub slot_id: Option<Uuid>,
}
