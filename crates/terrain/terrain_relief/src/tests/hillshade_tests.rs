//! **Role:** unit tests of the hillshade: decimation, flat ground and the sun constants.
//! **Position:** test-only child of [`crate::hillshade`].
//! **Signals & state:** none; pure functions.
//! **Invariants:** flat ground shades one uniform grey.

use crate::hillshade::*;

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
