//! Route acceptance of the operations reservation routes: seat registration and withdrawal,
//! leader seat assignment and clearing, squad holds, waitlist promotion, and the game-runtime
//! roster and player deployments.
//!
//! One test function per acceptance dimension runs every probe of that dimension over
//! `specs/operations_reservations.rs` against its own world (`world/operations_reservations.rs`);
//! the contract parity function re-checks every authorized JSON exchange against its schema and
//! generated type.

mod common;
mod contract_support;
mod event_eligibility_support;
mod fleet_support;
mod route_acceptance_support;

#[path = "route_acceptance_support/world/operations_reservations.rs"]
mod operations_reservations_world;

use operations_reservations_world::OperationsReservationsWorld;
use route_acceptance_support::dimension_runner::{run_contract_parity, run_dimension};
use route_acceptance_support::spec::Dimension;
use route_acceptance_support::specs::operations_reservations::specs;

const SUITE: &str = "route_acceptance_operations_reservations";

async fn run(dimension: Dimension) {
    run_dimension::<OperationsReservationsWorld>(SUITE, specs(), dimension).await;
}

#[tokio::test]
async fn route_acceptance_operations_reservations_authorized_callers_get_the_documented_success() {
    run(Dimension::Authorized).await;
}

#[tokio::test]
async fn route_acceptance_operations_reservations_unauthorized_callers_are_refused() {
    run(Dimension::Unauthorized).await;
}

#[tokio::test]
async fn route_acceptance_operations_reservations_ownership_is_enforced_or_declared() {
    run(Dimension::Ownership).await;
}

#[tokio::test]
async fn route_acceptance_operations_reservations_guest_sessions_meet_their_access_class() {
    run(Dimension::Guest).await;
}

#[tokio::test]
async fn route_acceptance_operations_reservations_ban_revokes_every_session() {
    run(Dimension::Ban).await;
}

#[tokio::test]
async fn route_acceptance_operations_reservations_malformed_requests_are_refused() {
    run(Dimension::Malformed).await;
}

#[tokio::test]
async fn route_acceptance_operations_reservations_boundary_values_hold() {
    run(Dimension::Boundary).await;
}

#[tokio::test]
async fn contract_parity_operations_reservations_responses_match_their_contracts() {
    run_contract_parity::<OperationsReservationsWorld>(SUITE, specs()).await;
}
