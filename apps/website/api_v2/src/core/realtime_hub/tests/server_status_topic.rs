use super::*;

use chrono::Utc;
use uuid::Uuid;

fn sample_status(id: Uuid) -> ServerStatus {
    ServerStatus {
        server_id: id,
        is_online: true,
        player_count: 12,
        max_players: 64,
        server_fps: 58.7,
        uptime_seconds: 100,
        current_match_id: None,
        ingame_time: "06:42".into(),
        ingame_weather: "overcast".into(),
        updated_at: Utc::now(),
    }
}

/// Payload shape pin: the helper puts one JSON shape on the wire — topic `server:{uuid}` and a
/// body `ServerStatus` deserializes — so ingest and the scheduled publisher cannot drift apart.
#[tokio::test]
async fn publish_server_status_matches_ingest_shape() {
    let hub = Hub::new();
    let id = Uuid::parse_str("00000000-0000-4000-d000-000000000001").unwrap();
    let status = sample_status(id);
    let mut rx = hub.subscribe(&format!("server:{id}"));
    publish_server_status(&hub, &status);
    let bytes = rx.recv().await.expect("frame delivered");
    let back: ServerStatus =
        serde_json::from_slice(&bytes).expect("payload must be ServerStatus JSON");
    assert_eq!(back.server_id, id);
    assert!(back.is_online);
    assert_eq!(back.player_count, 12);
    assert_eq!(back.server_fps, 58.7);
    assert_eq!(back.ingame_time, "06:42");
}
