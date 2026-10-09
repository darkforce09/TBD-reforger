//! Route acceptance of the identity and core routes: the router's own routes (`/healthz`,
//! `/metrics`, `/uploads`, `/map-assets`), the `/auth/*` routes, every `/me*` route and the
//! link confirmation.
//!
//! One test function per acceptance dimension runs every probe of that dimension over
//! `specs/identity_and_core.rs` against its own world (`world/identity_and_core.rs`); the
//! contract parity function re-checks every authorized JSON exchange against its schema and
//! generated type. This binary is the reference layout for the other parts' binaries.

mod common;
mod contract_support;
mod route_acceptance_support;

#[path = "route_acceptance_support/world/identity_and_core.rs"]
mod identity_and_core_world;

use identity_and_core_world::IdentityAndCoreWorld;
use route_acceptance_support::dimension_runner::{run_contract_parity, run_dimension};
use route_acceptance_support::spec::Dimension;
use route_acceptance_support::specs::identity_and_core::specs;

const SUITE: &str = "route_acceptance_identity_and_core";

async fn run(dimension: Dimension) {
    run_dimension::<IdentityAndCoreWorld>(SUITE, specs(), dimension).await;
}

#[tokio::test]
async fn route_acceptance_identity_and_core_authorized_callers_get_the_documented_success() {
    run(Dimension::Authorized).await;
}

#[tokio::test]
async fn route_acceptance_identity_and_core_unauthorized_callers_are_refused() {
    run(Dimension::Unauthorized).await;
}

#[tokio::test]
async fn route_acceptance_identity_and_core_ownership_is_declared_for_every_route() {
    run(Dimension::Ownership).await;
}

#[tokio::test]
async fn route_acceptance_identity_and_core_guest_sessions_meet_their_access_class() {
    run(Dimension::Guest).await;
}

#[tokio::test]
async fn route_acceptance_identity_and_core_ban_revokes_every_session() {
    run(Dimension::Ban).await;
}

#[tokio::test]
async fn route_acceptance_identity_and_core_malformed_requests_are_refused() {
    run(Dimension::Malformed).await;
}

#[tokio::test]
async fn route_acceptance_identity_and_core_boundary_values_hold() {
    run(Dimension::Boundary).await;
}

#[tokio::test]
async fn contract_parity_identity_and_core_responses_match_their_contracts() {
    run_contract_parity::<IdentityAndCoreWorld>(SUITE, specs()).await;
}
