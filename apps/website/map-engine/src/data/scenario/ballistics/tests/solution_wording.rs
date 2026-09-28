use super::*;

fn solved_row(rings: u32) -> ChargeSolution {
    ChargeSolution {
        rings,
        elevation_deg: Some(66.371_362_654),
        elevation_mils: Some(1_179.935_336_072),
        time_of_flight_s: Some(24.266_840_078),
        apex_m: Some(734.496_704_101_562_5),
        aim_azimuth_deg: Some(359.596),
        aim_azimuth_mils: Some(6_392.8),
        deflection_correction_mils: Some(-7.2),
        range_correction_m: Some(0.01),
        refusal: None,
    }
}

fn refused_row(rings: u32, refusal: SolutionRefusal) -> ChargeSolution {
    ChargeSolution {
        rings,
        elevation_deg: None,
        elevation_mils: None,
        time_of_flight_s: None,
        apex_m: None,
        aim_azimuth_deg: None,
        aim_azimuth_mils: None,
        deflection_correction_mils: None,
        range_correction_m: None,
        refusal: Some(refusal),
    }
}

fn gun(charges: Vec<ChargeSolution>, recommended_rings: Option<u32>) -> GunFireSolution {
    GunFireSolution {
        gun_index: 0,
        label: "Gun 1".to_string(),
        distance_m: 1_200.4,
        height_difference_m: 23.5,
        azimuth_deg: 0.0,
        azimuth_mils: 0.0,
        mils_per_circle: 6_400,
        charges,
        recommended_rings,
        dispersion: None,
    }
}

#[test]
fn figures_round_to_nearest_and_never_truncate() {
    assert_eq!(mils_and_degrees(1_179.86, 66.366), "1179.9 mils · 66.37°");
    assert_eq!(mils_and_degrees(1_179.84, 66.364), "1179.8 mils · 66.36°");
    assert_eq!(time_of_flight_text(24.26), "24.3 s");
    assert_eq!(time_of_flight_text(24.24), "24.2 s");
    assert_eq!(apex_text(1_150.56), "1151 m");
    assert_eq!(apex_text(1_150.44), "1150 m");
}

#[test]
fn an_exact_decimal_tie_rounds_to_the_even_digit() {
    assert_eq!(apex_text(0.5), "0 m");
    assert_eq!(apex_text(1.5), "2 m");
    assert_eq!(time_of_flight_text(0.25), "0.2 s");
    assert_eq!(time_of_flight_text(0.75), "0.8 s");
}

#[test]
fn corrections_carry_a_side_or_sign_only_above_the_dead_band() {
    assert_eq!(deflection_text(0.049), "0.0 mils");
    assert_eq!(deflection_text(-0.049), "0.0 mils");
    assert_eq!(deflection_text(4.26), "R 4.3 mils");
    assert_eq!(deflection_text(-4.26), "L 4.3 mils");
    assert_eq!(range_correction_text(0.049), "0.0 m");
    assert_eq!(range_correction_text(12.34), "+12.3 m");
    assert_eq!(range_correction_text(-12.36), "-12.4 m");
}

#[test]
fn a_solved_row_shows_every_figure() {
    let words = charge_row_words(&solved_row(2));
    assert_eq!(
        words,
        ChargeRowWords {
            charge: "Charge 2".to_string(),
            elevation: "1179.9 mils · 66.37°".to_string(),
            aim_azimuth: "6392.8 mils · 359.60°".to_string(),
            deflection: "L 7.2 mils".to_string(),
            range_correction: "0.0 m".to_string(),
            time_of_flight: "24.3 s".to_string(),
            apex: "734 m".to_string(),
        }
    );
}

#[test]
fn a_refused_or_incomplete_row_shows_why_and_no_figures() {
    let refused = charge_row_words(&refused_row(0, SolutionRefusal::OutOfRange));
    assert_eq!(refused.elevation, "out of range");
    assert!(refused.aim_azimuth.is_empty() && refused.apex.is_empty());
    let mut incomplete = solved_row(1);
    incomplete.apex_m = None;
    let words = charge_row_words(&incomplete);
    assert_eq!(words.elevation, NO_SOLUTION_TEXT);
    assert!(words.time_of_flight.is_empty());
}

#[test]
fn the_battery_line_lays_the_pinned_charge_else_the_recommendation() {
    let g = gun(
        vec![refused_row(1, SolutionRefusal::TooClose), solved_row(2)],
        Some(2),
    );
    let recommended = battery_line_words(&g, None);
    assert_eq!(recommended.charge, "Charge 2");
    assert_eq!(recommended.elevation, "1179.9 mils · 66.37°");
    assert_eq!(recommended.time_of_flight, "24.3 s");
    let pinned = battery_line_words(&g, Some(1));
    assert_eq!(pinned.charge, "Charge 1");
    assert_eq!(pinned.elevation, "too close for this charge");
    assert!(pinned.aim_azimuth.is_empty());
    let none = battery_line_words(
        &gun(vec![refused_row(1, SolutionRefusal::OutOfRange)], None),
        None,
    );
    assert_eq!(none.charge, "No charge solves");
}

#[test]
fn the_gun_heading_names_distance_line_and_height_difference() {
    let g = gun(Vec::new(), None);
    assert_eq!(
        gun_heading(&g),
        "Gun 1 — 1200 m · line 0.0 mils · 0.00° · Δh +23.5 m"
    );
}
