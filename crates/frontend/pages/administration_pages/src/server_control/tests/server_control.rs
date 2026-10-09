//! Guards on the server control screen: the card's terrain and telemetry readings and the server
//! it picks by default.

use super::*;

/// The card reads the theatre off the server row, and a server between matches reads as a dash.
#[test]
fn the_card_reads_the_terrain_off_the_row() {
    let servers: frontend_api_dtos::DataEnvelope<ServerRowDto> = serde_json::from_str(
        frontend_test_support::fixtures::golden!("GET__servers.json"),
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

/// The queue reading off the captured row: backlog against capacity, and the oldest entry's age
/// in the unit that reads naturally for its size.
#[test]
fn the_band_formats_the_telemetry_queue_reading() {
    let servers: frontend_api_dtos::DataEnvelope<ServerRowDto> = serde_json::from_str(
        frontend_test_support::fixtures::golden!("GET__servers.json"),
    )
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
