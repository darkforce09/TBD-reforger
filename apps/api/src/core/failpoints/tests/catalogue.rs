//! Unit coverage for the failpoint catalogue: exactly the named points in declaration order,
//! unique names equal to the variant identifiers, and the two unit-test points kept out of it.

use std::collections::HashSet;

use super::*;

/// The catalogue the failure and race suites rely on, in declaration order.
const EXPECTED_NAMES: [&str; 18] = [
    "SessionRotationBeforeCommit",
    "SessionRotationAfterCommit",
    "SessionLogoutBeforeCommit",
    "ReservationClaimBeforeCommit",
    "ReservationClaimAfterCommit",
    "ReviewDecisionBeforeCommit",
    "DeploymentRequestBeforeCommit",
    "DeploymentRequestAfterCommit",
    "ResultsRevisionBeforeCommit",
    "ResultsRevisionAfterCommit",
    "FleetCommandClaimAfterCommit",
    "FleetCommandResultBeforeCommit",
    "FleetCommandResultAfterCommit",
    "AuditPublicationBeforeCommit",
    "AuditDeliveryRead",
    "DiscordRoleSyncBeforeEffect",
    "DiscordRoleSyncAfterEffect",
    "IdentityLinkConfirmBeforeCommit",
];

#[test]
fn failpoints_catalogue_holds_exactly_the_named_points_in_order() {
    let names: Vec<&str> = CATALOGUE.iter().map(|failpoint| failpoint.name()).collect();
    assert_eq!(names, EXPECTED_NAMES);
}

#[test]
fn failpoints_catalogue_names_are_unique_and_equal_the_variant_identifiers() {
    let unique: HashSet<&str> = CATALOGUE.iter().map(|failpoint| failpoint.name()).collect();
    assert_eq!(
        unique.len(),
        CATALOGUE.len(),
        "every catalogue name is unique"
    );
    for failpoint in CATALOGUE {
        assert_eq!(failpoint.name(), format!("{failpoint:?}"));
    }
}

#[test]
fn failpoints_catalogue_leaves_out_the_unit_test_points() {
    for unit_test_point in [
        Failpoint::RegistryUnitTestFirst,
        Failpoint::RegistryUnitTestSecond,
    ] {
        assert!(
            !CATALOGUE.contains(&unit_test_point),
            "{} is not a catalogue entry",
            unit_test_point.name()
        );
        assert_eq!(unit_test_point.name(), format!("{unit_test_point:?}"));
    }
}
