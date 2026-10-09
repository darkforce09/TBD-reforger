//! **Role:** unit tests of [`crate::water_export_images::polygon_rasterization`]: the ring field
//! fallback, the skip of rings too short or not arrays, the depth fields' defaults, and the
//! heightfield depth rule.
//! **Position:** `src/tests` of `map_raster_pipeline`, declared by
//! `water_export_images/polygon_rasterization.rs`.
//! **Signals & state:** the heightfield cases write one PNG under the system temporary folder and
//! remove it.
//! **Invariants:** the grid is 5 × 5 samples one metre apart from the origin, and the square ring
//! `(0, 0)–(4, 4)` covers rows 0 to 3 (the top edge's row has no crossing) and every column.

use std::path::Path;

use grid_rasterization::prelude::SampleGrid;
use serde_json::json;

use super::*;
use crate::image_operations::png_writing::{PngPixelLayout, write_png_rows};

/// The square ring `(0, 0)–(4, 4)` as `[x, y, z]` points.
fn square() -> serde_json::Value {
    json!([[0, 0, 0], [4, 0, 0], [4, 0, 4], [0, 0, 4]])
}

/// An all-land raster on the 5 × 5 grid.
fn raster() -> WaterRaster {
    WaterRaster::new(SampleGrid {
        origin_x: 0.0,
        origin_z: 0.0,
        spacing_x: 1.0,
        spacing_z: 1.0,
        width: 5,
        height: 5,
    })
}

/// A one-sample heightfield of sample value 0 (−204.781 m).
fn lowest_heightfield() -> DemHeightfield {
    let path = std::env::temp_dir().join(format!(
        "tbd-water-polygon-dem-{}-{:?}.png",
        std::process::id(),
        std::thread::current().id()
    ));
    write_png_rows(&path, 1, 1, PngPixelLayout::Gray16, |_, row| row.fill(0))
        .expect("write the heightfield");
    let heightfield = DemHeightfield::load(Path::new(&path)).expect("load the heightfield");
    std::fs::remove_file(&path).ok();
    heightfield
}

#[test]
fn the_square_fills_rows_zero_to_three_with_the_class_and_default_depth() {
    let mut raster = raster();
    let drawn = fill_water_polygons(&json!([{ "polygon": square() }]), 2, None, &mut raster);
    assert_eq!(drawn, 1);
    for row in 0..5 {
        for column in 0..5 {
            let index = raster.index(column, row);
            let expected = if row < 4 { (2, 15) } else { (0, 0) };
            assert_eq!(
                (raster.mask[index], raster.depth_decimetres[index]),
                expected,
                "sample ({column}, {row})"
            );
        }
    }
}

#[test]
fn an_absent_polygon_falls_back_to_the_perimeter_but_a_short_one_does_not() {
    let mut raster = raster();
    let items = json!([
        { "polygon": null, "perimeter": square() },
        { "polygon": [[0, 0, 0], [1, 0, 0]], "perimeter": square() },
        { "points": {} },
        7,
    ]);
    assert_eq!(fill_water_polygons(&items, 2, None, &mut raster), 1);
    assert_eq!(
        fill_water_polygons(&json!({"lakes": []}), 2, None, &mut raster),
        0
    );
}

#[test]
fn the_depth_falls_back_from_the_average_to_six_tenths_of_the_maximum() {
    let mut raster = raster();
    fill_water_polygons(
        &json!([{ "maxDepthM": 2, "polygon": square() }]),
        2,
        None,
        &mut raster,
    );
    assert_eq!(raster.depth_decimetres[0], 12);
    fill_water_polygons(
        &json!([{ "avgDepthM": 0.04, "polygon": square() }]),
        2,
        None,
        &mut raster,
    );
    assert_eq!(
        raster.depth_decimetres[0], 12,
        "a shallower depth never replaces a deeper one"
    );
}

#[test]
fn a_zero_average_depth_counts_and_stores_nothing() {
    let mut raster = raster();
    fill_water_polygons(
        &json!([{ "avgDepthM": 0, "maxDepthM": 9, "polygon": square() }]),
        2,
        None,
        &mut raster,
    );
    assert_eq!(raster.mask[0], 2);
    assert_eq!(raster.depth_decimetres[0], 0);
}

#[test]
fn a_heightfield_below_the_surface_gives_the_capped_water_column() {
    let heightfield = lowest_heightfield();
    let mut raster = raster();
    let item =
        json!([{ "surfaceElevationYM": 2, "avgDepthM": 1, "maxDepthM": 4, "polygon": square() }]);
    fill_water_polygons(&item, 2, Some(&heightfield), &mut raster);
    assert_eq!(
        raster.depth_decimetres[0], 40,
        "206.781 m capped at maxDepthM"
    );

    let mut raster = self::raster();
    let item = json!([{ "surfaceElevationYM": -204.7, "avgDepthM": 1, "polygon": square() }]);
    fill_water_polygons(&item, 2, Some(&heightfield), &mut raster);
    assert_eq!(
        raster.depth_decimetres[0], 1,
        "0.081 m raised to the 0.1 m floor"
    );
}

#[test]
fn a_heightfield_above_the_surface_keeps_the_default_depth() {
    let heightfield = lowest_heightfield();
    let mut raster = raster();
    let item = json!([{ "surfaceElevationYM": -300, "avgDepthM": 2.5, "polygon": square() }]);
    fill_water_polygons(&item, 2, Some(&heightfield), &mut raster);
    assert_eq!(raster.depth_decimetres[0], 25);
}
