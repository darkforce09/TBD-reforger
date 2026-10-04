//! Canvas backing-store sizing: rounding, device pixel ratio, and the one-pixel floor.

use super::device_size;

#[test]
fn rounds_half_up_at_the_device_pixel_ratio() {
    assert_eq!(device_size(800.0, 600.0, 1.0), (800, 600));
    assert_eq!(device_size(800.0, 600.0, 2.0), (1600, 1200));
    assert_eq!(device_size(100.25, 100.75, 2.0), (201, 202));
    assert_eq!(device_size(333.3, 333.2, 1.5), (500, 500));
}

#[test]
fn never_answers_a_zero_dimension() {
    assert_eq!(device_size(0.0, 0.0, 1.0), (1, 1));
    assert_eq!(device_size(-20.0, 0.2, 1.0), (1, 1));
}

#[test]
fn matches_the_engine_resize_rounding() {
    for (css, dpr) in [(640.4, 1.25), (1023.5, 1.0), (97.0, 3.0), (512.1, 1.75)] {
        let engine_rounding = ((css * dpr + 0.5_f64).floor().max(1.0)) as u32;
        assert_eq!(
            device_size(css, css, dpr).0,
            engine_rounding,
            "{css} @ {dpr}"
        );
    }
}
