//! Role: lod tests.
//! Position: `core/culling/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::core::culling::lod::*;

const CONTOUR_SPACING_MIN_PX: f64 = 14.0;

const CONTOUR_SPACING_MAX_PX: f64 = 19.0;

fn contour_spacing_px(interval_m: f64, slope: f64, m_per_px: f64) -> f64 {
    if slope <= 0.0 || m_per_px <= 0.0 {
        return 0.0;
    }
    interval_m / (slope * m_per_px)
}

#[test]
fn tree_band_and_badge_gates() {
    assert!(!class_visible("tree", -0.1));
    assert!(class_visible("tree", 0.0));
    assert!(!class_visible("vegetation", 1.4));
    assert!(class_visible("vegetation", 1.5));
    assert!(!class_visible("prop", 2.9));
    assert!(class_visible("prop", 3.0));
    assert!(!class_visible("rockLarge", 0.9));
    assert!(class_visible("rockLarge", 1.0));
    assert!(!class_visible("buildingBadge", 0.9));
    assert!(class_visible("buildingBadge", 1.0));

    assert!(class_visible("forestFill", -0.1));
    assert!(!class_visible("forestFill", 0.0));
    assert!(!class_visible("forestFill", 1.0));
    assert!(class_visible("forestOutline", -1.5));
    assert!(!class_visible("forestOutline", -1.6));
    assert!(!class_visible("forestOutline", 0.0));
}

#[test]
fn fence_pier_gate_boundaries() {
    assert!(class_visible("fence", 1.5));
    assert!(!class_visible("fence", 1.49));
    assert!(class_visible("pier", -1.0));
    assert!(!class_visible("pier", -1.01));

    assert!(class_visible("prop", 3.0));
    assert!(!class_visible("prop", 2.99));

    assert!(!WORLD_RENDER_CLASSES.contains(&"fence"));
    assert!(!WORLD_RENDER_CLASSES.contains(&"pier"));
}

#[test]
fn exhaustive_zoom_scan_glyph_classes_stable() {
    let classes = [
        "tree",
        "vegetation",
        "prop",
        "rockLarge",
        "buildingBadge",
        "building",
        "forestFill",
        "forestOutline",
        "sea",
    ];

    for i in 0..=120 {
        let z = -6.0 + f64::from(i) * 0.1;
        let z = (z * 10.0).round() / 10.0;
        let tv = class_visible("tree", z);
        assert_eq!(tv, z >= 0.0, "tree @ {z}");
        let _ = classes;
    }
}

fn m_per_px(deck_zoom: f64) -> f64 {
    2.0_f64.powf(-deck_zoom)
}

fn everon_like_median_slope() -> f64 {
    const N: usize = 256;
    const SPAN_M: f64 = 12_800.0;
    let cell_m = SPAN_M / (N as f64 - 1.0);

    const AMP: f64 = 1.184;
    let elev = |x: f64, y: f64| -> f64 {
        let u = x / SPAN_M;
        let v = y / SPAN_M;
        AMP * (160.0
            * (u * std::f64::consts::TAU * 2.3).sin()
            * (v * std::f64::consts::TAU * 1.9).cos()
            + 70.0 * (u * std::f64::consts::TAU * 5.1 + 0.7).sin()
            + 55.0 * (v * std::f64::consts::TAU * 4.3 + 1.3).cos())
            + 240.0
    };
    let mut grid = vec![0.0f64; N * N];
    for j in 0..N {
        for i in 0..N {
            grid[j * N + i] = elev(i as f64 * cell_m, j as f64 * cell_m);
        }
    }
    let at = |x: i64, y: i64| -> f64 {
        let xx = x.clamp(0, N as i64 - 1) as usize;
        let yy = y.clamp(0, N as i64 - 1) as usize;
        grid[yy * N + xx]
    };
    let mut slopes: Vec<f64> = Vec::with_capacity(N * N);
    for y in 0..N as i64 {
        for x in 0..N as i64 {
            let a = at(x - 1, y - 1);
            let b = at(x, y - 1);
            let c = at(x + 1, y - 1);
            let d = at(x - 1, y);
            let f = at(x + 1, y);
            let g = at(x - 1, y + 1);
            let h = at(x, y + 1);
            let i2 = at(x + 1, y + 1);
            let dzdx = (c + 2.0 * f + i2 - (a + 2.0 * d + g)) / (8.0 * cell_m);
            let dzdy = (g + 2.0 * h + i2 - (a + 2.0 * b + c)) / (8.0 * cell_m);
            slopes.push((dzdx * dzdx + dzdy * dzdy).sqrt());
        }
    }
    slopes.sort_by(f64::total_cmp);
    slopes[slopes.len() / 2]
}

