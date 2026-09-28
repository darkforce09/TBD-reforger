//! Tests of the meteorological wind convention: direction, magnitude and refusals.

use super::*;

const TOLERANCE: f64 = 1e-12;

fn assert_vector(actual: [f64; 3], expected: [f64; 3]) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() <= TOLERANCE,
            "axis {axis}: got {actual:?}, expected {expected:?}"
        );
    }
}

#[test]
fn wind_from_north_moves_the_air_south() {
    let wind = Wind {
        speed_m_s: 5.0,
        from_deg: 0.0,
    };
    assert_vector(wind.air_velocity_m_s(), [0.0, -5.0, 0.0]);
}

#[test]
fn wind_from_east_moves_the_air_west() {
    let wind = Wind {
        speed_m_s: 4.0,
        from_deg: 90.0,
    };
    assert_vector(wind.air_velocity_m_s(), [-4.0, 0.0, 0.0]);
}

#[test]
fn wind_from_south_west_moves_the_air_north_east() {
    let wind = Wind {
        speed_m_s: 2.0,
        from_deg: 225.0,
    };
    let component = 2.0 * std::f64::consts::FRAC_1_SQRT_2;
    assert_vector(wind.air_velocity_m_s(), [component, component, 0.0]);
}

#[test]
fn air_speed_equals_the_reported_speed_and_stays_horizontal() {
    for from_deg in [0.0, 17.0, 133.5, 270.0, 359.9, -45.0, 720.0] {
        let wind = Wind {
            speed_m_s: 7.5,
            from_deg,
        };
        let [east, north, up] = wind.air_velocity_m_s();
        assert!(((east * east + north * north).sqrt() - 7.5).abs() <= TOLERANCE);
        assert_eq!(up, 0.0, "wind from {from_deg} has a vertical component");
    }
}

#[test]
fn calm_has_zero_air_velocity() {
    assert_eq!(Wind::CALM.air_velocity_m_s(), [0.0, 0.0, 0.0]);
    assert_eq!(Wind::CALM.validate(), Ok(()));
}

#[test]
fn negative_or_non_finite_speed_is_refused() {
    for speed_m_s in [-0.1, f64::NAN, f64::INFINITY] {
        let wind = Wind {
            speed_m_s,
            from_deg: 10.0,
        };
        assert!(
            matches!(wind.validate(), Err(WindError::InvalidSpeed(_))),
            "speed {speed_m_s} was accepted"
        );
    }
}

#[test]
fn non_finite_direction_is_refused() {
    for from_deg in [f64::NAN, f64::NEG_INFINITY] {
        let wind = Wind {
            speed_m_s: 3.0,
            from_deg,
        };
        assert!(
            matches!(wind.validate(), Err(WindError::InvalidDirection(_))),
            "direction {from_deg} was accepted"
        );
    }
}
