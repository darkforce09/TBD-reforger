//! Every part's route specs, concatenated for the coverage binary.
//!
//! **Role:** declares one spec module per part and [`all_specs`], the list the coverage binary
//! matches against the route table.
//!
//! **Position:** test support; each `specs/<part>.rs` is written by its part agent and read by
//! that part's binary and by `tests/route_acceptance_coverage.rs`.
//!
//! **Signals & state:** none.
//!
//! **Invariants:** spec modules are pure data (no world or support-module imports), so every
//! binary that mounts the framework compiles all of them; a route belongs to exactly one part.

use super::spec::RouteSpec;

pub(crate) mod administration_center_content;
pub(crate) mod fleet_and_telemetry;
pub(crate) mod identity_and_core;
pub(crate) mod missions_library;
pub(crate) mod missions_reviews;
pub(crate) mod operations_ballistics;
pub(crate) mod operations_events;
pub(crate) mod operations_reservations;

/// Every spec of every part.
pub(crate) fn all_specs() -> Vec<RouteSpec> {
    [
        identity_and_core::specs(),
        operations_events::specs(),
        operations_reservations::specs(),
        operations_ballistics::specs(),
        missions_library::specs(),
        missions_reviews::specs(),
        fleet_and_telemetry::specs(),
        administration_center_content::specs(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
