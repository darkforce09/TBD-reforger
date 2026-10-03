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
    crate::foundation::test_support::class_r_scrub::live_source(
        &crate::foundation::test_support::pins::server_control_source(),
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

/// The list is read through the typed endpoint, which asks `GET /servers` for the rows the
/// picker and the card render.
#[test]
fn loads_servers_via_typed_api() {
    let src = live();
    assert!(
        src.contains("load_servers(self.store)"),
        "the registry must read the list through the typed endpoint on a live path"
    );
    assert_eq!(
        crate::foundation::transport::endpoints::server_registry::servers_path(),
        "/servers"
    );
}

/// The backend serves no RCON route any more, and nothing on this screen calls one: every action on
/// a server is a fleet command or a mission deployment.
#[test]
fn no_rcon_route_is_called_or_served() {
    let routes = crate::foundation::test_support::fixtures::api_route_source();
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
/// shows, from that server's id, and reads that server's row from the registry, so a write updates
/// the card without rebuilding that state.
#[test]
fn every_card_builds_the_state_of_its_own_server() {
    let src = crate::foundation::test_support::class_r_scrub::live_code(
        &crate::foundation::test_support::pins::server_control_source(),
    );
    let card = crate::foundation::test_support::class_r_scrub::only_body(
        &src,
        "pub(super) fn server_detail(",
    );
    for needle in [
        "move |_| registry.row(&id)",
        "CredentialPanel::new(store, id.clone(), name)",
        "CommandConsole::new(store, toasts, id.clone())",
        "DeploymentPanel::new(store, toasts, id.clone())",
        "Signal::derive(move || deployments.latest_confirmed_session())",
        "registry.open_to_edit(edit_id.get_value())",
    ] {
        assert!(card.contains(needle), "the card must build {needle}");
    }
    let board =
        crate::foundation::test_support::class_r_scrub::only_body(&src, "fn control_board(");
    assert!(
        board.contains("Some(id) => server_detail(registry, id)")
            && board.contains(".then_some(id)"),
        "the card is keyed on the selected id alone, so a row write never rebuilds it"
    );
}

/// The card reads the theatre off the server row, and a server between matches reads as a dash.
#[test]
fn the_card_reads_the_terrain_off_the_row() {
    let servers: crate::foundation::transport::dto::DataEnvelope<ServerRowDto> =
        serde_json::from_str(crate::foundation::test_support::fixtures::golden!(
            "GET__servers.json"
        ))
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

/// The queue reading off the captured row: backlog against capacity, and the oldest entry's age
/// in the unit that reads naturally for its size.
#[test]
fn the_band_formats_the_telemetry_queue_reading() {
    let servers: crate::foundation::transport::dto::DataEnvelope<ServerRowDto> =
        serde_json::from_str(crate::foundation::test_support::fixtures::golden!(
            "GET__servers.json"
        ))
        .unwrap();
    let queue = servers.data[0]
        .status
        .as_ref()
        .and_then(|status| status.telemetry_queue.as_ref())
        .expect("the primary reported a queue");
    assert_eq!(queue_fill(queue), "3 / 512");
    assert_eq!(format_queue_age(queue.oldest_age_seconds), "12s");
    assert_eq!(format_queue_age(125), "2m 05s");
    assert_eq!(format_queue_age(3_660), "01h 01m");
    assert!(
        servers.data[1]
            .status
            .as_ref()
            .is_some_and(|status| status.telemetry_queue.is_none()),
        "the secondary never reported a queue"
    );
}

/// A server without a queue reading says so instead of showing zeros, and an inactive server is
/// badged both in the list and in the card header.
#[test]
fn the_card_says_no_reading_and_badges_inactive_servers() {
    let src = live();
    assert!(
        src.contains("\"No reading\""),
        "an absent queue reading renders \"No reading\""
    );
    assert!(
        src.contains("\"Inactive\""),
        "an inactive server carries the badge"
    );
    let code = crate::foundation::test_support::class_r_scrub::live_code(
        &crate::foundation::test_support::pins::server_control_source(),
    );
    let list = crate::foundation::test_support::class_r_scrub::only_body(
        &code,
        "pub(super) fn server_list(",
    );
    let card = crate::foundation::test_support::class_r_scrub::only_body(
        &code,
        "pub(super) fn server_detail(",
    );
    assert!(
        list.contains("inactive_badge(is_active)"),
        "the list badges inactive rows"
    );
    assert!(
        card.contains("inactive_badge(is_active.get())"),
        "the card header badges it too"
    );
    assert!(
        card.contains("map(telemetry_columns)"),
        "the card renders the telemetry band"
    );
}

/// The picker offers the registration sheet at its foot, and an empty fleet offers it in the
/// detail pane with the reason to register one.
#[test]
fn the_picker_and_an_empty_fleet_offer_to_add_a_server() {
    let src = live();
    for needle in [
        "data-testid=\"server-control-add\"",
        "data-testid=\"server-control-add-first\"",
        "\"No servers configured.\"",
        "\"Add server\"",
    ] {
        assert!(src.contains(needle), "the screen must render {needle}");
    }
    let code = crate::foundation::test_support::class_r_scrub::live_code(
        &crate::foundation::test_support::pins::server_control_source(),
    );
    let add =
        crate::foundation::test_support::class_r_scrub::only_body(&code, "fn add_server_control(");
    assert!(add.contains("registry.open_to_register()"));
}
