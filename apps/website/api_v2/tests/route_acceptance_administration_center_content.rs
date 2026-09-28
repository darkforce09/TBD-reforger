//! Route acceptance of the administration, command center and community content routes: the
//! personnel, discipline, membership grace, role resync and audit log routes, the dashboard,
//! leaderboards and player statistics, the announcement, upload, modpack, vehicle database and
//! wiki routes, and the development-only equipment data viewer reads.
//!
//! One test function per acceptance dimension runs every probe of that dimension over
//! `specs/administration_center_content.rs` against its own world
//! (`world/administration_center_content.rs`); the contract parity function re-checks every
//! authorized JSON exchange against its schema and generated type.

mod common;
mod content_support;
mod contract_support;
mod route_acceptance_support;
mod wiki_support;

#[path = "route_acceptance_support/world/administration_center_content.rs"]
mod administration_center_content_world;

use administration_center_content_world::AdministrationCenterContentWorld;
use route_acceptance_support::dimension_runner::{run_contract_parity, run_dimension};
use route_acceptance_support::spec::Dimension;
use route_acceptance_support::specs::administration_center_content::specs;

const SUITE: &str = "route_acceptance_administration_center_content";

async fn run(dimension: Dimension) {
    run_dimension::<AdministrationCenterContentWorld>(SUITE, specs(), dimension).await;
}

#[tokio::test]
async fn route_acceptance_administration_center_content_authorized_callers_get_the_documented_success()
 {
    run(Dimension::Authorized).await;
}

#[tokio::test]
async fn route_acceptance_administration_center_content_unauthorized_callers_are_refused() {
    run(Dimension::Unauthorized).await;
}

#[tokio::test]
async fn route_acceptance_administration_center_content_ownership_is_declared_for_every_route() {
    run(Dimension::Ownership).await;
}

#[tokio::test]
async fn route_acceptance_administration_center_content_guest_sessions_meet_their_access_class() {
    run(Dimension::Guest).await;
}

#[tokio::test]
async fn route_acceptance_administration_center_content_ban_revokes_every_session() {
    run(Dimension::Ban).await;
}

#[tokio::test]
async fn route_acceptance_administration_center_content_malformed_requests_are_refused() {
    run(Dimension::Malformed).await;
}

#[tokio::test]
async fn route_acceptance_administration_center_content_boundary_values_hold() {
    run(Dimension::Boundary).await;
}

#[tokio::test]
async fn contract_parity_administration_center_content_responses_match_their_contracts() {
    run_contract_parity::<AdministrationCenterContentWorld>(SUITE, specs()).await;
}
