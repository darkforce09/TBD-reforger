//! Integration tests of match telemetry and member statistics: result revisions, detailed
//! event batches, player counters, service records, leaderboards and the dashboard reads.
//!
//! One test binary on one database: every module shares the database
//! [`common::require_test_database_url`] provisions once and isolates its tests through rows
//! and ids it mints for itself.

#[path = "../common/mod.rs"]
mod common;
#[path = "../contract_support/mod.rs"]
mod contract_support;
#[path = "../telemetry_support/mod.rs"]
mod telemetry_support;

mod detailed_events;
mod telemetry_player_counters;
