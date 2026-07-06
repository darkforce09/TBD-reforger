//! Mortar ballistics — Rust port of `services/mortar.go`. High-angle firing solution
//! selecting the lowest charge that reaches the target.

use std::f64::consts::PI;

use serde::Serialize;

/// Computed firing data for a mortar fire mission (snake_case wire).
#[derive(Debug, Clone, Serialize)]
pub struct FireSolution {
    pub weapon_system: String,
    pub distance_m: i64,
    pub azimuth_deg: f64,
    pub azimuth_mils: i64,
    pub elevation_mils: i64,
    pub charge: i64,
    pub time_of_flight_s: f64,
}

/// Used when an unknown weapon is requested.
pub const DEFAULT_MORTAR: &str = "M252 81mm";

const GRAVITY: f64 = 9.80665;
const MILS_PER_CIRCLE: f64 = 6400.0;

/// Per-ring muzzle velocities (m/s) for a simplified projectile model.
fn charges_for(weapon: &str) -> Option<&'static [f64]> {
    match weapon {
        "M252 81mm" | "M821 81mm" => Some(&[70.0, 105.0, 150.0, 210.0, 270.0]),
        "2B14 82mm" => Some(&[65.0, 100.0, 145.0, 200.0, 255.0]),
        "M120 120mm" => Some(&[110.0, 170.0, 230.0, 318.0]),
        _ => None,
    }
}

/// Compute the high-angle solution from a firing position to a target (flat
/// game-world meters, x=east, y=north). Selects the lowest charge that can reach;
/// `false` if beyond max range for every charge.
pub fn solve_fire_mission(
    weapon: &str,
    fp_x: f64,
    fp_y: f64,
    tgt_x: f64,
    tgt_y: f64,
) -> (FireSolution, bool) {
    let (weapon, charges) = match charges_for(weapon) {
        Some(c) => (weapon.to_string(), c),
        None => (
            DEFAULT_MORTAR.to_string(),
            charges_for(DEFAULT_MORTAR).expect("default charges"),
        ),
    };

    let dx = tgt_x - fp_x;
    let dy = tgt_y - fp_y;
    let rng = dx.hypot(dy);

    // Grid azimuth: clockwise from north (+y) toward east (+x).
    let mut az_deg = dx.atan2(dy) * 180.0 / PI;
    if az_deg < 0.0 {
        az_deg += 360.0;
    }

    let mut sol = FireSolution {
        weapon_system: weapon,
        distance_m: rng.round() as i64,
        azimuth_deg: (az_deg * 10.0).round() / 10.0,
        azimuth_mils: (az_deg * MILS_PER_CIRCLE / 360.0).round() as i64,
        elevation_mils: 0,
        charge: 0,
        time_of_flight_s: 0.0,
    };

    for (ch, &v) in charges.iter().enumerate() {
        let k = rng * GRAVITY / (v * v); // = sin(2θ)
        if k <= 1.0 {
            // High-angle (mortar) root: 2θ = 180° − arcsin(k).
            let theta = (PI - k.asin()) / 2.0;
            sol.charge = ch as i64;
            sol.elevation_mils = (theta * 180.0 / PI * MILS_PER_CIRCLE / 360.0).round() as i64;
            sol.time_of_flight_s = (2.0 * v * theta.sin() / GRAVITY * 10.0).round() / 10.0;
            return (sol, true);
        }
    }
    (sol, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_distance_and_high_angle() {
        let (sol, ok) = solve_fire_mission("M252 81mm", 0.0, 0.0, 0.0, 1000.0);
        assert!(ok);
        assert_eq!(sol.distance_m, 1000);
        assert_eq!(sol.weapon_system, "M252 81mm");
        // Due north → azimuth 0.
        assert!((sol.azimuth_deg - 0.0).abs() < 0.05);
        assert!(sol.elevation_mils > 800); // high-angle
    }

    #[test]
    fn azimuth_cardinals() {
        let east = solve_fire_mission("M252 81mm", 0.0, 0.0, 1000.0, 0.0).0;
        assert!((east.azimuth_deg - 90.0).abs() < 0.05);
        let south = solve_fire_mission("M252 81mm", 0.0, 0.0, 0.0, -1000.0).0;
        assert!((south.azimuth_deg - 180.0).abs() < 0.05);
        let west = solve_fire_mission("M252 81mm", 0.0, 0.0, -1000.0, 0.0).0;
        assert!((west.azimuth_deg - 270.0).abs() < 0.05);
    }

    #[test]
    fn lower_charge_for_shorter_range() {
        let near = solve_fire_mission("M252 81mm", 0.0, 0.0, 0.0, 300.0).0;
        let far = solve_fire_mission("M252 81mm", 0.0, 0.0, 0.0, 2000.0).0;
        assert!(near.charge <= far.charge);
    }

    #[test]
    fn out_of_range_returns_false() {
        let (_, ok) = solve_fire_mission("M252 81mm", 0.0, 0.0, 0.0, 100_000.0);
        assert!(!ok);
    }

    #[test]
    fn unknown_weapon_falls_back() {
        let (sol, ok) = solve_fire_mission("Potato Launcher", 0.0, 0.0, 0.0, 1000.0);
        assert!(ok);
        assert_eq!(sol.weapon_system, DEFAULT_MORTAR);
    }
}
