//! Route acceptance of the operations ballistics routes: the public ballistics catalog list and
//! version reads, the administrator catalog upload, and the fire-mission save and an event's
//! saved list.
//!
//! One test function per acceptance dimension runs every probe of that dimension over
//! `specs/operations_ballistics.rs` against its own world (`world/operations_ballistics.rs`);
//! the contract parity function re-checks every authorized JSON exchange against its schema and
//! generated type.

mod common;
mod contract_support;
mod route_acceptance_support;

#[path = "route_acceptance_support/world/operations_ballistics.rs"]
mod operations_ballistics_world;

use operations_ballistics_world::OperationsBallisticsWorld;
use route_acceptance_support::dimension_runner::{run_contract_parity, run_dimension};
use route_acceptance_support::spec::Dimension;
use route_acceptance_support::specs::operations_ballistics::specs;

const SUITE: &str = "route_acceptance_operations_ballistics";

async fn run(dimension: Dimension) {
    run_dimension::<OperationsBallisticsWorld>(SUITE, specs(), dimension).await;
}

#[tokio::test]
async fn route_acceptance_operations_ballistics_authorized_callers_get_the_documented_success() {
    run(Dimension::Authorized).await;
}

#[tokio::test]
async fn route_acceptance_operations_ballistics_unauthorized_callers_are_refused() {
    run(Dimension::Unauthorized).await;
}

#[tokio::test]
async fn route_acceptance_operations_ballistics_ownership_is_enforced_or_declared() {
    run(Dimension::Ownership).await;
}

#[tokio::test]
async fn route_acceptance_operations_ballistics_guest_sessions_meet_their_access_class() {
    run(Dimension::Guest).await;
}

#[tokio::test]
async fn route_acceptance_operations_ballistics_ban_revokes_every_session() {
    run(Dimension::Ban).await;
}

#[tokio::test]
async fn route_acceptance_operations_ballistics_malformed_requests_are_refused() {
    run(Dimension::Malformed).await;
}

#[tokio::test]
async fn route_acceptance_operations_ballistics_boundary_values_hold() {
    run(Dimension::Boundary).await;
}

#[tokio::test]
async fn contract_parity_operations_ballistics_responses_match_their_contracts() {
    run_contract_parity::<OperationsBallisticsWorld>(SUITE, specs()).await;
}
