//! Contract parity: live answers of the API against the committed goldens, the contract
//! schemas and the generated types.
//!
//! One test binary on one database: every module shares the database
//! [`common::require_test_database_url`] provisions once and isolates its tests through rows
//! and ids it mints for itself.

#[path = "../common/mod.rs"]
mod common;
mod contract_parity_support;
#[path = "../contract_support/mod.rs"]
mod contract_support;
#[path = "../event_eligibility_support/mod.rs"]
mod event_eligibility_support;
#[path = "../fleet_support/mod.rs"]
mod fleet_support;
#[path = "../mission_artifact_support/mod.rs"]
mod mission_artifact_support;

mod current_profile;
mod equipment_viewer;
mod event_access;
mod game_runtime;
mod goldens;
mod mission_review;
