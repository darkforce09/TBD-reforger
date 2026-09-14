//! Captured-response round trips for the dashboard, leaderboards and fire solutions.

use super::*;

#[test]
fn dashboard() {
    // Three nested bodies that are still untyped. The live server status was a fourth and is now
    // typed, so it must not reappear here: if it does, this test has gone blind to the third place
    // telemetry is read.
    assert_golden::<DashboardResponse>(
        golden!("GET__dashboard.json"),
        &["my_assignment", "next_event", "recent_announcements/*"],
    );
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
