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

/// Why a fire mission produced no firing solution.
///
/// **The two variants are deliberately distinct, and callers must keep them distinct (T-365).**
/// `handlers/field_tools.rs` maps `UnknownWeapon` → **400** and `OutOfRange` → **422**, and checks
/// the weapon *first*: pre-T-349, a misspelled weapon aimed beyond the substituted tube's reach was
/// answered "target out of range" — a range verdict for a weapon the caller never named, about a
/// target that may be well inside the range of the one they did. Collapsing these into one generic
/// error silently regresses that fix, so do not merge them.
///
/// Here the ordering is **structural rather than conventional**: an unknown weapon never reaches
/// the charge loop, so there is no range verdict in existence to report first. That is the main
/// reason this is a `Result` and not a `bool` plus a separate `is_known_weapon` predicate — the
/// latter leaves "solve without checking" spellable, and for four months that spelling silently
/// returned another tube's numbers.
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
    /// are zero because no charge reaches. `field_tools.rs` serialises this into the 422 response's
    /// `details`, so the payload is on the wire — do not drop the field to slim the enum.
    #[error("target out of range for every charge of '{}' at {} m", .0.weapon_system, .0.distance_m)]
    OutOfRange(FireSolution),
}

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
/// game-world meters, x=east, y=north). Selects the lowest charge that can reach.
///
/// **Refuses an unknown weapon; it does not substitute one (T-365).** This function used to answer
/// a weapon it had no table for by silently computing with `DEFAULT_MORTAR`'s charges and labelling
/// the result with that weapon's name — so an absent, misspelled or padded `weapon_system` did not
/// fail, it returned a complete and confident firing solution *for a different tube*. Measured on
/// the pre-fix code at FP (0,0) → TGT (0,3000):
///
/// | request | returned as | charge | elevation | TOF |
/// |---|---|---|---|---|
/// | `"M120 120mm"` | `M120 120mm` | 2 | **1300 mils** | 44.9 s |
/// | `"M120 120mmm"` (one typo) | `M252 81mm` | 3 | **1228 mils** | 40.0 s |
/// | `"M120 120mm "` (one trailing space) | `M252 81mm` | 3 | **1228 mils** | 40.0 s |
/// | `""` / field omitted | `M252 81mm` | 3 | **1228 mils** | 40.0 s |
///
/// A 120mm crew that mistyped its own tube was handed the 81mm elevation — **72 mils** low and 4.9
/// seconds early — on an HTTP 200. `2B14 82mm` at 2000 m is 988 mils against the substitute's 1061,
/// a 73-mil error the same way. For a mortar calculator that is not a data-quality nit; it is a
/// round landing somewhere nobody aimed.
///
/// **The substitution also hid a shipped client bug for the feature's entire life.**
/// `frontend/src/mortar.rs` sent `"m252_81mm"`, which was never a key of [`charges_for`] (it wants
/// `"M252 81mm"`), so every request from the live Mortar Calculator took the fallback path — and it
/// looked correct the whole time only because the fallback happened to be the weapon that page
/// hardcodes in its own header. A fallback that can mask its own caller being wrong for months is
/// not a safety net; it is the reason nobody found out. T-349 fixed that one-line client key.
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
mod tests {
    use super::*;

    /// Every known-weapon assertion in this module is a *ballistics* assertion. This is a mortar
    /// calculator: a change that shifts a correct solution by one mil is worse than the bug T-365
    /// fixed, because the bug at least announced itself with the wrong weapon's name in the
    /// response. `solved` exists so that reading the solution can never be confused with checking
    /// whether there is one.
    fn solved(weapon: &str, fp_x: f64, fp_y: f64, tgt_x: f64, tgt_y: f64) -> FireSolution {
        match solve_fire_mission(weapon, fp_x, fp_y, tgt_x, tgt_y) {
            Ok(sol) => sol,
            Err(e) => panic!("expected a solution for a known weapon in range, got {e:?}"),
        }
    }

    #[test]
    fn solves_distance_and_high_angle() {
        let sol = solved("M252 81mm", 0.0, 0.0, 0.0, 1000.0);
        assert_eq!(sol.distance_m, 1000);
        assert_eq!(sol.weapon_system, "M252 81mm");
        // Due north → azimuth 0.
        assert!((sol.azimuth_deg - 0.0).abs() < 0.05);
        assert!(sol.elevation_mils > 800); // high-angle
    }

    #[test]
    fn azimuth_cardinals() {
        let east = solved("M252 81mm", 0.0, 0.0, 1000.0, 0.0);
        assert!((east.azimuth_deg - 90.0).abs() < 0.05);
        let south = solved("M252 81mm", 0.0, 0.0, 0.0, -1000.0);
        assert!((south.azimuth_deg - 180.0).abs() < 0.05);
        let west = solved("M252 81mm", 0.0, 0.0, -1000.0, 0.0);
        assert!((west.azimuth_deg - 270.0).abs() < 0.05);
    }

