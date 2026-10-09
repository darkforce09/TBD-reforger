//! **Role:** unit tests of [`crate::bathymetry_palette`]: the ramps, their interpolation and
//! clamping, the contour lines and the mask byte classes.
//! **Position:** test-only child of [`crate::bathymetry_palette`].
//! **Signals & state:** none; pure functions.
//! **Invariants:** every expected colour and factor is written out by hand.

use crate::bathymetry_palette::*;

#[test]
fn a_midpoint_interpolates_each_channel_and_rounds_a_tie_up() {
    // Halfway from [160, 232, 242] to [110, 215, 235]: 135, 223.5 and 238.5.
    assert_eq!(
        interpolate_depth_stops(&OCEAN_DEPTH_STOPS, 1.0),
        [135, 224, 239]
    );
    // A quarter of the way from [30, 155, 140] to [16, 110, 100]: 26.5, 143.75 and 130.
    assert_eq!(
        interpolate_depth_stops(&LAKE_DEPTH_STOPS, 9.0),
        [27, 144, 130]
    );
}

#[test]
fn depths_beyond_the_ramp_clamp_to_its_ends() {
    assert_eq!(
        interpolate_depth_stops(&OCEAN_DEPTH_STOPS, -3.0),
        [160, 232, 242]
    );
    assert_eq!(
        interpolate_depth_stops(&OCEAN_DEPTH_STOPS, 500.0),
        [4, 16, 65]
    );
    assert_eq!(
        interpolate_depth_stops(&RIVER_DEPTH_STOPS, 30.0),
        [15, 85, 180]
    );
}

#[test]
fn a_nan_depth_takes_the_last_colour() {
    assert_eq!(
        interpolate_depth_stops(&OCEAN_DEPTH_STOPS, f64::NAN),
        [4, 16, 65]
    );
    assert_eq!(
        interpolate_depth_stops(&LAKE_DEPTH_STOPS, f64::NAN),
        [16, 110, 100]
    );
}

#[test]
fn the_sea_darkens_within_the_contour_half_width() {
    assert_eq!(contour_multiplier(5.0, WaterClass::Ocean), 0.78);
    assert_eq!(contour_multiplier(5.17, WaterClass::Ocean), 0.78);
    assert_eq!(contour_multiplier(4.83, WaterClass::Ocean), 0.78);
    assert_eq!(contour_multiplier(100.1, WaterClass::Ocean), 0.78);
    assert_eq!(contour_multiplier(0.1, WaterClass::Ocean), 0.78);
}

#[test]
fn every_water_mask_byte_but_lake_and_river_paints_as_sea() {
    let classes: Vec<WaterClass> = (0..=4).map(palette_class_for_mask_code).collect();
    assert_eq!(
        classes,
        vec![
            WaterClass::Ocean,
            WaterClass::Ocean,
            WaterClass::LakeOrPond,
            WaterClass::River,
            WaterClass::Ocean,
        ]
    );
}
