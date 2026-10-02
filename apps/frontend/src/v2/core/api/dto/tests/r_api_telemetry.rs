//! Captured-response round trips for the dashboard and the leaderboards.

use super::*;

#[test]
fn dashboard() {
    // Two nested bodies that are still untyped. The fleet — every server status with its
    // telemetry queue, and the totals — is typed, so no `fleet` path may appear here: if one
    // does, this test has gone blind to the third place telemetry is read. The announcements are
    // typed `Announcement` rows, so no `recent_announcements` path may appear either.
    assert_golden::<DashboardResponse>(
        golden!("GET__dashboard.json"),
        &["my_assignment", "next_event"],
    );
}

/// The captured fleet: the one active server with its queue reading, and the backend's totals.
/// The inactive servers of the seed are not in it.
#[test]
fn dashboard_fleet_lists_the_active_servers_with_their_totals() {
    let dashboard: DashboardResponse =
        serde_json::from_str(golden!("GET__dashboard.json")).unwrap();
    let fleet = dashboard.fleet;
    assert_eq!(fleet.servers.len(), 1);
    let server = &fleet.servers[0];
    assert_eq!(server.server_id, "00000000-0000-4000-d000-000000000001");
    let status = server
        .status
        .as_ref()
        .expect("the active server has a status");
    let queue = status
        .telemetry_queue
        .as_ref()
        .expect("the active server reported a queue");
    assert_eq!(
        (queue.backlog, queue.capacity, queue.dropped_total),
        (3, 512, 0)
    );
    assert_eq!(
        fleet.totals,
        FleetTotalsDto {
            configured: 1,
            online: 1,
            players: 47,
            max_players: 64,
            telemetry_backlog: 3,
            telemetry_dropped_total: 0,
        }
    );
}

/// A fleet server without a status row has no `status` key, and it must round-trip that way.
#[test]
fn a_fleet_server_without_a_status_has_no_status_key() {
    let wire = r#"{"name":"Staging","server_id":"s3"}"#;
    assert_golden::<FleetServerDto>(wire, &[]);
    let server: FleetServerDto = serde_json::from_str(wire).unwrap();
    assert!(server.status.is_none());
}

/// Every key of every leaderboard row is a named `LeaderboardRow` field, and the rows arrive
/// ranked from one.
#[test]
fn leaderboards() {
    const G: &str = golden!("GET__leaderboards.json");
    assert_golden::<Leaderboard>(G, &[]);
    let board: Leaderboard = serde_json::from_str(G).unwrap();
    let ranks: Vec<i64> = board.data.iter().map(|row| row.rank).collect();
    assert_eq!(ranks, (1..=board.data.len() as i64).collect::<Vec<_>>());
    assert!(board.data.iter().all(|row| row.kd_ratio.is_some()));
}

/// A member with no measured death count has a `null` K/D ratio, and it round-trips as an
/// explicit `null` rather than a dropped key or a zero.
#[test]
fn a_leaderboard_row_without_a_measured_kd_ratio_keeps_its_null() {
    let mut board: Value = serde_json::from_str(golden!("GET__leaderboards.json")).unwrap();
    board["data"][0]["kd_ratio"] = Value::Null;
    let wire = board.to_string();
    assert_golden::<Leaderboard>(&wire, &[]);
    let decoded: Leaderboard = serde_json::from_str(&wire).unwrap();
    assert_eq!(decoded.data[0].kd_ratio, None);
}
