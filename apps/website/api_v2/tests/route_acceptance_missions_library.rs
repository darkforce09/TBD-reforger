//! Route acceptance of the missions library routes: the mission library and bookmarks, mission
//! creation, metadata, deletion and export, versions, the armory, the review workspace, the
//! default-override report, the faction library and the registry.
//!
//! One test function per acceptance dimension runs every probe of that dimension over
//! `specs/missions_library.rs` against its own world (`world/missions_library.rs`); the
//! contract parity function re-checks every authorized JSON exchange against its schema and
//! generated type.

mod common;
mod contract_support;
mod route_acceptance_support;

#[path = "route_acceptance_support/world/missions_library.rs"]
mod missions_library_world;

use missions_library_world::MissionsLibraryWorld;
use route_acceptance_support::dimension_runner::{run_contract_parity, run_dimension};
use route_acceptance_support::spec::Dimension;
use route_acceptance_support::specs::missions_library::specs;

const SUITE: &str = "route_acceptance_missions_library";

async fn run(dimension: Dimension) {
    run_dimension::<MissionsLibraryWorld>(SUITE, specs(), dimension).await;
}

#[tokio::test]
async fn route_acceptance_missions_library_authorized_callers_get_the_documented_success() {
    run(Dimension::Authorized).await;
}

#[tokio::test]
async fn route_acceptance_missions_library_unauthorized_callers_are_refused() {
    run(Dimension::Unauthorized).await;
}

#[tokio::test]
async fn route_acceptance_missions_library_ownership_is_declared_for_every_route() {
    run(Dimension::Ownership).await;
}

#[tokio::test]
async fn route_acceptance_missions_library_guest_sessions_meet_their_access_class() {
    run(Dimension::Guest).await;
}

#[tokio::test]
async fn route_acceptance_missions_library_ban_revokes_every_session() {
    run(Dimension::Ban).await;
}

#[tokio::test]
async fn route_acceptance_missions_library_malformed_requests_are_refused() {
    run(Dimension::Malformed).await;
}

#[tokio::test]
async fn route_acceptance_missions_library_boundary_values_hold() {
    run(Dimension::Boundary).await;
}

#[tokio::test]
async fn contract_parity_missions_library_responses_match_their_contracts() {
    run_contract_parity::<MissionsLibraryWorld>(SUITE, specs()).await;
}
