//! The surface size policy: JavaScript's rounding of `css × dpr`, at least one pixel, and a refusal
//! of every non-positive side or ratio.

use super::device_surface_size;
use crate::error::Error;

#[test]
fn each_side_is_the_css_size_times_the_ratio_rounded_half_up() {
    assert_eq!(device_surface_size(800.0, 600.0, 1.0), Ok((800, 600)));
    assert_eq!(device_surface_size(800.0, 600.0, 2.0), Ok((1600, 1200)));
    // 0.5 · 3 = 1.5 and 100.5 · 3 = 301.5 both round up, as `Math.round` does.
    assert_eq!(device_surface_size(0.5, 100.5, 3.0), Ok((2, 302)));
    assert_eq!(device_surface_size(333.3, 10.0, 1.5), Ok((500, 15)));
}

#[test]
fn a_side_below_half_a_pixel_still_gets_one_pixel() {
    assert_eq!(device_surface_size(0.1, 0.1, 1.0), Ok((1, 1)));
}

#[test]
fn a_non_positive_side_or_ratio_is_refused() {
    for (width, height, ratio) in [
        (0.0, 600.0, 1.0),
        (800.0, -1.0, 1.0),
        (800.0, 600.0, 0.0),
        (f64::NAN, 600.0, 1.0),
        (800.0, 600.0, f64::NAN),
    ] {
        let refused = device_surface_size(width, height, ratio);
        assert!(
            matches!(refused, Err(Error::NonPositiveResize { .. })),
            "{width}x{height} at {ratio}: {refused:?}"
        );
    }
}
