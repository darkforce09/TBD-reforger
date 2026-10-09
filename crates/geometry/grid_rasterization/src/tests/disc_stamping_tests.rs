//! **Role:** unit tests of [`crate::disc_stamping`]: disc coverage, its feathered rim and visit order,
//! clipping, and the stamp centres along a segment.
//! **Position:** `src/tests` of `grid_rasterization`, declared by `disc_stamping.rs`.
//! **Signals & state:** none; pure functions over literal discs and segments.
//! **Invariants:** every expected value is written out by hand, never derived from the code under
//! test.

use super::{disc_coverage, segment_stamp_centres};

/// Every visit of one disc, in visit order.
fn visits(
    centre_x: f64,
    centre_y: f64,
    radius: f64,
    alpha: u8,
    size: usize,
) -> Vec<(usize, usize, u8)> {
    let mut seen = Vec::new();
    disc_coverage(centre_x, centre_y, radius, alpha, size, size, |x, y, a| {
        seen.push((x, y, a));
    });
    seen
}

#[test]
fn a_radius_two_disc_on_a_pixel_feathers_its_diagonals_and_drops_its_rim() {
    let seen = visits(5.0, 5.0, 2.0, 255, 11);
    // (2 − √2) / 0.75 × 255 = 199.17; the four pixels at distance 2 have zero coverage.
    assert_eq!(
        seen,
        vec![
            (4, 4, 199),
            (5, 4, 255),
            (6, 4, 199),
            (4, 5, 255),
            (5, 5, 255),
            (6, 5, 255),
            (4, 6, 199),
            (5, 6, 255),
            (6, 6, 199),
        ]
    );
}

#[test]
fn coverage_scales_with_the_stamp_alpha() {
    let seen = visits(5.0, 5.0, 2.0, 220, 11);
    // (2 − √2) / 0.75 × 220 = 171.83.
    assert_eq!(seen.first(), Some(&(4, 4, 172)));
    assert_eq!(seen.get(1), Some(&(5, 4, 220)));
}

#[test]
fn a_disc_is_clipped_to_the_canvas() {
    let seen = visits(0.0, 0.0, 2.0, 255, 11);
    assert_eq!(
        seen,
        vec![(0, 0, 255), (1, 0, 255), (0, 1, 255), (1, 1, 199)]
    );
}

#[test]
fn a_disc_off_the_canvas_or_nan_visits_nothing() {
    assert!(visits(-10.0, 5.0, 2.0, 255, 11).is_empty());
    assert!(visits(5.0, 30.0, 2.0, 255, 11).is_empty());
    assert!(visits(f64::NAN, 5.0, 2.0, 255, 11).is_empty());
    assert!(visits(5.0, 5.0, f64::NAN, 255, 11).is_empty());
    assert!(visits(5.0, 5.0, 2.0, 255, 0).is_empty());
}

#[test]
fn a_zero_alpha_stamp_visits_nothing() {
    assert!(visits(5.0, 5.0, 2.0, 0, 11).is_empty());
}

#[test]
fn a_segment_shorter_than_half_a_pixel_is_one_stamp() {
    assert_eq!(segment_stamp_centres(1.0, 2.0, 1.3, 2.0), vec![[1.0, 2.0]]);
}

#[test]
fn a_one_pixel_segment_takes_the_minimum_two_steps() {
    assert_eq!(
        segment_stamp_centres(0.0, 0.0, 1.0, 0.0),
        vec![[0.0, 0.0], [0.5, 0.0], [1.0, 0.0]]
    );
}

#[test]
fn a_long_segment_is_stamped_every_half_pixel() {
    let centres = segment_stamp_centres(0.0, 0.0, 0.0, 10.0);
    assert_eq!(centres.len(), 21);
    assert_eq!(centres.first(), Some(&[0.0, 0.0]));
    assert_eq!(centres.get(1), Some(&[0.0, 0.5]));
    assert_eq!(centres.last(), Some(&[0.0, 10.0]));
}

#[test]
fn a_non_finite_segment_has_no_stamp() {
    assert!(segment_stamp_centres(0.0, 0.0, f64::NAN, 0.0).is_empty());
    assert!(segment_stamp_centres(0.0, 0.0, f64::INFINITY, 0.0).is_empty());
}
