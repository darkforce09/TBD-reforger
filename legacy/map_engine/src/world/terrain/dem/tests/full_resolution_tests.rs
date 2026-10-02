//! Unit tests for the full-resolution elevation raster: its frame, its encoding, its refusals,
//! and its agreement with the box-averaged vector grid.

use crate::world::terrain::dem::full_resolution::*;
use crate::world::terrain::dem::grid::{downsample_dem_grid, sample_grid_meters};
use crate::world::terrain::dem::sampling::uint16_to_meters;

fn footprint(extent: f64) -> RasterFootprint {
    RasterFootprint::from_world_bounds([0.0, 0.0, extent, extent])
}

/// A `width × height` raster whose sample value is `x_index · 10 + y_index · 1000`.
fn planar_samples(width: u32, height: u32) -> Vec<u16> {
    let mut out = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            out.push((x * 10 + y * 1000) as u16);
        }
    }
    out
}

fn unit_encoding() -> SampleEncoding {
    SampleEncoding {
        offset_m: -100.0,
        scale_m: 0.5,
    }
}

#[test]
fn corners_map_to_the_footprint_corners() {
    let dem = FullResolutionDem::new(planar_samples(3, 3), 3, 3, unit_encoding(), footprint(4.0))
        .expect("valid raster");
    assert_eq!(dem.height_at(0.0, 0.0), Some(-100.0));
    assert_eq!(dem.height_at(4.0, 0.0), Some(-100.0 + 20.0 * 0.5));
    assert_eq!(dem.height_at(0.0, 4.0), Some(-100.0 + 2000.0 * 0.5));
    assert_eq!(dem.height_at(4.0, 4.0), Some(-100.0 + 2020.0 * 0.5));
}

#[test]
fn interior_points_interpolate_bilinearly() {
    let dem = FullResolutionDem::new(
        vec![0, 100, 200, 300],
        2,
        2,
        unit_encoding(),
        footprint(2.0),
    )
    .expect("valid raster");
    let centre = dem.height_at(1.0, 1.0).expect("inside");
    assert!((centre - (-100.0 + 150.0 * 0.5)).abs() < 1e-12, "{centre}");
    let quarter = dem.height_at(0.5, 0.0).expect("inside");
    assert!((quarter - (-100.0 + 25.0 * 0.5)).abs() < 1e-12, "{quarter}");
}

#[test]
fn outside_or_non_finite_positions_are_refused() {
    let dem = FullResolutionDem::new(planar_samples(4, 4), 4, 4, unit_encoding(), footprint(6.0))
        .expect("valid raster");
    assert_eq!(dem.height_at(-0.001, 3.0), None);
    assert_eq!(dem.height_at(3.0, 6.001), None);
    assert_eq!(dem.height_at(f64::NAN, 1.0), None);
    assert_eq!(dem.height_at(1.0, f64::INFINITY), None);
}

#[test]
fn malformed_rasters_are_refused() {
    let enc = unit_encoding();
    assert!(FullResolutionDem::new(vec![0; 3], 2, 2, enc, footprint(1.0)).is_none());
    assert!(FullResolutionDem::new(vec![0; 2], 1, 2, enc, footprint(1.0)).is_none());
    assert!(FullResolutionDem::new(vec![0; 4], 2, 2, enc, footprint(0.0)).is_none());
    let nan = SampleEncoding {
        offset_m: f64::NAN,
        scale_m: 1.0,
    };
    assert!(FullResolutionDem::new(vec![0; 4], 2, 2, nan, footprint(1.0)).is_none());
    let bounds = RasterFootprint::from_world_bounds([0.0, 0.0, f64::INFINITY, 1.0]);
    assert!(FullResolutionDem::new(vec![0; 4], 2, 2, enc, bounds).is_none());
}

#[test]
fn linear_range_encoding_matches_the_png_export_formula() {
    let (min_m, max_m) = (-204.78, 375.53);
    let enc = SampleEncoding::linear_range(min_m, max_m);
    for sample in [0.0, 1.0, 12_345.0, 40_000.5, 65_535.0] {
        let expected = uint16_to_meters(sample, min_m, max_m);
        assert!((enc.metres(sample) - expected).abs() < 1e-9, "{sample}");
    }
}

/// The vector grid box-averages four-sample windows whose rounded edges put the window mean up
/// to one sample off the cell position, so the two sources agree to one sample of slope per
/// axis; a flipped or shifted axis would disagree by tens of metres on this gradient.
#[test]
fn heights_agree_with_the_vector_grid_on_a_planar_raster() {
    let (w, h, extent) = (64u32, 64u32, 126.0);
    let mut samples = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            samples.push((x + 2 * y) as u16);
        }
    }
    let enc = unit_encoding();
    let metres: Vec<f32> = samples
        .iter()
        .map(|&s| enc.metres(f64::from(s)) as f32)
        .collect();
    let grid = downsample_dem_grid(&metres, w as usize, h as usize, 4, extent, extent);
    let dem = FullResolutionDem::new(samples, w, h, enc, footprint(extent)).expect("valid raster");
    let one_sample_of_slope = 0.5 + 1.0 + 1e-6;
    for (x, y) in [
        (10.0, 10.0),
        (63.0, 40.5),
        (100.25, 90.75),
        (60.0, 120.0),
        (5.0, 110.0),
    ] {
        let fine = dem.height_at(x, y).expect("inside");
        let coarse = sample_grid_meters(&grid, x, y).expect("inside");
        assert!(
            (fine - coarse).abs() <= one_sample_of_slope,
            "({x}, {y}): full {fine} vs grid {coarse}"
        );
    }
}

#[test]
fn the_handle_reads_none_until_a_raster_is_published() {
    let handle = new_full_resolution_dem_handle();
    assert_eq!(height_from_handle(&handle, 1.0, 1.0), None);
    let dem = FullResolutionDem::new(vec![0, 0, 0, 0], 2, 2, unit_encoding(), footprint(2.0))
        .expect("valid raster");
    assert_eq!(dem.resident_bytes(), 8);
    *handle.borrow_mut() = Some(std::rc::Rc::new(dem));
    assert_eq!(height_from_handle(&handle, 1.0, 1.0), Some(-100.0));
}
