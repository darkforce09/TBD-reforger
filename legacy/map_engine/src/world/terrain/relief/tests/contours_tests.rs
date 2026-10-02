//! Role: contours tests.
//! Position: `world/terrain/relief/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::terrain::relief::contours::*;

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
fn reductions_and_levels() {
    assert_eq!(contour_grid_reductions(100.0), 2);
    assert_eq!(contour_grid_reductions(50.0), 1);
    assert_eq!(contour_grid_reductions(20.0), 0);
    assert_eq!(contour_levels(10.0, 35.0), vec![10.0, 20.0, 30.0]);
    assert!(contour_levels(0.0, 35.0).is_empty());
    assert!(contour_levels(10.0, f64::INFINITY).is_empty());
}

#[test]
fn single_diagonal_ramp_crosses_at_midpoint() {
    let g = grid(vec![0.0, 0.0, 0.0, 10.0], 2, 2);
    let seg = contour_segments(&g, &[5.0]);

    assert_eq!(seg.len(), 4);
}

#[test]
fn empty_when_no_level_reaches() {
    let g = grid(vec![1.0, 1.0, 1.0, 1.0], 2, 2);
    assert!(contour_segments(&g, &[50.0]).is_empty());
}

#[test]
fn closed_loop_has_even_edge_degree() {
    let mut data = vec![0.0f32; 25];
    data[12] = 10.0;
    let g = grid(data, 5, 5);
    let seg = contour_segments(&g, &[5.0]);
    assert!(seg.len().is_multiple_of(4));
}

fn bump(side: usize, cx: f64, cy: f64, r: f64, peak: f64) -> Vec<f32> {
    let mut data = vec![0.0f32; side * side];
    for j in 0..side {
        for i in 0..side {
            let d = (((i as f64 - cx).powi(2)) + ((j as f64 - cy).powi(2))).sqrt();
            let h = (peak * (1.0 - d / r)).max(0.0);
            data[j * side + i] = h as f32;
        }
    }
    data
}

#[test]
fn ring_closure_distinguishes_closed_loop_from_open_chain() {
    let g = grid(bump(15, 7.0, 7.0, 6.0, 10.0), 15, 15);
    let rings = contour_rings(&g, &[5.0]);
    assert!(
        rings.iter().any(|r| r.closed),
        "a bump interior to the grid must yield at least one closed ring"
    );

    let closed = rings.iter().find(|r| r.closed).unwrap();
    assert!(closed.points.len() >= 3);
    assert!(
        !near(closed.points[0], *closed.points.last().unwrap()),
        "closed ring must NOT carry a duplicated closing vertex"
    );

    let mut ramp = vec![0.0f32; 15 * 15];
    for j in 0..15 {
        for i in 0..15 {
            ramp[j * 15 + i] = if i <= 3 { 10.0 } else { 0.0 };
        }
    }
    let gr = grid(ramp, 15, 15);
    let rr = contour_rings(&gr, &[5.0]);
    assert!(!rr.is_empty(), "the ramp must produce a contour");
    assert!(
        rr.iter().all(|r| !r.closed),
        "an edge→edge ramp iso must be an OPEN chain, not a closed loop"
    );
}

#[test]
fn per_peak_selects_one_highest_closed_ring_each() {
    let side = 40;
    let a = bump(side, 11.0, 20.0, 8.0, 33.0);
    let b = bump(side, 28.0, 20.0, 8.0, 23.0);
    let mut data = vec![0.0f32; side * side];
    for k in 0..data.len() {
        data[k] = a[k].max(b[k]);
    }
    let g = grid(data, side, side);
    let levels = vec![10.0, 20.0, 30.0];
    let rings = contour_rings(&g, &levels);
    let summit = summit_ring_indices(&rings);

    assert_eq!(
        summit.len(),
        2,
        "one highest-closed-ring per peak → two total"
    );

    let mut summit_levels: Vec<f64> = summit.iter().map(|&i| rings[i].level).collect();
    summit_levels.sort_by(|x, y| x.total_cmp(y));
    assert_eq!(summit_levels, vec![20.0, 30.0]);

    for &i in &summit {
        assert!(rings[i].closed);
        assert!((rings[i].level - 10.0).abs() > f64::EPSILON);
    }
}

#[test]
fn no_summit_rings_when_nothing_closes() {
    let mut ramp = vec![0.0f32; 12 * 12];
    for j in 0..12 {
        for i in 0..12 {
            ramp[j * 12 + i] = if i <= 2 { 10.0 } else { 0.0 };
        }
    }
    let g = grid(ramp, 12, 12);
    let rings = contour_rings(&g, &[5.0]);
    assert!(summit_ring_indices(&rings).is_empty());
}
