//! Guards on the server control screen: the server list it reads, the card's readings, and the
//! state each card builds for the server it shows.
//!
//! The fleet command console, the deployments panel and the scenario registry carry their own
//! tests beside them; the command paths' parity with the backend's route table lives with the
//! endpoint tests.

use super::*;

/// The shipped half of this page, scrubbed of comments and unreachable branches.
///
/// String literals survive the scrub: a route path and a `data-testid` are the contract here, not
/// a mention of one.
fn live() -> String {
    crate::v2::core::test_support::class_r_scrub::live_source(
        &crate::v2::core::test_support::pins::server_control_source(),
    )
}

/// No compile-time server table and no fabricated console transcript.
#[test]
fn no_mock_servers_or_fabricated_console() {
    // The needles are assembled at runtime so the pin cannot go green off this test's own
    // literals. Belt and braces: the scrub already cuts the test module out of `live()`.
    let src = live();
    let mock = format!("{}{}", "MOCK_", "SERVERS");
    let fake_listener = format!("{}{}", "RCON listener bound", " to 0.0.0.0:19999");
    let fake_init = format!("{}{}", "Server initialized on ", "Everon");
    assert!(
        !src.contains(&mock),
        "compile-time mock server table must be gone (perturbation: reintroduce it)"
    );
    assert!(
        !src.contains(&fake_listener) && !src.contains(&fake_init),
        "fabricated console log lines must be gone"
    );
}

#[test]
fn loads_servers_via_typed_api() {
    let src = live();
    let get = format!("{}{}", "api_get", "::<DataEnvelope<ServerRowDto>>");
    assert!(
        src.contains(&get) && src.contains(r#""/servers""#),
        "page must GET /servers as DataEnvelope<ServerRowDto> on a live path"
    );
}

/// The backend serves no RCON route any more, and nothing on this screen calls one: every action on
/// a server is a fleet command or a mission deployment.
#[test]
fn no_rcon_route_is_called_or_served() {
    let routes = crate::v2::core::test_support::fixtures::api_route_source();
    let rcon = format!("{}{}", "/rc", "on");
    assert!(
        !routes.contains(&format!("{rcon}\"")),
        "the backend route tables register no RCON route"
    );
    let src = live();
    assert!(
        !src.contains(&rcon),
        "the screen must not call an RCON route"
    );
    for call in [
        "request_fleet_command(",
        "load_fleet_command(",
        "request_mission_deployment(",
        "load_mission_deployment(",
    ] {
        assert!(
            src.contains(call),
            "the screen must reach the backend through {call}"
        );
    }
}

/// Each card builds the console, the deployments panel and the credential sheet of the server it
/// shows, from that server's id.
#[test]
fn every_card_builds_the_state_of_its_own_server() {
    let src = crate::v2::core::test_support::class_r_scrub::live_code(
        &crate::v2::core::test_support::pins::server_control_source(),
    );
    let card = crate::v2::core::test_support::class_r_scrub::only_body(
        &src,
        "pub(super) fn server_detail(",
    );
    for needle in [
        "CredentialPanel::new(store, s.id.clone(), name.clone())",
        "CommandConsole::new(store, toasts, s.id.clone())",
        "DeploymentPanel::new(store, toasts, s.id.clone())",
        "Signal::derive(move || deployments.latest_confirmed_session())",
    ] {
        assert!(card.contains(needle), "the card must build {needle}");
    }
}

/// The card reads the theatre off the server row, and a server between matches reads as a dash.
#[test]
fn the_card_reads_the_terrain_off_the_row() {
    let servers: crate::v2::core::api::dto::DataEnvelope<ServerRowDto> = serde_json::from_str(
        crate::v2::core::test_support::fixtures::golden!("GET__servers.json"),
    )
    .unwrap();
    assert_eq!(terrain_reading(&servers.data[0]), "Everon");
    assert_eq!(terrain_reading(&servers.data[1]), "—");
}

#[test]
fn pick_default_prefers_active() {
    let inactive = ServerRowDto {
        id: "a".into(),
        name: "A".into(),
        ip: "1.1.1.1".into(),
        port: 1,
        required_modpack_id: None,
        is_active: false,
        status: None,
        required_modpack: None,
        terrain: None,
    };
    let mut active = inactive.clone();
    active.id = "b".into();
    active.is_active = true;
    assert_eq!(
        pick_default_id(&[inactive.clone(), active.clone()]).as_deref(),
        Some("b")
    );
    assert_eq!(pick_default_id(&[inactive]).as_deref(), Some("a"));
    assert_eq!(pick_default_id(&[]), None);
}
