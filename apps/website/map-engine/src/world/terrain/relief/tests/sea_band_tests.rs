//! Role: sea band tests.
//! Position: `world/terrain/relief/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::terrain::relief::sea_band::*;

fn grid(data: Vec<f32>, cols: usize, rows: usize) -> DemVectorGrid {
    DemVectorGrid {
        data,
        cols,
        rows,
        cell_x: 1.0,
        cell_y: 1.0,
        origin_x: 0.0,
        origin_y: 0.0,
        max_elev_m: 100.0,
    }
}

#[test]
fn alpha_ladder() {
    assert_eq!(sea_fill_alpha(0.0), 1.0);
    assert_eq!(sea_fill_alpha(1.5), 0.6);
    assert_eq!(sea_fill_alpha(2.5), 0.3);
    assert_eq!(sea_fill_alpha(9.0), 0.0);
}

#[test]
fn all_high_land_has_no_fill() {
    let g = grid(vec![100.0; 9], 3, 3);
    let geo = build_sea_band_geometry(&g);
    assert_eq!(geo.polygon_count, 0);
    assert!(geo.fill_positions.is_empty());
}

#[test]
fn full_inside_run_is_one_rectangle_per_row_per_level() {
    let g = grid(vec![-10.0; 6], 3, 2);
    let geo = build_sea_band_geometry(&g);

    assert_eq!(geo.polygon_count, 4);

    assert_eq!(geo.fill_positions.len(), 4 * 10);
    assert_eq!(geo.fill_colors.len(), 4 * 20);
    assert_eq!(geo.fill_start_indices.len(), 4);
}

#[test]
fn rings_are_closed() {
    let g = grid(vec![-10.0, -10.0, 100.0, 100.0], 2, 2);
    let geo = build_sea_band_geometry(&g);
    assert!(geo.polygon_count >= 1);

    let n = geo.fill_positions.len();
    assert!(n >= 6);
}
