//! The guards on the solution panel: the battery summary's figures and the crest line.

use super::battery_rows::battery_row_text;
use super::crest_warning::crest_text;
use crate::mortar::solve_bridge::{laid_rings, mils_and_degrees};
use crate::mortar::test_mission::solved_mission;
use ballistics_solver::crest_clearance::CrestClearance;

/// Each gun's line is its laid charge's own figures, in the weapon's mils and degrees.
#[test]
fn the_battery_line_is_the_laid_charge_in_mils_and_degrees() {
    let solved = solved_mission();
    for gun in &solved.solution.guns {
        let line = battery_row_text(gun, None);
        let rings = laid_rings(gun, None).expect("the fixture guns solve");
        let row = gun.charges.iter().find(|c| c.rings == rings).unwrap();
        assert_eq!(line.label, gun.label);
        assert_eq!(line.charge, format!("Charge {rings}"));
        assert_eq!(
            line.elevation,
            mils_and_degrees(row.elevation_mils.unwrap(), row.elevation_deg.unwrap())
        );
        assert_eq!(
            line.aim_azimuth,
            mils_and_degrees(row.aim_azimuth_mils.unwrap(), row.aim_azimuth_deg.unwrap())
        );
        assert_eq!(
            line.time_of_flight,
            format!("{:.1} s", row.time_of_flight_s.unwrap())
        );
    }
}

#[test]
fn the_crest_line_warns_only_over_a_blocking_sample() {
    let clear = CrestClearance {
        min_clearance_m: 12.34,
        min_clearance_downrange_m: 420.0,
        first_blocking_downrange_m: None,
    };
    let (warns, text) = crest_text(Some(&clear));
    assert!(!warns);
    assert!(text.contains("12.3 m at 420 m"), "{text}");
    let blocked = CrestClearance {
        min_clearance_m: -8.0,
        min_clearance_downrange_m: 390.0,
        first_blocking_downrange_m: Some(355.0),
    };
    let (warns, text) = crest_text(Some(&blocked));
    assert!(warns);
    assert!(text.contains("355 m from the lead gun"), "{text}");
    let (warns, text) = crest_text(None);
    assert!(!warns);
    assert!(
        text.starts_with("No crest check"),
        "no profile is never shown as clear: {text}"
    );
}

/// The panel rounds each figure to its shown digit and never truncates: 1179.86 mils reads
/// 1179.9, 66.366° reads 66.37°, 24.26 s reads 24.3 s.
#[test]
fn the_battery_line_rounds_its_figures_to_the_nearest_shown_digit() {
    let solved = solved_mission();
    let mut gun = solved.solution.guns[0].clone();
    let rings = laid_rings(&gun, None).expect("the fixture guns solve");
    let row = gun.charges.iter_mut().find(|c| c.rings == rings).unwrap();
    row.elevation_mils = Some(1_179.86);
    row.elevation_deg = Some(66.366);
    row.time_of_flight_s = Some(24.26);
    let line = battery_row_text(&gun, None);
    assert_eq!(line.elevation, "1179.9 mils · 66.37°");
    assert_eq!(line.time_of_flight, "24.3 s");
}
