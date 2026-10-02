//! Role: grid tests.
//! Position: `world/terrain/dem/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::terrain::dem::grid::*;

#[test]
fn sample_grid_constant_and_bounds() {
    let g = DemVectorGrid {
        data: vec![5.0f32; 16],
        cols: 4,
        rows: 4,
        cell_x: 10.0,
        cell_y: 10.0,
        origin_x: 0.0,
        origin_y: 0.0,
        max_elev_m: 5.0,
    };

    assert_eq!(sample_grid_meters(&g, 0.0, 0.0), Some(5.0));
    assert_eq!(sample_grid_meters(&g, 30.0, 30.0), Some(5.0));
    assert_eq!(sample_grid_meters(&g, 15.5, 22.3), Some(5.0));

    assert_eq!(sample_grid_meters(&g, -0.1, 0.0), None);
    assert_eq!(sample_grid_meters(&g, 0.0, 30.1), None);
}

#[test]
fn sample_grid_interpolates_linearly() {
    let g = DemVectorGrid {
        data: vec![0.0, 1.0, 2.0, 3.0, 0.0, 1.0, 2.0, 3.0],
        cols: 4,
        rows: 2,
        cell_x: 8.0,
        cell_y: 8.0,
        origin_x: 0.0,
        origin_y: 0.0,
        max_elev_m: 3.0,
    };
    let v = sample_grid_meters(&g, 12.0, 4.0).unwrap();
    assert!((v - 1.5).abs() < 1e-9);
}

#[test]
fn dims_and_factor() {
    assert_eq!(DEM_VECTOR_GRID_FACTOR, 4);
    assert_eq!(dem_grid_dims(6400, 6400, 4), (1600, 1600));
    assert_eq!(dem_grid_dims(1, 1, 4), (2, 2));
}

#[test]
fn factor_one_is_identity_with_endpoint_anchoring() {
    let data: Vec<f32> = (0..16).map(|v| v as f32).collect();
    let g = downsample_dem_grid(&data, 4, 4, 1, 12.0, 12.0);
    assert_eq!((g.cols, g.rows), (4, 4));
    assert_eq!(g.data, data);
    assert_eq!(g.cell_x, 4.0);
}

#[test]
fn box_average_of_constant_is_constant() {
    let data = vec![7.0f32; 64];
    let g = downsample_dem_grid(&data, 8, 8, 4, 100.0, 100.0);
    assert!(g.data.iter().all(|&v| (v - 7.0).abs() < 1e-6));
    assert!((g.max_elev_m - 7.0).abs() < 1e-9);
}

#[test]
fn reduce_2x_block_average() {
    let data: Vec<f32> = (0..16).map(|v| v as f32).collect();
    let g = DemVectorGrid {
        data,
        cols: 4,
        rows: 4,
        cell_x: 8.0,
        cell_y: 8.0,
        origin_x: 0.0,
        origin_y: 0.0,
        max_elev_m: 15.0,
    };
    let r = reduce_grid_2x(&g);
    assert_eq!((r.cols, r.rows), (2, 2));

    assert!((r.data[0] - 2.5).abs() < 1e-6);
    assert_eq!(r.cell_x, 16.0);
}
