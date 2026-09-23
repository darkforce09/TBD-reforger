//! The endpoint paths: each one lands on a route the backend registers, and data is encoded.

use super::event_access_administration::*;
use super::event_registration::*;
use super::fleet_commands::*;
use super::fleet_scenarios::*;
use super::machine_credentials::*;
use super::mission_deployments::*;
use super::mission_reviews::*;
use super::*;

/// Every route template the backend's domain route tables register.
fn backend_routes() -> Vec<String> {
    let source = crate::v2::core::test_support::fixtures::api_route_source();
    let mut routes = Vec::new();
    let mut rest = source.as_str();
    while let Some(start) = rest.find("\"/") {
        let tail = &rest[start + 1..];
        let Some(end) = tail.find('"') else { break };
        routes.push(tail[..end].to_string());
        rest = &tail[end..];
    }
    routes
}

/// Whether a concrete request path — its query dropped — fits a route template such as
/// `/events/{id}/groups/{groupId}`, where each `{…}` stands for one non-empty segment.
fn fits(template: &str, path: &str) -> bool {
    let path = path.split('?').next().unwrap_or_default();
    let want: Vec<&str> = template.split('/').collect();
    let got: Vec<&str> = path.split('/').collect();
    want.len() == got.len()
        && want.iter().zip(&got).all(|(w, g)| {
            if w.starts_with('{') && w.ends_with('}') {
                !g.is_empty()
            } else {
                w == g
            }
        })
}

/// Every path this module builds names a route the backend actually serves. A renamed or removed
/// route fails here rather than as a 404 in front of an operator.
#[test]
fn every_endpoint_path_lands_on_a_registered_route() {
    let routes = backend_routes();
    assert!(
        routes.len() > 20,
        "the route tables were not read: {routes:?}"
    );
    let paths = [
        event_access_path("e"),
        access_participants_path("e"),
        event_access_policy_path("e"),
        squad_access_policy_path("m", "BLUFOR", "Alpha 1-1"),
        slot_access_policy_path("m", "s"),
        reservation_quotas_path("e"),
        event_groups_path("e"),
        event_group_path("e", "g"),
        event_group_member_path("e", "g", "000000000000000006"),
        with_expected_revision(&event_group_path("e", "g"), 4),
        mission_registration_path("m"),
        waitlist_promotion_path("m"),
        server_credentials_path("s"),
        credential_revocation_path("s", "c", "Rotated after the host rebuild"),
        mission_submission_path("m"),
        mission_reviews_path("m"),
        review_comments_path("m"),
        mission_artifact_path("m", "a"),
        review_workspace_path("m", "a"),
        approval_path("m"),
        rejection_path("m"),
        server_commands_path("s"),
        server_command_path("s", "c"),
        command_cancellation_path("s", "c"),
        server_deployments_path("s"),
        server_deployment_path("s", "d"),
        deployment_cancellation_path("s", "d"),
        deployable_mission_choices_path(),
        upcoming_operations_path(),
        operation_path("e"),
        fleet_scenarios_path(),
        fleet_scenario_path("arland"),
    ];
    for path in paths {
        assert!(
            routes.iter().any(|route| fits(route, &path)),
            "{path} fits no route the backend registers"
        );
    }
}

/// The matcher itself can say no: a path one segment short, or with a literal segment changed,
/// fits nothing.
#[test]
fn the_route_matcher_refuses_near_misses() {
    assert!(fits("/events/{id}/access", "/events/e/access"));
    assert!(!fits("/events/{id}/access", "/events/e/acess"));
    assert!(!fits("/events/{id}/access", "/events/e"));
    assert!(!fits("/events/{id}/access", "/events//access"));
}

/// Data in a path segment is encoded, so a squad name with a space or a slash still addresses one
/// squad, and the reserved characters of a query value cannot split it.
#[test]
fn path_segments_and_query_values_are_percent_encoded() {
    assert_eq!(encode_path_segment("Alpha-1.2_x~"), "Alpha-1.2_x~");
    assert_eq!(encode_path_segment("Alpha 1/1"), "Alpha%201%2F1");
    assert_eq!(
        encode_path_segment("a+b&c=d?e#f%"),
        "a%2Bb%26c%3Dd%3Fe%23f%25"
    );
    assert_eq!(encode_path_segment("Überwachung"), "%C3%9Cberwachung");
    assert_eq!(
        squad_access_policy_path("m", "BLUFOR", "Alpha 1/1"),
        "/event-missions/m/squads/BLUFOR/Alpha%201%2F1/access-policy"
    );
    assert_eq!(
        credential_revocation_path("s", "c", "Rotated & reissued"),
        "/servers/s/credentials/c?reason=Rotated%20%26%20reissued"
    );
    assert_eq!(
        fleet_scenario_path("arland/../x"),
        "/fleet/scenarios/arland%2F..%2Fx"
    );
    assert_eq!(
        review_workspace_path("m 1", "a/2"),
        "/missions/m%201/artifacts/a%2F2/workspace"
    );
    assert_eq!(
        command_cancellation_path("s", "c?x"),
        "/servers/s/commands/c%3Fx/cancel"
    );
    assert_eq!(
        deployment_cancellation_path("s#", "d"),
        "/servers/s%23/deployments/d/cancel"
    );
}

/// The command, deployment and scenario paths name exactly the routes the server infrastructure and
/// mission route tables register — the parity the removed RCON console path used to be held to.
#[test]
fn fleet_paths_name_the_registered_route_templates() {
    let routes = backend_routes();
    for (path, template) in [
        (server_commands_path("s"), "/servers/{id}/commands"),
        (
            server_command_path("s", "c"),
            "/servers/{id}/commands/{commandId}",
        ),
        (
            command_cancellation_path("s", "c"),
            "/servers/{id}/commands/{commandId}/cancel",
        ),
        (server_deployments_path("s"), "/servers/{id}/deployments"),
        (
            server_deployment_path("s", "d"),
            "/servers/{id}/deployments/{deploymentId}",
        ),
        (
            deployment_cancellation_path("s", "d"),
            "/servers/{id}/deployments/{deploymentId}/cancel",
        ),
        (fleet_scenarios_path(), "/fleet/scenarios"),
        (
            fleet_scenario_path("everon"),
            "/fleet/scenarios/{terrainKey}",
        ),
    ] {
        assert!(
            routes.iter().any(|route| route == template),
            "the backend no longer registers {template}"
        );
        assert!(fits(template, &path), "{path} does not fit {template}");
    }
}

/// A `DELETE` carries its precondition in the query, as the backend's query extractor reads it.
#[test]
fn a_removal_names_its_revision_in_the_query() {
    assert_eq!(
        with_expected_revision(&slot_access_policy_path("m", "s"), 9),
        "/event-missions/m/slots/s/access-policy?expected_access_revision=9"
    );
}

/// A seat is registered by id, and a seatless place by the empty id the backend documents.
#[test]
fn the_registration_body_names_a_seat_or_asks_for_a_seatless_place() {
    assert_eq!(
        registration_body(Some("00000000-0000-4000-5000-000000000006")),
        serde_json::json!({"slot_id": "00000000-0000-4000-5000-000000000006"})
    );
    assert_eq!(registration_body(None), serde_json::json!({"slot_id": ""}));
}
