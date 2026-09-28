//! Captured-response round trips for the dashboard, leaderboards and fire solutions.

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

/// The saved fire missions of one event. The capture holds a row stored before the charge,
/// azimuth in mils and flight time were recorded (all three `null`) and one that records them, so
/// both arms of the nullable figures round-trip as sent.
#[test]
fn saved_fire_missions_of_an_event() {
    const G: &str =
        golden!("GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__fire-missions.json");
    assert_golden::<DataEnvelope<SavedFire>>(G, &[]);
    let list: DataEnvelope<SavedFire> = serde_json::from_str(G).unwrap();
    assert!(list
        .data
        .iter()
        .all(|row| row.event_id.as_deref() == Some("c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7")));
    assert!(list.data.iter().any(|row| row.charge.is_none()
        && row.azimuth_mils.is_none()
        && row.time_of_flight_s.is_none()));
    assert!(list.data.iter().any(|row| row.charge == Some(1)
        && row.azimuth_mils == Some(0)
        && row.time_of_flight_s == Some(18.3)));
}

/// A fire mission saved with no event carries no `event_id` key, and it round-trips that way.
#[test]
fn a_fire_mission_saved_without_an_event_has_no_event_id_key() {
    let list: Value = serde_json::from_str(golden!(
        "GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__fire-missions.json"
    ))
    .unwrap();
    let mut row = list["data"][1].clone();
    row.as_object_mut().unwrap().remove("event_id");
    let wire = row.to_string();
    assert_golden::<SavedFire>(&wire, &[]);
    let decoded: SavedFire = serde_json::from_str(&wire).unwrap();
    assert!(decoded.event_id.is_none());
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

/// The answer to saving a fire mission: the solution and the stored row. The mortar page decodes
/// it through a private pairing of these two DTOs, so the round trip holds the same pairing here.
#[derive(Serialize, Deserialize)]
struct SavedFireAnswer {
    solution: FireSolution,
    fire_mission: SavedFire,
}

#[test]
fn fire_mission_saved() {
    const G: &str = golden!("POST__fire-missions.json");
    assert_golden::<SavedFireAnswer>(G, &[]);
    let saved: SavedFireAnswer = serde_json::from_str(G).unwrap();
    assert_eq!(saved.fire_mission.charge, Some(saved.solution.charge));
}
