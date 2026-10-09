//! **Role:** unit tests of [`crate::water_export_images::raster_geometry`]: the meta defaults,
//! the resolution override, the region of interest's size, spacing and name suffix, and the
//! refusal of a size that is not a whole number of pixels.
//! **Position:** `src/tests` of `map_raster_pipeline`, declared by
//! `water_export_images/raster_geometry.rs`.
//! **Signals & state:** none; pure calls.
//! **Invariants:** every expected value is the geometry rule's arithmetic, worked by hand.

use serde_json::json;

use super::*;

#[test]
fn the_meta_grid_is_the_export_grid() {
    let meta = json!({"widthPx": 32, "heightPx": 16, "worldSizeM": 64, "planarResolutionM": 2});
    let geometry = water_raster_geometry(&meta, None, None).expect("geometry");
    assert_eq!((geometry.grid.width, geometry.grid.height), (32, 16));
    assert!(geometry.is_export_grid);
    assert_eq!(geometry.metres_per_pixel, 2.0);
    assert_eq!(geometry.grid.spacing_x, 64.0 / 31.0);
    assert_eq!(geometry.grid.spacing_z, 64.0 / 15.0);
    assert_eq!((geometry.grid.origin_x, geometry.grid.origin_z), (0.0, 0.0));
    assert_eq!(geometry.region_suffix, None);
}

#[test]
fn missing_null_and_zero_meta_sizes_default_to_12800() {
    let meta = json!({"widthPx": 0, "heightPx": null, "planarResolutionM": 0});
    let geometry = water_raster_geometry(&meta, None, None).expect("geometry");
    assert_eq!((geometry.grid.width, geometry.grid.height), (12800, 12800));
    assert_eq!(geometry.metres_per_pixel, 1.0);
    assert!(geometry.is_export_grid);
}

#[test]
fn a_resolution_makes_a_square_of_the_world_and_leaves_the_export_grid() {
    let meta = json!({"widthPx": 100, "heightPx": 100, "worldSizeM": 100});
    let geometry = water_raster_geometry(&meta, Some(0.4), None).expect("geometry");
    assert_eq!((geometry.grid.width, geometry.grid.height), (250, 250));
    assert!(!geometry.is_export_grid);

    let same = water_raster_geometry(&meta, Some(1.0), None).expect("geometry");
    assert!(same.is_export_grid, "the meta's own size reads the grids");
}

#[test]
fn a_region_takes_its_extent_at_the_resolution_with_a_floor_of_16_pixels() {
    let meta =
        json!({"widthPx": 12800, "heightPx": 12800, "worldSizeM": 12800, "planarResolutionM": 1});
    let geometry =
        water_raster_geometry(&meta, None, Some([4000.0, 5500.0, 5500.0, 7000.0])).expect("roi");
    assert_eq!((geometry.grid.width, geometry.grid.height), (1500, 1500));
    assert_eq!(geometry.grid.spacing_x, 1500.0 / 1499.0);
    assert_eq!(
        (geometry.grid.origin_x, geometry.grid.origin_z),
        (4000.0, 5500.0)
    );
    assert_eq!(geometry.region_suffix.as_deref(), Some("-roi-4000_5500"));
    assert!(!geometry.is_export_grid);

    let small = water_raster_geometry(&meta, None, Some([0.0, 0.0, 3.0, 2.5])).expect("small");
    assert_eq!((small.grid.width, small.grid.height), (16, 16));
}

#[test]
fn a_region_suffix_rounds_ties_up_and_writes_minus_zero_as_zero() {
    let meta = json!({"planarResolutionM": 1});
    let geometry =
        water_raster_geometry(&meta, None, Some([-0.4, -2.5, 100.0, 100.0])).expect("roi");
    assert_eq!(geometry.region_suffix.as_deref(), Some("-roi-0_-2"));
    let geometry =
        water_raster_geometry(&meta, None, Some([10.5, 7.49, 100.0, 100.0])).expect("roi");
    assert_eq!(geometry.region_suffix.as_deref(), Some("-roi-11_7"));
}

#[test]
fn a_region_side_of_half_a_pixel_rounds_up() {
    let meta = json!({"planarResolutionM": 2});
    let geometry = water_raster_geometry(&meta, None, Some([0.0, 0.0, 41.0, 43.0])).expect("roi");
    assert_eq!((geometry.grid.width, geometry.grid.height), (21, 22));
}

#[test]
fn a_fractional_meta_size_is_refused() {
    let meta = json!({"widthPx": 12.5, "heightPx": 12});
    let error = water_raster_geometry(&meta, None, None).expect_err("fraction");
    assert!(
        error.to_string().contains("width would be 12.5 pixels"),
        "{error}"
    );
}

#[test]
fn a_resolution_that_rounds_the_world_to_no_pixel_is_refused() {
    let meta = json!({"worldSizeM": 10});
    assert!(water_raster_geometry(&meta, Some(100.0), None).is_err());
}
