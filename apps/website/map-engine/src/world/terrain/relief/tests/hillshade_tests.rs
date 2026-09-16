//! Role: hillshade tests.
//! Position: `world/terrain/relief/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::terrain::relief::hillshade::*;

#[test]
fn dims_downsample() {
    let hs = build_hillshade_image(&vec![0f32; 8 * 8], 8, 8);
    assert_eq!((hs.w, hs.h), (8, 8));
    assert_eq!(hs.data.len(), 8 * 8 * 4);
}

#[test]
fn flat_grid_is_uniform_cos_zenith() {
    let hs = build_hillshade_image(&vec![10.0f32; 16 * 16], 16, 16);
    for px in hs.data.chunks_exact(4) {
        assert_eq!(px[0], 180);
        assert_eq!(px[1], 180);
        assert_eq!(px[2], 180);
        assert_eq!(px[3], 255);
    }
}

#[test]
fn constants_are_bit_identical_to_js() {
    assert_eq!(AZIMUTH_RAD, (315.0 * core::f64::consts::PI) / 180.0);
    assert_eq!(ZENITH_RAD, core::f64::consts::PI / 2.0 - ALTITUDE_RAD);
}
