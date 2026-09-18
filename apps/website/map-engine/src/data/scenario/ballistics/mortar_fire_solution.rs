//! Role: mortar firing solutions — per-weapon charge tables and the high-angle solver.
//! Position: `data/scenario/ballistics` in the map engine's headless mission data domain.
//! Signals & state: explicit firing-position and target coordinates; no UI or graphics state.
//! Invariants: preserve the ballistic model, the per-weapon muzzle velocities, the rounding of
//! every reported quantity, and the wire representation of [`FireSolution`].

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

/// Why a fire mission produced no firing solution.
///
/// **The two variants are deliberately distinct, and callers must keep them distinct.** The HTTP
/// field-tools handler maps `UnknownWeapon` → **400** and `OutOfRange` → **422**, and checks the
/// weapon *first*: a misspelled weapon aimed beyond the reach of some substituted tube must not be
/// answered "target out of range", because that is a range verdict for a weapon the caller never
/// named, about a target that may be well inside the range of the one they did. Collapsing these
/// into one generic error destroys that distinction.
///
/// Here the ordering is **structural rather than conventional**: an unknown weapon never reaches
/// the charge loop, so there is no range verdict in existence to report first. That is the main
/// reason this is a `Result` and not a `bool` plus a separate `is_known_weapon` predicate — the
/// latter leaves "solve without checking" spellable, and that spelling returns another tube's
/// numbers while looking entirely successful.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SolveError {
    /// The requested weapon is not a key of [`charges_for`] — there is no muzzle-velocity table to
    /// compute against. Carries the weapon as requested, verbatim and un-canonicalised: `"M120
    /// 120mm "` is a weapon this API does not have, and guessing which one the caller meant is how
    /// 81mm numbers get computed for a 120mm tube.
    #[error("unknown weapon system '{0}'")]
    UnknownWeapon(String),
    /// Every charge in the weapon's table falls short of the target.
    ///
    /// Carries the **partial** solution — `weapon_system`, `distance_m`, `azimuth_deg` and
    /// `azimuth_mils` are computed and correct; `charge`, `elevation_mils` and `time_of_flight_s`
    /// are zero because no charge reaches. The HTTP field-tools handler serialises this into the
    /// 422 response's `details`, so the payload is on the wire — do not drop the field to slim the
    /// enum.
    #[error("target out of range for every charge of '{}' at {} m", .0.weapon_system, .0.distance_m)]
    OutOfRange(FireSolution),
}

const GRAVITY: f64 = 9.80665;
const MILS_PER_CIRCLE: f64 = 6400.0;

/// Per-ring muzzle velocities (m/s) for a simplified projectile model.
///
/// Keys match **exactly**: case, spacing and padding all count, so `"m252_81mm"`, `"M252 81MM"` and
/// `"M120 120mm "` name no weapon here. `"M252 81mm"` and `"M821 81mm"` share one table because
/// they share one tube, but each keeps its own name in the response.
fn charges_for(weapon: &str) -> Option<&'static [f64]> {
    match weapon {
        "M252 81mm" | "M821 81mm" => Some(&[70.0, 105.0, 150.0, 210.0, 270.0]),
        "2B14 82mm" => Some(&[65.0, 100.0, 145.0, 200.0, 255.0]),
        "M120 120mm" => Some(&[110.0, 170.0, 230.0, 318.0]),
        _ => None,
    }
}

/// Compute the high-angle solution from a firing position to a target (flat
/// game-world meters, x=east, y=north). Selects the lowest charge that can reach.
///
/// **Refuses an unknown weapon; it does not substitute one.** Every weapon this function knows has
/// its own muzzle velocities, so the same range produces materially different firing data per tube.
/// At FP (0,0) → TGT (0,3000):
///
/// | weapon | charge | elevation | TOF |
/// |---|---|---|---|
/// | `"M120 120mm"` | 2 | **1300 mils** | 44.9 s |
/// | `"M252 81mm"` | 3 | **1228 mils** | 40.0 s |
///
/// A 120mm crew handed the 81mm elevation is **72 mils** low and 4.9 seconds early; `2B14 82mm` at
/// 2000 m is 988 mils against the 81mm's 1061, a 73-mil error the same way. For a mortar calculator
/// that is not a data-quality nit; it is a round landing somewhere nobody aimed. So an absent,
/// misspelled or padded `weapon_system` is an error — never a fallback labelled with some other
/// tube's name, which would answer a wrong request with a confident HTTP 200 and hide a caller
/// sending the wrong key indefinitely.
///
/// Out of range is the other refusal: when no charge in the table reaches, the partial solution
/// travels back inside [`SolveError::OutOfRange`] rather than being reported as a solution.
pub fn solve_fire_mission(
    weapon: &str,
    fp_x: f64,
    fp_y: f64,
    tgt_x: f64,
    tgt_y: f64,
) -> Result<FireSolution, SolveError> {
    let charges =
        charges_for(weapon).ok_or_else(|| SolveError::UnknownWeapon(weapon.to_string()))?;

    let dx = tgt_x - fp_x;
    let dy = tgt_y - fp_y;
    let rng = dx.hypot(dy);

    // Grid azimuth: clockwise from north (+y) toward east (+x).
    let mut az_deg = dx.atan2(dy) * 180.0 / PI;
    if az_deg < 0.0 {
        az_deg += 360.0;
    }

    let mut sol = FireSolution {
        weapon_system: weapon.to_string(),
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
            return Ok(sol);
        }
    }
    Err(SolveError::OutOfRange(sol))
}

#[cfg(test)]
#[path = "tests/mortar_fire_solution.rs"]
mod tests;
