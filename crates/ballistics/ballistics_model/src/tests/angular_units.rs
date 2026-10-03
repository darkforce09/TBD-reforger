//! Tests of the mils conventions and azimuth normalisation.

use super::*;

const TOLERANCE: f64 = 1e-9;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "got {actual}, expected {expected}"
    );
}

fn convention_6000() -> MilsConvention {
    MilsConvention::new(6000).expect("6000 mils is a valid convention")
}

#[test]
fn zero_mils_per_circle_is_refused() {
    assert_eq!(
        MilsConvention::new(0),
        Err(AngularUnitsError::ZeroMilsPerCircle)
    );
}

#[test]
fn quarter_turn_converts_in_both_conventions() {
    let six_four = MilsConvention::MILS_6400;
    assert_eq!(six_four.mils_per_circle(), 6400);
    assert_close(six_four.degrees_to_mils(90.0), 1600.0);
    assert_close(six_four.radians_to_mils(TAU / 4.0), 1600.0);
    assert_close(six_four.mils_to_degrees(1600.0), 90.0);
    assert_close(six_four.mils_to_radians(1600.0), TAU / 4.0);

    let six_thousand = convention_6000();
    assert_close(six_thousand.degrees_to_mils(90.0), 1500.0);
    assert_close(six_thousand.radians_to_mils(TAU / 4.0), 1500.0);
    assert_close(six_thousand.mils_to_degrees(1500.0), 90.0);
}

#[test]
fn one_mil_6400_is_the_calibration_angle() {
    assert_close(MilsConvention::MILS_6400.mils_to_radians(1.0), TAU / 6400.0);
}

#[test]
fn conversions_round_trip() {
    for convention in [MilsConvention::MILS_6400, convention_6000()] {
        for degrees in [-725.5, -1.0, 0.0, 0.1, 45.0, 89.999, 359.9, 1234.5] {
            let mils = convention.degrees_to_mils(degrees);
            assert_close(convention.mils_to_degrees(mils), degrees);
            let radians = convention.mils_to_radians(mils);
            assert_close(radians_to_degrees(radians), degrees);
            assert_close(degrees_to_radians(degrees), radians);
        }
    }
}

#[test]
fn azimuth_mils_fold_into_one_turn() {
    let six_four = MilsConvention::MILS_6400;
    assert_close(six_four.normalise_azimuth_mils(-1.0), 6399.0);
    assert_close(six_four.normalise_azimuth_mils(6400.0), 0.0);
    assert_close(six_four.normalise_azimuth_mils(12_801.5), 1.5);
    assert_close(convention_6000().normalise_azimuth_mils(-3000.0), 3000.0);
}

#[test]
fn azimuth_radians_become_weapon_mils() {
    assert_close(
        MilsConvention::MILS_6400.azimuth_radians_to_mils(-TAU / 4.0),
        4800.0,
    );
    assert_close(convention_6000().azimuth_radians_to_mils(TAU / 2.0), 3000.0);
}

#[test]
fn normalised_azimuths_never_reach_the_turn_or_negative_zero() {
    let tiny_negative = -1e-20;
    for (value, turn) in [
        (normalise_azimuth_degrees(tiny_negative), 360.0),
        (normalise_azimuth_radians(tiny_negative), TAU),
        (
            MilsConvention::MILS_6400.normalise_azimuth_mils(tiny_negative),
            6400.0,
        ),
    ] {
        assert!((0.0..turn).contains(&value), "{value} escapes [0, {turn})");
    }
    for value in [
        normalise_azimuth_degrees(-0.0),
        normalise_azimuth_degrees(-360.0),
        normalise_azimuth_radians(-0.0),
    ] {
        assert!(
            value == 0.0 && value.is_sign_positive(),
            "{value} is not +0"
        );
    }
}

#[test]
fn azimuth_degrees_fold_into_one_turn() {
    assert_close(normalise_azimuth_degrees(-90.0), 270.0);
    assert_close(normalise_azimuth_degrees(720.25), 0.25);
    assert_close(normalise_azimuth_radians(-TAU / 2.0), TAU / 2.0);
}

#[test]
fn non_finite_azimuths_stay_not_a_number() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(normalise_azimuth_degrees(value).is_nan());
        assert!(
            MilsConvention::MILS_6400
                .normalise_azimuth_mils(value)
                .is_nan()
        );
    }
}
