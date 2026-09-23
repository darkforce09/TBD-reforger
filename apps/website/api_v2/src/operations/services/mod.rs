//! Operations business logic shared by the domain's handlers: the event status derivation and
//! transition table, the lifecycle convergence sweep, and the canonical event row reads.

pub mod access_administration;
pub mod event_access;
pub mod event_lifecycle_sweep;
pub(crate) mod event_lifecycle_transition;
pub mod event_lookup;
pub mod event_reservations;
pub mod event_status_rules;
pub mod live_slot_occupancy;
pub mod participation_attribution;

// The ORBAT template shapes and parser live in the map engine, where the scenario document is
// defined. Re-exported here so the handlers that seat an ORBAT name one path.
pub use website_map_engine::data::scenario::orbat::{
    OrbatSlotTemplate, OrbatSquadTemplate, parse_orbat_template,
};
