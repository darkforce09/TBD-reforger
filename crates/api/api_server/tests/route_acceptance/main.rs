//! Route acceptance: every route of the API, part by part, answers its authorized caller with
//! the documented success and contract and refuses its unauthorized callers; and the debug
//! routes exist only in a development router.
//!
//! Each part runs one test function per dimension over `specs/<part>.rs` against its own world
//! (`world/<part>.rs`), built in its own namespace of the binary's one database.

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
#[path = "../wiki_support/mod.rs"]
mod wiki_support;

mod route_acceptance_support;

mod debug_routes;

use route_acceptance_support::dimension_runner::run_dimension;
use route_acceptance_support::spec::Dimension;
use route_acceptance_support::specs;
use route_acceptance_support::world::{
    administration_center_content::AdministrationCenterContentWorld,
    fleet_and_telemetry::FleetAndTelemetryWorld, identity_and_core::IdentityAndCoreWorld,
    missions_library::MissionsLibraryWorld, missions_reviews::MissionsReviewsWorld,
    operations_ballistics::OperationsBallisticsWorld, operations_events::OperationsEventsWorld,
    operations_reservations::OperationsReservationsWorld,
};

#[tokio::test]
async fn identity_and_core_authorized_callers_get_the_documented_success() {
    run_dimension::<IdentityAndCoreWorld>(
        "route_acceptance_identity_and_core",
        0,
        specs::identity_and_core::specs(),
        Dimension::Authorized,
    )
    .await;
}

#[tokio::test]
async fn identity_and_core_unauthorized_callers_are_refused() {
    run_dimension::<IdentityAndCoreWorld>(
        "route_acceptance_identity_and_core",
        0,
        specs::identity_and_core::specs(),
        Dimension::Unauthorized,
    )
    .await;
}
#[tokio::test]
async fn operations_events_authorized_callers_get_the_documented_success() {
    run_dimension::<OperationsEventsWorld>(
        "route_acceptance_operations_events",
        1,
        specs::operations_events::specs(),
        Dimension::Authorized,
    )
    .await;
}

#[tokio::test]
async fn operations_events_unauthorized_callers_are_refused() {
    run_dimension::<OperationsEventsWorld>(
        "route_acceptance_operations_events",
        1,
        specs::operations_events::specs(),
        Dimension::Unauthorized,
    )
    .await;
}
#[tokio::test]
async fn operations_reservations_authorized_callers_get_the_documented_success() {
    run_dimension::<OperationsReservationsWorld>(
        "route_acceptance_operations_reservations",
        2,
        specs::operations_reservations::specs(),
        Dimension::Authorized,
    )
    .await;
}

#[tokio::test]
async fn operations_reservations_unauthorized_callers_are_refused() {
    run_dimension::<OperationsReservationsWorld>(
        "route_acceptance_operations_reservations",
        2,
        specs::operations_reservations::specs(),
        Dimension::Unauthorized,
    )
    .await;
}
#[tokio::test]
async fn operations_ballistics_authorized_callers_get_the_documented_success() {
    run_dimension::<OperationsBallisticsWorld>(
        "route_acceptance_operations_ballistics",
        3,
        specs::operations_ballistics::specs(),
        Dimension::Authorized,
    )
    .await;
}

#[tokio::test]
async fn operations_ballistics_unauthorized_callers_are_refused() {
    run_dimension::<OperationsBallisticsWorld>(
        "route_acceptance_operations_ballistics",
        3,
        specs::operations_ballistics::specs(),
        Dimension::Unauthorized,
    )
    .await;
}
#[tokio::test]
async fn missions_library_authorized_callers_get_the_documented_success() {
    run_dimension::<MissionsLibraryWorld>(
        "route_acceptance_missions_library",
        4,
        specs::missions_library::specs(),
        Dimension::Authorized,
    )
    .await;
}

#[tokio::test]
async fn missions_library_unauthorized_callers_are_refused() {
    run_dimension::<MissionsLibraryWorld>(
        "route_acceptance_missions_library",
        4,
        specs::missions_library::specs(),
        Dimension::Unauthorized,
    )
    .await;
}
#[tokio::test]
async fn missions_reviews_authorized_callers_get_the_documented_success() {
    run_dimension::<MissionsReviewsWorld>(
        "route_acceptance_missions_reviews",
        5,
        specs::missions_reviews::specs(),
        Dimension::Authorized,
    )
    .await;
}

#[tokio::test]
async fn missions_reviews_unauthorized_callers_are_refused() {
    run_dimension::<MissionsReviewsWorld>(
        "route_acceptance_missions_reviews",
        5,
        specs::missions_reviews::specs(),
        Dimension::Unauthorized,
    )
    .await;
}
#[tokio::test]
async fn fleet_and_telemetry_authorized_callers_get_the_documented_success() {
    run_dimension::<FleetAndTelemetryWorld>(
        "route_acceptance_fleet_and_telemetry",
        6,
        specs::fleet_and_telemetry::specs(),
        Dimension::Authorized,
    )
    .await;
}

#[tokio::test]
async fn fleet_and_telemetry_unauthorized_callers_are_refused() {
    run_dimension::<FleetAndTelemetryWorld>(
        "route_acceptance_fleet_and_telemetry",
        6,
        specs::fleet_and_telemetry::specs(),
        Dimension::Unauthorized,
    )
    .await;
}
#[tokio::test]
async fn administration_center_content_authorized_callers_get_the_documented_success() {
    run_dimension::<AdministrationCenterContentWorld>(
        "route_acceptance_administration_center_content",
        7,
        specs::administration_center_content::specs(),
        Dimension::Authorized,
    )
    .await;
}

#[tokio::test]
async fn administration_center_content_unauthorized_callers_are_refused() {
    run_dimension::<AdministrationCenterContentWorld>(
        "route_acceptance_administration_center_content",
        7,
        specs::administration_center_content::specs(),
        Dimension::Unauthorized,
    )
    .await;
}
