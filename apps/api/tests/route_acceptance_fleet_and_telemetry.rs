//! Route acceptance of the fleet and telemetry routes: the server registry and its status
//! stream, machine credentials, the fleet command ledger and its executor routes, game-runtime
//! sessions and heartbeats, the `/ingest/*` match telemetry routes and the match event read.
//!
//! One test function per acceptance dimension runs every probe of that dimension over
//! `specs/fleet_and_telemetry.rs` against its own world (`world/fleet_and_telemetry.rs`); the
//! contract parity function re-checks every authorized JSON exchange against its schema and
//! generated type.

mod common;
mod contract_support;
mod route_acceptance_support;
mod telemetry_support;

#[path = "route_acceptance_support/world/fleet_and_telemetry.rs"]
mod fleet_and_telemetry_world;

use fleet_and_telemetry_world::FleetAndTelemetryWorld;
use route_acceptance_support::dimension_runner::{run_contract_parity, run_dimension};
use route_acceptance_support::spec::Dimension;
use route_acceptance_support::specs::fleet_and_telemetry::specs;

const SUITE: &str = "route_acceptance_fleet_and_telemetry";

async fn run(dimension: Dimension) {
    run_dimension::<FleetAndTelemetryWorld>(SUITE, specs(), dimension).await;
}

#[tokio::test]
async fn route_acceptance_fleet_and_telemetry_authorized_callers_get_the_documented_success() {
    run(Dimension::Authorized).await;
}

#[tokio::test]
async fn route_acceptance_fleet_and_telemetry_unauthorized_callers_are_refused() {
    run(Dimension::Unauthorized).await;
}

#[tokio::test]
async fn route_acceptance_fleet_and_telemetry_ownership_is_declared_for_every_route() {
    run(Dimension::Ownership).await;
}

#[tokio::test]
async fn route_acceptance_fleet_and_telemetry_guest_sessions_meet_their_access_class() {
    run(Dimension::Guest).await;
}

#[tokio::test]
async fn route_acceptance_fleet_and_telemetry_ban_revokes_every_session() {
    run(Dimension::Ban).await;
}

#[tokio::test]
async fn route_acceptance_fleet_and_telemetry_malformed_requests_are_refused() {
    run(Dimension::Malformed).await;
}

#[tokio::test]
async fn route_acceptance_fleet_and_telemetry_boundary_values_hold() {
    run(Dimension::Boundary).await;
}

#[tokio::test]
async fn contract_parity_fleet_and_telemetry_responses_match_their_contracts() {
    run_contract_parity::<FleetAndTelemetryWorld>(SUITE, specs()).await;
}
