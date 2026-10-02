//! The guards on the solution panel's wording: the battery summary, the dispersion caveat, the
//! crest line and the fuze card.

use super::battery_rows::battery_row_text;
use super::crest_warning::crest_text;
use super::dispersion_card::{dispersion_lines, DISPERSION_CAVEAT};
use super::fuze_card::fuze_lines;
use crate::v2::pages::field_tools::mortar::solve_bridge::{laid_rings, mils_and_degrees};
use crate::v2::pages::field_tools::mortar::test_mission::solved_mission;
use map_engine::data::scenario::ballistics::crest_clearance::CrestClearance;
use map_engine::data::scenario::ballistics::fire_mission::{FireMissionFuze, FuzeBurstAim};
use map_engine::data::scenario::ballistics::fuze::FuzeRefusal;
use map_engine::data::scenario::ballistics::solver::SolutionRefusal;

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
fn a_gun_that_cannot_lay_its_charge_says_why_and_shows_no_figures() {
    let solved = solved_mission();
    let mut gun = solved.solution.guns[0].clone();
    let pinned = gun.charges[0].rings;
    gun.charges[0].refusal = Some(SolutionRefusal::OutOfRange);
    let line = battery_row_text(&gun, Some(pinned));
    assert_eq!(line.elevation, "out of range");
    assert!(line.aim_azimuth.is_empty() && line.time_of_flight.is_empty());

    gun.recommended_rings = None;
    let none = battery_row_text(&gun, None);
    assert_eq!(none.charge, "No charge solves");
    assert!(none.elevation.is_empty());
}

#[test]
fn the_dispersion_is_labelled_an_interpretation_not_verified_in_engine() {
    assert_eq!(DISPERSION_CAVEAT, "Interpretation, not verified in-engine");
    let solved = solved_mission();
    let d = solved
        .solution
        .dispersion
        .expect("the lead gun has a dispersion");
    assert!(!d.verified_in_engine);
    let lines = dispersion_lines(&d);
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains(&format!("{:.1} m", d.range_probable_error_m)));
    assert!(lines[1].contains(&format!("{:.1} ×", 2.0 * d.ellipse_semi_major_m)));
    assert!(lines[2].contains(&format!("{:.1} m", d.standard_dispersion_m)));
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

#[test]
fn the_fuze_card_gives_the_time_or_the_refusal_and_the_burst_lay() {
    let solved = solved_mission();
    let fuze = solved
        .solution
        .fuze
        .expect("the fixture mission sets a fuze");
    let lines = fuze_lines(&fuze);
    match fuze.time_s {
        Some(time) => assert!(lines[0].contains(&format!("Fuze {time:.1} s")), "{lines:?}"),
        None => assert!(lines[0].starts_with("No fuze time"), "{lines:?}"),
    }
    assert!(lines[1].starts_with("Fuze window"));

    let refused = FireMissionFuze {
        burst_height_m: 500.0,
        time_s: None,
        min_s: 10.0,
        max_s: 40.0,
        default_s: 24.0,
        refusal: Some(FuzeRefusal::OutsideFuzeWindow),
        burst_aim: None,
    };
    let lines = fuze_lines(&refused);
    assert_eq!(
        lines[0],
        "No fuze time: the burst falls outside the fuze window"
    );
    assert_eq!(lines[1], "Fuze window 10.0–40.0 s, default 24.0 s");
    assert_eq!(lines.len(), 2, "no burst lay without a fuze time");
    for (refusal, cause) in [
        (FuzeRefusal::AboveApex, "above the apex"),
        (FuzeRefusal::BeyondRange, "beyond maximum range"),
        (
            FuzeRefusal::InsideMinimumRange,
            "inside the minimum range the elevation limit sets",
        ),
        (
            FuzeRefusal::DidNotConverge,
            "the elevation search does not converge",
        ),
        (
            FuzeRefusal::TimeToLiveExceeded,
            "the shell's lifetime ends first",
        ),
        (FuzeRefusal::InvalidInput, "an input is invalid"),
    ] {
        let unreached = FireMissionFuze {
            refusal: Some(refusal),
            ..refused
        };
        assert_eq!(
            fuze_lines(&unreached)[0],
            format!(
                "No fuze time: no charge reaches the burst point ({cause} at the lowest charge)"
            )
        );
    }

    let set = FireMissionFuze {
        time_s: Some(21.26),
        refusal: None,
        burst_aim: Some(FuzeBurstAim {
            rings: 2,
            aim_azimuth_deg: 60.0,
            aim_azimuth_mils: 1066.7,
            elevation_deg: 70.0,
            elevation_mils: 1244.4,
        }),
        ..refused
    };
    let lines = fuze_lines(&set);
    assert_eq!(lines[0], "Fuze 21.3 s for a burst 500 m above the target");
    assert_eq!(
        lines[2],
        "Burst lay: charge 2 · elevation 1244.4 mils · 70.00° · aim azimuth 1066.7 mils · 60.00°"
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
