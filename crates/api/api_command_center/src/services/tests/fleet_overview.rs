use chrono::Utc;
use uuid::Uuid;

use super::{FleetServer, FleetTotals, fleet_totals};
use api_server_infrastructure::models::server::{ServerStatus, TelemetryQueueStatus};

fn status(online: bool, players: i64, capacity: i64, queue: Option<(i64, i64)>) -> ServerStatus {
    ServerStatus {
        server_id: Uuid::new_v4().into(),
        is_online: online,
        player_count: players,
        max_players: capacity,
        server_fps: 50.0,
        uptime_seconds: 10,
        current_match_id: None,
        ingame_time: String::new(),
        ingame_weather: String::new(),
        updated_at: Utc::now(),
        telemetry_queue: queue.map(|(backlog, dropped_total)| TelemetryQueueStatus {
            backlog,
            capacity: 512,
            dropped_total,
            oldest_age_seconds: 4,
            reported_at: Utc::now(),
        }),
    }
}

fn server(status: Option<ServerStatus>) -> FleetServer {
    FleetServer {
        server_id: Uuid::new_v4().into(),
        name: "s".into(),
        status,
    }
}

#[test]
fn totals_count_every_configured_server_and_only_online_capacity() {
    let servers = vec![
        server(Some(status(true, 40, 64, Some((3, 1))))),
        server(Some(status(false, 9, 48, Some((5, 2))))),
        server(None),
    ];
    assert_eq!(
        fleet_totals(&servers),
        FleetTotals {
            configured: 3,
            online: 1,
            players: 40,
            max_players: 64,
            telemetry_backlog: 8,
            telemetry_dropped_total: 3,
        }
    );
}

#[test]
fn an_empty_fleet_has_zero_totals() {
    assert_eq!(fleet_totals(&[]), FleetTotals::default());
}
