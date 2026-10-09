//! Integration tests of the game-server fleet and ballistics: the fleet command ledger, machine
//! credentials, runtime session fencing, the server registry and the ballistics catalogs and
//! fire missions.
//!
//! One test binary on one database: every module shares the database
//! [`common::require_test_database_url`] provisions once and isolates its tests through rows
//! and ids it mints for itself.

#[path = "../common/mod.rs"]
mod common;
#[path = "../content_support/mod.rs"]
mod content_support;
#[path = "../contract_support/mod.rs"]
mod contract_support;
#[path = "../event_eligibility_support/mod.rs"]
mod event_eligibility_support;
#[path = "../fleet_support/mod.rs"]
mod fleet_support;
#[path = "../telemetry_support/mod.rs"]
mod telemetry_support;

mod fleet_command_ledger;
mod fleet_console_command;
mod fleet_dashboard;
mod game_ballistics_catalog_upload;
mod game_ballistics_fire_missions;
mod game_ballistics_migration;
mod runtime_session_fencing;
mod server_machine_credentials;
mod servers_crud;
