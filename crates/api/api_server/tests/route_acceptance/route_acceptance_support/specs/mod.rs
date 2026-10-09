//! Every part's route specs.
//!
//! **Role:** declares one spec module per part.
//!
//! **Position:** test support; each `specs/<part>.rs` is read by that part's test functions in
//! the `route_acceptance` binary.
//!
//! **Signals & state:** none.
//!
//! **Invariants:** spec modules are pure data (no world or support-module imports); a route
//! belongs to exactly one part.

pub(crate) mod administration_center_content;
pub(crate) mod fleet_and_telemetry;
pub(crate) mod identity_and_core;
pub(crate) mod missions_library;
pub(crate) mod missions_reviews;
pub(crate) mod operations_ballistics;
pub(crate) mod operations_events;
pub(crate) mod operations_reservations;
