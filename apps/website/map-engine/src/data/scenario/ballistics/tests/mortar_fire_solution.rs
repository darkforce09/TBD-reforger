//! Role: mortar firing solution tests.
//! Position: `data/scenario/ballistics/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

/// Every known-weapon assertion in this module is a *ballistics* assertion. This is a mortar
/// calculator: a change that shifts a correct solution by one mil is worse than a solution labelled
/// with the wrong weapon, because the wrong label at least announces itself in the response.
/// `solved` exists so that reading the solution can never be confused with checking whether there
/// is one.
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

/// Each tube's own numbers, pinned. Pinning them is what makes a substitution regression loud: the
/// 120mm/81mm pair below is exactly the 1300-versus-1228 confusion, and if the two ever agree
/// something has gone very wrong.
#[test]
fn per_weapon_solutions_are_pinned() {
    let m120 = solved("M120 120mm", 0.0, 0.0, 0.0, 3000.0);
    assert_eq!((m120.charge, m120.elevation_mils), (2, 1300));
    assert_eq!(m120.time_of_flight_s, 44.9);

    let m252 = solved("M252 81mm", 0.0, 0.0, 0.0, 3000.0);
    assert_eq!((m252.charge, m252.elevation_mils), (3, 1228));
    assert_eq!(m252.time_of_flight_s, 40.0);

    // 72 mils and 4.9 s apart at the same range — the cost of answering one tube with the other's
    // numbers.
    assert_eq!(m120.elevation_mils - m252.elevation_mils, 72);

    let b2000 = solved("2B14 82mm", 0.0, 0.0, 0.0, 2000.0);
    assert_eq!((b2000.charge, b2000.elevation_mils), (2, 988));
    let m2000 = solved("M252 81mm", 0.0, 0.0, 0.0, 2000.0);
    assert_eq!(m2000.elevation_mils, 1061); // the 73-mil error, the same way
}

#[test]
fn out_of_range_reports_out_of_range_with_the_partial_solution() {
    match solve_fire_mission("M252 81mm", 0.0, 0.0, 0.0, 100_000.0) {
        // The partial solution is not decoration: the HTTP field-tools handler serialises it into
        // the 422 response's `details`, so distance/azimuth must survive the error path.
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

/// An unrecognised weapon is refused, never silently swapped for a tube that does have a charge
/// table. A near miss is still a miss: the solver has no way to know which weapon a caller meant.
#[test]
fn unknown_weapon_is_refused_not_substituted() {
    // A typo, a trailing space, a leading space, the empty string, and case or separator variants
    // of a real key. None of these names a weapon this solver has a table for.
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

/// The unknown-weapon verdict must beat the out-of-range one. The HTTP field-tools handler maps
/// them to 400 and 422 respectively; here the ordering is structural, since an unknown weapon has
/// no charge table and so never reaches the range loop. A single generic error, or an `OutOfRange`
/// computed against some substituted tube, would destroy that distinction.
#[test]
fn unknown_weapon_beats_out_of_range() {
    match solve_fire_mission("M120 120mmm", 0.0, 0.0, 0.0, 100_000.0) {
        Err(SolveError::UnknownWeapon(w)) => assert_eq!(w, "M120 120mmm"),
        other => panic!("expected UnknownWeapon to win over OutOfRange, got {other:?}"),
    }
}

/// Both aliases of the 81mm share one table, and the response echoes the tube that was *asked for*
/// — `"M821 81mm"` must not come back relabelled `"M252 81mm"` just because they compute
/// identically. Substituting a name is the defect even when the numbers agree.
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
