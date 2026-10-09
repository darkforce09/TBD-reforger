//! **Role:** unit tests of [`crate::water_export_images::dem_sampling`]: the nearest-sample pick
//! with its half-up ties and clamping, the sample-to-metres scale, and the refusal of a PNG that
//! is not 16-bit.
//! **Position:** `src/tests` of `map_raster_pipeline`, declared by
//! `water_export_images/dem_sampling.rs`.
//! **Signals & state:** each case writes one PNG under the system temporary folder and removes it.
//! **Invariants:** the heightfield is 3 × 2 samples of distinct values, so every pick names its
//! sample.

use std::path::PathBuf;

use super::*;
use crate::image_operations::png_writing::{PngPixelLayout, write_png_rows};

/// The 3 × 2 heightfield's samples, row 0 first.
const SAMPLES: [[u16; 3]; 2] = [[0, 100, 200], [300, 400, 65535]];

/// A PNG under the system temporary folder for `case`.
fn temporary_png(case: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "tbd-water-dem-{case}-{}-{:?}.png",
        std::process::id(),
        std::thread::current().id()
    ))
}

/// Writes [`SAMPLES`] as a 16-bit grey PNG and loads it back.
fn loaded_heightfield(case: &str) -> DemHeightfield {
    let path = temporary_png(case);
    write_png_rows(&path, 3, 2, PngPixelLayout::Gray16, |row_index, row| {
        for (pixel, sample) in row.chunks_exact_mut(2).zip(SAMPLES[row_index]) {
            pixel.copy_from_slice(&sample.to_be_bytes());
        }
    })
    .expect("write the heightfield");
    let heightfield = DemHeightfield::load(&path).expect("load the heightfield");
    std::fs::remove_file(&path).ok();
    heightfield
}

/// The elevation in metres of sample value `value`.
fn metres_of(value: u16) -> f64 {
    DEM_MINIMUM_ELEVATION_M
        + (f64::from(value) / 65535.0) * (DEM_MAXIMUM_ELEVATION_M - DEM_MINIMUM_ELEVATION_M)
}

#[test]
fn the_extreme_sample_values_are_the_elevation_range() {
    let heightfield = loaded_heightfield("range");
    assert_eq!(heightfield.size(), (3, 2));
    assert_eq!(heightfield.elevation_m(0.0, 0.0), -204.781);
    assert_eq!(heightfield.elevation_m(4.0, 2.0), metres_of(65535));
    assert!((heightfield.elevation_m(4.0, 2.0) - 375.531).abs() < 1e-9);
}

#[test]
fn a_point_takes_the_nearest_sample_two_metres_apart_with_ties_up() {
    let heightfield = loaded_heightfield("nearest");
    assert_eq!(
        heightfield.elevation_m(2.9, 0.0),
        metres_of(100),
        "1.45 rounds to 1"
    );
    assert_eq!(
        heightfield.elevation_m(3.0, 0.0),
        metres_of(200),
        "1.5 rounds to 2"
    );
    assert_eq!(
        heightfield.elevation_m(1.0, 1.0),
        metres_of(400),
        "0.5 rounds to 1 on both axes"
    );
}

#[test]
fn a_point_off_the_heightfield_clamps_to_its_edge() {
    let heightfield = loaded_heightfield("clamp");
    assert_eq!(heightfield.elevation_m(-50.0, 100.0), metres_of(300));
    assert_eq!(heightfield.elevation_m(1.0e9, -3.0), metres_of(200));
    assert_eq!(
        heightfield.elevation_m(-0.9, 0.0),
        metres_of(0),
        "-0.45 rounds to -0"
    );
}

#[test]
fn a_nan_point_reads_sample_value_zero() {
    let heightfield = loaded_heightfield("nan");
    assert_eq!(heightfield.elevation_m(f64::NAN, 2.0), metres_of(0));
}

#[test]
fn an_eight_bit_png_is_refused() {
    let path = temporary_png("eight-bit");
    write_png_rows(&path, 2, 2, PngPixelLayout::Gray8, |_, row| row.fill(7))
        .expect("write the PNG");
    let error = DemHeightfield::load(&path).expect_err("8-bit");
    std::fs::remove_file(&path).ok();
    assert!(error.to_string().contains("decode the DEM"), "{error}");
}