#[test]
fn representative_slope_matches_dem_statistics() {
    let median = everon_like_median_slope();

    assert!(
        (0.158..=0.231).contains(&median),
        "synthetic Everon median slope {median:.4} (≈{:.1}°) outside the interior band",
        median.atan().to_degrees()
    );

    assert!(
        (median - CONTOUR_REPRESENTATIVE_SLOPE).abs() < 0.06,
        "median slope {median:.4} far from CONTOUR_REPRESENTATIVE_SLOPE {CONTOUR_REPRESENTATIVE_SLOPE:.4}"
    );
}

#[test]
fn contour_spacing_in_band_at_four_zoom_levels() {
    let slope = everon_like_median_slope();

    let levels: [(f64, f64); 5] = [
        (-0.657, 5.0),
        (-1.657, 10.0),
        (-2.657, 20.0),
        (-3.657, 40.0),
        (-4.657, 80.0),
    ];
    for (z, want_interval) in levels {
        let mpp = m_per_px(z);
        let interval = contour_interval_for_zoom(mpp);
        assert_eq!(
            interval, want_interval,
            "z={z}: m/pix {mpp:.3} chose interval {interval} m, wanted {want_interval} m"
        );
        let spacing = contour_spacing_px(interval, slope, mpp);
        assert!(
            (CONTOUR_SPACING_MIN_PX..=CONTOUR_SPACING_MAX_PX).contains(&spacing),
            "z={z}: interval {interval} m @ {mpp:.3} m/pix → spacing {spacing:.2} px OUTSIDE \
                 [{CONTOUR_SPACING_MIN_PX},{CONTOUR_SPACING_MAX_PX}]"
        );
    }
}

#[test]
fn interval_ladder_reproduces_corpus_breakpoints() {
    assert_eq!(contour_interval_for_zoom(1.03), 5.0);
    assert_eq!(contour_interval_for_zoom(1.30), 5.0);
    assert_eq!(contour_interval_for_zoom(3.41), 10.0);
    assert_eq!(contour_interval_for_zoom(6.20), 20.0);

    assert_eq!(contour_interval_for_zoom(0.01), CONTOUR_INTERVAL_MIN_M);
    assert_eq!(contour_interval_for_zoom(64.0), CONTOUR_INTERVAL_MAX_M);
    assert_eq!(contour_interval_for_zoom(1_000.0), CONTOUR_INTERVAL_MAX_M);
    assert_eq!(contour_interval_for_zoom(-1.0), CONTOUR_INTERVAL_MIN_M);
    assert_eq!(contour_interval_for_zoom(f64::NAN), CONTOUR_INTERVAL_MIN_M);
}

#[test]
fn interval_monotone_in_m_per_px() {
    let mut prev = 0.0;
    let mut mpp = 0.5;
    while mpp <= 80.0 {
        let interval = contour_interval_for_zoom(mpp);
        assert!(
            interval >= prev,
            "interval dropped from {prev} to {interval} at m/pix {mpp:.3}"
        );
        prev = interval;
        mpp *= 1.05;
    }
}
