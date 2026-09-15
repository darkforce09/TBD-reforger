//! Role: shaping tests.
//! Position: `camera/math/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::camera::math::shaping::round;

#[test]
fn matches_js_math_round() {
    assert_eq!(round(2.5), 3.0);
    assert_eq!(round(-2.5), -2.0);
    assert_eq!(round(0.5), 1.0);
    assert_eq!(round(-0.5), 0.0);
    assert_eq!(round(2.4), 2.0);
    assert_eq!(round(2.6), 3.0);
}
