use super::*;

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
