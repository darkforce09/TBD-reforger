//! **Role:** Unit tests of the elevation model's sampling: stored value to metres, the metres
//! cache, world to pixel mapping, bilinear interpolation and the out-of-raster point.
//! **Position:** Test-only child of [`crate::sampling`]; reads
//! [`crate::manifest::DemManifest`].
//! **Signals & state:** none; pure functions.
//! **Invariants:** a stored 0 reads exactly the minimum height and 65 535 the maximum; the world
//! rectangle's corners map to the first and last pixel, mirrored when the manifest flips an axis.

use crate::manifest::DemManifest;
use crate::sampling::{
    bilinear_sample, meters_cache, sample_elevation_meters, uint16_to_meters, world_to_pixel,
};

const MIN_M: f64 = -204.78;

const MAX_M: f64 = 375.53;

fn everon(width_px: usize, height_px: usize) -> DemManifest {
    DemManifest {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 12800.0,
        max_y: 12800.0,
        width_px,
        height_px,
        flip_x: false,
        flip_z: false,
        height_min_m: MIN_M,
        height_max_m: MAX_M,
    }
}

#[test]
fn zero_is_exact_min() {
    assert_eq!(uint16_to_meters(0.0, MIN_M, MAX_M), MIN_M);
}

#[test]
fn full_scale_is_max_within_epsilon() {
    assert!((uint16_to_meters(65535.0, MIN_M, MAX_M) - MAX_M).abs() < 1e-10);
}

#[test]
fn meters_cache_matches_scalar_and_stores_f32() {
    let raster: [u16; 5] = [0, 65535, 12345, 54321, 1];
    let out = meters_cache(&raster, MIN_M, MAX_M);
    assert_eq!(out.len(), raster.len());
    for (i, &u) in raster.iter().enumerate() {
        assert_eq!(out[i], uint16_to_meters(f64::from(u), MIN_M, MAX_M) as f32);
    }
}

#[test]
fn world_to_pixel_endpoints() {
    let m = everon(6400, 6400);
    let a = world_to_pixel(0.0, 0.0, &m);
    assert_eq!((a.px, a.py), (0.0, 0.0));
    let b = world_to_pixel(12800.0, 12800.0, &m);
    assert_eq!((b.px, b.py), (6399.0, 6399.0));
}

#[test]
fn world_to_pixel_axis_flip() {
    let mut m = everon(6400, 6400);
    m.flip_x = true;
    m.flip_z = true;
    let a = world_to_pixel(0.0, 0.0, &m);
    assert_eq!((a.px, a.py), (6399.0, 6399.0));
}

#[test]
fn bilinear_2x2_center_is_mean() {
    let raster: [f32; 4] = [0.0, 100.0, 200.0, 300.0];
    let v = bilinear_sample(&raster, 2, 2, 0.5, 0.5);
    assert!((v - 150.0).abs() < 1e-9);
}

#[test]
fn bilinear_u16_and_f32_agree_when_exact() {
    let u: [u16; 4] = [0, 100, 200, 300];
    let f: [f32; 4] = [0.0, 100.0, 200.0, 300.0];
    for (px, py) in [(0.0, 0.0), (0.25, 0.75), (0.9, 0.1)] {
        assert_eq!(
            bilinear_sample(&u, 2, 2, px, py),
            bilinear_sample(&f, 2, 2, px, py)
        );
    }
}

#[test]
fn sample_elevation_out_of_bounds_is_none() {
    let m = everon(6400, 6400);
    let raster = vec![0u16; 64];
    assert!(sample_elevation_meters(-1.0, 0.0, &m, &raster, 6400, 6400).is_none());
}
