//! Operations-domain database and wire models: the event container and its missions, the ORBAT
//! seats and squad holds, registrations, and leave requests.
//!
//! Field order and JSON keys are the wire contract: snake_case throughout, an absent value
//! expressed as `skip_serializing_if`, and RFC3339Nano timestamps rendered through
//! [`crate::core::wire_format`]. The enums map to the Postgres ENUM types. Soft-delete columns
//! are absent from these structs — the filter is enforced in the query layer.

pub mod event;
pub mod leave_request;

pub use event::{
    Event, EventMission, EventRegistration, EventStatus, OrbatReservation, OrbatSlot,
    RegistrationState,
};
pub use leave_request::{LeaveRequest, LeaveStatus};
