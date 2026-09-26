//! Captured-response round trips for the dashboard, leaderboards and fire solutions.

use super::*;

#[test]
fn dashboard() {
    // Three nested bodies that are still untyped. The fleet — every server status with its
    // telemetry queue, and the totals — is typed, so no `fleet` path may appear here: if one
    // does, this test has gone blind to the third place telemetry is read.
    assert_golden::<DashboardResponse>(
        golden!("GET__dashboard.json"),
        &["my_assignment", "next_event", "recent_announcements/*"],
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

#[test]
fn leaderboards() {
    // `data` is `Vec<Value>` — the envelope is proven, the row is not.
    assert_golden::<Leaderboard>(golden!("GET__leaderboards.json"), &["data/*"]);
}

/// A live firing-solution response, captured for a target a kilometre due north. It pins the
/// integer fields: a float distance deserialises happily and only fails on the way back out.
#[test]
fn fire_solution() {
    const G: &str = golden!("POST__fire-missions__solve.json");
    assert_golden::<FireSolution>(G, &[]);
    let sol: FireSolution = serde_json::from_str(G).unwrap();
    assert_eq!(sol.weapon_system, "M252 81mm");
    assert_eq!(sol.distance_m, 1000);
    assert_eq!(sol.azimuth_mils, 0);
    assert_eq!(sol.charge, 1);
    assert!(sol.elevation_mils > 800, "high-angle solution");
    assert!(
        sol.extra.is_empty(),
        "every wire key must be a named field, not absorbed by extra"
    );
}
