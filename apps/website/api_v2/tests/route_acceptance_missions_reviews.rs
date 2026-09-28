//! Route acceptance of the mission review and deployment routes: submission, the review thread
//! and its artifacts, the approval queue, the website and in-game deployment routes, and the
//! fleet scenario registry.
//!
//! One test function per acceptance dimension runs every probe of that dimension over
//! `specs/missions_reviews.rs` against its own world (`world/missions_reviews.rs`); the contract
//! parity function re-checks every authorized JSON exchange against its schema and generated
//! type.

mod common;
mod contract_support;
mod route_acceptance_support;

#[path = "route_acceptance_support/world/missions_reviews.rs"]
mod missions_reviews_world;

use missions_reviews_world::MissionsReviewsWorld;
use route_acceptance_support::dimension_runner::{run_contract_parity, run_dimension};
use route_acceptance_support::spec::Dimension;
use route_acceptance_support::specs::missions_reviews::specs;

const SUITE: &str = "route_acceptance_missions_reviews";

async fn run(dimension: Dimension) {
    run_dimension::<MissionsReviewsWorld>(SUITE, specs(), dimension).await;
}

#[tokio::test]
async fn route_acceptance_missions_reviews_authorized_callers_get_the_documented_success() {
    run(Dimension::Authorized).await;
}

#[tokio::test]
async fn route_acceptance_missions_reviews_unauthorized_callers_are_refused() {
    run(Dimension::Unauthorized).await;
}

#[tokio::test]
async fn route_acceptance_missions_reviews_ownership_limits_missions_to_author_and_administrators()
{
    run(Dimension::Ownership).await;
}

#[tokio::test]
async fn route_acceptance_missions_reviews_guest_sessions_meet_their_access_class() {
    run(Dimension::Guest).await;
}

#[tokio::test]
async fn route_acceptance_missions_reviews_ban_revokes_every_session() {
    run(Dimension::Ban).await;
}

#[tokio::test]
async fn route_acceptance_missions_reviews_malformed_requests_are_refused() {
    run(Dimension::Malformed).await;
}

#[tokio::test]
async fn route_acceptance_missions_reviews_boundary_values_hold() {
    run(Dimension::Boundary).await;
}

#[tokio::test]
async fn contract_parity_missions_reviews_responses_match_their_contracts() {
    run_contract_parity::<MissionsReviewsWorld>(SUITE, specs()).await;
}