    #[test]
    fn lower_charge_for_shorter_range() {
        let near = solved("M252 81mm", 0.0, 0.0, 0.0, 300.0);
        let far = solved("M252 81mm", 0.0, 0.0, 0.0, 2000.0);
        assert!(near.charge <= far.charge);
    }

    /// Each tube's own numbers, pinned. These are the values the T-365 substitution was handing out
    /// for *other* weapons, so pinning them is what makes a future regression loud: the 120mm/81mm
    /// pair below is exactly the 1300-vs-1228 confusion, and if the two ever agree again something
    /// has gone very wrong.
    #[test]
    fn per_weapon_solutions_are_pinned() {
        let m120 = solved("M120 120mm", 0.0, 0.0, 0.0, 3000.0);
        assert_eq!((m120.charge, m120.elevation_mils), (2, 1300));
        assert_eq!(m120.time_of_flight_s, 44.9);

        let m252 = solved("M252 81mm", 0.0, 0.0, 0.0, 3000.0);
        assert_eq!((m252.charge, m252.elevation_mils), (3, 1228));
        assert_eq!(m252.time_of_flight_s, 40.0);

        // 72 mils and 4.9 s apart at the same range — the measured cost of the old substitution.
        assert_eq!(m120.elevation_mils - m252.elevation_mils, 72);

        let b2000 = solved("2B14 82mm", 0.0, 0.0, 0.0, 2000.0);
        assert_eq!((b2000.charge, b2000.elevation_mils), (2, 988));
        let m2000 = solved("M252 81mm", 0.0, 0.0, 0.0, 2000.0);
        assert_eq!(m2000.elevation_mils, 1061); // the 73-mil error, the same way
    }

    #[test]
    fn out_of_range_reports_out_of_range_with_the_partial_solution() {
        match solve_fire_mission("M252 81mm", 0.0, 0.0, 0.0, 100_000.0) {
            // The partial solution is not decoration: `field_tools.rs` serialises it into the 422
            // response's `details`, so distance/azimuth must survive the error path.
            Err(SolveError::OutOfRange(sol)) => {
                assert_eq!(sol.weapon_system, "M252 81mm");
                assert_eq!(sol.distance_m, 100_000);
                assert_eq!(
                    (sol.charge, sol.elevation_mils, sol.time_of_flight_s),
                    (0, 0, 0.0)
                );
            }
            other => panic!("expected OutOfRange, got {other:?}"),
        }
    }

    /// **Rewritten from `unknown_weapon_falls_back` (T-365).** That test asserted the substitution
    /// as *intended behaviour* — `assert_eq!(sol.weapon_system, DEFAULT_MORTAR)` for a weapon the
    /// caller never named — which is why removing the fallback was this file's decision to make and
    /// not the HTTP layer's. The intent changed; the test records the change rather than vanishing
    /// with it. What was asserted as correct is now asserted to be refused.
    #[test]
    fn unknown_weapon_is_refused_not_substituted() {
        // The T-349 field report, verbatim: a typo, a trailing space, the empty string, and the key
        // the live SPA had been sending since the feature shipped. Every one of these previously
        // returned charge 3 / 1228 mils / 40.0 s labelled `M252 81mm`.
        for req in [
            "Potato Launcher",
            "M120 120mmm",
            "M120 120mm ",
            " M120 120mm",
            "",
            "m252_81mm",
            "m120 120mm",
            "M252 81MM",
        ] {
            match solve_fire_mission(req, 0.0, 0.0, 0.0, 3000.0) {
                Err(SolveError::UnknownWeapon(w)) => assert_eq!(w, req, "reported verbatim"),
                other => panic!("expected UnknownWeapon for {req:?}, got {other:?}"),
            }
        }
    }

    /// The unknown-weapon verdict must beat the out-of-range one. `field_tools.rs` maps them to 400
    /// and 422 respectively and T-349 fixed the ordering; here it is structural, since an unknown
    /// weapon has no charge table and so never reaches the range loop. A single generic error, or an
    /// `OutOfRange` computed against some substituted tube, would regress that fix.
    #[test]
    fn unknown_weapon_beats_out_of_range() {
        match solve_fire_mission("M120 120mmm", 0.0, 0.0, 0.0, 100_000.0) {
            Err(SolveError::UnknownWeapon(w)) => assert_eq!(w, "M120 120mmm"),
            other => panic!("expected UnknownWeapon to win over OutOfRange, got {other:?}"),
        }
    }

    /// Both aliases of the 81mm share one table, and the response echoes the tube that was *asked
    /// for* — `"M821 81mm"` must not come back relabelled `"M252 81mm"` just because they compute
    /// identically. Substituting a name is the whole defect, even when the numbers agree.
    #[test]
    fn aliases_keep_their_own_name() {
        let m821 = solved("M821 81mm", 0.0, 0.0, 0.0, 3000.0);
        let m252 = solved("M252 81mm", 0.0, 0.0, 0.0, 3000.0);
        assert_eq!(m821.weapon_system, "M821 81mm");
        assert_eq!(
            (m821.charge, m821.elevation_mils, m821.time_of_flight_s),
            (m252.charge, m252.elevation_mils, m252.time_of_flight_s),
        );
    }
}
