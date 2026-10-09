//! **Role:** unit tests of [`crate::catmull_rom::evaluate_uniform_catmull_rom`]: the segment ends, the
//! start tangent, straight lines, and zero and NaN plan tangents.
//! **Position:** `src/tests` of `grid_rasterization`, declared by `catmull_rom.rs`.
//! **Signals & state:** none; pure functions over literal control points.
//! **Invariants:** every expected value is written out by hand, never derived from the code under
//! test.

use super::evaluate_uniform_catmull_rom;

const P0: [f64; 3] = [0.0, 1.0, 0.0];
const P1: [f64; 3] = [2.0, 3.0, 4.0];
const P2: [f64; 3] = [6.0, -1.0, 8.0];
const P3: [f64; 3] = [10.0, 0.0, 6.0];

#[test]
fn the_ends_of_the_segment_are_the_middle_control_points() {
    assert_eq!(
        evaluate_uniform_catmull_rom(P0, P1, P2, P3, 0.0).position,
        P1
    );
    assert_eq!(
        evaluate_uniform_catmull_rom(P0, P1, P2, P3, 1.0).position,
        P2
    );
}

#[test]
fn the_start_tangent_follows_the_chord_across_the_neighbours() {
    let sample = evaluate_uniform_catmull_rom(P0, P1, P2, P3, 0.0);
    // The start velocity is (p2 − p0) / 2 = (3, −1, 4); its plan length is 5.
    assert_eq!(sample.tangent, [0.6, 0.0, 0.8]);
    assert_eq!(sample.normal, [-0.8, 0.0, 0.6]);
}

#[test]
fn a_straight_line_has_a_constant_direction_and_a_left_normal() {
    let line = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [3.0, 0.0, 0.0],
    ];
    for t in [0.0, 0.25, 0.5, 1.0] {
        let sample = evaluate_uniform_catmull_rom(line[0], line[1], line[2], line[3], t);
        assert_eq!(sample.position, [1.0 + t, 0.0, 0.0]);
        assert_eq!(sample.tangent, [1.0, 0.0, 0.0]);
        assert_eq!(sample.normal, [0.0, 0.0, 1.0]);
    }
}

#[test]
fn a_zero_plan_tangent_divides_by_one() {
    let point = [5.0, 2.0, -3.0];
    let sample = evaluate_uniform_catmull_rom(point, point, point, point, 0.5);
    assert_eq!(sample.position, point);
    assert_eq!(sample.tangent, [0.0, 0.0, 0.0]);
    assert_eq!(sample.normal, [0.0, 0.0, 0.0]);

    // A purely vertical curve also has no plan direction.
    let rising = evaluate_uniform_catmull_rom(
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 2.0, 0.0],
        [0.0, 3.0, 0.0],
        0.5,
    );
    assert_eq!(rising.position, [0.0, 1.5, 0.0]);
    assert_eq!(rising.tangent, [0.0, 0.0, 0.0]);
}

#[test]
fn a_nan_plan_tangent_divides_by_one() {
    let sample = evaluate_uniform_catmull_rom(
        [f64::NAN, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        0.5,
    );
    assert!(sample.tangent[0].is_nan());
    assert_eq!(sample.tangent[2], 0.0);
}
