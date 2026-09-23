//! Operations-domain database and wire models: the event container and its missions, the ORBAT
//! seats and squad holds, registrations, leave requests, and saved mortar fire missions.
//!
//! Field order and JSON keys are the wire contract: snake_case throughout, an absent value
//! expressed as `skip_serializing_if`, and RFC3339Nano timestamps rendered through
//! [`crate::core::wire_format`]. The enums map to the Postgres ENUM types. Soft-delete columns
//! are absent from these structs — the filter is enforced in the query layer.

pub mod event;
pub mod event_access_administration;
pub mod event_access_policy;
pub mod event_group;
pub mod event_viewer_access;
pub mod fire_mission;
pub mod game_runtime_roster;
pub mod leave_request;
pub mod live_occupancy;
pub mod participant_allocation;
pub mod reservation_quota;

pub use event::{
    Event, EventMission, EventRegistration, EventStatus, OrbatReservation, OrbatSlot,
    RegistrationState,
};
pub use fire_mission::FireMission;
pub use leave_request::{LeaveRequest, LeaveStatus};
pub mod generated;
pub mod reservation_response;
