//! Operations business logic shared by the domain's handlers: the event status derivation and
//! transition table, the lifecycle convergence sweep, and the canonical event row reads.

pub mod access_administration;
pub mod ballistics_catalogs;
pub mod event_access;
pub mod event_authoring;
pub mod event_lifecycle_sweep;
pub(crate) mod event_lifecycle_transition;
pub mod event_lookup;
pub mod event_reservations;
pub mod event_status_rules;
pub mod fire_mission_resolve;
pub mod fire_mission_store;
pub mod live_slot_occupancy;
