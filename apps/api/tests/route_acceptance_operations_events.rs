//! Route acceptance of the operations event routes: events and their mission attachments,
//! event access administration (policies, reservation quotas, groups and rosters), the ORBAT
//! read, the member directory and the leave review console.
//!
//! One test function per acceptance dimension runs every probe of that dimension over
//! `specs/operations_events.rs` against its own world (`world/operations_events.rs`); the
//! contract parity function re-checks every authorized JSON exchange against its schema and
//! generated type.

mod common;
mod contract_support;
mod route_acceptance_support;

#[path = "route_acceptance_support/world/operations_events.rs"]
mod operations_events_world;

use operations_events_world::OperationsEventsWorld;
use route_acceptance_support::dimension_runner::{run_contract_parity, run_dimension};
use route_acceptance_support::spec::Dimension;
use route_acceptance_support::specs::operations_events::specs;

const SUITE: &str = "route_acceptance_operations_events";

async fn run(dimension: Dimension) {
    run_dimension::<OperationsEventsWorld>(SUITE, specs(), dimension).await;
}

#[tokio::test]
async fn route_acceptance_operations_events_authorized_callers_get_the_documented_success() {
    run(Dimension::Authorized).await;
}

#[tokio::test]
async fn route_acceptance_operations_events_unauthorized_callers_are_refused() {
    run(Dimension::Unauthorized).await;
}

#[tokio::test]
async fn route_acceptance_operations_events_ownership_is_declared_for_every_route() {
    run(Dimension::Ownership).await;
}

#[tokio::test]
async fn route_acceptance_operations_events_guest_sessions_meet_their_access_class() {
    run(Dimension::Guest).await;
}

#[tokio::test]
async fn route_acceptance_operations_events_ban_revokes_every_session() {
    run(Dimension::Ban).await;
}

#[tokio::test]
async fn route_acceptance_operations_events_malformed_requests_are_refused() {
    run(Dimension::Malformed).await;
}

#[tokio::test]
async fn route_acceptance_operations_events_boundary_values_hold() {
    run(Dimension::Boundary).await;
}

#[tokio::test]
async fn contract_parity_operations_events_responses_match_their_contracts() {
    run_contract_parity::<OperationsEventsWorld>(SUITE, specs()).await;
}
